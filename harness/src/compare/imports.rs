use crate::wasm::{self, names};

pub fn report(sdk: &[u8], solang: &[u8]) {
    super::banner("import surface");
    println!(
        "{:<8} {:<46} {:>5} {:>8}",
        "import", "function", "sdk", "solang"
    );

    let sdk_imports = wasm::imports(sdk);
    let solang_imports = wasm::imports(solang);

    for (module, name) in union(&sdk_imports, &solang_imports) {
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

fn union(sdk: &[(String, String)], solang: &[(String, String)]) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = sdk.iter().chain(solang).cloned().collect();
    all.sort();
    all.dedup();
    all
}

fn mark(present: bool) -> &'static str {
    if present { "✓" } else { "—" }
}
