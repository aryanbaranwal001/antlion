//! What the lists show: the pairs and their build state, the reports, the options.

use std::{fs, path::Path, time::SystemTime};

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
