//! All the state of the UI. Nothing here draws, so every change can be tested with no
//! terminal.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::catalog::Pair;
use super::command;
use super::complete::{self, Suggestion};
use super::runner::Run;
use super::search::Search;

/// Where the keys go.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Focus {
    /// The command line: a key types its character.
    #[default]
    Line,
    /// The output pane: a key scrolls or searches it.
    Output,
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
    /// A `scripts/check.sh` task.
    Task,
}

impl Tab {
    pub fn new(run: Run) -> Self {
        Tab {
            run,
            top: None,
            left: 0,
            search: None,
            kind: Kind::Run,
        }
    }

    /// What the tab runs, as a shell line.
    pub fn title(&self) -> String {
        match self.kind {
            Kind::Task => format!("scripts/check.sh {}", self.run.args.join(" ")),
            Kind::Run | Kind::Build => command::line(&self.run.args),
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
    /// The command palette: the text filters the actions.
    Palette,
    /// The history of runs, in the palette box: the text filters the runs.
    History,
    /// Whether to do an act that cannot be undone. `y` agrees.
    Confirm(Act),
    /// The presets, in the palette box: the text filters them.
    Presets,
    /// A name to save the command line as a preset under.
    PresetName,
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
    /// Run `scripts/check.sh` with this task.
    Task(&'static str),
    /// Run `scripts/check.sh save`, once the user agreed to write over the golden output.
    SaveGolden,
    /// Ask for a name, and save the command line as a preset.
    SavePreset,
    /// Open the presets.
    Presets,
    /// Put the preset with this index in the presets into the command line.
    LoadPreset(usize),
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
    /// Where the keys go.
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
    /// The folder of the config file; `None` keeps no presets.
    pub config_dir: Option<std::path::PathBuf>,
    /// The presets as they were read when the presets were last opened.
    pub presets: Vec<(String, Vec<String>)>,
    /// Text to put in the clipboard, which the loop sends to the terminal after the draw.
    pub copy: Option<String>,
    /// `q` was pressed while a run was active, and needs a second `q`.
    quit_asked: bool,
    /// The first `g` of `gg` was pressed.
    g: bool,
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
            config_dir: None,
            presets: Vec::new(),
            quit_asked: false,
            g: false,
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
        self.details = complete::details(
            self.suggestions.get(self.pick),
            self.before(),
            &self.root,
            &self.pairs,
        );
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
            if t.kind != Kind::Task
                && was
                && let Some((end, time)) = t.run.end
            {
                let exit = match end {
                    super::runner::End::Exited(c) => c,
                    super::runner::End::Cancelled => None,
                };
                let r = super::store::Record::new(t.run.args.clone(), exit, time.as_secs_f64());
                ended.push(r);
            }
            if let Some(s) = &mut t.search {
                s.update(&t.run.lines);
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

    /// Save the command line as the preset `name` in the config file.
    fn save_preset(&mut self, name: &str) {
        let args = self.args();
        if let Some(e) = command::refusal(&args) {
            self.status = Some(format!("not saved: {e}"));
            return;
        }
        let Some(d) = &self.config_dir else {
            self.status = Some("no config folder: set ANTLION_CONFIG_DIR or HOME".into());
            return;
        };
        self.status = Some(match super::store::save_preset(d, name, &args) {
            Ok(()) => format!("saved the preset {name}"),
            Err(e) => e,
        });
    }

    /// Put `args` into the command line, with the cursor after a space at its end, so the
    /// suggestions show what can come next.
    fn load(&mut self, args: &[String]) {
        self.line = format!("{} ", args.join(" "));
        self.cursor = self.line.len();
        self.focus = Focus::Line;
        self.pick = 0;
        self.refresh();
        self.status = Some("loaded: press Ctrl+R to run it".to_string());
    }

    /// Read the pairs again, so the suggestions show a build made since.
    fn reload(&mut self) {
        self.pairs = super::catalog::pairs(&self.root);
        self.refresh();
    }

    /// Run `scripts/check.sh task` in a new tab. `build` changes `out/`, so it counts as a
    /// build.
    fn task(&mut self, task: &str) {
        if self.running() {
            self.status = Some("a run is active: wait for it, or cancel it".to_string());
            return;
        }
        let script = self.root.join("scripts/check.sh");
        let kind = if task == "build" {
            Kind::Build
        } else {
            Kind::Task
        };
        match Run::start(
            &script.to_string_lossy(),
            vec![task.to_string()],
            &self.root,
        ) {
            Ok(r) => self.add_kind(r, kind),
            Err(e) => self.status = Some(format!("cannot start {}: {e}", script.display())),
        }
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
        if matches!(p.ask, Ask::Palette | Ask::History | Ask::Presets) {
            self.palette_key(k);
            return;
        }
        if let Ask::Confirm(act) = p.ask {
            self.prompt = None;
            if k.code == KeyCode::Char('y') {
                self.act(act);
            } else {
                self.status = Some("not done".to_string());
            }
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
                    Ask::PresetName if p.text.is_empty() => {}
                    Ask::PresetName => self.save_preset(&p.text),
                    Ask::Save if p.text.is_empty() => {}
                    Ask::Save if std::path::Path::new(&p.text).exists() => {
                        self.prompt = Some(Prompt {
                            ask: Ask::Overwrite,
                            text: p.text,
                        });
                    }
                    Ask::Save => self.save(&p.text),
                    Ask::Palette | Ask::History | Ask::Presets => {
                        unreachable!("the palette has its own keys")
                    }
                    Ask::Overwrite | Ask::Confirm(_) => unreachable!("handled above"),
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
            e("help: every key, and the inputs", "?", Act::Help),
            e("run the command", "Ctrl+R", Act::Run),
            e("copy the command", "y", Act::Copy),
            e("cancel the run", "c", Act::Cancel),
            e("save the output to a file", "s", Act::Save),
            e("history of runs", "H", Act::History),
            e(
                "doctor: check the tools, out/ and the builds",
                "",
                Act::Doctor,
            ),
            e("check.sh build: build every pair", "", Act::Task("build")),
            e(
                "check.sh diff: compare the output with the golden output",
                "",
                Act::Task("diff"),
            ),
            e(
                "check.sh save: write over the golden output",
                "",
                Act::Task("save"),
            ),
            e(
                "check.sh wasm: compare the wasm files with the golden hashes",
                "",
                Act::Task("wasm"),
            ),
            e(
                "check.sh time: time a run of every report on every pair",
                "",
                Act::Task("time"),
            ),
            e("save the command as a preset", "", Act::SavePreset),
            e("load a preset", "", Act::Presets),
            e("quit", "q", Act::Quit),
        ]
    }

    /// The entries that match the text of the palette, the best match first.
    pub fn palette_hits(&self) -> Vec<Entry> {
        let text = self.prompt.as_ref().map_or("", |p| p.text.as_str());
        let entries = match self.prompt.as_ref().map(|p| p.ask) {
            Some(Ask::History) => self.history_entries(),
            Some(Ask::Presets) => self
                .presets
                .iter()
                .enumerate()
                .map(|(i, (name, args))| Entry {
                    label: format!("{name}   {}", command::line(args)),
                    key: "",
                    act: Act::LoadPreset(i),
                })
                .collect(),
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
            Act::Quit if self.running() => {
                self.status = Some("a run is active: cancel it first, then quit".to_string())
            }
            Act::Quit => self.quit = true,
            Act::History => self.ask(Ask::History),
            Act::Task("save") => {
                self.prompt = Some(Prompt {
                    ask: Ask::Confirm(Act::SaveGolden),
                    text: "check.sh save writes over the golden output".to_string(),
                })
            }
            Act::SaveGolden => self.task("save"),
            Act::Task(t) => self.task(t),
            Act::Doctor => {
                // Read the pairs first, so a build made outside the UI is seen.
                self.reload();
                let lines = super::catalog::doctor(&self.root, &self.pairs);
                self.info = Some(("doctor".to_string(), lines, 0));
            }
            Act::SavePreset => self.ask(Ask::PresetName),
            Act::Presets => {
                let Some(d) = &self.config_dir else {
                    self.status = Some("no config folder: set ANTLION_CONFIG_DIR or HOME".into());
                    return;
                };
                match super::store::presets(d) {
                    Err(e) => self.status = Some(e),
                    Ok(p) if p.is_empty() => {
                        self.status = Some("no preset yet: save one from the palette".into())
                    }
                    Ok(p) => {
                        self.presets = p;
                        self.ask(Ask::Presets);
                    }
                }
            }
            Act::LoadPreset(i) => {
                if let Some((_, args)) = self.presets.get(i) {
                    let args = args.clone();
                    self.load(&args);
                }
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

    /// Move to the next match of the search, or the one before.
    fn next_match(&mut self, step: isize) {
        let h = self.view_height;
        let Some(t) = self.current_mut() else { return };
        if let Some(l) = t.search.as_mut().and_then(|s| s.step(step)) {
            t.show_line(l, h);
        }
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
            Ok(r) => self.add_kind(r, kind),
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
            KeyCode::Char('/') => self.ask(Ask::Search),
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

    /// The keys of the command line.
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
            KeyCode::Up => self.pick = self.pick.saturating_sub(1),
            KeyCode::Down => self.pick += 1,
            KeyCode::Tab | KeyCode::Enter => self.accept(),
            KeyCode::Esc => self.focus = Focus::Output,
            _ => {}
        }
        // A new word gets new suggestions, so the cursor goes back to the best one.
        if (&self.line, self.cursor) != (&was.0, was.1) {
            self.pick = 0;
        }
        self.refresh();
    }

    /// The keys while the output pane has the focus.
    fn output_focus_key(&mut self, k: KeyEvent) {
        let g = std::mem::take(&mut self.g);
        let quit_asked = std::mem::take(&mut self.quit_asked);
        if k.code == KeyCode::Char('g') && !g {
            self.g = true;
            return;
        }
        if self.output_key(k, g) {
            return;
        }
        match k.code {
            KeyCode::Char('q') if quit_asked || !self.running() => self.quit = true,
            KeyCode::Char('q') => {
                self.quit_asked = true;
                self.status =
                    Some("a run is active: press q again to stop it and quit".to_string());
            }
            KeyCode::Char('c') => self.act(Act::Cancel),
            KeyCode::Char('?') => self.help = Some(0),
            KeyCode::Char('y') => self.act(Act::Copy),
            KeyCode::Char('r') => self.start(),
            KeyCode::Char('H') => self.act(Act::History),
            KeyCode::Char('i') => self.focus = Focus::Line,
            _ => {}
        }
    }

    pub fn key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        // Ctrl+C works everywhere, even while a box or a prompt is open.
        if ctrl && k.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if let Some((_, _, top)) = &mut self.info {
            match k.code {
                KeyCode::Char('j') | KeyCode::Down => *top += 1,
                KeyCode::Char('k') | KeyCode::Up => *top = top.saturating_sub(1),
                KeyCode::Esc | KeyCode::Char('q') => self.info = None,
                _ => {}
            }
            return;
        }
        if let Some(top) = &mut self.help {
            match k.code {
                KeyCode::Char('j') | KeyCode::Down => *top += 1,
                KeyCode::Char('k') | KeyCode::Up => *top = top.saturating_sub(1),
                KeyCode::Esc | KeyCode::Char('?' | 'q') => self.help = None,
                _ => {}
            }
            return;
        }
        if self.prompt.is_some() {
            self.prompt_key(k);
            return;
        }
        match k.code {
            KeyCode::Char('p') if ctrl => self.ask(Ask::Palette),
            KeyCode::Char('r') if ctrl => self.start(),
            _ => match self.focus {
                Focus::Line => self.line_key(k),
                Focus::Output => self.output_focus_key(k),
            },
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
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.line, "--compare ");
        assert_eq!(texts(&app), ["flow", "numeric"]);
        // Down picks the next suggestion, and Enter puts it in, as Tab does.
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        enter(&mut app);
        assert_eq!(app.line, "--compare numeric ");
        assert_eq!(texts(&app), ["flow", "--report"]);
        keys(&mut app, "--r");
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        // The word typed filters the reports, the best match first.
        keys(&mut app, "co");
        assert_eq!(texts(&app)[0], "cost");
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
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
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
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
        let mut app = App {
            focus: Focus::Output,
            ..App::default()
        };
        keys(&mut app, "?");
        assert_eq!(app.help, Some(0));
        // While help is open, keys scroll it and do nothing else.
        keys(&mut app, "jjkir");
        assert_eq!(app.help, Some(1));
        assert_eq!(app.focus, Focus::Output);
        keys(&mut app, "q");
        assert_eq!(app.help, None);
        assert!(!app.quit);
    }

    #[test]
    fn copy() {
        let mut app = App::default();
        keys(&mut app, "--compare flow   --report size");
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        keys(&mut app, "y");
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

        // Loading a run puts it in the command line, from the output pane too.
        app.history.push(super::super::store::Record::new(
            ["--compare", "numeric", "--report", "cost", "--json"]
                .map(String::from)
                .to_vec(),
            Some(0),
            1.0,
        ));
        app.focus = Focus::Output;
        keys(&mut app, "H");
        assert!(
            app.palette_hits()[0]
                .label
                .starts_with("antlion --compare numeric --report cost --json   done")
        );
        enter(&mut app);
        assert_eq!(app.line, "--compare numeric --report cost --json ");
        assert_eq!(app.cursor, app.line.len());
        assert_eq!(app.focus, Focus::Line);
        assert_eq!(texts(&app), ["--detail", "--dump", "--jobs"]);
    }

    #[test]
    fn presets() {
        let d = super::super::catalog::tests::scratch("app-presets");
        let mut app = App {
            config_dir: Some(d.clone()),
            ..App::default()
        };
        let palette = |app: &mut App, text: &str| {
            ctrl(app, 'p');
            keys(app, text);
            enter(app);
        };
        palette(&mut app, "load a preset");
        assert!(
            app.status
                .as_deref()
                .is_some_and(|s| s.starts_with("no preset yet"))
        );

        // A command the CLI refuses is not saved.
        keys(&mut app, "--compare numeric");
        palette(&mut app, "save the command");
        keys(&mut app, "bad");
        enter(&mut app);
        assert!(
            app.status
                .as_deref()
                .is_some_and(|s| s.starts_with("not saved"))
        );

        keys(&mut app, " --report cost");
        palette(&mut app, "save the command");
        keys(&mut app, "cost of numeric");
        enter(&mut app);
        assert_eq!(
            app.status.as_deref(),
            Some("saved the preset cost of numeric")
        );

        ctrl(&mut app, 'u');
        palette(&mut app, "load a preset");
        assert!(app.palette_hits()[0].label.starts_with("cost of numeric"));
        enter(&mut app);
        assert_eq!(app.line, "--compare numeric --report cost ");
    }

    #[test]
    fn tasks() {
        let d = super::super::catalog::tests::scratch("tasks");
        std::fs::create_dir_all(d.join("scripts")).unwrap();
        let script = d.join("scripts/check.sh");
        std::fs::write(&script, "#!/bin/sh\necho task $1\n").unwrap();
        std::process::Command::new("chmod")
            .arg("+x")
            .arg(&script)
            .status()
            .unwrap();
        let mut app = App {
            root: d.clone(),
            ..App::default()
        };
        let finish = |app: &mut App| {
            let stop = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while app.running() && std::time::Instant::now() < stop {
                app.tick();
            }
            app.tick();
        };
        let palette = |app: &mut App, text: &str| {
            ctrl(app, 'p');
            keys(app, text);
            enter(app);
        };

        palette(&mut app, "check.sh diff");
        finish(&mut app);
        assert_eq!(app.tabs[0].run.lines[0].text, "task diff");
        assert_eq!(app.tabs[0].title(), "scripts/check.sh diff");
        assert_eq!(app.tabs[0].kind, Kind::Task);
        // A task does not go into the history.
        assert!(app.history.is_empty());

        // `save` asks first; any key but `y` does nothing.
        palette(&mut app, "check.sh save");
        keys(&mut app, "n");
        assert_eq!(app.tabs.len(), 1);
        palette(&mut app, "check.sh save");
        keys(&mut app, "y");
        finish(&mut app);
        assert_eq!(app.tabs[1].run.lines[0].text, "task save");

        palette(&mut app, "check.sh build");
        assert_eq!(app.tabs[2].kind, Kind::Build);
    }

    #[test]
    fn quit_during_a_run() {
        let sleeping = || App {
            focus: Focus::Output,
            tabs: vec![Tab::new(
                Run::start("sleep", vec!["30".into()], Path::new(".")).unwrap(),
            )],
            ..App::default()
        };
        let mut app = sleeping();
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
        let mut app = sleeping();
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
        let mut r = Run::start("sh", vec!["-c".into(), script], Path::new(".")).unwrap();
        super::super::runner::tests::finish(&mut r);
        App {
            tabs: vec![Tab::new(r)],
            view_height: h,
            focus: Focus::Output,
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
        ctrl(&mut app, 'd');
        assert_eq!(app.output_top(), 5);
        ctrl(&mut app, 'u');
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
        enter(&mut app);
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
        std::fs::write(
            &app.program,
            "#!/bin/sh\nmkdir -p out/a && touch out/a/sdk.wasm out/a/solang.wasm\n",
        )
        .unwrap();
        std::process::Command::new("chmod")
            .arg("+x")
            .arg(&app.program)
            .status()
            .unwrap();

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
            focus: Focus::Output,
            view_height: 10,
            ..App::default()
        };
        // `[` and `]` with no tab do nothing.
        keys(&mut app, "[]");
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
        keys(&mut app, "[[[[[[[[[[[[");
        assert_eq!(app.tab, 0);
        assert_eq!(app.tabs[0].run.lines.len(), 22);

        // Each tab keeps its own scroll and search.
        keys(&mut app, "gg/7");
        enter(&mut app);
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
        assert_eq!(app.focus, Focus::Line);
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Output);
        // In the output pane, `i` goes back to the command line, and types nothing.
        keys(&mut app, "i");
        assert_eq!(app.focus, Focus::Line);
        assert_eq!(app.line, "");
    }

    #[test]
    fn q_and_ctrl_c_quit() {
        let mut app = App {
            focus: Focus::Output,
            ..App::default()
        };
        press(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!app.quit);
        press(&mut app, KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(app.quit);

        // Ctrl+C quits from the command line, and from an open prompt.
        let mut app = App::default();
        ctrl(&mut app, 'c');
        assert!(app.quit);
        let mut app = App::default();
        ctrl(&mut app, 'p');
        ctrl(&mut app, 'c');
        assert!(app.quit);
    }
}
