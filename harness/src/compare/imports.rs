use serde_json::{Value, json};

use crate::wasm::{Contract, names};

/// One imported host function, and which side imports it.
pub struct Row {
    module: String,
    field: String,
    sdk: bool,
    solang: bool,
}

pub struct Data {
    rows: Vec<Row>,
    sdk_total: usize,
    solang_total: usize,
}

pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let (sdk_imports, solang_imports) = (&sdk.imports, &solang.imports);

    let rows = union(sdk_imports, solang_imports)
        .into_iter()
        .map(|key| Row {
            sdk: sdk_imports.contains(&key),
            solang: solang_imports.contains(&key),
            module: key.0,
            field: key.1,
        })
        .collect();

    Data {
        rows,
        sdk_total: sdk_imports.len(),
        solang_total: solang_imports.len(),
    }
}

/// Which host functions each side imports.
pub fn text(d: &Data) {
    super::banner("import surface");
    println!(
        "{:<8} {:<46} {:>5} {:>8}",
        "import", "function", "sdk", "solang"
    );

    for r in &d.rows {
        println!(
            "{:<8} {:<46} {:>5} {:>8}",
            format!("{}.{}", r.module, r.field),
            names::host_fn_name(&r.module, &r.field).unwrap_or("?"),
            mark(r.sdk),
            mark(r.solang),
        );
    }

    println!(
        "{:<8} {:<46} {:>5} {:>8}",
        "total", "", d.sdk_total, d.solang_total
    );
    println!();
}

/// An unknown host function, `?` in the text, is `null`.
pub fn json(d: &Data) -> Value {
    let functions: Vec<Value> = d
        .rows
        .iter()
        .map(|r| {
            json!({
                "import": format!("{}.{}", r.module, r.field),
                "name": names::host_fn_name(&r.module, &r.field),
                "sdk": r.sdk,
                "solang": r.solang,
            })
        })
        .collect();

    json!({
        "functions": functions,
        "total": { "sdk": d.sdk_total, "solang": d.solang_total },
    })
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
