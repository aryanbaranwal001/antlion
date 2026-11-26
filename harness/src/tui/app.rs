//! All the state of the UI. Nothing here draws, so every change can be tested with no
//! terminal.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::catalog::Pair;
use super::command;
use super::complete::{self, Suggestion};
use super::runner::Run;

/// The panel that `Up`, `Down`, `PgUp`, `PgDn`, `Ctrl+Home` and `Ctrl+End` move in. The
/// letters always go into the command line, whatever has the focus.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Focus {
    #[default]
    Suggestions,
    Output,
    Details,
}

impl Focus {
    /// Every panel, in `Tab` order.
    const ALL: [Focus; 3] = [Focus::Suggestions, Focus::Output, Focus::Details];

    /// The panel `step` places after this one in `Tab` order, going round.
    fn next(self, step: isize) -> Focus {
        let i = Focus::ALL.iter().position(|f| *f == self).unwrap_or(0) as isize;
        Focus::ALL[(i + step).rem_euclid(Focus::ALL.len() as isize) as usize]
    }
}

/// The most run tabs kept. A new run past this drops the oldest.
pub const MAX_TABS: usize = 10;

/// One run and how the user looks at it: where the output pane is.
pub struct Tab {
    pub run: Run,
    /// The top line of the output pane; `None` follows the end of the output.
    pub top: Option<usize>,
    /// How many columns the output is moved to the left.
    pub left: usize,
    kind: Kind,
}

/// What a tab runs.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// An antlion run that builds nothing. It goes into the history.
    Run,
    /// An antlion run that builds pairs. It goes into the history, and its end changes the
    /// build state of the pairs.
    Build,
}

impl Tab {
    pub fn new(run: Run) -> Self {
        Tab {
            run,
            top: None,
            left: 0,
            kind: Kind::Run,
        }
    }

    /// What the tab runs, as a shell line.
    pub fn title(&self) -> String {
        command::line(&self.run.args)
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
}

/// What a line of typed text is for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ask {
    /// The command palette: the text filters the actions.
    Palette,
    /// The history of runs, in the palette box: the text filters the runs.
    History,
    /// A file name to save the output of the tab to.
    Save,
    /// Whether to write over the file the text names. `y` agrees.
    Overwrite,
}

/// What a palette entry does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    Help,
    Copy,
    Run,
    Cancel,
    Save,
    Quit,
    /// Open the history of runs.
    History,
    /// Check the tools, `out/` and the pairs.
    Doctor,
    /// Put the run with this index in the history into the command line.
    Load(usize),
}

/// One entry of the command palette: what it says, its key, and what it does.
pub struct Entry {
    pub label: String,
    pub key: &'static str,
    pub act: Act,
}

/// A line the user types into, shown in the status bar. It gets every key until `Enter`
/// or `Esc`.
pub struct Prompt {
    pub ask: Ask,
    pub text: String,
}

pub struct App {
    /// The panel the moving keys act on.
    pub focus: Focus,
    /// The loop stops at the end of this pass.
    pub quit: bool,
    /// One line for the status bar, such as a warning at start.
    pub status: Option<String>,
    pub pairs: Vec<Pair>,
    /// What the user typed after `antlion`.
    pub line: String,
    /// Where the cursor is in `line`, in bytes.
    pub cursor: usize,
    /// The words that can come at the cursor, the best match first.
    pub suggestions: Vec<Suggestion>,
    /// The suggestion under the cursor of the suggestions pane.
    pub pick: usize,
    /// The title and the lines of the details pane.
    pub details: (String, Vec<String>),
    /// The first line the details pane shows.
    pub details_top: usize,
    /// The folder the pairs are read from, to read them again after a build.
    pub root: std::path::PathBuf,
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
    /// The row under the cursor of the palette, in the entries that match.
    pub palette_cur: usize,
    /// The help overlay, while it is open: the first line it shows.
    pub help: Option<usize>,
    /// A box of text over the screen, while it is open: its title, its lines and the
    /// first line shown.
    pub info: Option<(String, Vec<String>, usize)>,
    /// The folder of the history file; `None` keeps no history.
    pub state_dir: Option<std::path::PathBuf>,
    /// The runs in the history, the oldest first.
    pub history: Vec<super::store::Record>,
    /// While Alt+Up and Alt+Down step through the history: the run in the command line,
    /// and the text the line had before the first step.
    pub recall: Option<(usize, String)>,
    /// Text to put in the clipboard, which the loop sends to the terminal after the draw.
    pub copy: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        let mut app = App {
            focus: Focus::default(),
            quit: false,
            status: None,
            pairs: Vec::new(),
            line: String::new(),
            cursor: 0,
            suggestions: Vec::new(),
            pick: 0,
            details: (String::new(), Vec::new()),
            details_top: 0,
            root: std::path::PathBuf::from("."),
            program: "antlion".to_string(),
            tabs: Vec::new(),
            tab: 0,
            view_height: 0,
            prompt: None,
            palette_cur: 0,
            help: None,
            info: None,
            copy: None,
            state_dir: None,
            history: Vec::new(),
            recall: None,
        };
        app.refresh();
        app
    }
}

impl App {
    pub fn new(pairs: Vec<Pair>) -> Self {
        let mut app = App {
            pairs,
            ..App::default()
        };
        app.refresh();
        app
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

    /// The arguments of the command line, without the program name.
    pub fn args(&self) -> Vec<String> {
        self.line.split_whitespace().map(String::from).collect()
    }

    /// The text of the command line up to the cursor.
    pub fn before(&self) -> &str {
        &self.line[..self.cursor]
    }

    /// Find the words that can come at the cursor again, and the details of the one under
    /// the cursor of the suggestions pane.
    fn refresh(&mut self) {
        self.suggestions = complete::suggest(self.before(), &self.pairs);
        self.pick = self.pick.min(self.suggestions.len().saturating_sub(1));
        let details = complete::details(
            self.suggestions.get(self.pick),
            self.before(),
            &self.root,
            &self.pairs,
        );
        // New details start at their top.
        if details != self.details {
            self.details_top = 0;
        }
        self.details = details;
    }

    /// Take the new output of each run, and search it. When a build ends, read the build
    /// state of the pairs again.
    pub fn tick(&mut self) {
        let mut built = false;
        let mut ended = Vec::new();
        for t in &mut self.tabs {
            let was = t.run.running();
            t.run.poll();
            built |= t.kind == Kind::Build && was && !t.run.running();
            if was && let Some((end, time)) = t.run.end {
                let exit = match end {
                    super::runner::End::Exited(c) => c,
                    super::runner::End::Cancelled => None,
                };
                let r = super::store::Record::new(t.run.args.clone(), exit, time.as_secs_f64());
                ended.push(r);
            }
        }
        if built {
            self.reload();
        }
        for r in ended {
            self.remember(r);
        }
    }

    /// Add a run to the history, in memory and in its file.
    fn remember(&mut self, r: super::store::Record) {
        if let Some(d) = &self.state_dir
            && let Err(e) = super::store::add(d, &r)
        {
            self.status = Some(format!("cannot write the history in {}: {e}", d.display()));
        }
        self.history.push(r);
        let from = self.history.len().saturating_sub(super::store::MAX_HISTORY);
        self.history.drain(..from);
    }

    /// Put `args` into the command line, with the cursor after a space at its end, so the
    /// suggestions show what can come next.
    fn load(&mut self, args: &[String]) {
        self.line = format!("{} ", args.join(" "));
        self.cursor = self.line.len();
        self.pick = 0;
        self.refresh();
        self.status = Some("loaded: press Ctrl+R to run it".to_string());
    }

    /// Put the run `step` places older (-1) or newer (+1) in the history into the command
    /// line. A step past the newest run puts back the text the line had before.
    fn recall(&mut self, step: isize) {
        let (at, draft) = self
            .recall
            .take()
            .unwrap_or_else(|| (self.history.len(), self.line.clone()));
        let at = at.saturating_add_signed(step).min(self.history.len());
        if at == self.history.len() {
            self.line = draft;
            self.cursor = self.line.len();
            self.pick = 0;
            self.refresh();
            return;
        }
        let args = self.history[at].args.clone();
        self.load(&args);
        self.recall = Some((at, draft));
    }

    /// Read the pairs again, so the suggestions show a build made since.
    fn reload(&mut self) {
        self.pairs = super::catalog::pairs(&self.root);
        self.refresh();
    }

    /// Show `run` in a new tab of `kind`.
    fn add_kind(&mut self, run: Run, kind: Kind) {
        self.add_tab(run);
        if let Some(t) = self.tabs.last_mut() {
            t.kind = kind;
        }
    }

    /// A key while the prompt is open: type, delete, confirm or give up.
    fn prompt_key(&mut self, k: KeyEvent) {
        let Some(p) = &mut self.prompt else { return };
        if matches!(p.ask, Ask::Palette | Ask::History) {
            self.palette_key(k);
            return;
        }
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
                    Ask::Save if p.text.is_empty() => {}
                    Ask::Save if std::path::Path::new(&p.text).exists() => {
                        self.prompt = Some(Prompt {
                            ask: Ask::Overwrite,
                            text: p.text,
                        });
                    }
                    Ask::Save => self.save(&p.text),
                    Ask::Palette | Ask::History => {
                        unreachable!("the palette has its own keys")
                    }
                    Ask::Overwrite => unreachable!("handled above"),
                }
            }
            _ => {}
        }
    }

    /// Every entry of the palette.
    pub fn entries(&self) -> Vec<Entry> {
        let e = |label: &str, key, act| Entry {
            label: label.to_string(),
            key,
            act,
        };
        vec![
            e("help: every key", "F1", Act::Help),
            e("run the command", "Ctrl+R", Act::Run),
            e("copy the command", "Ctrl+Y", Act::Copy),
            e("cancel the run", "Ctrl+C", Act::Cancel),
            e("save the output to a file", "Ctrl+S", Act::Save),
            e("history of runs", "", Act::History),
            e(
                "doctor: check the tools, out/ and the builds",
                "",
                Act::Doctor,
            ),
            e("quit", "Ctrl+Q", Act::Quit),
        ]
    }

    /// The entries that match the text of the palette, the best match first.
    pub fn palette_hits(&self) -> Vec<Entry> {
        let text = self.prompt.as_ref().map_or("", |p| p.text.as_str());
        let entries = match self.prompt.as_ref().map(|p| p.ask) {
            Some(Ask::History) => self.history_entries(),
            _ => self.entries(),
        };
        let mut all: Vec<Option<Entry>> = entries.into_iter().map(Some).collect();
        let names: Vec<&str> = all.iter().flatten().map(|e| e.label.as_str()).collect();
        let order = super::search::ranked(text, &names);
        order.into_iter().filter_map(|i| all[i].take()).collect()
    }

    /// The runs of the history as palette entries, the newest first.
    fn history_entries(&self) -> Vec<Entry> {
        self.history
            .iter()
            .enumerate()
            .rev()
            .map(|(i, r)| {
                let end = match r.exit {
                    Some(0) => "done".to_string(),
                    Some(c) => format!("exit {c}"),
                    None => "cancelled".to_string(),
                };
                Entry {
                    label: format!(
                        "{}   {end}, {:.1} s, {} ago",
                        command::line(&r.args),
                        r.secs,
                        super::store::ago(r.time)
                    ),
                    key: "",
                    act: Act::Load(i),
                }
            })
            .collect()
    }

    /// A key while the palette is open.
    fn palette_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let Some(p) = &mut self.prompt else { return };
        match k.code {
            KeyCode::Esc => self.prompt = None,
            KeyCode::Down | KeyCode::Tab => self.palette_cur += 1,
            KeyCode::Char('n') if ctrl => self.palette_cur += 1,
            KeyCode::Up | KeyCode::BackTab => self.palette_cur = self.palette_cur.saturating_sub(1),
            KeyCode::Char('p') if ctrl => self.palette_cur = self.palette_cur.saturating_sub(1),
            KeyCode::Char(c) => {
                p.text.push(c);
                self.palette_cur = 0;
            }
            KeyCode::Backspace => {
                p.text.pop();
                self.palette_cur = 0;
            }
            KeyCode::Enter => {
                let hits = self.palette_hits();
                let act = hits.get(self.palette_cur.min(hits.len().saturating_sub(1)));
                let act = act.map(|e| e.act);
                self.prompt = None;
                if let Some(a) = act {
                    self.act(a);
                }
            }
            _ => {}
        }
        let n = self.palette_hits().len();
        self.palette_cur = self.palette_cur.min(n.saturating_sub(1));
    }

    /// Open the prompt for `ask`, empty.
    fn ask(&mut self, ask: Ask) {
        self.palette_cur = 0;
        self.prompt = Some(Prompt {
            ask,
            text: String::new(),
        });
    }

    /// Do what a palette entry says.
    fn act(&mut self, a: Act) {
        match a {
            Act::Help => self.help = Some(0),
            Act::Copy => {
                let line = command::line(&self.args());
                self.status = Some(format!("copied: {line}"));
                self.copy = Some(line);
            }
            Act::Run => self.start(),
            Act::Cancel => self.tabs.iter_mut().for_each(|t| t.run.cancel()),
            Act::Save => {
                if let Some(t) = self.current() {
                    self.prompt = Some(Prompt {
                        ask: Ask::Save,
                        text: command::file_name(&t.run.args),
                    });
                }
            }
            Act::Quit => self.quit = true,
            Act::History => self.ask(Ask::History),
            Act::Doctor => {
                // Read the pairs first, so a build made outside the UI is seen.
                self.reload();
                let lines = super::catalog::doctor(&self.root, &self.pairs);
                self.info = Some(("doctor".to_string(), lines, 0));
            }
            Act::Load(i) => {
                if let Some(r) = self.history.get(i) {
                    let args = r.args.clone();
                    self.load(&args);
                }
            }
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

    /// Start the command line, unless the CLI refuses it or a run is active.
    fn start(&mut self) {
        let args = self.args();
        if let Some(e) = command::refusal(&args) {
            self.status = Some(format!("not run: {e}"));
            return;
        }
        if self.running() {
            self.status = Some("a run is active: wait for it, or cancel it".to_string());
            return;
        }
        let kind = if args[0] == "--build" {
            Kind::Build
        } else {
            Kind::Run
        };
        match Run::start(&self.program, args, &self.root) {
            Ok(r) => {
                self.recall = None;
                self.add_kind(r, kind)
            }
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

    /// Move in the panel that has the focus: a line or an item with `Up` and `Down`, a page
    /// with `PgUp` and `PgDn`, the start and the end with `Ctrl+Home` and `Ctrl+End`.
    fn move_in(&mut self, k: KeyCode) {
        let h = self.view_height.max(1) as isize;
        let step = match k {
            KeyCode::Up => -1,
            KeyCode::Down => 1,
            KeyCode::PageUp => -h,
            KeyCode::PageDown => h,
            KeyCode::Home => isize::MIN / 2,
            _ => isize::MAX / 2,
        };
        match self.focus {
            Focus::Suggestions => {
                let last = self.suggestions.len().saturating_sub(1) as isize;
                self.pick = (self.pick as isize + step).clamp(0, last) as usize;
                self.refresh();
            }
            Focus::Output => {
                let Some(t) = self.tabs.get_mut(self.tab) else {
                    return;
                };
                if k == KeyCode::End {
                    t.top = None;
                } else {
                    t.scroll(step, h as usize);
                }
            }
            Focus::Details => {
                let last = self.details.1.len().saturating_sub(1) as isize;
                self.details_top = (self.details_top as isize + step).clamp(0, last) as usize;
            }
        }
    }

    /// The char before the cursor starts at this byte; 0 at the start of the line.
    fn char_before(&self) -> usize {
        self.before()
            .chars()
            .next_back()
            .map_or(0, |c| self.cursor - c.len_utf8())
    }

    /// The char after the cursor ends at this byte; the cursor at the end of the line.
    fn char_after(&self) -> usize {
        self.line[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |c| self.cursor + c.len_utf8())
    }

    /// Put the suggestion under the cursor in place of the word typed so far, and a space
    /// after it, so the next suggestions show at once.
    fn accept(&mut self) {
        let Some(s) = self.suggestions.get(self.pick) else {
            return;
        };
        let start = self.cursor - complete::word(self.before()).len();
        let text = s.text.clone();
        self.line.replace_range(start..self.cursor, &text);
        self.cursor = start + text.len();
        if !self.line[self.cursor..].starts_with(' ') {
            self.line.insert(self.cursor, ' ');
        }
        self.cursor += 1;
    }

    /// The keys that edit the command line.
    fn line_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        let was = (self.line.clone(), self.cursor);
        match k.code {
            KeyCode::Char('u') if ctrl => {
                self.line.replace_range(..self.cursor, "");
                self.cursor = 0;
            }
            KeyCode::Char('w') if ctrl => {
                let head = self.before().trim_end();
                let start = head.rfind(' ').map_or(0, |i| i + 1);
                self.line.replace_range(start..self.cursor, "");
                self.cursor = start;
            }
            KeyCode::Char(c) if !ctrl && !alt => {
                self.line.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }
            KeyCode::Backspace if self.cursor > 0 => {
                let at = self.char_before();
                self.line.replace_range(at..self.cursor, "");
                self.cursor = at;
            }
            KeyCode::Delete => {
                let to = self.char_after();
                self.line.replace_range(self.cursor..to, "");
            }
            KeyCode::Left => self.cursor = self.char_before(),
            KeyCode::Right => self.cursor = self.char_after(),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.line.len(),
            KeyCode::Enter => self.accept(),
            _ => {}
        }
        // A new word gets new suggestions, so the cursor goes back to the best one.
        if (&self.line, self.cursor) != (&was.0, was.1) {
            self.pick = 0;
        }
        // An edit makes the line the user's own, so the next Alt+Up starts again from
        // the newest run.
        if self.line != was.0 {
            self.recall = None;
        }
        self.refresh();
    }

    /// Every key. The letters always go into the command line; the other keys have no
    /// text, so they never need a mode.
    pub fn key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        // Ctrl+C and Ctrl+Q work everywhere, even while a box or a prompt is open.
        match k.code {
            KeyCode::Char('c') if ctrl && self.running() => {
                self.act(Act::Cancel);
                return;
            }
            KeyCode::Char('c' | 'q') if ctrl => {
                self.quit = true;
                return;
            }
            _ => {}
        }
        if let Some((_, _, top)) = &mut self.info {
            match k.code {
                KeyCode::Down => *top += 1,
                KeyCode::Up => *top = top.saturating_sub(1),
                KeyCode::Esc => self.info = None,
                _ => {}
            }
            return;
        }
        if let Some(top) = &mut self.help {
            match k.code {
                KeyCode::Down => *top += 1,
                KeyCode::Up => *top = top.saturating_sub(1),
                KeyCode::Esc | KeyCode::F(1) => self.help = None,
                _ => {}
            }
            return;
        }
        if self.prompt.is_some() {
            self.prompt_key(k);
            return;
        }
        let n = self.tabs.len();
        match k.code {
            KeyCode::Char('p') if ctrl => self.ask(Ask::Palette),
            KeyCode::Char('r') if ctrl => self.start(),
            KeyCode::Char('s') if ctrl => self.act(Act::Save),
            KeyCode::Char('y') if ctrl => self.act(Act::Copy),
            KeyCode::F(1) => self.help = Some(0),
            KeyCode::Tab => self.focus = self.focus.next(1),
            KeyCode::BackTab => self.focus = self.focus.next(-1),
            KeyCode::PageUp if ctrl => self.tab = self.tab.saturating_sub(1),
            KeyCode::PageDown if ctrl => self.tab = (self.tab + 1).min(n.saturating_sub(1)),
            KeyCode::Left | KeyCode::Right if shift => {
                if let Some(t) = self.current_mut() {
                    t.left = if k.code == KeyCode::Left {
                        t.left.saturating_sub(8)
                    } else {
                        t.left + 8
                    };
                }
            }
            KeyCode::Up if alt => self.recall(-1),
            KeyCode::Down if alt => self.recall(1),
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                self.move_in(k.code)
            }
            KeyCode::Home | KeyCode::End if ctrl => self.move_in(k.code),
            _ => self.line_key(k),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
        app.key(KeyEvent::new(code, modifiers));
    }

    fn keys(app: &mut App, keys: &str) {
        for c in keys.chars() {
            press(app, KeyCode::Char(c), KeyModifiers::NONE);
        }
    }

    fn enter(app: &mut App) {
        press(app, KeyCode::Enter, KeyModifiers::NONE);
    }

    fn ctrl(app: &mut App, c: char) {
        press(app, KeyCode::Char(c), KeyModifiers::CONTROL);
    }

    fn pair(name: &str) -> Pair {
        Pair {
            name: name.to_string(),
            state: super::super::catalog::State::Built,
            source: true,
        }
    }

    /// Write `text` to `path` as a script, and wait until it can run. When another test
    /// thread forks just then, its child holds the file open for writing for a moment, and
    /// Linux refuses to run it with "Text file busy".
    fn script(path: &Path, text: &str) {
        std::fs::write(path, text).unwrap();
        std::process::Command::new("chmod")
            .arg("+x")
            .arg(path)
            .status()
            .unwrap();
        // A run of the script in a folder of its own, so it changes nothing of the test.
        let probe = path.with_extension("probe");
        std::fs::create_dir_all(&probe).unwrap();
        let stop = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < stop {
            match std::process::Command::new(path)
                .current_dir(&probe)
                .output()
            {
                Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                    std::thread::sleep(std::time::Duration::from_millis(10))
                }
                _ => return,
            }
        }
    }

    fn texts(app: &App) -> Vec<&str> {
        app.suggestions.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn typing() {
        let mut app = App::default();
        // Every character is text in the command line, `q` too.
        keys(&mut app, "--compare flw");
        assert!(!app.quit);
        assert_eq!(app.line, "--compare flw");
        // Fix the typo: one left, a delete, then the right letters.
        press(&mut app, KeyCode::Left, KeyModifiers::NONE);
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        keys(&mut app, "lo");
        assert_eq!(app.line, "--compare flow");
        press(&mut app, KeyCode::Home, KeyModifiers::NONE);
        press(&mut app, KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(app.line, "-compare flow");
        press(&mut app, KeyCode::End, KeyModifiers::NONE);
        ctrl(&mut app, 'w');
        assert_eq!(app.line, "-compare ");
        ctrl(&mut app, 'u');
        assert_eq!((app.line.as_str(), app.cursor), ("", 0));
        // Keys at the ends of the line do nothing.
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        press(&mut app, KeyCode::Left, KeyModifiers::NONE);
        press(&mut app, KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!((app.line.as_str(), app.cursor), ("", 0));
    }

    #[test]
    fn suggestions_follow_the_line() {
        let mut app = App::new(vec![pair("flow"), pair("numeric")]);
        assert_eq!(texts(&app)[0], "--compare");
        keys(&mut app, "--c");
        enter(&mut app);
        assert_eq!(app.line, "--compare ");
        assert_eq!(texts(&app), ["flow", "numeric"]);
        // Down picks the next suggestion, and Enter puts it in.
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        enter(&mut app);
        assert_eq!(app.line, "--compare numeric ");
        assert_eq!(texts(&app), ["flow", "--report"]);
        keys(&mut app, "--r");
        enter(&mut app);
        // The word typed filters the reports, the best match first.
        keys(&mut app, "co");
        assert_eq!(texts(&app)[0], "cost");
        enter(&mut app);
        assert_eq!(app.line, "--compare numeric --report cost ");
        // The cursor never goes past the last suggestion.
        for _ in 0..50 {
            press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        }
        assert_eq!(app.pick, app.suggestions.len() - 1);
        // Typing puts it back on the best one.
        keys(&mut app, "-");
        assert_eq!(app.pick, 0);
    }

    #[test]
    fn accept_in_the_middle() {
        let mut app = App::new(vec![pair("flow")]);
        keys(&mut app, "--compare fl --report size");
        for _ in 0.." --report size".len() {
            press(&mut app, KeyCode::Left, KeyModifiers::NONE);
        }
        enter(&mut app);
        // The space that was there is kept, and no second one is added.
        assert_eq!(app.line, "--compare flow --report size");
        assert_eq!(&app.line[app.cursor..], "--report size");
    }

    #[test]
    fn run_only_what_the_cli_takes() {
        let mut app = App::default();
        keys(&mut app, "--compare flow");
        ctrl(&mut app, 'r');
        assert!(app.tabs.is_empty());
        assert!(
            app.status
                .as_deref()
                .is_some_and(|s| s.starts_with("not run: no report requested"))
        );
    }

    #[test]
    fn palette() {
        let mut app = App::default();
        ctrl(&mut app, 'p');
        keys(&mut app, "help");
        assert_eq!(app.palette_hits()[0].act, Act::Help);
        enter(&mut app);
        assert!(app.prompt.is_none());
        assert_eq!(app.help, Some(0));
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);

        // While the palette is open, its keys are text: `q` does not quit.
        ctrl(&mut app, 'p');
        keys(&mut app, "quit");
        assert!(!app.quit);
        // `Esc` closes it and does nothing.
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.quit && app.prompt.is_none());
        ctrl(&mut app, 'p');
        keys(&mut app, "quit");
        enter(&mut app);
        assert!(app.quit);

        // No match, and `Enter` does nothing.
        let mut app = App::default();
        ctrl(&mut app, 'p');
        keys(&mut app, "zzzzz");
        enter(&mut app);
        assert!(!app.quit && app.prompt.is_none());
    }

    #[test]
    fn help() {
        let mut app = App::default();
        press(&mut app, KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(app.help, Some(0));
        // While help is open, keys scroll it and do nothing else.
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        press(&mut app, KeyCode::Up, KeyModifiers::NONE);
        keys(&mut app, "abc");
        assert_eq!(app.help, Some(1));
        assert_eq!(app.line, "");
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(app.help, None);
    }

    #[test]
    fn copy() {
        let mut app = App::default();
        keys(&mut app, "--compare flow   --report size");
        ctrl(&mut app, 'y');
        assert_eq!(
            app.copy.as_deref(),
            Some("antlion --compare flow --report size")
        );
        assert!(
            app.status
                .as_deref()
                .is_some_and(|s| s.starts_with("copied: "))
        );
    }

    #[test]
    fn history() {
        let d = super::super::catalog::tests::scratch("app-history");
        let mut app = App::new(vec![pair("flow"), pair("numeric")]);
        app.state_dir = Some(d.clone());

        // A run that ends goes into the history, in memory and in the file.
        let args: Vec<String> = ["-c", "true"].map(String::from).to_vec();
        app.add_tab(Run::start("sh", args.clone(), Path::new(".")).unwrap());
        let stop = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.running() && std::time::Instant::now() < stop {
            app.tick();
        }
        app.tick();
        assert_eq!(app.history.len(), 1);
        assert_eq!(app.history[0].args, args);
        assert_eq!(app.history[0].exit, Some(0));
        assert_eq!(super::super::store::history(&d).len(), 1);

        // Loading a run puts it in the command line.
        app.history.push(super::super::store::Record::new(
            ["--compare", "numeric", "--report", "cost", "--json"]
                .map(String::from)
                .to_vec(),
            Some(0),
            1.0,
        ));
        ctrl(&mut app, 'p');
        keys(&mut app, "history of runs");
        enter(&mut app);
        assert!(
            app.palette_hits()[0]
                .label
                .starts_with("antlion --compare numeric --report cost --json   done")
        );
        enter(&mut app);
        assert_eq!(app.line, "--compare numeric --report cost --json ");
        assert_eq!(app.cursor, app.line.len());
        assert_eq!(texts(&app), ["--detail", "--bare", "--jobs"]);
    }

    #[test]
    fn recall_with_alt() {
        let record = |args: &[&str]| {
            super::super::store::Record::new(
                args.iter().map(|a| a.to_string()).collect(),
                Some(0),
                1.0,
            )
        };
        let mut app = App {
            history: vec![
                record(&["--compare", "flow", "--report", "size"]),
                record(&["--build", "contracts/flow"]),
            ],
            ..App::default()
        };
        let alt = |app: &mut App, code| press(app, code, KeyModifiers::ALT);
        keys(&mut app, "--comp");

        // Alt+Up goes to the newest run first, then to older ones, and stops at the oldest.
        alt(&mut app, KeyCode::Up);
        assert_eq!(app.line, "--build contracts/flow ");
        assert_eq!(app.cursor, app.line.len());
        alt(&mut app, KeyCode::Up);
        assert_eq!(app.line, "--compare flow --report size ");
        alt(&mut app, KeyCode::Up);
        assert_eq!(app.line, "--compare flow --report size ");

        // Alt+Down goes back, and past the newest run puts back what was typed.
        alt(&mut app, KeyCode::Down);
        assert_eq!(app.line, "--build contracts/flow ");
        alt(&mut app, KeyCode::Down);
        assert_eq!(app.line, "--comp");
        assert_eq!(app.recall, None);
        alt(&mut app, KeyCode::Down);
        assert_eq!(app.line, "--comp");

        // An edit keeps the line, and the next Alt+Up starts from the newest run again.
        alt(&mut app, KeyCode::Up);
        alt(&mut app, KeyCode::Up);
        keys(&mut app, "x");
        assert_eq!(app.line, "--compare flow --report size x");
        assert_eq!(app.recall, None);
        alt(&mut app, KeyCode::Up);
        assert_eq!(app.line, "--build contracts/flow ");
        alt(&mut app, KeyCode::Down);
        assert_eq!(app.line, "--compare flow --report size x");

        // With no history, the keys change nothing.
        let mut app = App::default();
        keys(&mut app, "--b");
        alt(&mut app, KeyCode::Up);
        assert_eq!(app.line, "--b");
    }

    #[test]
    fn ctrl_c_cancels_then_quits() {
        let mut app = App {
            tabs: vec![Tab::new(
                Run::start("sleep", vec!["30".into()], Path::new(".")).unwrap(),
            )],
            ..App::default()
        };
        // During a run, Ctrl+C cancels it and does not quit.
        ctrl(&mut app, 'c');
        assert!(!app.running() && !app.quit);
        ctrl(&mut app, 'c');
        assert!(app.quit);

        // Ctrl+Q quits at once, and `q` is only text.
        let mut app = App::default();
        keys(&mut app, "q");
        assert!(!app.quit);
        assert_eq!(app.line, "q");
        ctrl(&mut app, 'q');
        assert!(app.quit);

        // Both work while a prompt is open.
        let mut app = App::default();
        ctrl(&mut app, 'p');
        ctrl(&mut app, 'c');
        assert!(app.quit);
    }

    /// An app whose run printed `n` lines, with an output pane `h` rows high, and the
    /// focus on the output.
    fn with_output(n: usize, h: usize) -> App {
        let script = format!("seq 1 {n}");
        let mut r = Run::start("sh", vec!["-c".into(), script], Path::new(".")).unwrap();
        super::super::runner::tests::finish(&mut r);
        App {
            tabs: vec![Tab::new(r)],
            view_height: h,
            focus: Focus::Output,
            ..App::default()
        }
    }

    fn key(app: &mut App, code: KeyCode) {
        press(app, code, KeyModifiers::NONE);
    }

    #[test]
    fn scroll() {
        let mut app = with_output(100, 10);
        assert_eq!(app.output_top(), 90);
        key(&mut app, KeyCode::Up);
        assert_eq!((app.output_top(), app.tabs[0].top), (89, Some(89)));
        press(&mut app, KeyCode::Home, KeyModifiers::CONTROL);
        assert_eq!(app.output_top(), 0);
        key(&mut app, KeyCode::Up);
        assert_eq!(app.output_top(), 0);
        key(&mut app, KeyCode::PageDown);
        assert_eq!(app.output_top(), 10);
        key(&mut app, KeyCode::PageUp);
        assert_eq!(app.output_top(), 0);
        press(&mut app, KeyCode::End, KeyModifiers::CONTROL);
        assert_eq!(app.tabs[0].top, None);
        // Down at the end stays at the end, and follows.
        key(&mut app, KeyCode::Down);
        assert_eq!((app.output_top(), app.tabs[0].top), (90, None));
        for _ in 0..3 {
            press(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
        }
        press(&mut app, KeyCode::Left, KeyModifiers::SHIFT);
        assert_eq!(app.tabs[0].left, 16);
        // Plain Home and End move the cursor of the command line, not the output.
        keys(&mut app, "ab");
        key(&mut app, KeyCode::Home);
        assert_eq!((app.cursor, app.tabs[0].top), (0, None));

        // Output shorter than the pane never scrolls.
        let mut app = with_output(3, 10);
        key(&mut app, KeyCode::Up);
        key(&mut app, KeyCode::PageUp);
        assert_eq!(app.output_top(), 0);
    }

    #[test]
    fn details_scroll() {
        let mut app = App::new(vec![pair("flow"), pair("numeric")]);
        keys(&mut app, "--compare ");
        assert_eq!(app.details.0, "pair flow");
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Details);
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Down);
        assert_eq!(app.details_top, 2);
        // It stops at the last line.
        press(&mut app, KeyCode::End, KeyModifiers::CONTROL);
        assert_eq!(app.details_top, app.details.1.len() - 1);
        // New details start at their top.
        keys(&mut app, "num");
        assert_eq!(app.details_top, 0);
    }

    #[test]
    fn save() {
        let d = super::super::catalog::tests::scratch("save");
        let path = d.join("out.txt").to_string_lossy().into_owned();
        let mut app = with_output(3, 10);
        ctrl(&mut app, 's');
        assert_eq!(app.prompt.as_ref().unwrap().text, "antlion.txt");
        // Clear the default name and type the path.
        for _ in 0.."antlion.txt".len() {
            press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        }
        keys(&mut app, &path);
        enter(&mut app);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "1\n2\n3\n");

        // A file that is there needs a `y`; any other key keeps it.
        std::fs::write(&path, "old").unwrap();
        app.prompt = Some(Prompt {
            ask: Ask::Save,
            text: path.clone(),
        });
        enter(&mut app);
        assert_eq!(app.prompt.as_ref().unwrap().ask, Ask::Overwrite);
        keys(&mut app, "n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
        app.prompt = Some(Prompt {
            ask: Ask::Save,
            text: path.clone(),
        });
        enter(&mut app);
        keys(&mut app, "y");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "1\n2\n3\n");
    }

    #[test]
    fn build_reads_the_states_again() {
        use super::super::catalog::State;
        let d = super::super::catalog::tests::scratch("build");
        std::fs::create_dir_all(d.join("contracts/a")).unwrap();
        std::fs::create_dir_all(d.join("contracts/b")).unwrap();
        let mut app = App::new(super::super::catalog::pairs(&d));
        app.root = d.clone();
        // A stand in for antlion: `--build contracts/a` writes both files of `a`.
        app.program = d.join("fake").to_string_lossy().into_owned();
        script(
            Path::new(&app.program),
            "#!/bin/sh\nmkdir -p out/a && touch out/a/sdk.wasm out/a/solang.wasm\n",
        );

        keys(&mut app, "--build contracts/a");
        ctrl(&mut app, 'r');
        assert_eq!(app.tabs[0].run.args, ["--build", "contracts/a"]);
        assert_eq!(app.tabs[0].kind, Kind::Build);
        let stop = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.running() && std::time::Instant::now() < stop {
            app.tick();
        }
        app.tick();
        assert_eq!(app.pairs[0].state, State::Built);
        assert_eq!(app.pairs[1].state, State::None);
        // A build goes into the history too.
        assert_eq!(app.history.len(), 1);
    }

    #[test]
    fn tabs() {
        let mut app = App {
            view_height: 10,
            ..App::default()
        };
        let tab = |app: &mut App, code| press(app, code, KeyModifiers::CONTROL);
        // With no tab, the keys do nothing.
        tab(&mut app, KeyCode::PageUp);
        tab(&mut app, KeyCode::PageDown);
        assert_eq!(app.tab, 0);

        for i in 0..12 {
            let mut r = Run::start(
                "sh",
                vec!["-c".into(), format!("seq 1 {}", 20 + i)],
                Path::new("."),
            )
            .unwrap();
            super::super::runner::tests::finish(&mut r);
            app.add_tab(r);
        }
        assert_eq!(app.tabs.len(), MAX_TABS);
        assert_eq!(app.tab, 9);
        // The oldest tab now is the third run, with 22 lines.
        for _ in 0..12 {
            tab(&mut app, KeyCode::PageUp);
        }
        assert_eq!(app.tab, 0);
        assert_eq!(app.tabs[0].run.lines.len(), 22);

        // Each tab keeps its own scroll.
        app.focus = Focus::Output;
        tab(&mut app, KeyCode::Home);
        tab(&mut app, KeyCode::PageDown);
        assert_eq!(app.tab, 1);
        assert_eq!(app.output_top(), 13);
        tab(&mut app, KeyCode::PageUp);
        assert_eq!(app.output_top(), 0);
        for _ in 0..12 {
            tab(&mut app, KeyCode::PageDown);
        }
        assert_eq!(app.tab, 9);
    }

    #[test]
    fn focus() {
        let mut app = App::new(vec![pair("flow"), pair("numeric")]);
        assert_eq!(app.focus, Focus::Suggestions);
        // Tab goes round the panels, and Shift+Tab goes back.
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Output);
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Details);
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Suggestions);
        press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(app.focus, Focus::Details);
        // Letters go into the command line, whatever has the focus.
        keys(&mut app, "--compare ");
        assert_eq!(app.line, "--compare ");
        // Down moves in the suggestions only when they have the focus.
        key(&mut app, KeyCode::Down);
        assert_eq!(app.pick, 0);
        app.focus = Focus::Suggestions;
        key(&mut app, KeyCode::Down);
        assert_eq!(app.pick, 1);
        assert_eq!(app.details.0, "pair numeric");
    }
}
