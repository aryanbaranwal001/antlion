use std::{fs, path::Path};

use serde_json::{Map, Value, json};

mod access;
mod call;
mod cost;
mod events;
mod functions;
mod imports;
mod interface;
mod layout;
mod ledger;
mod meta;
mod opcodes;
mod returns;
mod scaling;
mod section;
mod sequence;
mod size;
mod trace;

use crate::cli::{self, Mode};
use crate::error::{Error, Result};
use crate::wasm::Contract;

/// One of the reports `--compare` can run.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Report {
    Size,
    Sections,
    Imports,
    Interface,
    Meta,
    Returns,
    Cost,
    Scaling,
    Ledger,
    Access,
    Trace,
    Sequence,
    Events,
    Opcodes,
    Functions,
    Layout,
}

impl Report {
    /// Every report, in the order `all` runs them.
    pub const ALL: [Report; 16] = [
        Report::Size,
        Report::Sections,
        Report::Imports,
        Report::Interface,
        Report::Meta,
        Report::Returns,
        Report::Cost,
        Report::Scaling,
        Report::Ledger,
        Report::Access,
        Report::Trace,
        Report::Sequence,
        Report::Events,
        Report::Opcodes,
        Report::Functions,
        Report::Layout,
    ];

    /// The name used on the command line and as the JSON key.
    pub fn name(self) -> &'static str {
        match self {
            Report::Size => "size",
            Report::Sections => "sections",
            Report::Imports => "imports",
            Report::Interface => "interface",
            Report::Meta => "meta",
            Report::Returns => "returns",
            Report::Cost => "cost",
            Report::Scaling => "scaling",
            Report::Ledger => "ledger",
            Report::Access => "access",
            Report::Trace => "trace",
            Report::Sequence => "sequence",
            Report::Events => "events",
            Report::Opcodes => "opcodes",
            Report::Functions => "functions",
            Report::Layout => "layout",
        }
    }

    /// The report with this name.
    pub fn parse(s: &str) -> Result<Report> {
        Report::ALL
            .into_iter()
            .find(|r| r.name() == s)
            .ok_or_else(|| {
                let names: Vec<&str> = Report::ALL.iter().map(|r| r.name()).collect();
                Error::usage(format!(
                    "unknown report `{s}` — pick from: {}, all",
                    names.join(", ")
                ))
            })
    }
}

const WIDTH: usize = 90;

/// Bumped when the shape of the `--json` document changes.
const SCHEMA_VERSION: u32 = 8;

/// Run the requested reports over every named contract, in the order given.
pub fn run(args: &cli::Compare) -> Result<()> {
    pool(args.jobs.unwrap_or_else(default_jobs))?.install(|| all(args))
}

/// Threads for the reports that call contracts when `--jobs` is not given: 4, or fewer
/// when the machine has fewer cores. One when the number of cores cannot be read.
fn default_jobs() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().min(4))
}

/// A pool of `jobs` threads. If they cannot be started, for example where the system
/// allows no more threads, the pool runs on this thread alone: slower, same results.
fn pool(jobs: usize) -> Result<rayon::ThreadPool> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .or_else(|e| {
            eprintln!(
                "[warn] could not start {jobs} threads ({e}), so every call runs on the main thread"
            );
            rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .use_current_thread()
                .build()
        })
        .map_err(|e| Error::failed(format!("failed to start threads: {e}")))
}

/// Every pair, inside the pool.
fn all(args: &cli::Compare) -> Result<()> {
    let json = args.json;
    let mut pairs = Vec::new();
    let mut no_json = Vec::new();
    // A pair that cannot be compared is reported and skipped, so the others still run.
    let mut failed: Vec<(&str, String)> = Vec::new();

    for name in &args.names {
        match pair(name, args, &mut no_json) {
            Ok(Some(p)) => pairs.push(p),
            Ok(None) => {}
            Err(Error::Failed(msg)) => {
                eprintln!("[err] {name}: {msg}");
                failed.push((name, msg));
            }
            Err(e) => return Err(e),
        }
    }

    if json {
        let doc = json!({
            "schema_version": SCHEMA_VERSION,
            "antlion": crate::VERSION.trim_start_matches("antlion version "),
            "pairs": pairs,
            "no_json": no_json,
            "failed": failed
                .iter()
                .map(|(name, error)| json!({ "name": name, "error": error }))
                .collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).expect("[err] failed to write the JSON")
        );
    }

    if !failed.is_empty() {
        let names: Vec<&str> = failed.iter().map(|(name, _)| *name).collect();
        return Err(Error::failed(format!(
            "{} of {} pairs failed: {}",
            failed.len(),
            args.names.len(),
            names.join(", ")
        )));
    }
    Ok(())
}

/// Compare one pair. In JSON mode the result is the pair's object; in text mode the
/// reports are printed and the result is `None`.
fn pair(name: &str, args: &cli::Compare, no_json: &mut Vec<String>) -> Result<Option<Value>> {
    let (sdk_wasm, solang_wasm) = load(name)?;
    let sdk =
        Contract::load(&sdk_wasm).map_err(|e| Error::failed(format!("{name} sdk.wasm: {e}")))?;
    let solang = Contract::load(&solang_wasm)
        .map_err(|e| Error::failed(format!("{name} solang.wasm: {e}")))?;

    // In JSON mode each report adds its key here; in text mode it prints.
    let mut pair = args
        .json
        .then(|| Map::from_iter([("name".to_string(), json!(name))]));
    if !args.json {
        title(name);
    }

    for &r in &args.reports {
        let out = &mut pair;
        match r {
            Report::Size => emit(
                r.name(),
                size::collect(&sdk, &solang),
                size::text,
                size::json,
                out,
            ),
            Report::Sections => emit(
                r.name(),
                section::collect(&sdk, &solang),
                |d| section::text(d),
                |d| section::json(d),
                out,
            ),
            Report::Imports => emit(
                r.name(),
                imports::collect(&sdk, &solang),
                imports::text,
                imports::json,
                out,
            ),
            Report::Interface => emit(
                r.name(),
                interface::collect(&sdk, &solang),
                |d| interface::text(d),
                |d| interface::json(d),
                out,
            ),
            Report::Meta => emit(
                r.name(),
                meta::collect(&sdk, &solang),
                meta::text,
                meta::json,
                out,
            ),
            Report::Returns => emit(
                r.name(),
                returns::collect(&sdk, &solang),
                |d| returns::text(name, d),
                returns::json,
                out,
            ),
            Report::Cost => emit(
                r.name(),
                cost::collect(&sdk, &solang),
                |d| cost::text(name, d, args.mode),
                cost::json,
                out,
            ),
            Report::Scaling => emit(
                r.name(),
                scaling::collect(&sdk, &solang),
                |d| scaling::text(name, d),
                scaling::json,
                out,
            ),
            Report::Ledger => emit(
                r.name(),
                ledger::collect(&sdk, &solang),
                |d| ledger::text(name, d),
                ledger::json,
                out,
            ),
            Report::Access => emit(
                r.name(),
                access::collect(&sdk, &solang),
                |d| access::text(name, d),
                access::json,
                out,
            ),
            Report::Trace => emit(
                r.name(),
                trace::collect(&sdk, &solang),
                |d| trace::text(name, d),
                trace::json,
                out,
            ),
            Report::Sequence => emit(
                r.name(),
                sequence::collect(&sdk, &solang),
                |d| sequence::text(name, d),
                sequence::json,
                out,
            ),
            Report::Events => emit(
                r.name(),
                events::collect(&sdk, &solang),
                |d| events::text(name, d),
                events::json,
                out,
            ),
            Report::Opcodes => match out {
                None => opcodes::text(&sdk, &solang, args.mode),
                // A disassembly is not numbers, so it has no JSON form.
                Some(_) if args.mode == Mode::Dump => {
                    let note = "opcodes --dump".to_string();
                    if !no_json.contains(&note) {
                        no_json.push(note);
                    }
                }
                Some(m) => {
                    m.insert(
                        r.name().to_string(),
                        opcodes::json(&opcodes::collect(&sdk, &solang)),
                    );
                }
            },
            Report::Functions => emit(
                r.name(),
                functions::collect(&sdk, &solang),
                functions::text,
                functions::json,
                out,
            ),
            Report::Layout => emit(
                r.name(),
                layout::collect(&sdk, &solang),
                layout::text,
                layout::json,
                out,
            ),
        }
    }

    Ok(pair.map(Value::Object))
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
        return Err(Error::failed(format!(
            "no build for `{name}` in out/{name}/"
        )));
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

/// The three cells of a row: each side, then `sdk` minus `solang`. A missing side is `—`,
/// and so is the difference.
fn cells(sdk: Option<usize>, solang: Option<usize>) -> (String, String, String) {
    let dash = || "—".to_string();
    match (sdk, solang) {
        (Some(s), Some(l)) => (s.to_string(), l.to_string(), signed(s as i64 - l as i64)),
        (Some(s), None) => (s.to_string(), dash(), dash()),
        (None, Some(l)) => (dash(), l.to_string(), dash()),
        (None, None) => (dash(), dash(), dash()),
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
