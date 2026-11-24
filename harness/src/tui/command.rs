//! The arguments of a run, made into the line the user sees, a file name for its output,
//! and the code that copies text to the clipboard.

/// Why the CLI refuses `args`, on one line, or `None` when it takes them. The TUI never
/// runs what the CLI would refuse, and it says why in the CLI's own words.
pub fn refusal(args: &[String]) -> Option<String> {
    let e = crate::cli::parse(args).err()?;
    let text = e.to_string();
    let first = text.lines().next().unwrap_or_default();
    Some(first.trim_start_matches("[err] ").to_string())
}

/// A file name for the output of a `--compare` run of `args`: the pairs and the reports,
/// with `.json` for JSON output and `.txt` for text. For example `antlion_flow_cost.txt`.
pub fn file_name(args: &[String]) -> String {
    let part = |flag: &str| -> Vec<&str> {
        args.iter()
            .skip_while(|a| *a != flag)
            .skip(1)
            .take_while(|a| !a.starts_with("--"))
            .map(String::as_str)
            .collect()
    };
    let mut words = vec!["antlion"];
    words.extend(part("--compare"));
    words.extend(part("--report"));
    let mut name: String = words
        .join("_")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    name.truncate(80);
    let ext = if args.iter().any(|a| a == "--json") {
        "json"
    } else {
        "txt"
    };
    format!("{name}.{ext}")
}

/// The terminal code that puts `text` in the clipboard: OSC 52. It works over SSH and in
/// tmux, and needs no clipboard program.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

/// `bytes` in standard base64, with `=` padding.
fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `args` as one line to type in a shell, after `antlion`.
pub fn line(args: &[String]) -> String {
    let mut out = String::from("antlion");
    for a in args {
        out.push(' ');
        out.push_str(&quote(a));
    }
    out
}

/// `s` as the shell reads it back: bare when it has only safe characters, else in single
/// quotes.
fn quote(s: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "_-./=:,+".contains(c);
    if !s.is_empty() && s.chars().all(safe) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn lines() {
        assert_eq!(
            line(&args("--compare flow --report size")),
            "antlion --compare flow --report size"
        );
        assert_eq!(line(&["my pair".to_string()]), "antlion 'my pair'");
    }

    #[test]
    fn refusals() {
        assert_eq!(
            refusal(&args("--compare --report size")).as_deref(),
            Some("no contract given")
        );
        assert!(
            refusal(&args("--compare flow --report"))
                .is_some_and(|e| e.starts_with("`--report` needs a report name"))
        );
        assert_eq!(refusal(&args("--compare flow --report size")), None);
        assert_eq!(refusal(&args("--build contracts/flow")), None);
    }

    #[test]
    fn file_names() {
        assert_eq!(
            file_name(&args("--compare flow numeric --report cost")),
            "antlion_flow_numeric_cost.txt"
        );
        assert_eq!(
            file_name(&args(
                "--compare flow numeric --report cost --json --jobs 2"
            )),
            "antlion_flow_numeric_cost.json"
        );
        let spaced = ["--compare", "my pair", "--report", "all"].map(String::from);
        assert_eq!(file_name(&spaced), "antlion_my-pair_all.txt");
    }

    #[test]
    fn base64_and_osc52() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(
            base64(b"antlion --compare flow"),
            "YW50bGlvbiAtLWNvbXBhcmUgZmxvdw=="
        );
        assert_eq!(osc52("foo"), "\x1b]52;c;Zm9v\x07");
    }

    #[test]
    fn quoting() {
        assert_eq!(quote("flow"), "flow");
        assert_eq!(quote("my pair"), "'my pair'");
        assert_eq!(quote("it's"), r"'it'\''s'");
        assert_eq!(quote(""), "''");
    }
}
