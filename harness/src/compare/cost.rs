use rayon::prelude::*;
use serde_json::{Map, Value, json};
use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol,
    xdr::{ContractCostType, ScSpecTypeDef},
};

use super::call::{Cost, Setup, TypeCost, charged, deploy, deploy_and_call, trackers};
use crate::cli::Mode;
use crate::wasm::{Contract, spec};

/// How many times each call is measured, each time on a fresh host. The host's metering
/// must give the same cost every time: the network agrees on it.
const RUNS: usize = 3;

const LABEL: usize = 13;
const NUM: usize = 12;
const TYPE: usize = 32;

/// A function name no module exports.
const ABSENT: &str = "antlion_absent";

/// One side's measurement of one call, and whether every run gave the same cost.
struct Side {
    cost: Result<Cost, String>,
    steady: bool,
}

/// One function's measurement, or a function skipped for its argument types.
enum Item {
    Unsupported(String),
    Measured {
        name: String,
        sig: String,
        sdk: Side,
        solang: Side,
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
                    let run = |wasm| {
                        let runs: Vec<Result<Cost, String>> = (0..RUNS)
                            .map(|_| measure(wasm, iface, &f.name, &f.inputs).map_err(text))
                            .collect();
                        Side {
                            steady: runs.iter().all(|r| *r == runs[0]),
                            cost: runs.into_iter().next().expect("RUNS is at least one"),
                        }
                    };
                    Item::Measured {
                        name: f.name.clone(),
                        sig: spec::sig(f),
                        sdk: run(sw),
                        solang: run(lw),
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

        if let Err(e) = &s.cost {
            println!("[err] sdk invoke failed: {e}");
        }
        if let Err(e) = &l.cost {
            println!("[err] solang invoke failed: {e}");
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

    println!(
        "note: run is a call's cpu less the module's instantiate cost: what its function alone used"
    );
    if unsteady.is_empty() {
        println!(
            "note: every call ran {RUNS} times, each on a fresh host, and cost the same each time"
        );
    } else {
        for u in &unsteady {
            println!("[warn] {u}: the cost was not the same in all {RUNS} runs");
        }
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

/// A failed call is `{"err": ...}` and has no `diff`, as in the text.
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
        v["steady"] = json!(x.steady);
        v
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
            } => {
                let mut v = json!({
                    "function": name,
                    "signature": sig,
                    "sdk": side(sdk),
                    "solang": side(solang),
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

/// What one call of `func` consumed, with `small` arguments and the module cached.
fn measure(
    wasm: &[u8],
    iface: &spec::Interface,
    func: &str,
    inputs: &[ScSpecTypeDef],
) -> Result<Cost, HostError> {
    let setup = Setup {
        cached: true,
        authed: false,
    };
    deploy_and_call(
        wasm,
        func,
        setup,
        |h, addr| spec::build_args(h, addr, iface, inputs, spec::Inputs::Small),
        |_, _, cost| Ok(cost),
    )
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
