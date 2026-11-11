use crate::compare::REPORTS;
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
    let usage = format!(
        "[err] usage: --compare <name>... --report {}|all [--json] [--detail|--dump]",
        REPORTS.join("|")
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
        REPORTS.iter().map(|r| r.to_string()).collect()
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
