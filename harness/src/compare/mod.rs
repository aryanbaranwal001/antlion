use std::{fs, path::Path};

mod cost;
mod functions;
mod imports;
mod interface;
mod layout;
mod opcodes;
mod returns;
mod section;
mod size;

use crate::wasm::Contract;

const SECTIONS: [&str; 9] = [
    "size",
    "sections",
    "imports",
    "interface",
    "returns",
    "cost",
    "opcodes",
    "functions",
    "layout",
];
const WIDTH: usize = 90;

/// Run the requested reports over every named contract, in the order given.
pub fn run(args: &[String]) {
    let usage = format!(
        "[err] usage: --compare <name>... --report {}|all [--detail|--dump]",
        SECTIONS.join("|")
    );

    let names: Vec<&String> = args.iter().take_while(|a| !a.starts_with("--")).collect();
    if names.is_empty() {
        panic!("[err] no contract given\n{usage}");
    }

    let Some(("--report", rest)) = args[names.len()..]
        .split_first()
        .map(|(flag, rest)| (flag.as_str(), rest))
    else {
        panic!("[err] no report requested — pass `--report all` for every report\n{usage}");
    };

    let (mode, reports) = match rest.split_last() {
        Some((last, elements)) if last == "--detail" || last == "--dump" => {
            (Some(last.as_str()), elements)
        }
        _ => (None, rest),
    };

    let requested: Vec<&str> = reports.iter().map(String::as_str).collect();
    if requested.is_empty() {
        panic!("[err] `--report` needs a report name — pass `all` for every report\n{usage}");
    }

    let selected: Vec<&str> = if requested.contains(&"all") {
        SECTIONS.to_vec()
    } else {
        requested
    };

    for name in names {
        let (sdk_wasm, solang_wasm) = load(name);
        let sdk = Contract::load(&sdk_wasm);
        let solang = Contract::load(&solang_wasm);

        title(name);

        for s in &selected {
            match *s {
                "size" => size::report(&sdk, &solang),
                "sections" => section::report(&sdk, &solang),
                "imports" => imports::report(&sdk, &solang),
                "interface" => interface::report(&sdk, &solang),
                "returns" => returns::report(name, &sdk, &solang),
                "cost" => cost::report(name, &sdk, &solang),
                "opcodes" => opcodes::report(&sdk, &solang, mode),
                "functions" => functions::report(&sdk, &solang),
                "layout" => layout::report(&sdk, &solang),

                other => panic!(
                    "[err] unknown report `{other}` — pick from: {}, all",
                    SECTIONS.join(", ")
                ),
            }
        }
    }
}

/// Both builds of `name`, read from `out/`.
fn load(name: &str) -> (Vec<u8>, Vec<u8>) {
    let sdk_path = format!("out/{name}/sdk.wasm");
    let solang_path = format!("out/{name}/solang.wasm");

    if !Path::new(&sdk_path).exists() || !Path::new(&solang_path).exists() {
        panic!("[err] no build for `{name}` in out/{name}/");
    }

    (
        fs::read(&sdk_path).expect("[err] failed to read sdk.wasm"),
        fs::read(&solang_path).expect("[err] failed to read solang.wasm"),
    )
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
