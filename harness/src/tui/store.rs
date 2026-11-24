//! What the UI keeps between runs: the history of runs, and the presets in the config
//! file.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use toml_edit::{Array, DocumentMut, Item, Table};

/// The most runs the history keeps. A new run past this drops the oldest.
pub const MAX_HISTORY: usize = 200;

/// The folder for the history: `ANTLION_CONFIG_DIR` when it is set, else
/// `$XDG_STATE_HOME/antlion`, else `~/.local/state/antlion`. `None` when none is known.
pub fn state_dir() -> Option<PathBuf> {
    dir("XDG_STATE_HOME", ".local/state")
}

/// The folder for the config file: `ANTLION_CONFIG_DIR` when it is set, else
/// `$XDG_CONFIG_HOME/antlion`, else `~/.config/antlion`.
pub fn config_dir() -> Option<PathBuf> {
    dir("XDG_CONFIG_HOME", ".config")
}

fn dir(xdg: &str, home: &str) -> Option<PathBuf> {
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    var("ANTLION_CONFIG_DIR")
        .or_else(|| var(xdg).map(|d| d.join("antlion")))
        .or_else(|| var("HOME").map(|h| h.join(home).join("antlion")))
}

/// One run in the history.
#[derive(Clone, PartialEq, Debug)]
pub struct Record {
    /// When the run ended, in seconds since 1970.
    pub time: u64,
    /// The arguments, without the program name.
    pub args: Vec<String>,
    /// The exit code; `None` when the run was cancelled or stopped by a signal.
    pub exit: Option<i32>,
    pub secs: f64,
}

impl Record {
    pub fn new(args: Vec<String>, exit: Option<i32>, secs: f64) -> Self {
        Record {
            time: now(),
            args,
            exit,
            secs,
        }
    }

    fn json(&self) -> Value {
        json!({ "time": self.time, "args": self.args, "exit": self.exit, "secs": self.secs })
    }

    fn from_json(v: &Value) -> Option<Self> {
        Some(Record {
            time: v["time"].as_u64()?,
            args: v["args"]
                .as_array()?
                .iter()
                .map(|a| a.as_str().map(str::to_string))
                .collect::<Option<_>>()?,
            exit: v["exit"].as_i64().map(|c| c as i32),
            secs: v["secs"].as_f64().unwrap_or(0.0),
        })
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn history_file(dir: &Path) -> PathBuf {
    dir.join("history.jsonl")
}

/// Every run in the history of `dir`, the oldest first. A line that cannot be read is
/// left out, so one bad line does not lose the rest.
pub fn history(dir: &Path) -> Vec<Record> {
    let Ok(text) = fs::read_to_string(history_file(dir)) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| Record::from_json(&v))
        .collect()
}

/// Add `r` to the history of `dir`, and keep the last `MAX_HISTORY` runs.
pub fn add(dir: &Path, r: &Record) -> std::io::Result<()> {
    let mut all = history(dir);
    all.push(r.clone());
    let from = all.len().saturating_sub(MAX_HISTORY);
    let text: String = all[from..]
        .iter()
        .map(|r| format!("{}\n", r.json()))
        .collect();
    fs::create_dir_all(dir)?;
    fs::write(history_file(dir), text)
}

fn config_file(dir: &Path) -> PathBuf {
    dir.join("config.toml")
}

/// The config file of `dir`, or an empty one when there is none.
fn config(dir: &Path) -> Result<DocumentMut, String> {
    let path = config_file(dir);
    match fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DocumentMut::new()),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
        Ok(text) => text
            .parse::<DocumentMut>()
            .map_err(|e| format!("{}: {}", path.display(), e.to_string().trim())),
    }
}

/// Every preset in the config file of `dir`: its name and its arguments, in file order.
/// A preset whose `args` is not a list of strings is an error.
pub fn presets(dir: &Path) -> Result<Vec<(String, Vec<String>)>, String> {
    let doc = config(dir)?;
    let Some(table) = doc.get("presets").and_then(Item::as_table) else {
        return Ok(Vec::new());
    };
    table
        .iter()
        .map(|(name, item)| {
            let args = item
                .get("args")
                .and_then(Item::as_array)
                .and_then(|a| a.iter().map(|v| v.as_str().map(str::to_string)).collect())
                .ok_or_else(|| format!("preset `{name}` needs args = [\"--compare\", ...]"))?;
            Ok((name.to_string(), args))
        })
        .collect()
}

/// Save `args` as the preset `name` in the config file of `dir`. Every other line of the
/// file, comments too, stays as it was.
pub fn save_preset(dir: &Path, name: &str, args: &[String]) -> Result<(), String> {
    let mut doc = config(dir)?;
    if !doc.contains_key("presets") {
        let mut t = Table::new();
        t.set_implicit(true);
        doc["presets"] = Item::Table(t);
    }
    let mut preset = Table::new();
    preset["args"] = toml_edit::value(args.iter().map(String::as_str).collect::<Array>());
    doc["presets"][name] = Item::Table(preset);
    fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let path = config_file(dir);
    fs::write(&path, doc.to_string()).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// How long ago `time` was, in one short word: `12 s`, `5 min`, `3 h` or `2 d`.
pub fn ago(time: u64) -> String {
    let s = now().saturating_sub(time);
    match s {
        0..60 => format!("{s} s"),
        60..3600 => format!("{} min", s / 60),
        3600..86400 => format!("{} h", s / 3600),
        _ => format!("{} d", s / 86400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::catalog::tests::scratch;

    #[test]
    fn add_and_read() {
        let d = scratch("history");
        assert!(history(&d).is_empty());
        let a = Record::new(vec!["--compare".into(), "flow".into()], Some(0), 1.5);
        let b = Record::new(vec!["--compare".into(), "ttl".into()], None, 0.2);
        add(&d, &a).unwrap();
        add(&d, &b).unwrap();
        assert_eq!(history(&d), [a.clone(), b]);

        // A damaged line is left out, and the others stay.
        let path = history_file(&d);
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path, format!("not json\n{text}")).unwrap();
        assert_eq!(history(&d).len(), 2);
    }

    #[test]
    fn keeps_the_last() {
        let d = scratch("history-max");
        for i in 0..MAX_HISTORY + 5 {
            add(&d, &Record::new(vec![i.to_string()], Some(0), 0.0)).unwrap();
        }
        let h = history(&d);
        assert_eq!(h.len(), MAX_HISTORY);
        assert_eq!(h[0].args, ["5"]);
    }

    #[test]
    fn presets_keep_the_rest_of_the_file() {
        let d = scratch("presets");
        assert_eq!(presets(&d).unwrap(), []);
        let args: Vec<String> = ["--compare", "numeric", "--report", "cost"]
            .map(String::from)
            .to_vec();
        save_preset(&d, "cost of numeric pairs", &args).unwrap();
        assert_eq!(
            presets(&d).unwrap(),
            [("cost of numeric pairs".to_string(), args.clone())]
        );

        // A comment and another key stay; saving a name again replaces it.
        let path = config_file(&d);
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path, format!("# my comment\njobs = 2\n\n{text}")).unwrap();
        save_preset(&d, "flow", &args[..2]).unwrap();
        save_preset(&d, "flow", &args).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# my comment\njobs = 2\n"));
        assert_eq!(presets(&d).unwrap().len(), 2);
        assert_eq!(presets(&d).unwrap()[1].1, args);
    }

    #[test]
    fn bad_config() {
        let d = scratch("bad-config");
        fs::write(config_file(&d), "[presets.x]\nargs = 3\n").unwrap();
        assert!(presets(&d).unwrap_err().contains("preset `x` needs args"));
        fs::write(config_file(&d), "this is not toml").unwrap();
        assert!(presets(&d).unwrap_err().contains("config.toml"));
        assert!(save_preset(&d, "y", &[]).is_err());
    }

    #[test]
    fn ago_words() {
        assert_eq!(ago(now()), "0 s");
        assert_eq!(ago(now() - 120), "2 min");
        assert_eq!(ago(now() - 7200), "2 h");
        assert_eq!(ago(now() - 3 * 86400), "3 d");
    }
}
