//! All the state of the UI. Nothing here draws, so every change can be tested with no
//! terminal.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::catalog::Pair;
use super::command::{self, Selection};
use super::runner::Run;
use super::search::Search;
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

/// The most run tabs kept. A new run past this drops the oldest.
pub const MAX_TABS: usize = 10;

/// One run and how the user looks at it: where the output pane is, and the search in it.
pub struct Tab {
    pub run: Run,
    /// The top line of the output pane; `None` follows the end of the output.
    pub top: Option<usize>,
    /// How many columns the output is moved to the left.
    pub left: usize,
    /// The last search in the output.
    pub search: Option<Search>,
}

impl Tab {
    pub fn new(run: Run) -> Self {
        Tab {
            run,
            top: None,
            left: 0,
            search: None,
        }
    }

    /// The first line the output pane shows, when it is `height` rows high.
    pub fn top_line(&self, height: usize) -> usize {
        let last = self.run.lines.len().saturating_sub(height);
        self.top.map_or(last, |t| t.min(last))
    }

    /// Move the output by `step` lines. Reaching the end follows the output again.
    fn scroll(&mut self, step: isize, height: usize) {
        let last = self.run.lines.len().saturating_sub(height);
        let to = (self.top_line(height) as isize + step).clamp(0, last as isize) as usize;
        self.top = (to < last).then_some(to);
    }

    /// Scroll so that line `l` is in the middle of the output pane.
    fn show_line(&mut self, l: usize, height: usize) {
        self.top = Some(l.saturating_sub(height / 2));
        self.scroll(0, height);
    }
}

/// What a line of typed text is for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ask {
    /// A search in the output.
    Search,
    /// A file name to save the output of the tab to.
    Save,
    /// Whether to write over the file the text names. `y` agrees.
    Overwrite,
}

/// A line the user types into, shown in place of the command line. It gets every key
/// until `Enter` or `Esc`.
pub struct Prompt {
    pub ask: Ask,
    pub text: String,
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
    /// One tab for each run, the oldest first.
    pub tabs: Vec<Tab>,
    /// The tab the output pane shows.
    pub tab: usize,
    /// The rows inside the output pane, as the last frame drew it.
    pub view_height: usize,
    /// The line the user types into, while it is open.
    pub prompt: Option<Prompt>,
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
            tabs: Vec::new(),
            tab: 0,
            view_height: 0,
            prompt: None,
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
        self.tabs.iter().any(|t| t.run.running())
    }

    /// The tab the output pane shows.
    pub fn current(&self) -> Option<&Tab> {
        self.tabs.get(self.tab)
    }

    fn current_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.tab)
    }

    /// Take the new output of each run, and search it.
    pub fn tick(&mut self) {
        for t in &mut self.tabs {
            t.run.poll();
            if let Some(s) = &mut t.search {
                s.update(&t.run.lines);
            }
        }
    }

    /// A key while the prompt is open: type, delete, confirm or give up.
    fn prompt_key(&mut self, k: KeyEvent) {
        let Some(p) = &mut self.prompt else { return };
        if p.ask == Ask::Overwrite {
            let p = self.prompt.take().expect("the prompt is open");
            if k.code == KeyCode::Char('y') {
                self.save(&p.text);
            } else {
                self.status = Some(format!("not saved: {} is there", p.text));
            }
            return;
        }
        match k.code {
            KeyCode::Char(c) => p.text.push(c),
            KeyCode::Backspace => {
                p.text.pop();
            }
            KeyCode::Esc => self.prompt = None,
            KeyCode::Enter => {
                let p = self.prompt.take().expect("the prompt is open");
                match p.ask {
                    Ask::Search => {
                        let h = self.view_height;
                        let Some(t) = self.current_mut() else { return };
                        if p.text.is_empty() {
                            t.search = None;
                            return;
                        }
                        let mut s = Search::new(p.text);
                        s.update(&t.run.lines);
                        // Start at the first match at or after the top line.
                        let top = t.top_line(h);
                        s.at = s.hits.iter().position(|&x| x >= top).unwrap_or(0);
                        let line = s.hits.get(s.at).copied();
                        t.search = Some(s);
                        if let Some(l) = line {
                            t.show_line(l, h);
                        }
                    }
                    Ask::Save if p.text.is_empty() => {}
                    Ask::Save if std::path::Path::new(&p.text).exists() => {
                        self.prompt = Some(Prompt {
                            ask: Ask::Overwrite,
                            text: p.text,
                        });
                    }
                    Ask::Save => self.save(&p.text),
                    Ask::Overwrite => unreachable!("handled above"),
                }
            }
            _ => {}
        }
    }

    /// Write every line of the tab to `path`, stdout and stderr in the order they came.
    fn save(&mut self, path: &str) {
        let Some(t) = self.current() else { return };
        let mut text = String::new();
        for l in &t.run.lines {
            text.push_str(&l.text);
            text.push('\n');
        }
        let n = t.run.lines.len();
        self.status = Some(match std::fs::write(path, text) {
            Ok(()) => format!("saved {n} lines to {path}"),
            Err(e) => format!("cannot save to {path}: {e}"),
        });
    }

    /// Move to the next match of the search, or the one before.
    fn next_match(&mut self, step: isize) {
        let h = self.view_height;
        let Some(t) = self.current_mut() else { return };
        if let Some(l) = t.search.as_mut().and_then(|s| s.step(step)) {
            t.show_line(l, h);
        }
    }

    /// Start the command of the selection, unless the CLI refuses it or a run is active.
    fn start(&mut self) {
        let args = self.selection().args();
        if command::refusal(&args).is_some() || self.running() {
            return;
        }
        match Run::start(&self.program, args) {
            Ok(r) => self.add_tab(r),
            Err(e) => self.status = Some(format!("cannot start {}: {e}", self.program)),
        }
    }

    /// Show `run` in a new tab. Past `MAX_TABS` the oldest tab is dropped.
    fn add_tab(&mut self, run: Run) {
        self.tabs.push(Tab::new(run));
        if self.tabs.len() > MAX_TABS {
            self.tabs.remove(0);
        }
        self.tab = self.tabs.len() - 1;
    }

    /// The first line the output pane shows; 0 with no run.
    #[cfg(test)]
    fn output_top(&self) -> usize {
        self.current().map_or(0, |t| t.top_line(self.view_height))
    }

    /// The keys of the output pane. True when the key was one of them.
    fn output_key(&mut self, k: KeyEvent, g: bool) -> bool {
        let h = self.view_height;
        let half = (h / 2).max(1) as isize;
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let n = self.tabs.len();
        match k.code {
            KeyCode::Char('[') => self.tab = self.tab.saturating_sub(1),
            KeyCode::Char(']') => self.tab = (self.tab + 1).min(n.saturating_sub(1)),
            _ => {}
        }
        let Some(t) = self.tabs.get_mut(self.tab) else {
            return matches!(k.code, KeyCode::Char('[' | ']'));
        };
        match k.code {
            KeyCode::Char('[' | ']') => {}
            KeyCode::Char('j') | KeyCode::Down => t.scroll(1, h),
            KeyCode::Char('k') | KeyCode::Up => t.scroll(-1, h),
            KeyCode::Char('d') if ctrl => t.scroll(half, h),
            KeyCode::Char('u') if ctrl => t.scroll(-half, h),
            KeyCode::PageDown => t.scroll(half * 2, h),
            KeyCode::PageUp => t.scroll(-half * 2, h),
            KeyCode::Char('g') if g => t.scroll(isize::MIN / 2, h),
            KeyCode::Char('G') => t.top = None,
            KeyCode::Char('l') | KeyCode::Right => t.left += 8,
            KeyCode::Char('h') | KeyCode::Left => t.left = t.left.saturating_sub(8),
            KeyCode::Char('0') => t.left = 0,
            KeyCode::Char('/') => {
                self.prompt = Some(Prompt {
                    ask: Ask::Search,
                    text: String::new(),
                })
            }
            KeyCode::Char('s') => {
                self.prompt = Some(Prompt {
                    ask: Ask::Save,
                    text: command::file_name(&t.run.args),
                })
            }
            KeyCode::Char('n') => self.next_match(1),
            KeyCode::Char('N') => self.next_match(-1),
            _ => return false,
        }
        true
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
        if self.prompt.is_some() {
            self.prompt_key(k);
            return;
        }
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
        if self.focus == Pane::Output {
            if k.code == KeyCode::Char('g') && !g {
                self.g = true;
                return;
            }
            if self.output_key(k, g) {
                return;
            }
        }

        match k.code {
            KeyCode::Char('q') if quit_asked || !self.running() => self.quit = true,
            KeyCode::Char('q') => {
                self.quit_asked = true;
                self.status =
                    Some("a run is active: press q again to stop it and quit".to_string());
            }
            KeyCode::Char('c') if k.modifiers.is_empty() => {
                for t in &mut self.tabs {
                    t.run.cancel();
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
        app.tabs
            .push(Tab::new(Run::start("sleep", vec!["30".into()]).unwrap()));
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
        app.tabs
            .push(Tab::new(Run::start("sleep", vec!["30".into()]).unwrap()));
        keys(&mut app, "qjq");
        assert!(!app.quit);

        // `c` cancels, and then one `q` quits.
        keys(&mut app, "c");
        assert!(!app.running());
        keys(&mut app, "q");
        assert!(app.quit);
    }

    /// An app whose run printed `n` lines, with an output pane `h` rows high.
    fn with_output(n: usize, h: usize) -> App {
        let script = format!("seq 1 {n}");
        let mut r = Run::start("sh", vec!["-c".into(), script]).unwrap();
        super::super::runner::tests::finish(&mut r);
        App {
            tabs: vec![Tab::new(r)],
            view_height: h,
            focus: Pane::Output,
            ..App::default()
        }
    }

    #[test]
    fn scroll() {
        let mut app = with_output(100, 10);
        assert_eq!(app.output_top(), 90);
        keys(&mut app, "k");
        assert_eq!((app.output_top(), app.tabs[0].top), (89, Some(89)));
        keys(&mut app, "gg");
        assert_eq!(app.output_top(), 0);
        keys(&mut app, "k");
        assert_eq!(app.output_top(), 0);
        press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(app.output_top(), 5);
        press(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert_eq!(app.output_top(), 0);
        keys(&mut app, "G");
        assert_eq!(app.tabs[0].top, None);
        // Down at the end stays at the end, and follows.
        keys(&mut app, "j");
        assert_eq!((app.output_top(), app.tabs[0].top), (90, None));
        keys(&mut app, "lll h");
        assert_eq!(app.tabs[0].left, 16);
        keys(&mut app, "0");
        assert_eq!(app.tabs[0].left, 0);

        // Output shorter than the pane never scrolls.
        let mut app = with_output(3, 10);
        keys(&mut app, "kggj");
        assert_eq!(app.output_top(), 0);
    }

    #[test]
    fn search() {
        let mut app = with_output(100, 10);
        keys(&mut app, "/");
        assert!(app.prompt.is_some());
        // While the prompt is open, `q` and `j` are text, not keys.
        keys(&mut app, "5qj");
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        assert!(!app.quit);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        let s = app.tabs[0].search.as_ref().unwrap();
        // 5, 15, 25, ... 95, and 50 to 59: 19 lines.
        assert_eq!(s.hits.len(), 19);
        // The search starts at the first match at or after the top line, 90: the line
        // "95", which is the last match. So `n` goes round to the first.
        assert_eq!((s.at, s.hits[s.at]), (18, 94));
        keys(&mut app, "n");
        assert_eq!(app.tabs[0].search.as_ref().unwrap().at, 0);
        assert_eq!(app.output_top(), 0);
        keys(&mut app, "N");
        assert_eq!(app.tabs[0].search.as_ref().unwrap().at, 18);
        assert_eq!(app.output_top(), 89);

        // `Esc` closes the prompt and keeps the old search.
        keys(&mut app, "/x");
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.prompt.is_none());
        assert_eq!(app.tabs[0].search.as_ref().unwrap().query, "5");
    }

    #[test]
    fn save() {
        let d = super::super::catalog::tests::scratch("save");
        let path = d.join("out.txt").to_string_lossy().into_owned();
        let mut app = with_output(3, 10);
        keys(&mut app, "s");
        assert_eq!(app.prompt.as_ref().unwrap().text, "antlion.txt");
        // Clear the default name and type the path.
        for _ in 0.."antlion.txt".len() {
            press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        }
        keys(&mut app, &path);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "1\n2\n3\n");

        // A file that is there needs a `y`; any other key keeps it.
        std::fs::write(&path, "old").unwrap();
        app.prompt = Some(Prompt {
            ask: Ask::Save,
            text: path.clone(),
        });
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.prompt.as_ref().unwrap().ask, Ask::Overwrite);
        keys(&mut app, "n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
        app.prompt = Some(Prompt {
            ask: Ask::Save,
            text: path.clone(),
        });
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        keys(&mut app, "y");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "1\n2\n3\n");
    }

    #[test]
    fn tabs() {
        let mut app = App {
            focus: Pane::Output,
            view_height: 10,
            ..App::default()
        };
        // `[` and `]` with no tab do nothing.
        keys(&mut app, "[]");
        assert_eq!(app.tab, 0);

        for i in 0..12 {
            let mut r = Run::start("sh", vec!["-c".into(), format!("seq 1 {}", 20 + i)]).unwrap();
            super::super::runner::tests::finish(&mut r);
            app.add_tab(r);
        }
        assert_eq!(app.tabs.len(), MAX_TABS);
        assert_eq!(app.tab, 9);
        // The oldest tab now is the third run, with 22 lines.
        keys(&mut app, "[[[[[[[[[[[[");
        assert_eq!(app.tab, 0);
        assert_eq!(app.tabs[0].run.lines.len(), 22);

        // Each tab keeps its own scroll and search.
        keys(&mut app, "gg/7");
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        keys(&mut app, "]");
        assert_eq!(app.tab, 1);
        assert!(app.tabs[1].search.is_none());
        assert_eq!(app.output_top(), 13);
        keys(&mut app, "[");
        assert_eq!(app.tabs[0].search.as_ref().unwrap().query, "7");
        keys(&mut app, "]]]]]]]]]]]]");
        assert_eq!(app.tab, 9);
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
