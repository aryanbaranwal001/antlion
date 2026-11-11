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

use crate::cli::{self, Mode};
use crate::error::{Error, Result};
use crate::wasm::Contract;

pub const REPORTS: [&str; 10] = [
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
pub fn run(args: &cli::Compare) -> Result<()> {
    let json = args.json;
    let mut pairs = Vec::new();
    let mut no_json = Vec::new();

    for name in &args.names {
        let (sdk_wasm, solang_wasm) = load(name)?;
        let sdk = Contract::load(&sdk_wasm);
        let solang = Contract::load(&solang_wasm);

        // In JSON mode each report adds its key here; in text mode it prints.
        let mut pair = json.then(|| Map::from_iter([("name".to_string(), json!(name))]));
        if !json {
            title(name);
        }

        for s in &args.reports {
            let out = &mut pair;
            match s.as_str() {
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
                    None => opcodes::text(&sdk, &solang, args.mode),
                    // A disassembly is not numbers, so it has no JSON form.
                    Some(_) if args.mode == Mode::Dump => {
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

                other => {
                    return Err(Error::usage(format!(
                        "unknown report `{other}` — pick from: {}, all",
                        REPORTS.join(", ")
                    )));
                }
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
    Ok(())
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
fn load(name: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let sdk_path = format!("out/{name}/sdk.wasm");
    let solang_path = format!("out/{name}/solang.wasm");

    if !Path::new(&sdk_path).exists() || !Path::new(&solang_path).exists() {
        return Err(Error::failed(format!("no build for `{name}` in out/{name}/")));
    }

    Ok((
        fs::read(&sdk_path).map_err(|_| Error::failed("failed to read sdk.wasm"))?,
        fs::read(&solang_path).map_err(|_| Error::failed("failed to read solang.wasm"))?,
    ))
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
