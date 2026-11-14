use rayon::prelude::*;
use serde_json::{Value, json};
use soroban_env_host::{HostError, xdr::ScSpecTypeDef};

use super::call::{Cost, Setup, deploy_and_call};
use crate::wasm::{Contract, spec};

/// One function's measurement, or a function skipped for its argument types.
enum Item {
    Unsupported(String),
    Measured {
        name: String,
        sig: String,
        sdk: Result<Cost, String>,
        solang: Result<Cost, String>,
    },
}

pub struct Data {
    no_spec: bool,
    items: Vec<Item>,
}

/// Invoke every function on both builds and measure metered cost. Each call gets a
/// fresh host, so every function is measured against empty storage.
///
/// The module is put in the host's module cache before the call, as the network keeps
/// every live contract parsed (CAP-0065, protocol 23). So a call pays for instantiating the
/// module but not for parsing it, which is charged once at upload instead.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let (sw, lw, iface) = (sdk.wasm, solang.wasm, &sdk.interface);
    let items = iface
        .funcs
        .par_iter()
        .map(|f| {
            if !spec::args_supported(iface, &f.inputs) {
                return Item::Unsupported(spec::sig(f));
            }
            // Errors are kept as text, so a result can come back from another thread.
            let run = |wasm| measure(wasm, iface, &f.name, &f.inputs).map_err(|e| format!("{e:?}"));
            Item::Measured {
                name: f.name.clone(),
                sig: spec::sig(f),
                sdk: run(sw),
                solang: run(lw),
            }
        })
        .collect();

    Data {
        no_spec: iface.funcs.is_empty(),
        items,
    }
}

/// Compare metered cost per call.
pub fn text(name: &str, d: &Data) {
    super::banner("runtime cost per call");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}` — no runtime cost measured");
        println!();
        return;
    }

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

        if let Err(e) = s {
            println!("[err] sdk invoke failed: {e}");
        }
        if let Err(e) = l {
            println!("[err] solang invoke failed: {e}");
        }

        if let (Ok(s), Ok(l)) = (s, l) {
            println!(
                "{:<10} {:>12} {:>12} {:>12}",
                "metric", "sdk", "solang", "diff"
            );
            metric_row("cpu insns", s.cpu, l.cpu);
            metric_row("mem bytes", s.mem, l.mem);
        }
        println!();
    }
    println!();
}

/// A failed call is `{"err": ...}` and has no `diff`, as in the text.
pub fn json(d: &Data) -> Value {
    let side = |c: &Result<Cost, String>| match c {
        Ok(c) => json!({ "cpu": c.cpu, "mem": c.mem }),
        Err(e) => json!({ "err": super::one_line(e) }),
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
                if let (Ok(s), Ok(l)) = (sdk, solang) {
                    v["diff"] = json!({
                        "cpu": s.cpu as i64 - l.cpu as i64,
                        "mem": s.mem as i64 - l.mem as i64,
                    });
                }
                functions.push(v);
            }
        }
    }

    json!({ "no_spec": d.no_spec, "functions": functions, "skipped": skipped })
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
    println!(
        "{:<10} {:>12} {:>12} {:>12}",
        label,
        sdk,
        solang,
        super::signed(sdk as i64 - solang as i64)
    );
}
