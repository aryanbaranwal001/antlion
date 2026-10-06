use serde_json::{Value, json};
use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol, TryFromVal, Val,
    testutils::{generate_account_id, generate_bytes_array},
    xdr::ScVal,
};

use crate::wasm::{Contract, spec};

const NAME: usize = 15;
const KIND: usize = 6;
const VALUE: usize = 24;

/// What one side answered, as one line: the value, or the error.
struct Answer {
    ok: bool,
    text: String,
}

impl Answer {
    fn of(v: &Result<ScVal, HostError>) -> Self {
        match v {
            Ok(val) => Answer {
                ok: true,
                text: super::one_line(&format!("{val:?}")),
            },
            Err(e) => Answer {
                ok: false,
                text: super::one_line(&format!("{e:?}")),
            },
        }
    }

    /// One value, on a single line, as the text prints it in full.
    fn full(&self) -> String {
        if self.ok {
            self.text.clone()
        } else {
            format!("[err] {}", self.text)
        }
    }

    fn json(&self) -> Value {
        if self.ok {
            json!({ "ok": self.text })
        } else {
            json!({ "err": self.text })
        }
    }
}

/// One table row, or a function skipped for its argument types.
enum Item {
    Unsupported(String),
    Row {
        func: String,
        input: &'static str,
        sdk: Answer,
        solang: Answer,
        verdict: &'static str,
    },
}

pub struct Data {
    no_spec: bool,
    items: Vec<Item>,
    checked: usize,
    differ: usize,
}

/// Invoke every function on both builds and compare what each one answers. Cost says how
/// much the two spent; this says whether they agree.
///
/// Each function runs twice. `small` uses values that ride inline in the `Val` word;
/// `edge` uses boundary values, including 64 and 128 bit numbers too large for the 56 bit
/// payload, which the host therefore passes as object handles.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let mut d = Data {
        no_spec: sdk.interface.funcs.is_empty(),
        items: Vec::new(),
        checked: 0,
        differ: 0,
    };

    for f in &sdk.interface.funcs {
        if !spec::args_supported(&sdk.interface, &f.inputs) {
            d.items.push(Item::Unsupported(f.name.clone()));
            continue;
        }

        for kind in spec::ALL_INPUTS {
            let args = |h: &Host, addr| spec::build_args(h, addr, &sdk.interface, &f.inputs, kind);
            let s = invoke(sdk.wasm, &f.name, false, args);
            let l = invoke(solang.wasm, &f.name, false, args);
            d.row(&f.name, kind.label(), &s, &l);
        }

        // Every other row runs with no authorization, so `require_auth` fails. This one
        // grants whatever is asked for, to see what each side does once it passes.
        if spec::takes_address(&f.inputs) {
            let args = |h: &Host, addr| {
                spec::build_args(h, addr, &sdk.interface, &f.inputs, spec::Inputs::Small)
            };
            let s = invoke(sdk.wasm, &f.name, true, args);
            let l = invoke(solang.wasm, &f.name, true, args);
            d.row(&f.name, "authed", &s, &l);
        }

        // A struct argument is also sent in shapes neither side should accept. Both
        // rejecting is the expected answer; one side accepting is the divergence.
        if spec::takes_struct(&f.inputs) {
            for m in spec::ALL_MALFORMED {
                let args =
                    |h: &Host, addr| spec::build_malformed(h, addr, &sdk.interface, &f.inputs, m);
                let s = invoke(sdk.wasm, &f.name, false, args);
                let l = invoke(solang.wasm, &f.name, false, args);
                d.row(&f.name, m.label(), &s, &l);
            }
        }

        // The first argument that is not a struct, sent as the wrong type. Soroban leaves
        // type checking to the contract, so this asks whether each side does it.
        if spec::takes_plain(&f.inputs) {
            let args = |h: &Host, addr| spec::build_wrong_arg(h, addr, &sdk.interface, &f.inputs);
            let s = invoke(sdk.wasm, &f.name, false, args);
            let l = invoke(solang.wasm, &f.name, false, args);
            d.row(&f.name, "badarg", &s, &l);
        }
    }
    d
}

impl Data {
    /// Record one row and tally it. Only calls that ran on both sides count as checked.
    fn row(
        &mut self,
        func: &str,
        input: &'static str,
        s: &Result<ScVal, HostError>,
        l: &Result<ScVal, HostError>,
    ) {
        let verdict = match (s, l) {
            (Ok(a), Ok(b)) => {
                self.checked += 1;
                if a == b {
                    "same"
                } else {
                    self.differ += 1;
                    "DIFFER"
                }
            }
            (Err(_), Err(_)) => "both failed",
            (Err(_), Ok(_)) => "sdk failed",
            (Ok(_), Err(_)) => "solang failed",
        };

        self.items.push(Item::Row {
            func: func.to_string(),
            input,
            sdk: Answer::of(s),
            solang: Answer::of(l),
            verdict,
        });
    }
}

pub fn text(name: &str, d: &Data) {
    super::banner("return value equivalence");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}` — nothing to invoke");
        println!();
        return;
    }

    println!(
        "{:<NAME$} {:<KIND$} {:<VALUE$} {:<VALUE$} {}",
        "function", "input", "sdk", "solang", "verdict"
    );
    println!("{}", "─".repeat(NAME + KIND + VALUE * 2 + 10));

    for item in &d.items {
        match item {
            Item::Unsupported(func) => println!("{:<NAME$} {}", func, "— unsupported arg types"),
            Item::Row {
                func,
                input,
                sdk,
                solang,
                verdict,
            } => row(func, input, sdk, solang, verdict),
        }
    }

    println!();
    println!(
        "note: {} call(s) ran on both sides, {} disagreed",
        d.checked, d.differ
    );
    println!();
}

/// Values and errors are given in full, never cut to a column.
pub fn json(d: &Data) -> Value {
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for item in &d.items {
        match item {
            Item::Unsupported(func) => skipped.push(json!(func)),
            Item::Row {
                func,
                input,
                sdk,
                solang,
                verdict,
            } => rows.push(json!({
                "function": func,
                "input": input,
                "sdk": sdk.json(),
                "solang": solang.json(),
                "verdict": verdict,
            })),
        }
    }

    json!({
        "no_spec": d.no_spec,
        "rows": rows,
        "skipped": skipped,
        "checked": d.checked,
        "disagreed": d.differ,
    })
}

/// Print one row.
fn row(func: &str, label: &str, s: &Answer, l: &Answer, verdict: &str) {
    println!(
        "{:<NAME$} {:<KIND$} {:<VALUE$} {:<VALUE$} {}",
        func,
        label,
        show(s),
        show(l),
        verdict
    );

    // A truncated column is fine when the two agree. When they do not, the difference is
    // the whole point, so print both in full. Two failures with different errors count too:
    // one side may have accepted the input and failed later for another reason.
    let different_errors = !s.ok && !l.ok && s.full() != l.full();
    if verdict == "DIFFER" || different_errors {
        println!("{:>NAME$}   sdk    {}", "", s.full());
        println!("{:>NAME$}   solang {}", "", l.full());
    }
}

/// Deploy `wasm`, call `func` with the arguments `args` builds, and return what it answered.
/// `args` gets the host and the contract's own address, because object handles belong to
/// the host that runs the call. With `authed`, the host grants every authorization the call
/// asks for; without it, none.
fn invoke(
    wasm: &[u8],
    func: &str,
    authed: bool,
    args: impl Fn(&Host, Val) -> Result<Vec<Val>, HostError>,
) -> Result<ScVal, HostError> {
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract = host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;
    if authed {
        host.switch_to_recording_auth(false)?;
    }

    let args = args(&host, contract.to_val())?;
    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;

    let out = host.call(contract, sym, argv)?;
    Ok(ScVal::try_from_val(&host, &out)?)
}

/// One value, trimmed to fit its column.
fn show(v: &Answer) -> String {
    let flat = v.full();
    if flat.chars().count() > VALUE - 1 {
        format!("{}…", flat.chars().take(VALUE - 2).collect::<String>())
    } else {
        flat
    }
}
