use crate::wasm::{Contract, names};

/// Which host functions each side imports.
pub fn report(sdk: &Contract, solang: &Contract) {
    super::banner("import surface");
    println!(
        "{:<8} {:<46} {:>5} {:>8}",
        "import", "function", "sdk", "solang"
    );

    let (sdk_imports, solang_imports) = (&sdk.imports, &solang.imports);

    for (module, name) in union(sdk_imports, solang_imports) {
        let key = (module, name);
        println!(
            "{:<8} {:<46} {:>5} {:>8}",
            format!("{}.{}", key.0, key.1),
            names::host_fn_name(&key.0, &key.1).unwrap_or("?"),
            mark(sdk_imports.contains(&key)),
            mark(solang_imports.contains(&key)),
        );
    }

    println!(
        "{:<8} {:<46} {:>5} {:>8}",
        "total",
        "",
        sdk_imports.len(),
        solang_imports.len()
    );
    println!();
}

/// Sorted, deduplicated imports from both sides.
fn union(sdk: &[(String, String)], solang: &[(String, String)]) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = sdk.iter().chain(solang).cloned().collect();
    all.sort();
    all.dedup();
    all
}

/// Presence cell: `✓` or `—`.
fn mark(present: bool) -> &'static str {
    if present { "✓" } else { "—" }
}
