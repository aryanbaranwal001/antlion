use crate::wasm::{Contract, Section};

/// Compare the two modules section by section, in bytes.
pub fn report(sdk: &Contract, solang: &Contract) {
    super::banner("aggregate section byte breakdown");
    println!(
        "{:<24} {:>7} {:>7} {:>7}",
        "section", "sdk", "solang", "diff"
    );

    let sdk_secs = sizes_by_name(&sdk.sections);
    let solang_secs = sizes_by_name(&solang.sections);

    for name in section_order(&sdk_secs, &solang_secs) {
        let a = sizes_of(&sdk_secs, &name);
        let b = sizes_of(&solang_secs, &name);
        line(&name, Some(a.iter().sum()), Some(b.iter().sum()));
        occurrences(a, b);
    }
    println!();
}

/// Every size under each name, in file order. Only custom sections can repeat, so
/// most names carry exactly one.
fn sizes_by_name(secs: &[Section]) -> Vec<(String, Vec<usize>)> {
    let mut out: Vec<(String, Vec<usize>)> = Vec::new();
    for s in secs {
        match out.iter_mut().find(|(n, _)| *n == s.name) {
            Some(e) => e.1.push(s.size),
            None => out.push((s.name.clone(), vec![s.size])),
        }
    }
    out
}

/// Break a repeated name into one indented row per occurrence, pairing them by
/// file order. Silent for the usual case of a single occurrence.
fn occurrences(sdk: &[usize], solang: &[usize]) {
    let n = sdk.len().max(solang.len());
    if n < 2 {
        return;
    }

    for i in 0..n {
        let tee = if i + 1 == n { '└' } else { '├' };
        line(
            &format!("  {tee} ({})", i + 1),
            sdk.get(i).copied(),
            solang.get(i).copied(),
        );
    }
}

/// Row order: the longer side's order, then whatever only the other side has.
fn section_order(a: &[(String, Vec<usize>)], b: &[(String, Vec<usize>)]) -> Vec<String> {
    let (base, extra) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut order: Vec<String> = base.iter().map(|(n, _)| n.clone()).collect();
    for (n, _) in extra {
        if !order.contains(n) {
            order.push(n.clone());
        }
    }
    order
}

/// Sizes under the named section, or empty if absent.
fn sizes_of<'a>(secs: &'a [(String, Vec<usize>)], name: &str) -> &'a [usize] {
    secs.iter()
        .find(|(n, _)| n == name)
        .map_or(&[], |(_, s)| s.as_slice())
}

/// One table line. A side without that occurrence prints `—`.
fn line(label: &str, sdk: Option<usize>, solang: Option<usize>) {
    let (a, b, diff) = match (sdk, solang) {
        (Some(a), Some(b)) => (
            a.to_string(),
            b.to_string(),
            super::signed(a as i64 - b as i64),
        ),
        (Some(a), None) => (a.to_string(), "—".to_string(), "—".to_string()),
        (None, Some(b)) => ("—".to_string(), b.to_string(), "—".to_string()),
        (None, None) => return,
    };
    println!("{label:<24} {a:>7} {b:>7} {diff:>7}");
}
