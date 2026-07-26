use crate::wasm::{Contract, spec};

/// Each function's signature on both sides.
pub fn report(sdk: &Contract, solang: &Contract) {
    super::banner("contract interface");
    println!("{:<40} {:<40}", "sdk", "solang\n");

    for fname in fn_names(sdk, solang) {
        let s = sig_for(sdk, &fname);
        let l = sig_for(solang, &fname);
        println!("{s:<40} {l:<40}");
    }
    println!();
}

/// Function names from both contracts, sdk order first.
fn fn_names(sdk: &Contract, solang: &Contract) -> Vec<String> {
    let mut names: Vec<String> = sdk.interface.funcs.iter().map(|f| f.name.clone()).collect();
    for f in &solang.interface.funcs {
        if !names.contains(&f.name) {
            names.push(f.name.clone());
        }
    }
    names
}

/// This contract's signature for `name`, or `—` if it has none.
fn sig_for(c: &Contract, name: &str) -> String {
    match c.interface.funcs.iter().find(|f| f.name == name) {
        Some(f) => spec::sig(f),
        None => "—".to_string(),
    }
}
