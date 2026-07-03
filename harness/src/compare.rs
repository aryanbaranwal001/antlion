use std::{fs, path::Path, process};

pub fn run(name: &str) {
    let sdk_path = format!("out/{name}/sdk.wasm");
    let solang_path = format!("out/{name}/solang.wasm");

    if !Path::new(&sdk_path).exists() || !Path::new(&solang_path).exists() {
        eprintln!("error: built wasm for `{name}` not found in out/{name}/");
        eprintln!("       expected both {sdk_path} and {solang_path}");
        eprintln!("       build them first: cargo run -- --build {name}");
        process::exit(1);
    }

    let sdk = fs::read(&sdk_path).expect("read sdk.wasm");
    let solang = fs::read(&solang_path).expect("read solang.wasm");

    // total size
    println!("== {name}: total size ==");
    println!("  solang {:>6} bytes", solang.len());
    println!("  sdk    {:>6} bytes", sdk.len());
    println!(
        "  diff   {:>6} (solang - sdk)\n",
        signed(solang.len() as i64 - sdk.len() as i64)
    );

    // bytes section
    let sdk_secs = aggregate(&sections(&sdk));
    let solang_secs = aggregate(&sections(&solang));

    println!("== {name}: section byte breakdown ==");
    println!(
        "  {:<24} {:>7} {:>7} {:>7}",
        "section", "sdk", "solang", "diff"
    );
    for name in section_order(&sdk_secs, &solang_secs) {
        let a = size_of(&sdk_secs, &name);
        let b = size_of(&solang_secs, &name);
        println!(
            "  {:<24} {:>7} {:>7} {:>7}",
            name,
            a,
            b,
            signed(b as i64 - a as i64)
        );
    }
    println!(
        "  {:<24} {:>7} {:>7} {:>7}",
        "total",
        sdk.len(),
        solang.len(),
        signed(solang.len() as i64 - sdk.len() as i64)
    );
}

struct Section {
    name: String,
    size: usize, // payload bytes
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
            let (nlen, nread) = read_leb128(&bytes[i..]);
            let start = i + nread;
            let sname = String::from_utf8_lossy(&bytes[start..start + nlen as usize]);
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

fn signed(d: i64) -> String {
    if d >= 0 {
        format!("+{d}")
    } else {
        d.to_string()
    }
}
