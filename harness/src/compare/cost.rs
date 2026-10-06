use serde_json::{Value, json};
use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol,
    testutils::{generate_account_id, generate_bytes_array},
    xdr::ScSpecTypeDef,
};

use crate::wasm::{Contract, spec};

struct Cost {
    cpu: u64,
    mem: u64,
}

/// One function's measurement, or a function skipped for its argument types.
enum Item {
    Unsupported(String),
    Measured {
        name: String,
        sig: String,
        sdk: Result<Cost, HostError>,
        solang: Result<Cost, HostError>,
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
    let items = sdk
        .interface
        .funcs
        .iter()
        .map(|f| {
            if !spec::args_supported(&sdk.interface, &f.inputs) {
                return Item::Unsupported(spec::sig(f));
            }
            Item::Measured {
                name: f.name.clone(),
                sig: spec::sig(f),
                sdk: measure(sdk.wasm, &sdk.interface, &f.name, &f.inputs),
                solang: measure(solang.wasm, &sdk.interface, &f.name, &f.inputs),
            }
        })
        .collect();

    Data {
        no_spec: sdk.interface.funcs.is_empty(),
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
            println!("[err] sdk invoke failed: {e:?}");
        }
        if let Err(e) = l {
            println!("[err] solang invoke failed: {e:?}");
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
    let side = |c: &Result<Cost, HostError>| match c {
        Ok(c) => json!({ "cpu": c.cpu, "mem": c.mem }),
        Err(e) => json!({ "err": super::one_line(&format!("{e:?}")) }),
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

/// Deploy `wasm`, call `func` with synthesized arguments, and return what the call alone
/// consumed.
///
/// Arguments are built here rather than passed in, because object handles belong to one
/// host's object table and each side gets its own host.
fn measure(
    wasm: &[u8],
    iface: &spec::Interface,
    func: &str,
    inputs: &[ScSpecTypeDef],
) -> Result<Cost, HostError> {
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract = host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;
    host.ensure_module_cache_contains_host_storage_contracts()?;

    let args = spec::build_args(&host, contract.to_val(), iface, inputs, spec::Inputs::Small)?;

    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;

    let budget = host.budget_cloned();
    let base_cpu = budget.get_cpu_insns_consumed()?;
    let base_mem = budget.get_mem_bytes_consumed()?;

    host.call(contract, sym, argv)?;

    Ok(Cost {
        cpu: budget.get_cpu_insns_consumed()? - base_cpu,
        mem: budget.get_mem_bytes_consumed()? - base_mem,
    })
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
