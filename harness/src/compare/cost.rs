use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol, Val,
    testutils::{generate_account_id, generate_bytes_array},
};

use super::spec::{self, Contract};

struct Cost {
    cpu: u64,
    mem: u64,
}

pub fn report(name: &str, sdk: &Contract, solang: &Contract) {
    super::banner("runtime cost per call");

    if sdk.interface.funcs.is_empty() {
        println!("no contract spec found for `{name}` — skipping runtime cost");
        println!();
        return;
    }

    for f in &sdk.interface.funcs {
        let Some(args) = spec::synth_args(&f.inputs) else {
            println!("{} — unsupported arg types, skipping", spec::sig(f));
            continue;
        };

        println!("{}\n", spec::sig(f));

        let s = measure(sdk.wasm, &f.name, &args);
        let l = measure(solang.wasm, &f.name, &args);

        if let Err(e) = &s {
            println!("sdk invoke failed: {e:?}");
        }
        if let Err(e) = &l {
            println!("solang invoke failed: {e:?}");
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

/// Deploy `wasm` into a fresh test host, call `func(args)`, and return metered CPU
/// instructions and memory bytes the call alone consumed. Deploy and Argument
/// marshalling are excluded.
fn measure(wasm: &[u8], func: &str, args: &[Val]) -> Result<Cost, HostError> {
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract = host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;

    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(args)?;

    let budget = host.budget_cloned();
    let base_cpu = budget.get_cpu_insns_consumed()?;
    let base_mem = budget.get_mem_bytes_consumed()?;

    host.call(contract, sym, argv)?;

    Ok(Cost {
        cpu: budget.get_cpu_insns_consumed()? - base_cpu,
        mem: budget.get_mem_bytes_consumed()? - base_mem,
    })
}

fn metric_row(label: &str, sdk: u64, solang: u64) {
    println!(
        "{:<10} {:>12} {:>12} {:>12}",
        label,
        sdk,
        solang,
        super::signed(sdk as i64 - solang as i64)
    );
}
