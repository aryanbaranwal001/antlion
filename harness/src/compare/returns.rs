use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol, TryFromVal, Val,
    testutils::{generate_account_id, generate_bytes_array},
    xdr::ScVal,
};

use crate::wasm::{Contract, spec};

const NAME: usize = 15;
const KIND: usize = 6;
const VALUE: usize = 24;

/// Invoke every function on both builds and compare what each one answers. Cost says how
/// much the two spent; this says whether they agree.
///
/// Each function runs twice. `small` uses values that ride inline in the `Val` word;
/// `edge` uses boundary values, including 64 and 128 bit numbers too large for the 56 bit
/// payload, which the host therefore passes as object handles.
pub fn report(name: &str, sdk: &Contract, solang: &Contract) {
    super::banner("return value equivalence");

    if sdk.interface.funcs.is_empty() {
        println!("[skip] no contract spec found for `{name}` — nothing to invoke");
        println!();
        return;
    }

    println!(
        "{:<NAME$} {:<KIND$} {:<VALUE$} {:<VALUE$} {}",
        "function", "input", "sdk", "solang", "verdict"
    );
    println!("{}", "─".repeat(NAME + KIND + VALUE * 2 + 10));

    let mut differ = 0;
    let mut checked = 0;

    for f in &sdk.interface.funcs {
        if !spec::args_supported(&sdk.interface, &f.inputs) {
            println!("{:<NAME$} {}", f.name, "— unsupported arg types");
            continue;
        }

        for kind in spec::ALL_INPUTS {
            let args = |h: &Host, addr| spec::build_args(h, addr, &sdk.interface, &f.inputs, kind);
            let s = invoke(sdk.wasm, &f.name, args);
            let l = invoke(solang.wasm, &f.name, args);
            row(&f.name, kind.label(), &s, &l, &mut checked, &mut differ);
        }

        // A struct argument is also sent in shapes neither side should accept. Both
        // rejecting is the expected answer; one side accepting is the divergence.
        if spec::takes_struct(&f.inputs) {
            for m in spec::ALL_MALFORMED {
                let args =
                    |h: &Host, addr| spec::build_malformed(h, addr, &sdk.interface, &f.inputs, m);
                let s = invoke(sdk.wasm, &f.name, args);
                let l = invoke(solang.wasm, &f.name, args);
                row(&f.name, m.label(), &s, &l, &mut checked, &mut differ);
            }
        }
    }

    println!();
    println!("note: {checked} call(s) ran on both sides, {differ} disagreed");
    println!();
}

/// Print one row and tally it. Only calls that ran on both sides count as checked.
fn row(
    func: &str,
    label: &str,
    s: &Result<ScVal, HostError>,
    l: &Result<ScVal, HostError>,
    checked: &mut usize,
    differ: &mut usize,
) {
    let verdict = match (s, l) {
        (Ok(a), Ok(b)) => {
            *checked += 1;
            if a == b {
                "same"
            } else {
                *differ += 1;
                "DIFFER"
            }
        }
        (Err(_), Err(_)) => "both failed",
        (Err(_), Ok(_)) => "sdk failed",
        (Ok(_), Err(_)) => "solang failed",
    };

    println!(
        "{:<NAME$} {:<KIND$} {:<VALUE$} {:<VALUE$} {}",
        func,
        label,
        show(s),
        show(l),
        verdict
    );

    // A truncated column is fine when the two agree. When they do not, the difference is
    // the whole point, so print both in full.
    if verdict == "DIFFER" {
        println!("{:>NAME$}   sdk    {}", "", full(s));
        println!("{:>NAME$}   solang {}", "", full(l));
    }
}

/// Deploy `wasm`, call `func` with the arguments `args` builds, and return what it answered.
/// `args` gets the host and the contract's own address, because object handles belong to
/// the host that runs the call.
fn invoke(
    wasm: &[u8],
    func: &str,
    args: impl Fn(&Host, Val) -> Result<Vec<Val>, HostError>,
) -> Result<ScVal, HostError> {
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract = host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;

    let args = args(&host, contract.to_val())?;
    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;

    let out = host.call(contract, sym, argv)?;
    Ok(ScVal::try_from_val(&host, &out)?)
}

/// One value, on a single line.
fn full(v: &Result<ScVal, HostError>) -> String {
    let text = match v {
        Ok(val) => format!("{val:?}"),
        Err(e) => format!("[err] {e:?}"),
    };
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One value, trimmed to fit its column.
fn show(v: &Result<ScVal, HostError>) -> String {
    let flat = full(v);
    if flat.chars().count() > VALUE - 1 {
        format!("{}…", flat.chars().take(VALUE - 2).collect::<String>())
    } else {
        flat
    }
}
