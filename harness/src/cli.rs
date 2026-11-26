use crate::compare::Report;
use crate::error::{Error, Result};

/// What the command line asks for.
pub enum Command {
    Help,
    Version,
    /// One or more pair paths, in the order given.
    Build(Vec<String>),
    Compare(Compare),
    /// The terminal UI, which picks and runs the other commands.
    Interactive,
}

pub struct Compare {
    /// Pair names, in the order given.
    pub names: Vec<String>,
    /// The reports to run, in the order given, with `all` already replaced by every report.
    pub reports: Vec<Report>,
    pub json: bool,
    /// Leave out the notes and the lines that explain how a report ran.
    pub bare: bool,
    pub mode: Mode,
    /// Threads for the reports that call contracts. `None` leaves the choice to `compare`.
    pub jobs: Option<usize>,
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
    let usage = "usage: --build <contract>... | --compare <contract>... --report <report>... | --interactive | --help | --version";
    let (cmd, rest) = args.split_first().ok_or_else(|| Error::usage(usage))?;

    match cmd.as_str() {
        "--help" => Ok(Command::Help),
        "--version" => Ok(Command::Version),
        "--build" if rest.is_empty() => Err(Error::usage(usage)),
        "--build" if rest.iter().any(|a| a.starts_with("--")) => {
            Err(Error::usage("--build takes paths only"))
        }
        "--build" => Ok(Command::Build(rest.to_vec())),
        "--compare" => compare(rest).map(Command::Compare),
        "--interactive" if rest.is_empty() => Ok(Command::Interactive),
        "--interactive" => Err(Error::usage("--interactive takes no other arguments")),
        _ => Err(Error::usage(usage)),
    }
}

/// The arguments after `--compare`.
fn compare(args: &[String]) -> Result<Compare> {
    let names: Vec<&str> = Report::ALL.iter().map(|r| r.name()).collect();
    let usage = format!(
        "[err] usage: --compare <name>... --report {}|all [--json] [--bare] [--detail|--dump] [--jobs <n>]",
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

    // Trailing flags, in any order: `--json`, `--bare`, one of `--detail` or `--dump`,
    // and `--jobs <n>`.
    let bad_jobs = || Error::usage("--jobs needs a number, 1 or more");
    let (mut reports, mut json, mut bare, mut mode, mut jobs) = (rest, false, false, None, None);
    while let Some((last, elements)) = reports.split_last() {
        match last.as_str() {
            "--json" if !json => json = true,
            "--bare" if !bare => bare = true,
            "--detail" if mode.is_none() => mode = Some(Mode::Detail),
            "--dump" if mode.is_none() => mode = Some(Mode::Dump),
            "--jobs" if jobs.is_none() => return Err(bad_jobs()),
            n if jobs.is_none() && elements.last().is_some_and(|f| f == "--jobs") => {
                let n: usize = n.parse().map_err(|_| bad_jobs())?;
                if n == 0 {
                    return Err(bad_jobs());
                }
                jobs = Some(n);
                reports = &elements[..elements.len() - 1];
                continue;
            }
            _ => break,
        }
        reports = elements;
    }

    if reports.is_empty() {
        return Err(Error::usage(format!(
            "`--report` needs a report name — pass `all` for every report\n{usage}"
        )));
    }

    // Every name is checked before any pair is read, so a wrong one stops the run before
    // any output.
    let mut parsed = Vec::new();
    for r in reports.iter().filter(|r| *r != "all") {
        parsed.push(Report::parse(r)?);
    }
    let reports = if reports.iter().any(|r| r == "all") {
        Report::ALL.to_vec()
    } else {
        parsed
    };

    Ok(Compare {
        names: first_of_each(names),
        reports: first_of_each(reports),
        json,
        bare,
        mode: mode.unwrap_or(Mode::Totals),
        jobs,
    })
}

/// `items` with each repeat removed, keeping the first one.
fn first_of_each<T: PartialEq>(items: Vec<T>) -> Vec<T> {
    let mut out = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
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
    fn interactive() {
        assert!(matches!(
            parse_str("--interactive"),
            Ok(Command::Interactive)
        ));
        assert!(is_usage("--interactive --json"));
        assert!(is_usage("--interactive flow"));
    }

    #[test]
    fn build() {
        assert!(matches!(
            parse_str("--build contracts/flow"),
            Ok(Command::Build(p)) if p == ["contracts/flow"]
        ));
        assert!(matches!(
            parse_str("--build a b"),
            Ok(Command::Build(p)) if p == ["a", "b"]
        ));
        assert!(is_usage("--build"));
        assert!(is_usage("--build a --json"));
    }

    #[test]
    fn compare_one_report() {
        let c = compare_of("--compare flow --report size");
        assert_eq!(c.names, ["flow"]);
        assert_eq!(c.reports, [Report::Size]);
        assert!(!c.json);
        assert!(!c.bare);
        assert_eq!(c.mode, Mode::Totals);
    }

    #[test]
    fn compare_several_names_and_reports() {
        let c = compare_of("--compare a b --report size cost --json");
        assert_eq!(c.names, ["a", "b"]);
        assert_eq!(c.reports, [Report::Size, Report::Cost]);
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

        let c = compare_of("--compare flow --report size --bare --jobs 2 --json");
        assert!(c.bare && c.json);
        assert_eq!(c.jobs, Some(2));
        assert!(is_usage("--compare flow --report size --bare --bare"));
    }

    #[test]
    fn all_is_every_report() {
        let c = compare_of("--compare flow --report all");
        assert_eq!(c.reports, Report::ALL);
    }

    #[test]
    fn repeats_are_used_once() {
        assert_eq!(
            compare_of("--compare flow flow --report size").names,
            ["flow"]
        );
        assert_eq!(
            compare_of("--compare flow --report size size").reports,
            [Report::Size]
        );
    }

    #[test]
    fn jobs() {
        assert_eq!(compare_of("--compare flow --report size").jobs, None);
        assert_eq!(
            compare_of("--compare flow --report size --jobs 2").jobs,
            Some(2)
        );

        let c = compare_of("--compare flow --report size --jobs 2 --json");
        assert_eq!((c.jobs, c.json), (Some(2), true));
        let c = compare_of("--compare flow --report size --json --jobs 2");
        assert_eq!((c.jobs, c.json), (Some(2), true));

        assert!(is_usage("--compare flow --report size --jobs 0"));
        assert!(is_usage("--compare flow --report size --jobs x"));
        assert!(is_usage("--compare flow --report size --jobs"));
    }

    #[test]
    fn compare_errors() {
        assert!(is_usage("--compare flow"));
        assert!(is_usage("--compare --report all"));
        assert!(is_usage("--compare flow --report"));
        assert!(is_usage("--compare flow --report size bogus"));
    }
}
