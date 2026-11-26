use rayon::prelude::*;
use serde_json::{Map, Value, json};
use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol,
    xdr::{ContractCostType, ScSpecTypeDef},
};

use super::call::{Cost, Setup, TypeCost, charged, deploy, trackers};
use crate::cli::Mode;
use crate::wasm::{Contract, spec};

/// How many times each call is measured, each time on a fresh host. The host's metering
/// must give the same cost every time: the network agrees on it.
const RUNS: usize = 3;

const LABEL: usize = 13;
const NUM: usize = 12;
const TYPE: usize = 32;
const FUNC: usize = 18;
const INPUT: usize = 6;

/// The input classes past `small` that the cost to fail is looked for at.
const FAIL_INPUTS: [spec::Inputs; 3] =
    [spec::Inputs::Large, spec::Inputs::Bound, spec::Inputs::Neg];

/// A function name no module exports.
const ABSENT: &str = "antlion_absent";

/// One measured call: what it cost, and its error code when it failed. A failed call is
/// measured too: its cost is the cost to fail.
type Outcome = Result<(Cost, Option<String>), String>;

/// One side's measurement of one call, and whether every run gave the same cost.
struct Side {
    /// The error is one that stopped the measure itself, such as a failed deploy.
    cost: Result<Cost, String>,
    /// The call's error code, when the call failed.
    failed: Option<String>,
    steady: bool,
}

/// A call at an input past `small` that failed on one side or both.
struct Fail {
    input: &'static str,
    sdk: Outcome,
    solang: Outcome,
}

/// One function's measurement, or a function skipped for its argument types.
enum Item {
    Unsupported(String),
    Measured {
        name: String,
        sig: String,
        sdk: Side,
        solang: Side,
        fails: Vec<Fail>,
    },
}

/// The costs that belong to a module, not to one function, on one side.
struct Module {
    /// Parsing the module, which the network charges once, at upload.
    parse: Cost,
    /// Instantiating the cached module, which every call pays before its function runs.
    instantiate: Cost,
}

pub struct Data {
    no_spec: bool,
    module: (Result<Module, String>, Result<Module, String>),
    items: Vec<Item>,
}

/// Invoke every function on both builds and measure metered cost. Each call gets a
/// fresh host, so every function is measured against empty storage.
///
/// The module is put in the host's module cache before the call, as the network keeps
/// every live contract parsed (CAP-0065, protocol 23). So a call pays for instantiating the
/// module but not for parsing it, which is charged once at upload instead. Both are
/// measured on their own, once per module.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let (sw, lw, iface) = (sdk.wasm, solang.wasm, &sdk.interface);
    let text = |e: HostError| format!("{e:?}");
    let module = |wasm| {
        Ok(Module {
            parse: parse(wasm)?,
            instantiate: instantiate(wasm)?,
        })
    };
    let (m, items) = rayon::join(
        || rayon::join(|| module(sw).map_err(text), || module(lw).map_err(text)),
        || {
            iface
                .funcs
                .par_iter()
                .map(|f| {
                    if !spec::args_supported(iface, &f.inputs) {
                        return Item::Unsupported(spec::sig(f));
                    }
                    // Errors are kept as text, so a result can come back from another thread.
                    let once =
                        |wasm, kind| measure(wasm, iface, &f.name, &f.inputs, kind).map_err(text);
                    let run = |wasm| {
                        let runs: Vec<Outcome> =
                            (0..RUNS).map(|_| once(wasm, spec::Inputs::Small)).collect();
                        let steady = runs.iter().all(|r| *r == runs[0]);
                        let first = runs.into_iter().next().expect("RUNS is at least one");
                        Side {
                            failed: first.as_ref().ok().and_then(|(_, e)| e.clone()),
                            cost: first.map(|(c, _)| c),
                            steady,
                        }
                    };
                    // Only the inputs where a side fails are kept.
                    let fails = FAIL_INPUTS
                        .iter()
                        .map(|&kind| Fail {
                            input: kind.label(),
                            sdk: once(sw, kind),
                            solang: once(lw, kind),
                        })
                        .filter(|x| !(ok(&x.sdk) && ok(&x.solang)))
                        .collect();
                    Item::Measured {
                        name: f.name.clone(),
                        sig: spec::sig(f),
                        sdk: run(sw),
                        solang: run(lw),
                        fails,
                    }
                })
                .collect()
        },
    );

    Data {
        no_spec: iface.funcs.is_empty(),
        module: m,
        items,
    }
}

/// Whether the call ran and returned.
fn ok(o: &Outcome) -> bool {
    matches!(o, Ok((_, None)))
}

/// What a call costs before the function runs: instantiating the cached module. It is
/// measured as a call to a function the module does not export, which the host makes
/// ready and then fails, so only the instantiation is charged.
fn instantiate(wasm: &[u8]) -> Result<Cost, HostError> {
    let setup = Setup {
        cached: true,
        authed: false,
    };
    let (host, contract) = deploy(wasm, setup)?;
    let sym = Symbol::from(host.symbol_new_from_slice(ABSENT.as_bytes())?);
    let argv = host.vec_new_from_slice(&[])?;
    metered(&host, || {
        // The call is expected to fail. Its cost is all that is wanted.
        let _ = host.call(contract, sym, argv);
        Ok(())
    })
}

/// What `wasm`'s module costs to parse into the module cache.
fn parse(wasm: &[u8]) -> Result<Cost, HostError> {
    let setup = Setup {
        cached: false,
        authed: false,
    };
    let (host, _) = deploy(wasm, setup)?;
    metered(&host, || {
        host.ensure_module_cache_contains_host_storage_contracts()
    })
}

/// What `f` charged to `host`'s budget.
fn metered(host: &Host, f: impl FnOnce() -> Result<(), HostError>) -> Result<Cost, HostError> {
    let budget = host.budget_cloned();
    let (cpu, mem) = (
        budget.get_cpu_insns_consumed()?,
        budget.get_mem_bytes_consumed()?,
    );
    let before = trackers(&budget)?;
    f()?;
    Ok(Cost {
        cpu: budget.get_cpu_insns_consumed()? - cpu,
        mem: budget.get_mem_bytes_consumed()? - mem,
        types: charged(&before, &trackers(&budget)?),
    })
}

/// Compare metered cost per call. `--detail` adds what each cost type charged.
pub fn text(name: &str, d: &Data, mode: Mode) {
    super::banner("runtime cost per call");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}` — no runtime cost measured");
        println!();
        return;
    }

    // What the module costs, apart from any one function: parsed once at upload, and
    // instantiated once per call.
    let module = match &d.module {
        (Ok(s), Ok(l)) => Some((s, l)),
        (s, l) => {
            for (side, m) in [("sdk", s), ("solang", l)] {
                if let Err(e) = m {
                    println!("[err] {side} module measure failed: {}", super::one_line(e));
                }
            }
            println!();
            None
        }
    };
    if let Some((s, l)) = module {
        for (title, a, b) in [
            ("module parse, paid once at upload", &s.parse, &l.parse),
            (
                "module instantiate, paid by every call before its function runs",
                &s.instantiate,
                &l.instantiate,
            ),
        ] {
            println!("{title}\n");
            header();
            metric_row("cpu insns", a.cpu, b.cpu);
            metric_row("mem bytes", a.mem, b.mem);
            if mode == Mode::Detail {
                types(a, b);
            }
            println!();
        }
    }

    let mut unsteady = Vec::new();
    for item in &d.items {
        let (sig, s, l) = match item {
            Item::Unsupported(sig) => {
                println!("[skip] {sig} — unsupported arg types");
                continue;
            }
            Item::Measured {
                sig, sdk, solang, ..
            } => (sig, sdk, solang),
        };

        println!("{sig}\n");

        for (side, x) in [("sdk", s), ("solang", l)] {
            match (&x.cost, &x.failed) {
                (Err(e), _) => println!("[err] {side} measure failed: {}", super::one_line(e)),
                (Ok(_), Some(e)) => {
                    println!("[err] {side} call failed: {e}; its cost to fail is below")
                }
                _ => {}
            }
        }

        if let (Ok(s), Ok(l)) = (&s.cost, &l.cost) {
            header();
            metric_row("cpu insns", s.cpu, l.cpu);
            if let Some((ms, ml)) = module {
                metric_row("  instantiate", ms.instantiate.cpu, ml.instantiate.cpu);
                row(
                    "  run",
                    s.cpu as i64 - ms.instantiate.cpu as i64,
                    l.cpu as i64 - ml.instantiate.cpu as i64,
                );
            }
            metric_row("mem bytes", s.mem, l.mem);
            if mode == Mode::Detail {
                types(s, l);
            }
        }
        for (side, x) in [("sdk", s), ("solang", l)] {
            if !x.steady {
                unsteady.push(format!("{side} {sig}"));
            }
        }
        println!();
    }

    fails(d, module);

    if super::notes() {
        println!("note: run is the cpu less instantiate");
    }
    for u in &unsteady {
        println!("[warn] {u}: the cost was not the same in all {RUNS} runs");
    }
    if super::notes() || !unsteady.is_empty() {
        println!();
    }
}

/// Every call that failed on one side or both, at any input, with the run cost of each side.
fn fails(d: &Data, module: Option<(&Module, &Module)>) {
    let mut rows = Vec::new();
    for item in &d.items {
        let Item::Measured {
            name,
            sdk,
            solang,
            fails,
            ..
        } = item
        else {
            continue;
        };
        let small = |x: &Side| -> Outcome { x.cost.clone().map(|c| (c, x.failed.clone())) };
        let (s, l) = (small(sdk), small(solang));
        if !(ok(&s) && ok(&l)) {
            rows.push((name, "small", s, l));
        }
        for f in fails {
            rows.push((name, f.input, f.sdk.clone(), f.solang.clone()));
        }
    }
    if rows.is_empty() {
        return;
    }

    println!("cost to fail, for every call that failed on one side or both\n");
    println!(
        "{:<FUNC$} {:<INPUT$} {:>NUM$} {:>NUM$}  {:<8} solang",
        "function", "input", "sdk run", "solang run", "sdk"
    );
    let run = |o: &Outcome, m: Option<&Module>| match (o, m) {
        (Ok((c, _)), Some(m)) => (c.cpu as i64 - m.instantiate.cpu as i64).to_string(),
        _ => "—".to_string(),
    };
    let state = |o: &Outcome| match o {
        Ok((_, None)) => "ok",
        Ok((_, Some(_))) => "failed",
        Err(_) => "err",
    };
    for (name, input, s, l) in &rows {
        println!(
            "{name:<FUNC$} {input:<INPUT$} {:>NUM$} {:>NUM$}  {:<8} {}",
            run(s, module.map(|m| m.0)),
            run(l, module.map(|m| m.1)),
            state(s),
            state(l)
        );
    }
    println!();
}

fn header() {
    println!(
        "{:<LABEL$} {:>NUM$} {:>NUM$} {:>NUM$}",
        "metric", "sdk", "solang", "diff"
    );
}

/// What each cost type charged on each side, in the host's order. A type that only one
/// side charged shows 0 on the other. A type's mem is what it charged; the total is the
/// peak, which is lower when memory was given back.
fn types(s: &Cost, l: &Cost) {
    println!();
    println!(
        "{:<TYPE$} {:>NUM$} {:>NUM$} {:>NUM$} {:>NUM$}",
        "cost type", "sdk cpu", "solang cpu", "sdk mem", "solang mem"
    );
    let get = |c: &Cost, ty| c.types.iter().find(|t| t.ty == ty).copied();
    for ty in ContractCostType::variants() {
        let (a, b) = (get(s, ty), get(l, ty));
        if a.is_none() && b.is_none() {
            continue;
        }
        let zero = TypeCost { ty, cpu: 0, mem: 0 };
        let (a, b) = (a.unwrap_or(zero), b.unwrap_or(zero));
        println!(
            "{:<TYPE$} {:>NUM$} {:>NUM$} {:>NUM$} {:>NUM$}",
            ty.name(),
            a.cpu,
            b.cpu,
            a.mem,
            b.mem
        );
    }
}

/// A side whose measure failed is `{"err": ...}` and has no `diff`. A call that failed has
/// its cost and its error code in `failed`.
pub fn json(d: &Data) -> Value {
    let cost = |c: &Cost| {
        let types: Map<String, Value> = c
            .types
            .iter()
            .map(|t| {
                (
                    t.ty.name().to_string(),
                    json!({ "cpu": t.cpu, "mem": t.mem }),
                )
            })
            .collect();
        json!({ "cpu": c.cpu, "mem": c.mem, "types": types })
    };
    let err = |e: &String| json!({ "err": super::one_line(e) });
    let side = |x: &Side| {
        let mut v = x.cost.as_ref().map_or_else(err, cost);
        v["failed"] = json!(x.failed);
        v["steady"] = json!(x.steady);
        v
    };
    let outcome = |o: &Outcome| match o {
        Ok((c, failed)) => json!({ "cpu": c.cpu, "mem": c.mem, "failed": failed }),
        Err(e) => err(e),
    };
    let module = |m: &Result<Module, String>| {
        m.as_ref().map_or_else(
            err,
            |m| json!({ "parse": cost(&m.parse), "instantiate": cost(&m.instantiate) }),
        )
    };

    let mut functions = Vec::new();
    let mut skipped = Vec::new();
    for item in &d.items {
        match item {
            Item::Unsupported(sig) => skipped.push(json!(sig)),
            Item::Measured {
                name,
                sig,
                sdk,
                solang,
                fails,
            } => {
                let mut v = json!({
                    "function": name,
                    "signature": sig,
                    "sdk": side(sdk),
                    "solang": side(solang),
                    "fails": fails.iter().map(|f| json!({
                        "input": f.input,
                        "sdk": outcome(&f.sdk),
                        "solang": outcome(&f.solang),
                    })).collect::<Vec<_>>(),
                });
                if let (Ok(s), Ok(l)) = (&sdk.cost, &solang.cost) {
                    v["diff"] = json!({
                        "cpu": s.cpu as i64 - l.cpu as i64,
                        "mem": s.mem as i64 - l.mem as i64,
                    });
                }
                functions.push(v);
            }
        }
    }

    json!({
        "no_spec": d.no_spec,
        "runs": RUNS,
        "module": { "sdk": module(&d.module.0), "solang": module(&d.module.1) },
        "functions": functions,
        "skipped": skipped,
    })
}

/// What one call of `func` consumed, with `kind` arguments and the module cached, and its
/// error code if it failed. The cost is read around the call alone, as in `call`.
fn measure(
    wasm: &[u8],
    iface: &spec::Interface,
    func: &str,
    inputs: &[ScSpecTypeDef],
    kind: spec::Inputs,
) -> Result<(Cost, Option<String>), HostError> {
    let setup = Setup {
        cached: true,
        authed: false,
    };
    let (host, contract) = deploy(wasm, setup)?;
    let args = spec::build_args(&host, contract.to_val(), iface, inputs, kind)?;
    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;
    let mut failed = None;
    let cost = metered(&host, || {
        if let Err(e) = host.call(contract, sym, argv) {
            failed = Some(format!("{:?}", e.error));
        }
        Ok(())
    })?;
    Ok((cost, failed))
}

/// One metric line.
fn metric_row(label: &str, sdk: u64, solang: u64) {
    row(label, sdk as i64, solang as i64);
}

fn row(label: &str, sdk: i64, solang: i64) {
    println!(
        "{:<LABEL$} {:>NUM$} {:>NUM$} {:>NUM$}",
        label,
        sdk,
        solang,
        super::signed(sdk - solang)
    );
}
