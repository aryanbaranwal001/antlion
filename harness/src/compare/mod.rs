use std::{fs, path::Path};

mod cost;
mod imports;
mod interface;
mod opcodes;
mod section;
mod size;

use crate::wasm::Contract;

const SECTIONS: [&str; 6] = ["size", "sections", "imports", "interface", "cost", "opcodes"];
const WIDTH: usize = 60;

/// Load both builds of `name` from `out/` and run the requested reports. `args` may
/// start with `--report <names>`; anything after (the flags) is left for the report
/// itself to read. No `--report`, or `all`, runs every section in `SECTIONS` order.
pub fn run(name: &str, args: impl Iterator<Item = String>) {
    let sdk_path = format!("out/{name}/sdk.wasm");
    let solang_path = format!("out/{name}/solang.wasm");

    if !Path::new(&sdk_path).exists() || !Path::new(&solang_path).exists() {
        panic!("no build for `{name}` in out/{name}/ — run `cargo run -- --build {name}` first");
    }

    let sdk_wasm = fs::read(&sdk_path).expect("failed to read sdk.wasm");
    let solang_wasm = fs::read(&solang_path).expect("failed to read solang.wasm");

    let sdk = Contract::load(&sdk_wasm);
    let solang = Contract::load(&solang_wasm);

    title(name);

    let mut args = args.peekable();
    let mut requested: Vec<String> = Vec::new();
    if matches!(args.peek().map(String::as_str), Some("--report")) {
        args.next();
        while let Some(a) = args.peek() {
            if a.starts_with("--") {
                break;
            }
            requested.push(args.next().unwrap());
        }
    }

    let selected: Vec<&str> = if requested.is_empty() || requested.iter().any(|s| s == "all") {
        SECTIONS.to_vec()
    } else {
        requested.iter().map(String::as_str).collect()
    };

    for s in selected {
        match s {
            "size" => size::report(&sdk, &solang),
            "sections" => section::report(&sdk, &solang),
            "imports" => imports::report(&sdk, &solang),
            "interface" => interface::report(&sdk, &solang),
            "cost" => cost::report(name, &sdk, &solang),
            "opcodes" => opcodes::report(&sdk, &solang, &mut args),

            other => panic!(
                "unknown report `{other}` — pick from: {}, all",
                SECTIONS.join(", ")
            ),
        }
    }
}

/// Format a difference with an explicit sign, e.g. `+12` / `-3`.
fn signed(d: i64) -> String {
    if d >= 0 {
        format!("+{d}")
    } else {
        d.to_string()
    }
}

/// A report's heading: the text upper-cased and centred in a `━━━` rule.
fn banner(text: &str) {
    let label = format!("  {}  ", text.to_uppercase());
    let pad = WIDTH.saturating_sub(label.chars().count());

    let left = "━".repeat(pad / 2);
    let right = "━".repeat(pad - pad / 2);

    println!("{left}{label}{right}\n");
}

/// The contract name boxed in `═` at the top of a run.
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
