use crate::wasm::{Contract, code};
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

/// Dispatch on the trailing flag: `--detail`, `--dump`, or the bucket totals.
pub fn report(sdk: &Contract, solang: &Contract, mode: Option<&str>) {
    match mode {
        Some("--detail") => detail(sdk, solang),
        Some("--dump") => dump(sdk, solang),
        _ => default(sdk, solang),
    }
}

/// One row per bucket, skipping those empty on both sides.
fn default(sdk: &Contract, solang: &Contract) {
    super::banner("opcode histogram");
    println!("{:<20} {:>7} {:>7} {:>7}", "kind", "sdk", "solang", "diff");

    let sdk = feed(sdk.functions());
    let solang = feed(solang.functions());

    let (mut sdk_total, mut solang_total) = (0u64, 0u64);
    for b in BUCKETS {
        let s = sum_bucket(&sdk, b);
        let l = sum_bucket(&solang, b);
        if s == 0 && l == 0 {
            continue;
        }
        row(b, s, l, "");
        sdk_total += s;
        solang_total += l;
    }
    row("total", sdk_total, solang_total, "");
    println!();
}

/// As `default`, plus each bucket's individual opcodes and the one-sided ones.
fn detail(sdk: &Contract, solang: &Contract) {
    super::banner("opcode histogram");
    println!("{:<20} {:>7} {:>7} {:>7}", "kind", "sdk", "solang", "diff");

    let sdk = feed(sdk.functions());
    let solang = feed(solang.functions());

    let (mut sdk_total, mut solang_total) = (0u64, 0u64);
    for b in BUCKETS {
        let s = sum_bucket(&sdk, b);
        let l = sum_bucket(&solang, b);
        if s == 0 && l == 0 {
            continue;
        }

        println!();
        row(&heading(b), s, l, "");
        sdk_total += s;
        solang_total += l;

        for name in opcodes_in(b, &sdk, &solang) {
            let s = *sdk.get(name).unwrap_or(&0);
            let l = *solang.get(name).unwrap_or(&0);
            row(name, s, l, marker(s, l));
        }
    }

    println!();
    row("total", sdk_total, solang_total, "");
    exclusives(&sdk, &solang);
    println!();
}

const COL: usize = 44;

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

/// The opcodes present on only one side.
fn exclusives(sdk: &BTreeMap<&str, u64>, solang: &BTreeMap<&str, u64>) {
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

    println!();
    println!("exclusive opcodes (present on only one side)");
    println!("sdk only:    {}", join(&sdk_only));
    println!("solang only: {}", join(&solang_only));
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
