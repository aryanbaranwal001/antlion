//! What the lists show: the pairs and their build state, the reports, the options.

use std::{fs, path::Path, time::SystemTime};

use crate::compare::Report;

/// Whether a pair can be compared, and whether its build is up to date.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    /// Both wasm files are in `out/<name>/`, and no source file is newer.
    Built,
    /// Both wasm files are there, but a source file is newer than one of them.
    Stale,
    /// One wasm file or both are missing.
    None,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Built => "built",
            State::Stale => "stale",
            State::None => "none",
        }
    }
}

pub struct Pair {
    pub name: String,
    pub state: State,
    /// The pair's source folder, `contracts/<name>`, when it has one. A pair built from
    /// another path is in `out/` only.
    pub source: bool,
}

/// Every pair under `root`: each folder in `contracts/` and each build in `out/`, by name.
pub fn pairs(root: &Path) -> Vec<Pair> {
    let mut names: Vec<String> = ["contracts", "out"]
        .iter()
        .flat_map(|d| folders(&root.join(d)))
        .collect();
    names.sort();
    names.dedup();

    names
        .into_iter()
        .map(|name| {
            let src = root.join("contracts").join(&name);
            let out = root.join("out").join(&name);
            let built = ["sdk.wasm", "solang.wasm"].map(|f| modified(&out.join(f)));
            let state = match built {
                [Some(a), Some(b)] => {
                    let oldest = a.min(b);
                    if newest(&src).is_some_and(|t| t > oldest) {
                        State::Stale
                    } else {
                        State::Built
                    }
                }
                _ => State::None,
            };
            Pair {
                source: src.is_dir(),
                name,
                state,
            }
        })
        .collect()
}

/// The names of the folders in `dir`. None when `dir` cannot be read.
fn folders(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// The newest change time of any file under `dir`.
fn newest(dir: &Path) -> Option<SystemTime> {
    let mut best = None;
    for e in fs::read_dir(dir).ok()?.flatten() {
        let path = e.path();
        let t = if path.is_dir() {
            newest(&path)
        } else {
            modified(&path)
        };
        best = best.max(t);
    }
    best
}

/// The inputs a report calls each function with, in the words of the INPUTS part of
/// `help.md`.
pub fn inputs(r: Report) -> &'static str {
    match r {
        Report::Returns => {
            "every function once for each input row: small, large, bound and neg, and the \
             extra, short, wrong, asvec, badarg and authed rows where they apply"
        }
        Report::Cost => "small; large, bound and neg as well, for the cost to fail",
        Report::Scaling => "every integer argument n and every list n long, for n from 0 to 64",
        Report::Ledger
        | Report::Access
        | Report::Trace
        | Report::Ttl
        | Report::Sequence
        | Report::Events => "small",
        Report::Size
        | Report::Sections
        | Report::Imports
        | Report::Interface
        | Report::Meta
        | Report::Opcodes
        | Report::Functions
        | Report::Layout => "none: it reads the two wasm files and calls nothing",
    }
}

/// The text of `flag` in the OPTIONS part of `help.md`, on one line.
pub fn option_help(flag: &str) -> String {
    let help = include_str!("../../../help.md");
    let part = help
        .split("\nOPTIONS\n")
        .nth(1)
        .and_then(|s| s.split("\nREPORTS\n").next())
        .unwrap_or("");
    let mut out = String::new();
    let mut inside = false;
    for line in part.lines() {
        if line.starts_with("    --") {
            inside = line.split_whitespace().next() == Some(flag);
            if inside {
                let text = line.trim_start().trim_start_matches(flag);
                // The flag can have an argument, such as `<n>`; the text is after the gap.
                out.push_str(text.split_once("  ").map_or(text, |(_, t)| t).trim());
            }
        } else if inside && !line.trim().is_empty() {
            out.push(' ');
            out.push_str(line.trim());
        }
    }
    out
}

/// The signature of each function in the spec of each build of `name`, or why there is
/// none.
pub fn signatures(root: &Path, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for side in ["sdk", "solang"] {
        out.push(format!("{side}:"));
        let path = root.join("out").join(name).join(format!("{side}.wasm"));
        let lines = match fs::read(&path) {
            Err(_) => vec!["not built".to_string()],
            Ok(bytes) => match crate::wasm::Contract::load(&bytes) {
                Err(e) => vec![format!("cannot read it: {e}")],
                Ok(c) if c.interface.funcs.is_empty() => {
                    vec!["no functions in its spec".to_string()]
                }
                Ok(c) => c
                    .interface
                    .funcs
                    .iter()
                    .map(crate::wasm::spec::sig)
                    .collect(),
            },
        };
        out.extend(lines.into_iter().map(|l| format!("    {l}")));
        out.push(String::new());
    }
    out
}

/// The first line `program --version` prints, or `None` when it cannot be run.
fn version(program: &str) -> Option<String> {
    let out = std::process::Command::new(program)
        .arg("--version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    Some(text.lines().next().unwrap_or("").trim().to_string())
}

/// What antlion needs under `root`, checked: the two compilers, `out/`, and the state of
/// each pair. Each problem says how to fix it.
pub fn doctor(root: &Path, pairs: &[Pair]) -> Vec<String> {
    let mut out = Vec::new();
    let tools = [
        (
            "solang",
            "install Solang from https://github.com/hyperledger-solang/solang; a release \
             binary is named like solang-linux-x86-64, so rename it or link it as solang on PATH",
        ),
        (
            "stellar",
            "install stellar-cli from https://github.com/stellar/stellar-cli, for example \
             with cargo install --locked stellar-cli",
        ),
    ];
    for (tool, fix) in tools {
        match version(tool) {
            Some(v) => out.push(format!("ok      {tool}: {v}")),
            None => {
                out.push(format!("problem {tool} is not on PATH"));
                out.push(format!("        fix: {fix}"));
            }
        }
    }

    if root.join("out").is_dir() {
        out.push("ok      out/ is there".to_string());
    } else {
        out.push("problem out/ is not there, so no pair can be compared".to_string());
        out.push(
            "        fix: select the pairs and press b, or run scripts/check.sh build".to_string(),
        );
    }

    for (state, what) in [(State::Stale, "stale"), (State::None, "not built")] {
        let names: Vec<&str> = pairs
            .iter()
            .filter(|p| p.state == state)
            .map(|p| p.name.as_str())
            .collect();
        if names.is_empty() {
            continue;
        }
        out.push(format!("problem {what}: {}", names.join(", ")));
        out.push("        fix: select them and press b".to_string());
    }
    let built = pairs.iter().filter(|p| p.state == State::Built).count();
    out.push(format!(
        "ok      {built} of {} pairs built and up to date",
        pairs.len()
    ));
    out
}

/// A warning when `dir` is not the project root. antlion reads `out/` and `contracts/` from
/// the working directory, so outside the root no pair can be found.
pub fn root_warning(dir: &Path) -> Option<String> {
    if dir.join("out").is_dir() || dir.join("contracts").is_dir() {
        return None;
    }
    Some(format!(
        "no out/ or contracts/ in {}: start antlion from the project root",
        dir.display()
    ))
}

#[cfg(test)]
pub(super) mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    /// A new empty folder for one test, removed first if an old run left it.
    pub fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("antlion-tui-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn root() {
        let d = scratch("root");
        assert!(root_warning(&d).is_some());
        fs::create_dir(d.join("contracts")).unwrap();
        assert!(root_warning(&d).is_none());

        let d = scratch("root-out");
        fs::create_dir(d.join("out")).unwrap();
        assert!(root_warning(&d).is_none());
    }

    #[test]
    fn options_from_help() {
        assert_eq!(
            option_help("--json"),
            "print one JSON document in place of the text reports"
        );
        assert_eq!(
            option_help("--jobs"),
            "threads for the reports that call contracts; default 4, or fewer on a machine with fewer cores"
        );
        assert!(option_help("--detail").ends_with("cost: add what each cost type charged"));
        assert_eq!(option_help("--nope"), "");
    }

    #[test]
    fn doctor_lists_problems_and_fixes() {
        let d = scratch("doctor");
        let p = |name: &str, state| Pair {
            name: name.to_string(),
            state,
            source: true,
        };
        let pairs = [
            p("a", State::Built),
            p("b", State::Stale),
            p("c", State::None),
        ];
        let lines = doctor(&d, &pairs);
        assert!(
            lines
                .iter()
                .any(|l| l == "problem out/ is not there, so no pair can be compared")
        );
        assert!(lines.iter().any(|l| l == "problem stale: b"));
        assert!(lines.iter().any(|l| l == "problem not built: c"));
        assert_eq!(
            lines.last().unwrap(),
            "ok      1 of 3 pairs built and up to date"
        );
        // Each problem has its fix on the next line.
        for (i, l) in lines.iter().enumerate() {
            if l.starts_with("problem") {
                assert!(lines[i + 1].starts_with("        fix: "), "{l}");
            }
        }
    }

    #[test]
    fn signatures_of_a_pair() {
        let d = scratch("sigs");
        let s = signatures(&d, "x");
        assert_eq!(
            s,
            ["sdk:", "    not built", "", "solang:", "    not built", ""]
        );
    }

    fn touch(path: &Path, secs: u64) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let f = fs::File::create(path).unwrap();
        f.set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn states() {
        let d = scratch("states");
        // Built: both files, sources older.
        touch(&d.join("contracts/a/sdk/src/lib.rs"), 100);
        touch(&d.join("out/a/sdk.wasm"), 200);
        touch(&d.join("out/a/solang.wasm"), 200);
        // Stale: a source newer than one build.
        touch(&d.join("contracts/b/solang/b.sol"), 300);
        touch(&d.join("out/b/sdk.wasm"), 400);
        touch(&d.join("out/b/solang.wasm"), 200);
        // None: no build at all, and a build with one file.
        touch(&d.join("contracts/c/sdk/Cargo.toml"), 100);
        touch(&d.join("out/d/sdk.wasm"), 100);
        // Built from another path: in out/ only.
        touch(&d.join("out/e/sdk.wasm"), 100);
        touch(&d.join("out/e/solang.wasm"), 100);

        let got: Vec<(String, State, bool)> = pairs(&d)
            .into_iter()
            .map(|p| (p.name, p.state, p.source))
            .collect();
        let want = [
            ("a", State::Built, true),
            ("b", State::Stale, true),
            ("c", State::None, true),
            ("d", State::None, false),
            ("e", State::Built, false),
        ]
        .map(|(n, s, src)| (n.to_string(), s, src));
        assert_eq!(got, want);
    }
}
