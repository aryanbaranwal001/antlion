pub fn report(name: &str, sdk: &[u8], solang: &[u8]) {
    super::banner(&format!("{name}: section byte breakdown"));
    println!(
        "  {:<24} {:>7} {:>7} {:>7}",
        "section", "sdk", "solang", "diff"
    );

    let sdk_secs = aggregate(&sections(sdk));
    let solang_secs = aggregate(&sections(solang));

    for name in section_order(&sdk_secs, &solang_secs) {
        let a = size_of(&sdk_secs, &name);
        let b = size_of(&solang_secs, &name);
        println!(
            "  {:<24} {:>7} {:>7} {:>7}",
            name,
            a,
            b,
            super::signed(a as i64 - b as i64)
        );
    }
}

struct Section {
    name: String,
    size: usize,
}

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

fn section_order(a: &[(String, usize)], b: &[(String, usize)]) -> Vec<String> {
    let mut order: Vec<String> = a.iter().map(|(n, _)| n.clone()).collect();
    for (n, _) in b {
        if !order.contains(n) {
            order.push(n.clone());
        }
    }
    order
}

fn size_of(secs: &[(String, usize)], name: &str) -> usize {
    secs.iter().find(|(n, _)| n == name).map_or(0, |(_, s)| *s)
}

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

// u32: actual number
// usize: bytes containing that number
fn read_leb128(bytes: &[u8]) -> (u32, usize) {
    let mut result = 0u32;
    let mut shift = 0;
    let mut i = 0;

    loop {
        let byte = bytes[i];
        result |= ((byte & 0x7f) as u32) << shift;
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    (result, i)
}
