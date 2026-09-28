use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol, TryFromVal,
    testutils::{generate_account_id, generate_bytes_array},
    xdr::{ScSpecTypeDef, ScVal},
};

use crate::wasm::{Contract, spec};

const NAME: usize = 16;
const VALUE: usize = 26;

/// Invoke every function on both builds and compare what each one answers. Cost says how
/// much the two spent; this says whether they agree.
pub fn report(name: &str, sdk: &Contract, solang: &Contract) {
    super::banner("return value equivalence");

    if sdk.interface.funcs.is_empty() {
        println!("[skip] no contract spec found for `{name}` — nothing to invoke");
        println!();
        return;
    }

    println!(
        "{:<NAME$} {:<VALUE$} {:<VALUE$} {}",
        "function", "sdk", "solang", "verdict"
    );
    println!("{}", "─".repeat(NAME + VALUE * 2 + 10));

    let mut differ = 0;
    let mut checked = 0;

    for f in &sdk.interface.funcs {
        if !spec::args_supported(&f.inputs) {
            println!("{:<NAME$} {}", f.name, "— unsupported arg types");
            continue;
        }

        let s = invoke(sdk.wasm, &f.name, &f.inputs);
        let l = invoke(solang.wasm, &f.name, &f.inputs);

        let verdict = match (&s, &l) {
            (Ok(a), Ok(b)) => {
                checked += 1;
                if a == b {
                    "same"
                } else {
                    differ += 1;
                    "DIFFER"
                }
            }
            (Err(_), Err(_)) => "both failed",
            (Err(_), Ok(_)) => "sdk failed",
            (Ok(_), Err(_)) => "solang failed",
        };

        println!(
            "{:<NAME$} {:<VALUE$} {:<VALUE$} {}",
            f.name,
            show(&s),
            show(&l),
            verdict
        );
    }

    println!();
    println!("note: {checked} function(s) ran on both sides, {differ} disagreed");
    println!();
}

/// Deploy `wasm`, call `func` with synthesized arguments, and return what it answered.
fn invoke(wasm: &[u8], func: &str, inputs: &[ScSpecTypeDef]) -> Result<ScVal, HostError> {
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract = host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;

    let args = spec::build_args(&host, contract.to_val(), inputs)?;
    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;

    let out = host.call(contract, sym, argv)?;
    Ok(ScVal::try_from_val(&host, &out)?)
}

/// One value, trimmed to fit its column.
fn show(v: &Result<ScVal, HostError>) -> String {
    let text = match v {
        Ok(val) => format!("{val:?}"),
        Err(e) => format!("[err] {e:?}"),
    };

    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > VALUE - 1 {
        format!("{}…", flat.chars().take(VALUE - 2).collect::<String>())
    } else {
        flat
    }
}
