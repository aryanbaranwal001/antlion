use std::{fs, path::Path, process};

mod section;
mod size;

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

    size::report(name, &sdk, &solang);
    section::report(name, &sdk, &solang);
}

fn signed(d: i64) -> String {
    if d >= 0 {
        format!("+{d}")
    } else {
        d.to_string()
    }
}
