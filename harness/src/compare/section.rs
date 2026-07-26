use crate::wasm::{Contract, Section};

/// Compare the two modules section by section, in bytes.
pub fn report(sdk: &Contract, solang: &Contract) {
    super::banner("aggregate section byte breakdown");
    println!(
        "{:<24} {:>7} {:>7} {:>7}",
        "section", "sdk", "solang", "diff"
    );

    let sdk_secs = aggregate(&sdk.sections);
    let solang_secs = aggregate(&solang.sections);

    for name in section_order(&sdk_secs, &solang_secs) {
        let a = size_of(&sdk_secs, &name);
        let b = size_of(&solang_secs, &name);
        println!(
            "{:<24} {:>7} {:>7} {:>7}",
            name,
            a,
            b,
            super::signed(a as i64 - b as i64)
        );
    }
    println!();
}

/// Sum sizes per name, keeping first-seen order. Only custom sections can repeat —
/// every other id is allowed at most once per module.
fn aggregate(secs: &[Section]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for s in secs {
        match out.iter_mut().find(|(n, _)| *n == s.name) {
            Some(e) => e.1 += s.size,
            None => out.push((s.name.clone(), s.size)),
        }
    }
    out
}

/// Row order for the table: the longer side's order, then whatever only the other
/// side has, appended.
fn section_order(a: &[(String, usize)], b: &[(String, usize)]) -> Vec<String> {
    let (base, extra) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut order: Vec<String> = base.iter().map(|(n, _)| n.clone()).collect();
    for (n, _) in extra {
        if !order.contains(n) {
            order.push(n.clone());
        }
    }
    order
}

/// Size of the named section, or 0 if that side doesn't have it.
fn size_of(secs: &[(String, usize)], name: &str) -> usize {
    secs.iter().find(|(n, _)| n == name).map_or(0, |(_, s)| *s)
}
