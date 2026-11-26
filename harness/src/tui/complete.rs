//! What can come next in the command line, from the grammar of the CLI, filtered by the
//! word the user is typing.

use std::path::Path;

use super::catalog::{self, Pair};
use crate::compare::Report;

/// What kind of word a suggestion is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum What {
    /// The first word: `--compare`, `--build`, `--help` or `--version`.
    Command,
    /// A flag after the first word, such as `--report` or `--json`.
    Flag,
    /// A pair name after `--compare`.
    Pair,
    /// A pair folder after `--build`, such as `contracts/flow`.
    Path,
    /// A report name after `--report`, or `all`.
    Report,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Suggestion {
    pub text: String,
    pub what: What,
}

/// The first words, in the order they are suggested.
const COMMANDS: [&str; 4] = ["--compare", "--build", "--help", "--version"];

/// The flags after the reports that fit every report, in the order they are suggested.
const TRAILING: [&str; 3] = ["--json", "--bare", "--jobs"];

/// The word the cursor is at the end of: the text of `before` after its last space.
pub fn word(before: &str) -> &str {
    before.rsplit(' ').next().unwrap_or("")
}

/// The words that can come at the end of `before`, the text of the line up to the cursor,
/// that fuzzy match the word typed so far, the best match first.
pub fn suggest(before: &str, pairs: &[Pair]) -> Vec<Suggestion> {
    let all = next(before, pairs);
    let names: Vec<&str> = all.iter().map(|s| s.text.as_str()).collect();
    super::search::ranked(word(before), &names)
        .into_iter()
        .map(|i| all[i].clone())
        .collect()
}

fn suggestion(text: &str, what: What) -> Suggestion {
    Suggestion {
        text: text.to_string(),
        what,
    }
}

/// Every word that can come at the end of `before`, whatever the word typed so far.
fn next(before: &str, pairs: &[Pair]) -> Vec<Suggestion> {
    let done: Vec<&str> = before[..before.len() - word(before).len()]
        .split_whitespace()
        .collect();
    let Some((&first, rest)) = done.split_first() else {
        return COMMANDS
            .iter()
            .map(|c| suggestion(c, What::Command))
            .collect();
    };
    match first {
        // A pair without a folder in contracts/ was built from another path, which only
        // the user knows.
        "--build" => pairs
            .iter()
            .filter(|p| p.source)
            .map(|p| format!("contracts/{}", p.name))
            .filter(|path| !rest.contains(&path.as_str()))
            .map(|path| suggestion(&path, What::Path))
            .collect(),
        "--compare" => compare(rest, pairs),
        _ => Vec::new(),
    }
}

/// The words after `--compare`: pairs, then `--report`, then reports, then the trailing
/// flags, as `cli::parse` takes them.
fn compare(rest: &[&str], pairs: &[Pair]) -> Vec<Suggestion> {
    let Some(at) = rest.iter().position(|w| *w == "--report") else {
        let mut out: Vec<Suggestion> = pairs
            .iter()
            .filter(|p| !rest.contains(&p.name.as_str()))
            .map(|p| suggestion(&p.name, What::Pair))
            .collect();
        if !rest.is_empty() {
            out.push(suggestion("--report", What::Flag));
        }
        return out;
    };
    let after = &rest[at + 1..];
    // `--jobs` needs a number, and there is nothing to suggest for one.
    if after.last() == Some(&"--jobs") {
        return Vec::new();
    }
    // The CLI reads the flags from the end, so no report can come after one.
    let flag = after.iter().position(|w| w.starts_with("--"));
    let reports = &after[..flag.unwrap_or(after.len())];

    let mut out = Vec::new();
    let has = |f: &str| after.contains(&f);
    // `--detail` and `--dump` change one report each, so they show only right after it,
    // before the other reports. They never go together.
    let mode: &[&str] = match reports.last() {
        _ if has("--detail") || has("--dump") => &[],
        Some(&"opcodes" | &"all") => &["--detail", "--dump"],
        Some(&"cost") => &["--detail"],
        _ => &[],
    };
    out.extend(mode.iter().map(|f| suggestion(f, What::Flag)));
    if flag.is_none() && !reports.contains(&"all") {
        let names = Report::ALL.iter().map(|r| r.name());
        let all = reports.is_empty().then_some("all");
        out.extend(
            names
                .chain(all)
                .filter(|n| !reports.contains(n))
                .map(|n| suggestion(n, What::Report)),
        );
    }
    if !reports.is_empty() {
        for f in TRAILING {
            if !has(f) {
                out.push(suggestion(f, What::Flag));
            }
        }
    }
    out
}

/// The title and the lines of the details pane: the help for `s`, the suggestion under
/// the cursor. With none, the help for the last flag before the cursor, or the usage.
pub fn details(
    s: Option<&Suggestion>,
    before: &str,
    root: &Path,
    pairs: &[Pair],
) -> (String, Vec<String>) {
    let Some(s) = s else {
        let done = &before[..before.len() - word(before).len()];
        let words: Vec<&str> = done.split_whitespace().collect();
        return match words.iter().rposition(|w| w.starts_with("--")) {
            Some(0) => (words[0].to_string(), vec![catalog::command_help(words[0])]),
            Some(i) => (words[i].to_string(), vec![catalog::option_help(words[i])]),
            None => ("antlion".to_string(), catalog::usage()),
        };
    };
    let one = |text: String| (s.text.clone(), vec![text]);
    match s.what {
        What::Command => one(catalog::command_help(&s.text)),
        What::Flag => one(catalog::option_help(&s.text)),
        What::Report if s.text == "all" => {
            let names: Vec<&str> = Report::ALL.iter().map(|r| r.name()).collect();
            let lines = vec![
                "every report, in this order:".to_string(),
                String::new(),
                names.join(", "),
            ];
            ("report all".to_string(), lines)
        }
        What::Report => {
            let r = Report::parse(&s.text).expect("a report from Report::ALL");
            let lines = vec![
                r.about().to_string(),
                String::new(),
                format!("inputs: {}", catalog::inputs(r)),
            ];
            (format!("report {}", r.name()), lines)
        }
        What::Pair | What::Path => {
            let name = s.text.strip_prefix("contracts/").unwrap_or(&s.text);
            let mut lines = Vec::new();
            if let Some(p) = pairs.iter().find(|p| p.name == name) {
                lines.push(format!("state: {}", p.state.label()));
                lines.push(if p.source {
                    format!("source: contracts/{name}")
                } else {
                    "source: none in contracts/, built from another path".to_string()
                });
                lines.push(String::new());
            }
            lines.extend(catalog::signatures(root, name));
            (format!("pair {name}"), lines)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::catalog::State;
    use super::*;

    fn pairs() -> Vec<Pair> {
        let p = |name: &str, source| Pair {
            name: name.to_string(),
            state: State::Built,
            source,
        };
        vec![p("flow", true), p("numeric", true), p("other", false)]
    }

    fn texts(before: &str) -> Vec<String> {
        suggest(before, &pairs())
            .into_iter()
            .map(|s| s.text)
            .collect()
    }

    #[test]
    fn the_word_typed() {
        assert_eq!(word(""), "");
        assert_eq!(word("--comp"), "--comp");
        assert_eq!(word("--compare fl"), "fl");
        assert_eq!(word("--compare "), "");
    }

    #[test]
    fn first_word() {
        assert_eq!(texts(""), COMMANDS);
        assert_eq!(texts("--b")[0], "--build");
        assert_eq!(suggest("", &[])[0].what, What::Command);
        assert!(texts("--help ").is_empty());
    }

    #[test]
    fn build_paths() {
        // `other` has no folder in contracts/, so there is no path to suggest for it.
        assert_eq!(texts("--build "), ["contracts/flow", "contracts/numeric"]);
        assert_eq!(texts("--build contracts/flow "), ["contracts/numeric"]);
        assert_eq!(texts("--build num"), ["contracts/numeric"]);
    }

    #[test]
    fn pairs_then_report() {
        assert_eq!(texts("--compare "), ["flow", "numeric", "other"]);
        assert_eq!(texts("--compare flow "), ["numeric", "other", "--report"]);
        assert!(texts("--compare zzz").is_empty());
    }

    #[test]
    fn reports_then_flags() {
        let all = texts("--compare flow --report ");
        assert_eq!(all.len(), Report::ALL.len() + 1);
        assert_eq!(all.last().unwrap(), "all");

        // After a report that no mode flag is for, the other reports, then the flags.
        let after_size = texts("--compare flow --report size ");
        assert!(!after_size.contains(&"size".to_string()));
        assert!(!after_size.contains(&"all".to_string()));
        assert_eq!(after_size[after_size.len() - 3..], TRAILING);
        assert!(!after_size.contains(&"--detail".to_string()));
        assert!(!after_size.contains(&"--dump".to_string()));

        assert_eq!(
            texts("--compare flow --report all "),
            ["--detail", "--dump", "--json", "--bare", "--jobs"]
        );
        assert_eq!(texts("--compare flow --report opcodes --detail "), TRAILING);
        assert!(texts("--compare flow --report size --jobs ").is_empty());
        assert_eq!(
            texts("--compare flow --report size --jobs 2 "),
            ["--json", "--bare"]
        );
    }

    #[test]
    fn mode_flags_follow_their_report() {
        // After opcodes, its flags come first, then the other reports.
        let t = texts("--compare flow --report opcodes ");
        assert_eq!(t[..3], ["--detail", "--dump", "size"]);
        assert!(!t.contains(&"opcodes".to_string()));
        assert_eq!(t[t.len() - 3..], TRAILING);
        // After cost, only --detail fits.
        let t = texts("--compare flow --report cost ");
        assert_eq!(t[..2], ["--detail", "size"]);
        assert!(!t.contains(&"--dump".to_string()));
        // Once another report follows, the flags of opcodes do not show at all.
        let t = texts("--compare flow --report opcodes size ");
        assert_eq!(t[0], "sections");
        assert!(!t.iter().any(|f| f == "--detail" || f == "--dump"));
        assert_eq!(t[t.len() - 3..], TRAILING);
    }

    fn details_of(before: &str) -> (String, Vec<String>) {
        let s = suggest(before, &pairs());
        details(s.first(), before, Path::new("/nonexistent"), &pairs())
    }

    #[test]
    fn details_of_each_kind() {
        let (title, lines) = details_of("--b");
        assert_eq!(title, "--build");
        assert!(lines[0].starts_with("compile each"));

        let (title, lines) = details_of("--compare flow --report co");
        assert_eq!(title, "report cost");
        assert!(lines[0].starts_with("metered cost of calling"));
        assert!(lines[2].starts_with("inputs: small"));

        let (title, lines) = details_of("--compare flow --report al");
        assert_eq!(title, "report all");
        assert!(lines[2].starts_with("size, sections, imports"));

        let (title, lines) = details_of("--compare oth");
        assert_eq!(title, "pair other");
        assert_eq!(lines[0], "state: built");
        assert!(lines[1].contains("built from another path"));
        assert_eq!(lines[4], "    not built");

        let (title, lines) = details_of("--build num");
        assert_eq!(title, "pair numeric");
        assert_eq!(lines[1], "source: contracts/numeric");

        let (title, lines) = details_of("--compare flow --report size --js");
        assert_eq!(title, "--json");
        assert!(lines[0].starts_with("print one JSON document"));
    }

    #[test]
    fn details_with_no_suggestion() {
        // The help for the last flag before the cursor.
        let (title, lines) = details_of("--compare flow --report size --jobs ");
        assert_eq!(title, "--jobs");
        assert!(lines[0].starts_with("threads for the reports"));
        assert_eq!(details_of("--help ").0, "--help");
        // No flag at all: the usage.
        let (title, lines) = details_of("zz");
        assert_eq!(title, "antlion");
        assert!(lines[0].starts_with("antlion --build"));
    }

    #[test]
    fn filtered_best_first() {
        assert_eq!(texts("--compare flow --report co")[0], "cost");
        assert_eq!(texts("--compare flow --report size --js")[0], "--json");
    }
}
