use crate::compare::Report;
use crate::error::{Error, Result};

/// What the command line asks for.
pub enum Command {
    Help,
    Version,
    Build(String),
    Compare(Compare),
}

pub struct Compare {
    /// Pair names, in the order given.
    pub names: Vec<String>,
    /// Report names as typed, with `all` already replaced by every report.
    pub reports: Vec<String>,
    pub json: bool,
    pub mode: Mode,
}

/// How the `opcodes` report is shown.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Totals,
    Detail,
    Dump,
}

/// Read the arguments, without the program name.
pub fn parse(args: &[String]) -> Result<Command> {
    let usage = "usage: --build <contract> | --compare <contract>... --report <report>... | --help | --version";
    let (cmd, rest) = args.split_first().ok_or_else(|| Error::usage(usage))?;

    match cmd.as_str() {
        "--help" => Ok(Command::Help),
        "--version" => Ok(Command::Version),
        "--build" => {
            let arg = rest.first().ok_or_else(|| Error::usage(usage))?;
            Ok(Command::Build(arg.clone()))
        }
        "--compare" => compare(rest).map(Command::Compare),
        _ => Err(Error::usage(usage)),
    }
}

/// The arguments after `--compare`.
fn compare(args: &[String]) -> Result<Compare> {
    let names: Vec<&str> = Report::ALL.iter().map(|r| r.name()).collect();
    let usage = format!(
        "[err] usage: --compare <name>... --report {}|all [--json] [--detail|--dump]",
        names.join("|")
    );

    let names: Vec<String> = args
        .iter()
        .take_while(|a| !a.starts_with("--"))
        .cloned()
        .collect();
    if names.is_empty() {
        return Err(Error::usage(format!("no contract given\n{usage}")));
    }

    let Some(("--report", rest)) = args[names.len()..]
        .split_first()
        .map(|(flag, rest)| (flag.as_str(), rest))
    else {
        return Err(Error::usage(format!(
            "no report requested — pass `--report all` for every report\n{usage}"
        )));
    };

    // Trailing flags, in any order: `--json`, and one of `--detail` or `--dump`.
    let (mut reports, mut json, mut mode) = (rest, false, None);
    while let Some((last, elements)) = reports.split_last() {
        match last.as_str() {
            "--json" if !json => json = true,
            "--detail" if mode.is_none() => mode = Some(Mode::Detail),
            "--dump" if mode.is_none() => mode = Some(Mode::Dump),
            _ => break,
        }
        reports = elements;
    }

    if reports.is_empty() {
        return Err(Error::usage(format!(
            "`--report` needs a report name — pass `all` for every report\n{usage}"
        )));
    }

    let reports = if reports.iter().any(|r| r == "all") {
        Report::ALL.iter().map(|r| r.name().to_string()).collect()
    } else {
        reports.to_vec()
    };

    Ok(Compare {
        names,
        reports,
        json,
        mode: mode.unwrap_or(Mode::Totals),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(line: &str) -> Result<Command> {
        let args: Vec<String> = line.split_whitespace().map(String::from).collect();
        parse(&args)
    }

    fn compare_of(line: &str) -> Compare {
        match parse_str(line).unwrap() {
            Command::Compare(c) => c,
            _ => panic!("`{line}` is not a compare command"),
        }
    }

    fn is_usage(line: &str) -> bool {
        matches!(parse_str(line), Err(Error::Usage(_)))
    }

    #[test]
    fn no_arguments() {
        assert!(is_usage(""));
    }

    #[test]
    fn version() {
        assert!(matches!(parse_str("--version"), Ok(Command::Version)));
    }

    #[test]
    fn build() {
        assert!(matches!(
            parse_str("--build contracts/flow"),
            Ok(Command::Build(p)) if p == "contracts/flow"
        ));
        assert!(is_usage("--build"));
    }

    #[test]
    fn compare_one_report() {
        let c = compare_of("--compare flow --report size");
        assert_eq!(c.names, ["flow"]);
        assert_eq!(c.reports, ["size"]);
        assert!(!c.json);
        assert_eq!(c.mode, Mode::Totals);
    }

    #[test]
    fn compare_several_names_and_reports() {
        let c = compare_of("--compare a b --report size cost --json");
        assert_eq!(c.names, ["a", "b"]);
        assert_eq!(c.reports, ["size", "cost"]);
        assert!(c.json);
    }

    #[test]
    fn trailing_flags_in_any_order() {
        let c = compare_of("--compare flow --report opcodes --detail --json");
        assert!(c.json);
        assert_eq!(c.mode, Mode::Detail);

        let c = compare_of("--compare flow --report opcodes --json --dump");
        assert!(c.json);
        assert_eq!(c.mode, Mode::Dump);
    }

    #[test]
    fn all_is_every_report() {
        let c = compare_of("--compare flow --report all");
        let every: Vec<&str> = Report::ALL.iter().map(|r| r.name()).collect();
        assert_eq!(c.reports, every);
    }

    #[test]
    fn compare_errors() {
        assert!(is_usage("--compare flow"));
        assert!(is_usage("--compare --report all"));
        assert!(is_usage("--compare flow --report"));
    }
}
