use crate::wasm::{Contract, spec};

/// Each function's signature on both sides.
pub fn report(sdk: &Contract, solang: &Contract) {
    super::banner("contract interface");

    let rows: Vec<(String, String)> = fn_names(sdk, solang)
        .into_iter()
        .map(|f| (sig_for(sdk, &f), sig_for(solang, &f)))
        .collect();

    // Type-heavy signatures run well past any fixed width, so the column is sized to
    // what it holds.
    let w = rows
        .iter()
        .map(|(s, _)| s.chars().count())
        .max()
        .unwrap_or(0);

    println!("{:<w$}  {}\n", "sdk", "solang");

    for (s, l) in rows {
        println!("{s:<w$}  {l}");
    }

    println!();
    println!("note: these are the soroban types, read from contractspecv0");
    println!("note: in wasm itself every one of them is an i64 — a tagged Val");
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
