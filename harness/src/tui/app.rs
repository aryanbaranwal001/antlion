//! All the state of the UI. Nothing here draws, so every change can be tested with no
//! terminal.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::catalog::Pair;
use super::command::{self, Selection};
use super::runner::Run;
use crate::cli::Mode;
use crate::compare::Report;

/// The most threads `+` gives.
const MAX_JOBS: usize = 64;

/// The rows of the options pane, in order.
pub const OPTIONS: [&str; 3] = ["output", "mode", "jobs"];

/// One box on the screen.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Pane {
    #[default]
    Pairs,
    Reports,
    Options,
    Output,
}

impl Pane {
    /// Every pane, in `Tab` order.
    pub const ALL: [Pane; 4] = [Pane::Pairs, Pane::Reports, Pane::Options, Pane::Output];

    /// The pane `step` places after this one in `Tab` order, going round.
    fn next(self, step: isize) -> Pane {
        let i = Pane::ALL.iter().position(|p| *p == self).unwrap_or(0) as isize;
        Pane::ALL[(i + step).rem_euclid(Pane::ALL.len() as isize) as usize]
    }

    pub fn title(self) -> &'static str {
        match self {
            Pane::Pairs => "Pairs",
            Pane::Reports => "Reports",
            Pane::Options => "Options",
            Pane::Output => "Output",
        }
    }
}

/// A list where the user moves a cursor and selects items.
#[derive(Default)]
pub struct Pick {
    /// The item under the cursor.
    pub cur: usize,
    /// Whether each item is selected.
    pub on: Vec<bool>,
}

impl Pick {
    pub fn new(len: usize) -> Self {
        Pick {
            cur: 0,
            on: vec![false; len],
        }
    }

    pub fn count(&self) -> usize {
        self.on.iter().filter(|x| **x).count()
    }

    /// Move the cursor by `step`, and stop at each end.
    fn step(&mut self, step: isize) {
        let last = self.on.len().saturating_sub(1) as isize;
        self.cur = (self.cur as isize + step).clamp(0, last) as usize;
    }

    /// The list keys. True when the key was one of them.
    fn key(&mut self, code: KeyCode, g: bool) -> bool {
        match code {
            KeyCode::Char('j') | KeyCode::Down => self.step(1),
            KeyCode::Char('k') | KeyCode::Up => self.step(-1),
            KeyCode::Char('g') if g => self.cur = 0,
            KeyCode::Char('G') => self.cur = self.on.len().saturating_sub(1),
            KeyCode::Char(' ') => {
                if let Some(x) = self.on.get_mut(self.cur) {
                    *x = !*x;
                }
            }
            KeyCode::Char('a') => self.on.fill(true),
            KeyCode::Char('A') => self.on.fill(false),
            _ => return false,
        }
        true
    }
}

pub struct App {
    /// The pane that gets the keys.
    pub focus: Pane,
    /// The loop stops at the end of this pass.
    pub quit: bool,
    /// One line for the status bar, such as a warning at start.
    pub status: Option<String>,
    pub pairs: Vec<Pair>,
    pub pair_pick: Pick,
    /// One item for each report, in `Report::ALL` order.
    pub report_pick: Pick,
    /// The row under the cursor of the options pane, in `OPTIONS`.
    pub option: usize,
    pub json: bool,
    pub mode: Mode,
    /// `None` leaves the number of threads to antlion.
    pub jobs: Option<usize>,
    /// The program a run starts: the antlion binary itself.
    pub program: String,
    /// The last run, while it runs and after it ends.
    pub run: Option<Run>,
    /// `q` was pressed while a run was active, and needs a second `q`.
    quit_asked: bool,
    /// The first `g` of `gg` was pressed.
    g: bool,
}

impl Default for App {
    fn default() -> Self {
        App {
            focus: Pane::default(),
            quit: false,
            status: None,
            pairs: Vec::new(),
            pair_pick: Pick::default(),
            report_pick: Pick::new(Report::ALL.len()),
            option: 0,
            json: false,
            mode: Mode::Totals,
            jobs: None,
            program: "antlion".to_string(),
            run: None,
            quit_asked: false,
            g: false,
        }
    }
}

impl App {
    pub fn new(pairs: Vec<Pair>) -> Self {
        App {
            pair_pick: Pick::new(pairs.len()),
            pairs,
            ..App::default()
        }
    }

    pub fn running(&self) -> bool {
        self.run.as_ref().is_some_and(Run::running)
    }

    /// Take the new output of the run.
    pub fn tick(&mut self) {
        if let Some(r) = &mut self.run {
            r.poll();
        }
    }

    /// Start the command of the selection, unless the CLI refuses it or a run is active.
    fn start(&mut self) {
        let args = self.selection().args();
        if command::refusal(&args).is_some() || self.running() {
            return;
        }
        match Run::start(&self.program, args) {
            Ok(r) => self.run = Some(r),
            Err(e) => self.status = Some(format!("cannot start {}: {e}", self.program)),
        }
    }

    /// The keys of the options pane. True when the key was one of them.
    fn option_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.option = (self.option + 1).min(OPTIONS.len() - 1)
            }
            KeyCode::Char('k') | KeyCode::Up => self.option = self.option.saturating_sub(1),
            KeyCode::Char(' ') | KeyCode::Enter => match OPTIONS[self.option] {
                "output" => self.json = !self.json,
                "mode" => {
                    self.mode = match self.mode {
                        Mode::Totals => Mode::Detail,
                        Mode::Detail => Mode::Dump,
                        Mode::Dump => Mode::Totals,
                    }
                }
                _ => {}
            },
            // `auto` is below 1, so the number never goes under 1.
            KeyCode::Char('+') => self.jobs = Some(self.jobs.map_or(1, |n| (n + 1).min(MAX_JOBS))),
            KeyCode::Char('-') => self.jobs = self.jobs.and_then(|n| (n > 1).then(|| n - 1)),
            _ => return false,
        }
        true
    }

    /// The text of each row of the options pane.
    pub fn option_values(&self) -> [String; 3] {
        let mode = match self.mode {
            Mode::Totals => "totals",
            Mode::Detail => "detail",
            Mode::Dump => "dump",
        };
        [
            (if self.json { "json" } else { "text" }).to_string(),
            mode.to_string(),
            self.jobs.map_or("auto".to_string(), |n| n.to_string()),
        ]
    }

    /// What the lists and the options have selected: pairs in list order, reports in
    /// `Report::ALL` order.
    pub fn selection(&self) -> Selection {
        let picked = |pick: &Pick| {
            pick.on
                .iter()
                .enumerate()
                .filter(|(_, on)| **on)
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        };
        Selection {
            pairs: picked(&self.pair_pick)
                .into_iter()
                .map(|i| self.pairs[i].name.clone())
                .collect(),
            reports: picked(&self.report_pick)
                .into_iter()
                .map(|i| Report::ALL[i])
                .collect(),
            json: self.json,
            mode: self.mode,
            jobs: self.jobs,
        }
    }

    /// The report under the cursor of the reports pane.
    pub fn report_at_cursor(&self) -> Report {
        Report::ALL[self.report_pick.cur]
    }

    pub fn key(&mut self, k: KeyEvent) {
        let g = std::mem::take(&mut self.g);
        let quit_asked = std::mem::take(&mut self.quit_asked);
        let list = match self.focus {
            Pane::Pairs => Some(&mut self.pair_pick),
            Pane::Reports => Some(&mut self.report_pick),
            _ => None,
        };
        if let Some(list) = list {
            if k.code == KeyCode::Char('g') && !g {
                self.g = true;
                return;
            }
            if list.key(k.code, g) {
                return;
            }
        }

        if self.focus == Pane::Options && self.option_key(k.code) {
            return;
        }

        match k.code {
            KeyCode::Char('q') if quit_asked || !self.running() => self.quit = true,
            KeyCode::Char('q') => {
                self.quit_asked = true;
                self.status =
                    Some("a run is active: press q again to stop it and quit".to_string());
            }
            KeyCode::Char('c') if k.modifiers.is_empty() => {
                if let Some(r) = &mut self.run {
                    r.cancel();
                }
            }
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('r') => self.start(),
            KeyCode::Tab => self.focus = self.focus.next(1),
            KeyCode::BackTab => self.focus = self.focus.next(-1),
            KeyCode::Char(c @ '1'..='4') => self.focus = Pane::ALL[c as usize - '1' as usize],
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
        app.key(KeyEvent::new(code, modifiers));
    }

    fn keys(app: &mut App, keys: &str) {
        for c in keys.chars() {
            press(app, KeyCode::Char(c), KeyModifiers::NONE);
        }
    }

    fn pair(name: &str) -> Pair {
        Pair {
            name: name.to_string(),
            state: super::super::catalog::State::Built,
            source: true,
        }
    }

    #[test]
    fn pairs_list() {
        let mut app = App::new(vec![pair("a"), pair("b"), pair("c")]);
        keys(&mut app, "k");
        assert_eq!(app.pair_pick.cur, 0);
        keys(&mut app, "jj j");
        assert_eq!(app.pair_pick.cur, 2);
        assert_eq!(app.pair_pick.on, [false, false, true]);
        keys(&mut app, "gg ");
        assert_eq!(app.pair_pick.on, [true, false, true]);
        keys(&mut app, "G");
        assert_eq!(app.pair_pick.cur, 2);
        keys(&mut app, "A");
        assert_eq!(app.pair_pick.count(), 0);
        keys(&mut app, "a");
        assert_eq!(app.pair_pick.count(), 3);

        // One `g` and then another key is not `gg`.
        keys(&mut app, "gjG");
        assert_eq!(app.pair_pick.cur, 2);

        // An empty list takes every key.
        let mut app = App::new(Vec::new());
        keys(&mut app, "jkG gg aA");
        assert_eq!(app.pair_pick.cur, 0);
    }

    #[test]
    fn reports_list() {
        let mut app = App::default();
        keys(&mut app, "2");
        assert_eq!(app.focus, Pane::Reports);
        keys(&mut app, "jj ");
        assert_eq!(app.report_at_cursor(), Report::Imports);
        assert_eq!(app.report_pick.on[2], true);
        keys(&mut app, "G");
        assert_eq!(app.report_at_cursor(), Report::Layout);
        keys(&mut app, "a");
        assert_eq!(app.report_pick.count(), Report::ALL.len());
        // The pairs keep their own cursor and selection.
        assert_eq!(app.pair_pick.count(), 0);
    }

    #[test]
    fn options() {
        let mut app = App::default();
        keys(&mut app, "3");
        assert_eq!(app.option_values(), ["text", "totals", "auto"]);
        keys(&mut app, " ");
        assert!(app.json);
        keys(&mut app, "j   ");
        assert_eq!(app.mode, Mode::Totals);
        keys(&mut app, " ");
        assert_eq!(app.mode, Mode::Detail);
        keys(&mut app, " ");
        assert_eq!(app.mode, Mode::Dump);
        keys(&mut app, "jjj");
        assert_eq!(OPTIONS[app.option], "jobs");

        keys(&mut app, "-");
        assert_eq!(app.jobs, None);
        keys(&mut app, "++");
        assert_eq!(app.jobs, Some(2));
        keys(&mut app, "---");
        assert_eq!(app.jobs, None);
        for _ in 0..100 {
            keys(&mut app, "+");
        }
        assert_eq!(app.jobs, Some(MAX_JOBS));
    }

    #[test]
    fn quit_during_a_run() {
        let mut app = App::default();
        app.run = Some(Run::start("sleep", vec!["30".into()]).unwrap());
        keys(&mut app, "q");
        assert!(!app.quit);
        assert!(
            app.status
                .as_deref()
                .is_some_and(|s| s.contains("press q again"))
        );
        keys(&mut app, "q");
        assert!(app.quit);

        // Any other key in between asks again.
        let mut app = App::default();
        app.run = Some(Run::start("sleep", vec!["30".into()]).unwrap());
        keys(&mut app, "qjq");
        assert!(!app.quit);

        // `c` cancels, and then one `q` quits.
        keys(&mut app, "c");
        assert!(!app.running());
        keys(&mut app, "q");
        assert!(app.quit);
    }

    #[test]
    fn focus() {
        let mut app = App::default();
        assert_eq!(app.focus, Pane::Pairs);
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.focus, Pane::Reports);
        press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(app.focus, Pane::Output);
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.focus, Pane::Pairs);
        press(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
        assert_eq!(app.focus, Pane::Options);
        press(&mut app, KeyCode::Char('4'), KeyModifiers::NONE);
        assert_eq!(app.focus, Pane::Output);
    }

    #[test]
    fn q_and_ctrl_c_quit() {
        let mut app = App::default();
        press(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!app.quit);
        press(&mut app, KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(app.quit);

        let mut app = App::default();
        press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(app.quit);
    }
}
