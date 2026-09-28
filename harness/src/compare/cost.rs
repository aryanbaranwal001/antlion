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

/// Invoke every function on both builds and compare metered cost. Each call gets a
/// fresh host, so every function is measured against empty storage.
pub fn report(name: &str, sdk: &Contract, solang: &Contract) {
    super::banner("runtime cost per call");

    if sdk.interface.funcs.is_empty() {
        println!("[skip] no contract spec found for `{name}` — no runtime cost measured");
        println!();
        return;
    }

    for f in &sdk.interface.funcs {
        if !spec::args_supported(&f.inputs) {
            println!("[skip] {} — unsupported arg types", spec::sig(f));
            continue;
        }

        println!("{}\n", spec::sig(f));

        let s = measure(sdk.wasm, &f.name, &f.inputs);
        let l = measure(solang.wasm, &f.name, &f.inputs);

        if let Err(e) = &s {
            println!("[err] sdk invoke failed: {e:?}");
        }
        if let Err(e) = &l {
            println!("[err] solang invoke failed: {e:?}");
        }

        if let (Ok(s), Ok(l)) = (&s, &l) {
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

/// Deploy `wasm`, call `func` with synthesized arguments, and return what the call alone
/// consumed.
///
/// Arguments are built here rather than passed in, because object handles belong to one
/// host's object table and each side gets its own host.
fn measure(wasm: &[u8], func: &str, inputs: &[ScSpecTypeDef]) -> Result<Cost, HostError> {
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract = host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;

    let args = spec::build_args(&host, contract.to_val(), inputs)?;

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
