use crate::wasm::{Contract, code};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

const BUCKETS: [&str; 8] = [
    "control flow",
    "calls",
    "loads",
    "stores",
    "local/global",
    "const",
    "numeric",
    "memory/other",
];

/// One opcode's count on each side.
struct Count<'a> {
    name: &'a str,
    sdk: u64,
    solang: u64,
}

/// One bucket's total, and each opcode in it, sorted by name.
struct Bucket<'a> {
    name: &'static str,
    sdk: u64,
    solang: u64,
    opcodes: Vec<Count<'a>>,
}

/// The histogram of both builds. Buckets empty on both sides are left out.
pub struct Data<'a> {
    buckets: Vec<Bucket<'a>>,
    total: (u64, u64),
    sdk_only: Vec<&'a str>,
    solang_only: Vec<&'a str>,
}

pub fn collect<'a>(sdk: &'a Contract, solang: &'a Contract) -> Data<'a> {
    let sdk = feed(sdk.functions());
    let solang = feed(solang.functions());

    let mut buckets = Vec::new();
    let mut total = (0, 0);
    for b in BUCKETS {
        let s = sum_bucket(&sdk, b);
        let l = sum_bucket(&solang, b);
        if s == 0 && l == 0 {
            continue;
        }
        total.0 += s;
        total.1 += l;

        let opcodes = opcodes_in(b, &sdk, &solang)
            .into_iter()
            .map(|name| Count {
                name,
                sdk: *sdk.get(name).unwrap_or(&0),
                solang: *solang.get(name).unwrap_or(&0),
            })
            .collect();
        buckets.push(Bucket {
            name: b,
            sdk: s,
            solang: l,
            opcodes,
        });
    }

    let (sdk_only, solang_only) = exclusives(&sdk, &solang);
    Data {
        buckets,
        total,
        sdk_only,
        solang_only,
    }
}

/// Dispatch on the trailing flag: `--detail`, `--dump`, or the bucket totals.
pub fn text(sdk: &Contract, solang: &Contract, mode: Option<&str>) {
    match mode {
        Some("--detail") => detail(&collect(sdk, solang)),
        Some("--dump") => dump(sdk, solang),
        _ => default(&collect(sdk, solang)),
    }
}

/// Every bucket with its opcodes, so `--detail` adds nothing here. `--dump` has no JSON.
pub fn json(d: &Data) -> Value {
    let diff = |s: u64, l: u64| s as i64 - l as i64;

    let buckets: Vec<Value> = d
        .buckets
        .iter()
        .map(|b| json!({ "kind": b.name, "sdk": b.sdk, "solang": b.solang, "diff": diff(b.sdk, b.solang) }))
        .collect();

    let opcodes: Vec<Value> = d
        .buckets
        .iter()
        .flat_map(|b| b.opcodes.iter().map(move |c| (b.name, c)))
        .map(|(bucket, c)| {
            json!({
                "name": c.name,
                "bucket": bucket,
                "sdk": c.sdk,
                "solang": c.solang,
                "diff": diff(c.sdk, c.solang),
            })
        })
        .collect();

    let (s, l) = d.total;
    json!({
        "buckets": buckets,
        "opcodes": opcodes,
        "total": { "sdk": s, "solang": l, "diff": diff(s, l) },
        "sdk_only": d.sdk_only,
        "solang_only": d.solang_only,
    })
}

/// One row per bucket, skipping those empty on both sides.
fn default(d: &Data) {
    super::banner("opcode histogram");
    println!("{:<20} {:>7} {:>7} {:>7}", "kind", "sdk", "solang", "diff");

    for b in &d.buckets {
        row(b.name, b.sdk, b.solang, "");
    }
    row("total", d.total.0, d.total.1, "");
    println!();
}

/// As `default`, plus each bucket's individual opcodes and the one-sided ones.
fn detail(d: &Data) {
    super::banner("opcode histogram");
    println!("{:<20} {:>7} {:>7} {:>7}", "kind", "sdk", "solang", "diff");

    for b in &d.buckets {
        println!();
        row(&heading(b.name), b.sdk, b.solang, "");

        for c in &b.opcodes {
            row(c.name, c.sdk, c.solang, marker(c.sdk, c.solang));
        }
    }

    println!();
    row("total", d.total.0, d.total.1, "");

    println!();
    println!("exclusive opcodes (present on only one side)");
    println!("sdk only:    {}", join(&d.sdk_only));
    println!("solang only: {}", join(&d.solang_only));
    println!();
}

const COL: usize = 60;

/// Side-by-side disassembly: paired exports first, one-sided next, internals last.
fn dump(sdk: &Contract, solang: &Contract) {
    super::banner("code dump");

    let (sdk, solang) = (sdk.functions(), solang.functions());

    let sdk_exp = exported(sdk);
    let solang_exp = exported(solang);

    let empty: Vec<String> = Vec::new();
    println!("{} │ {}", fit("sdk", COL), fit("solang", COL));

    for f in sdk {
        let Some(name) = &f.name else { continue };
        let Some(g) = solang_exp.get(name.as_str()) else {
            continue;
        };
        println!("\n────  fn: {name} ──── ");
        columns(&f.lines(), &g.lines());
    }

    for f in sdk {
        let Some(name) = &f.name else { continue };
        if solang_exp.contains_key(name.as_str()) {
            continue;
        }
        println!("\n────  fn: {name} (sdk only) ────");
        columns(&f.lines(), &empty);
    }

    for f in solang {
        let Some(name) = &f.name else { continue };
        if sdk_exp.contains_key(name.as_str()) {
            continue;
        }
        println!("\n──── fn: {name} (solang only) ────");
        columns(&empty, &f.lines());
    }

    rule("internal functions");
    columns(&internal_lines(sdk), &internal_lines(solang));
    println!();
}

/// Two line lists as side-by-side columns, the shorter padded with blanks.
fn columns(a: &[String], b: &[String]) {
    for i in 0..a.len().max(b.len()) {
        let left = fit(a.get(i).map(String::as_str).unwrap_or(""), COL);
        let right = fit(b.get(i).map(String::as_str).unwrap_or(""), COL);
        println!("{left} │ {right}");
    }
}

/// Dump lines for every internal function, each under a `function #N` header.
fn internal_lines(fns: &[code::Function]) -> Vec<String> {
    let mut out = Vec::new();

    for f in fns {
        if f.name.is_some() {
            continue;
        }
        out.push(format!("function #{}", f.index));
        out.extend(f.lines());
        out.push(String::new());
    }
    out
}

/// Exported functions, indexed by name.
fn exported(fns: &[code::Function]) -> HashMap<&str, &code::Function> {
    fns.iter()
        .filter_map(|f| f.name.as_deref().map(|n| (n, f)))
        .collect()
}

/// A full-width `── text ──` divider.
fn rule(text: &str) {
    let text = format!(" {text} ");
    let pad = super::WIDTH.saturating_sub(text.chars().count());

    let left = "─".repeat(pad / 2);
    let right = "─".repeat(pad - pad / 2);

    println!("{left}{text}{right}\n");
}

/// Left-justify `s` to width `w`, truncating with `…`.
fn fit(s: &str, w: usize) -> String {
    if s.chars().count() <= w {
        format!("{s:<w$}")
    } else {
        let t: String = s.chars().take(w - 1).collect();
        format!("{t}…")
    }
}

/// Count each opcode name across every function body.
fn feed(fns: &[code::Function]) -> BTreeMap<&str, u64> {
    let mut m = BTreeMap::new();
    for op in fns.iter().flat_map(code::Function::ops) {
        *m.entry(op).or_insert(0) += 1;
    }
    m
}

/// Total count of the opcodes in bucket `b`.
fn sum_bucket(counts: &BTreeMap<&str, u64>, b: &str) -> u64 {
    counts
        .iter()
        .filter(|(n, _)| bucket(n) == b)
        .map(|(_, c)| c)
        .sum()
}

/// Sorted opcode names in bucket `b`, present on either side.
fn opcodes_in<'a>(
    b: &str,
    sdk: &BTreeMap<&'a str, u64>,
    solang: &BTreeMap<&'a str, u64>,
) -> Vec<&'a str> {
    let mut names: Vec<&str> = sdk
        .keys()
        .chain(solang.keys())
        .filter(|n| bucket(n) == b)
        .copied()
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The opcodes present on only one side, sdk's first.
fn exclusives<'a>(
    sdk: &BTreeMap<&'a str, u64>,
    solang: &BTreeMap<&'a str, u64>,
) -> (Vec<&'a str>, Vec<&'a str>) {
    let sdk_only: Vec<&str> = sdk
        .keys()
        .filter(|n| !solang.contains_key(*n))
        .copied()
        .collect();
    let solang_only: Vec<&str> = solang
        .keys()
        .filter(|n| !sdk.contains_key(*n))
        .copied()
        .collect();
    (sdk_only, solang_only)
}

/// Comma-separated list, or `—` when empty.
fn join(names: &[&str]) -> String {
    if names.is_empty() {
        "—".to_string()
    } else {
        names.join(", ")
    }
}

/// Trailing note when an opcode is one-sided.
fn marker(sdk: u64, solang: u64) -> &'static str {
    match (sdk, solang) {
        (_, 0) => "sdk only",
        (0, _) => "solang only",
        _ => "",
    }
}

/// A bucket name wrapped in a rule, e.g. `── control flow ────`.
fn heading(name: &str) -> String {
    let base = format!("── {name} ");
    let pad = 20usize.saturating_sub(base.chars().count());
    format!("{base}{}", "─".repeat(pad))
}

/// One table line.
fn row(label: &str, sdk: u64, solang: u64, marker: &str) {
    let diff = super::signed(sdk as i64 - solang as i64);
    if marker.is_empty() {
        println!("{label:<20} {sdk:>7} {solang:>7} {diff:>7}");
    } else {
        println!("{label:<20} {sdk:>7} {solang:>7} {diff:>7}   {marker}");
    }
}

/// Which bucket an opcode belongs to.
fn bucket(name: &str) -> &'static str {
    match name {
        "Call" | "CallIndirect" => "calls",

        "Unreachable" | "Nop" | "Block" | "Loop" | "If" | "Else" | "End" | "Br" | "BrIf"
        | "BrTable" | "Return" => "control flow",

        "LocalGet" | "LocalSet" | "LocalTee" | "GlobalGet" | "GlobalSet" => "local/global",

        n if n.starts_with("ReturnCall") => "calls",
        n if n.contains("Load") => "loads",
        n if n.contains("Store") => "stores",
        n if n.ends_with("Const") => "const",
        n if is_numtype(n) => "numeric",

        _ => "memory/other",
    }
}

/// A numeric op: type-prefixed, and not claimed by the arms above.
fn is_numtype(name: &str) -> bool {
    ["I32", "I64", "F32", "F64"]
        .iter()
        .any(|p| name.starts_with(p))
}
