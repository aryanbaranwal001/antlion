// Print the two contracts' interfaces side by side, from the specs parsed in `spec`.
use crate::wasm::spec::{self, Contract};

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

/// Union of function names across both contracts, sdk order first.
fn fn_names(sdk: &Contract, solang: &Contract) -> Vec<String> {
    let mut names: Vec<String> = sdk.interface.funcs.iter().map(|f| f.name.clone()).collect();
    for f in &solang.interface.funcs {
        if !names.contains(&f.name) {
            names.push(f.name.clone());
        }
    }
    names
}

fn sig_for(c: &Contract, name: &str) -> String {
    match c.interface.funcs.iter().find(|f| f.name == name) {
        Some(f) => spec::sig(f),
        None => "—".to_string(),
    }
}
