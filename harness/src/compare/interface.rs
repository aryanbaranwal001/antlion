use serde_json::{Value, json};

use crate::wasm::{Contract, spec};

/// One function's signature on each side, `None` where that side lacks it.
pub struct Row {
    name: String,
    sdk: Option<String>,
    solang: Option<String>,
}

pub fn collect(sdk: &Contract, solang: &Contract) -> Vec<Row> {
    fn_names(sdk, solang)
        .into_iter()
        .map(|f| Row {
            sdk: sig_for(sdk, &f),
            solang: sig_for(solang, &f),
            name: f,
        })
        .collect()
}

/// Each function's signature on both sides.
pub fn text(rows: &[Row]) {
    super::banner("contract interface");

    let shown = |s: &Option<String>| s.clone().unwrap_or_else(|| "—".to_string());
    let rows: Vec<(String, String)> = rows
        .iter()
        .map(|r| (shown(&r.sdk), shown(&r.solang)))
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

pub fn json(rows: &[Row]) -> Value {
    rows.iter()
        .map(|r| json!({ "name": r.name, "sdk": r.sdk, "solang": r.solang }))
        .collect()
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

/// This contract's signature for `name`, or `None` if it has none.
fn sig_for(c: &Contract, name: &str) -> Option<String> {
    c.interface
        .funcs
        .iter()
        .find(|f| f.name == name)
        .map(spec::sig)
}
