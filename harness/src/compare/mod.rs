use std::{fs, path::Path};

mod cost;
mod imports;
mod interface;
mod names;
mod section;
mod size;
mod spec;
mod wasm;

use spec::Contract;

const SECTIONS: [&str; 5] = ["size", "bytes", "imports", "interface", "cost"];
const WIDTH: usize = 60;

pub fn run(name: &str, sections: &[String]) {
    let sdk_path = format!("out/{name}/sdk.wasm");
    let solang_path = format!("out/{name}/solang.wasm");

    if !Path::new(&sdk_path).exists() || !Path::new(&solang_path).exists() {
        panic!("no build for `{name}` in out/{name}/ — run `cargo run -- --build {name}` first");
    }

    let sdk_wasm = fs::read(&sdk_path).expect("failed to read sdk.wasm");
    let solang_wasm = fs::read(&solang_path).expect("failed to read solang.wasm");

    let sdk = Contract {
        wasm: &sdk_wasm,
        interface: spec::parse(&sdk_wasm),
    };
    let solang = Contract {
        wasm: &solang_wasm,
        interface: spec::parse(&solang_wasm),
    };

    title(name);

    let selected: Vec<&str> = if sections.is_empty() || sections.iter().any(|s| s == "all") {
        SECTIONS.to_vec()
    } else {
        sections.iter().map(String::as_str).collect()
    };

    for s in selected {
        match s {
            "size" => size::report(sdk.wasm, solang.wasm),
            "bytes" => section::report(sdk.wasm, solang.wasm),
            "imports" => imports::report(sdk.wasm, solang.wasm),
            "interface" => interface::report(&sdk, &solang),
            "cost" => cost::report(name, &sdk, &solang),

            other => panic!(
                "unknown section `{other}` — pick from: {}, all",
                SECTIONS.join(", ")
            ),
        }
    }
}

fn signed(d: i64) -> String {
    if d >= 0 {
        format!("+{d}")
    } else {
        d.to_string()
    }
}

fn banner(text: &str) {
    let label = format!("  {}  ", text.to_uppercase());
    let pad = WIDTH.saturating_sub(label.chars().count());

    let left = "━".repeat(pad / 2);
    let right = "━".repeat(pad - pad / 2);

    println!("{left}{label}{right}\n");
}

fn title(name: &str) {
    let inner = WIDTH - 2;
    let line = "═".repeat(inner);
    let name = name.to_uppercase();
    let pad = inner.saturating_sub(name.chars().count());

    let left = " ".repeat(pad / 2);
    let right = " ".repeat(pad - pad / 2);

    println!("╔{line}╗");
    println!("║{left}{name}{right}║");
    println!("╚{line}╝\n");
}
