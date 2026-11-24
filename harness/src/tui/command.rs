//! A selection, made into the arguments of a `--compare` run and into the line the user sees.

use crate::cli::Mode;
use crate::compare::Report;

/// What the lists and the options have selected.
#[derive(Clone, PartialEq, Debug)]
pub struct Selection {
    /// Pair names, in list order.
    pub pairs: Vec<String>,
    /// Reports, in `Report::ALL` order.
    pub reports: Vec<Report>,
    pub json: bool,
    pub mode: Mode,
    /// `None` leaves the number of threads to antlion.
    pub jobs: Option<usize>,
}

impl Default for Selection {
    fn default() -> Self {
        Selection {
            pairs: Vec::new(),
            reports: Vec::new(),
            json: false,
            mode: Mode::Totals,
            jobs: None,
        }
    }
}

impl Selection {
    /// The arguments of the run, without the program name, in a fixed order. Every report
    /// selected is `all`.
    pub fn args(&self) -> Vec<String> {
        let mut a = vec!["--compare".to_string()];
        a.extend(self.pairs.iter().cloned());
        a.push("--report".to_string());
        if self.reports.len() == Report::ALL.len() {
            a.push("all".to_string());
        } else {
            a.extend(self.reports.iter().map(|r| r.name().to_string()));
        }
        if self.json {
            a.push("--json".to_string());
        }
        match self.mode {
            Mode::Totals => {}
            Mode::Detail => a.push("--detail".to_string()),
            Mode::Dump => a.push("--dump".to_string()),
        }
        if let Some(n) = self.jobs {
            a.push("--jobs".to_string());
            a.push(n.to_string());
        }
        a
    }
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

    fn sel(pairs: &[&str], reports: &[Report]) -> Selection {
        Selection {
            pairs: pairs.iter().map(|p| p.to_string()).collect(),
            reports: reports.to_vec(),
            ..Selection::default()
        }
    }

    fn shown(s: &Selection) -> String {
        line(&s.args())
    }

    #[test]
    fn one_and_many() {
        assert_eq!(
            shown(&sel(&["flow"], &[Report::Size])),
            "antlion --compare flow --report size"
        );
        assert_eq!(
            shown(&sel(&["flow", "numeric"], &[Report::Size, Report::Cost])),
            "antlion --compare flow numeric --report size cost"
        );
    }

    #[test]
    fn every_report_is_all() {
        assert_eq!(
            shown(&sel(&["flow"], &Report::ALL)),
            "antlion --compare flow --report all"
        );
    }

    #[test]
    fn options() {
        let mut s = sel(&["flow"], &[Report::Cost]);
        s.json = true;
        s.mode = Mode::Detail;
        s.jobs = Some(2);
        assert_eq!(
            shown(&s),
            "antlion --compare flow --report cost --json --detail --jobs 2"
        );
        s.mode = Mode::Dump;
        s.json = false;
        s.jobs = None;
        assert_eq!(shown(&s), "antlion --compare flow --report cost --dump");
    }

    /// What `cli::parse` makes of the arguments, as a selection again.
    fn parsed(args: &[String]) -> Selection {
        match crate::cli::parse(args) {
            Ok(crate::cli::Command::Compare(c)) => Selection {
                pairs: c.names,
                reports: c.reports,
                json: c.json,
                mode: c.mode,
                jobs: c.jobs,
            },
            Ok(_) => panic!("{args:?} is not a compare command"),
            Err(e) => panic!("{args:?} does not parse: {e}"),
        }
    }

    #[test]
    fn round_trip() {
        let pairs: [&[&str]; 3] = [&["flow"], &["flow", "numeric"], &["a", "b", "c"]];
        let reports: [&[Report]; 4] = [
            &[Report::Size],
            &[Report::Cost, Report::Trace],
            &[Report::Opcodes, Report::Functions, Report::Layout],
            &Report::ALL,
        ];
        let mut n = 0;
        for p in pairs {
            for r in reports {
                for json in [false, true] {
                    for mode in [Mode::Totals, Mode::Detail, Mode::Dump] {
                        for jobs in [None, Some(1), Some(8)] {
                            let s = Selection {
                                json,
                                mode,
                                jobs,
                                ..sel(p, r)
                            };
                            assert_eq!(parsed(&s.args()), s);
                            n += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(n, 216);
    }

    #[test]
    fn quoting() {
        assert_eq!(quote("flow"), "flow");
        assert_eq!(quote("my pair"), "'my pair'");
        assert_eq!(quote("it's"), r"'it'\''s'");
        assert_eq!(quote(""), "''");
    }
}
