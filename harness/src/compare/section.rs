use crate::wasm::read_leb128;

/// Compare the two modules section by section, in bytes.
pub fn report(sdk: &[u8], solang: &[u8]) {
    super::banner("aggregate section byte breakdown");
    println!(
        "{:<24} {:>7} {:>7} {:>7}",
        "section", "sdk", "solang", "diff"
    );

    let sdk_secs = aggregate(&sections(sdk));
    let solang_secs = aggregate(&sections(solang));

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

struct Section {
    name: String,
    size: usize,
}

/// Every section in the module, in file order. Custom sections (id 0) carry a name
/// of their own, so they're reported as `custom:<name>` rather than lumped together.
fn sections(bytes: &[u8]) -> Vec<Section> {
    let mut out = Vec::new();
    let mut i = 8;

    while i < bytes.len() {
        let id = bytes[i];
        i += 1;

        let (size, len) = read_leb128(&bytes[i..]);

        i += len;
        let size = size as usize;

        let mut name = section_name(id).to_string();

        if id == 0 {
            let (len, bytes_n) = read_leb128(&bytes[i..]);
            let start = i + bytes_n;
            let sname = String::from_utf8_lossy(&bytes[start..start + len as usize]);
            name = format!("custom:{sname}");
        }

        out.push(Section { name, size });
        i += size;
    }
    out
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

/// The spec's name for a section id.
fn section_name(id: u8) -> &'static str {
    match id {
        0 => "custom",
        1 => "type",
        2 => "import",
        3 => "function",
        4 => "table",
        5 => "memory",
        6 => "global",
        7 => "export",
        8 => "start",
        9 => "element",
        10 => "code",
        11 => "data",
        12 => "data_count",
        _ => "unknown",
    }
}
