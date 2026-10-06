use std::{fs, path::Path};

use serde_json::{Map, Value, json};

mod cost;
mod functions;
mod imports;
mod interface;
mod layout;
mod ledger;
mod opcodes;
mod returns;
mod section;
mod size;

use crate::wasm::Contract;

const SECTIONS: [&str; 10] = [
    "size",
    "sections",
    "imports",
    "interface",
    "returns",
    "cost",
    "ledger",
    "opcodes",
    "functions",
    "layout",
];
const WIDTH: usize = 90;

/// Bumped when the shape of the `--json` document changes.
const SCHEMA_VERSION: u32 = 1;

/// Run the requested reports over every named contract, in the order given.
pub fn run(args: &[String]) {
    let usage = format!(
        "[err] usage: --compare <name>... --report {}|all [--json] [--detail|--dump]",
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

    // Trailing flags, in any order: `--json`, and one of `--detail` or `--dump`.
    let (mut reports, mut json, mut mode) = (rest, false, None);
    while let Some((last, elements)) = reports.split_last() {
        match last.as_str() {
            "--json" if !json => json = true,
            "--detail" | "--dump" if mode.is_none() => mode = Some(last.as_str()),
            _ => break,
        }
        reports = elements;
    }

    let requested: Vec<&str> = reports.iter().map(String::as_str).collect();
    if requested.is_empty() {
        panic!("[err] `--report` needs a report name — pass `all` for every report\n{usage}");
    }

    let selected: Vec<&str> = if requested.contains(&"all") {
        SECTIONS.to_vec()
    } else {
        requested
    };

    let mut pairs = Vec::new();
    let mut no_json = Vec::new();

    for name in names {
        let (sdk_wasm, solang_wasm) = load(name);
        let sdk = Contract::load(&sdk_wasm);
        let solang = Contract::load(&solang_wasm);

        // In JSON mode each report adds its key here; in text mode it prints.
        let mut pair = json.then(|| Map::from_iter([("name".to_string(), json!(name))]));
        if !json {
            title(name);
        }

        for s in &selected {
            let out = &mut pair;
            match *s {
                "size" => emit(s, size::collect(&sdk, &solang), size::text, size::json, out),
                "sections" => emit(
                    s,
                    section::collect(&sdk, &solang),
                    |d| section::text(d),
                    |d| section::json(d),
                    out,
                ),
                "imports" => emit(
                    s,
                    imports::collect(&sdk, &solang),
                    imports::text,
                    imports::json,
                    out,
                ),
                "interface" => emit(
                    s,
                    interface::collect(&sdk, &solang),
                    |d| interface::text(d),
                    |d| interface::json(d),
                    out,
                ),
                "returns" => emit(
                    s,
                    returns::collect(&sdk, &solang),
                    |d| returns::text(name, d),
                    returns::json,
                    out,
                ),
                "cost" => emit(
                    s,
                    cost::collect(&sdk, &solang),
                    |d| cost::text(name, d),
                    cost::json,
                    out,
                ),
                "ledger" => emit(
                    s,
                    ledger::collect(&sdk, &solang),
                    |d| ledger::text(name, d),
                    ledger::json,
                    out,
                ),
                "opcodes" => match out {
                    None => opcodes::text(&sdk, &solang, mode),
                    // A disassembly is not numbers, so it has no JSON form.
                    Some(_) if mode == Some("--dump") => {
                        let note = "opcodes --dump".to_string();
                        if !no_json.contains(&note) {
                            no_json.push(note);
                        }
                    }
                    Some(m) => {
                        m.insert(s.to_string(), opcodes::json(&opcodes::collect(&sdk, &solang)));
                    }
                },
                "functions" => emit(
                    s,
                    functions::collect(&sdk, &solang),
                    functions::text,
                    functions::json,
                    out,
                ),
                "layout" => emit(s, layout::collect(&sdk, &solang), layout::text, layout::json, out),

                other => panic!(
                    "[err] unknown report `{other}` — pick from: {}, all",
                    SECTIONS.join(", ")
                ),
            }
        }

        if let Some(pair) = pair {
            pairs.push(Value::Object(pair));
        }
    }

    if json {
        let doc = json!({
            "schema_version": SCHEMA_VERSION,
            "antlion": crate::VERSION.trim_start_matches("antlion version "),
            "pairs": pairs,
            "no_json": no_json,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).expect("[err] failed to write the JSON")
        );
    }
}

/// Render one report's data: as text, or as JSON under its name in `pair`.
fn emit<D>(
    report: &str,
    data: D,
    text: impl Fn(&D),
    json: impl Fn(&D) -> Value,
    pair: &mut Option<Map<String, Value>>,
) {
    match pair {
        Some(m) => {
            m.insert(report.to_string(), json(&data));
        }
        None => text(&data),
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

/// `text` on one line, every run of whitespace a single space.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
