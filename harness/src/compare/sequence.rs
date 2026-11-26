use serde_json::{Value, json};
use soroban_env_host::{Error, Host, TryFromVal, xdr::ScVal};

use super::call::{Setup, call, deploy};
use super::ledger::{self, Entry};
use super::returns::{self, Answer};
use crate::wasm::{Contract, spec};

const STEP: usize = 5;
const NAME: usize = 15;
const VALUE: usize = 24;
const CHANGED: usize = 7;

/// How many times the full list of functions is called.
const ROUNDS: usize = 2;

/// One call on one side: what it answered, and the storage after it.
struct Step {
    value: Option<ScVal>,
    /// The error code, when the call failed.
    error: Option<Error>,
    answer: Answer,
    changed: bool,
    storage: Vec<Entry>,
}

/// One step on both sides.
struct Row {
    round: usize,
    func: String,
    sdk: Step,
    solang: Step,
    verdict: &'static str,
}

pub struct Data {
    no_spec: bool,
    skipped: Vec<String>,
    /// Why the sequence could not run on a side, if it could not.
    failed: Option<String>,
    rows: Vec<Row>,
    checked: usize,
    differ: usize,
    /// Steps that failed on both sides, with a different error on each.
    errors_differ: usize,
}

/// Deploy each build once and call every function in spec order on that one deploy,
/// then do the full round again. Every other report starts each call on a fresh deploy;
/// here each call sees what the calls before it wrote.
///
/// The two sides store data in different layouts, so their storage is never compared
/// as equal. What must agree at each step is the answer, and whether the call changed
/// the storage.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let iface = &sdk.interface;
    let (funcs, skipped): (Vec<_>, Vec<_>) = iface
        .funcs
        .iter()
        .partition(|f| spec::args_supported(iface, &f.inputs));
    let mut d = Data {
        no_spec: iface.funcs.is_empty(),
        skipped: skipped.iter().map(|f| spec::sig(f)).collect(),
        failed: None,
        rows: Vec::new(),
        checked: 0,
        differ: 0,
        errors_differ: 0,
    };

    let (s, l) = rayon::join(
        || run(sdk.wasm, iface, &funcs),
        || run(solang.wasm, iface, &funcs),
    );
    let (s, l) = match (s, l) {
        (Ok(s), Ok(l)) => (s, l),
        (Err(e), _) => {
            d.failed = Some(format!("sdk: {e}"));
            return d;
        }
        (_, Err(e)) => {
            d.failed = Some(format!("solang: {e}"));
            return d;
        }
    };

    let names = (0..ROUNDS).flat_map(|r| funcs.iter().map(move |f| (r + 1, f.name.clone())));
    for ((round, func), (sdk, solang)) in names.zip(s.into_iter().zip(l)) {
        let verdict = match (&sdk.value, &solang.value) {
            (Some(a), Some(b)) => {
                d.checked += 1;
                if a != b {
                    "DIFFER"
                } else if sdk.changed != solang.changed {
                    "changes differ"
                } else {
                    "same"
                }
            }
            (None, None) if sdk.error == solang.error => "both failed",
            (None, None) => {
                d.errors_differ += 1;
                "errors differ"
            }
            (None, Some(_)) => "sdk failed",
            (Some(_), None) => "solang failed",
        };
        if verdict == "DIFFER" || verdict == "changes differ" {
            d.differ += 1;
        }
        d.rows.push(Row {
            round,
            func,
            sdk,
            solang,
            verdict,
        });
    }
    d
}

/// Deploy `wasm` once and make every call of the sequence on it, in order.
fn run(wasm: &[u8], iface: &spec::Interface, funcs: &[&spec::FnSpec]) -> Result<Vec<Step>, String> {
    let text = |e| super::one_line(&format!("{e:?}"));
    let setup = Setup {
        cached: false,
        // As in `ledger`: a function guarded by `require_auth` still reaches its writes.
        authed: true,
    };
    let (host, contract) = deploy(wasm, setup).map_err(text)?;
    let mut before = ledger::entries(&host).map_err(text)?;

    let mut steps = Vec::new();
    for _ in 0..ROUNDS {
        for f in funcs {
            // Each call gets a full budget, as each would be its own transaction.
            host.budget_cloned().reset().map_err(text)?;

            let args =
                |h: &Host, addr| spec::build_args(h, addr, iface, &f.inputs, spec::Inputs::Small);
            let out = call(&host, contract, &f.name, args)
                .and_then(|(v, _)| Ok(ScVal::try_from_val(&host, &v)?));
            let storage = ledger::entries(&host).map_err(text)?;

            steps.push(Step {
                value: out.as_ref().ok().cloned(),
                error: out.as_ref().err().map(|e| e.error),
                answer: Answer::of(&out),
                changed: storage != before,
                storage: storage.clone(),
            });
            before = storage;
        }
    }
    Ok(steps)
}

pub fn text(name: &str, d: &Data) {
    super::banner("call sequence");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}`, nothing to invoke");
        println!();
        return;
    }
    if let Some(e) = &d.failed {
        println!("[err] {e}");
        println!();
        return;
    }

    if super::notes() {
        println!(
            "one deploy per side, every function in spec order, {ROUNDS} rounds, small inputs\n"
        );
    }
    println!(
        "{:<STEP$} {:<NAME$} {:<VALUE$} {:<VALUE$} {:<CHANGED$} verdict",
        "step", "function", "sdk", "solang", "changed"
    );
    println!("{}", "─".repeat(STEP + NAME + VALUE * 2 + CHANGED + 12));

    for (i, r) in d.rows.iter().enumerate() {
        if i > 0 && r.round != d.rows[i - 1].round {
            println!("{:<STEP$} round {}", "", r.round);
        }
        println!(
            "{:<STEP$} {:<NAME$} {:<VALUE$} {:<VALUE$} {:<CHANGED$} {}",
            i + 1,
            r.func,
            returns::show(&r.sdk.answer),
            returns::show(&r.solang.answer),
            changed(r),
            r.verdict
        );

        // On a step where the two do not agree, give both answers in full and the
        // storage each side holds after it.
        let (s, l) = (&r.sdk.answer, &r.solang.answer);
        let different_errors = r.verdict == "both failed" && s.full() != l.full();
        if r.verdict != "same" && r.verdict != "both failed" || different_errors {
            println!("{:>STEP$}  sdk    {}", "", s.full());
            println!("{:>STEP$}  solang {}", "", l.full());
            println!();
            ledger::print_side("sdk", &Ok(r.sdk.storage.clone()));
            ledger::print_side("solang", &Ok(r.solang.storage.clone()));
            println!();
        }
    }
    for sig in &d.skipped {
        println!("[skip] {sig}: unsupported arg types");
    }

    if super::notes() {
        println!();
        println!(
            "note: {} step(s) ran on both sides, {} disagreed, {} failed with different errors",
            d.checked, d.differ, d.errors_differ
        );
        // A failed call writes nothing, so after it the two sides hold different state.
        if d.rows
            .iter()
            .any(|r| matches!(r.verdict, "sdk failed" | "solang failed"))
        {
            println!("note: a step failed on one side, so later steps can differ because of it");
        }
    }
    println!();
}

/// Which sides changed their storage in this step.
fn changed(r: &Row) -> &'static str {
    match (r.sdk.changed, r.solang.changed) {
        (true, true) => "both",
        (true, false) => "sdk",
        (false, true) => "solang",
        (false, false) => "none",
    }
}

/// Each step with both answers, whether each side changed its storage, and the storage each side
/// holds after it.
pub fn json(d: &Data) -> Value {
    let side = |s: &Step| {
        let mut v = s.answer.json();
        v["changed"] = json!(s.changed);
        v["storage"] = json!(ledger::entries_json(&s.storage));
        v
    };
    let steps: Vec<Value> = d
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            json!({
                "step": i + 1,
                "round": r.round,
                "function": r.func,
                "sdk": side(&r.sdk),
                "solang": side(&r.solang),
                "verdict": r.verdict,
            })
        })
        .collect();

    json!({
        "no_spec": d.no_spec,
        "failed": d.failed,
        "rounds": ROUNDS,
        "steps": steps,
        "skipped": d.skipped,
        "checked": d.checked,
        "disagreed": d.differ,
        "errors_differ": d.errors_differ,
    })
}
