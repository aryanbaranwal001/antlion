//! All the state of the UI. Nothing here draws, so every change can be tested with no
//! terminal.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::catalog::Pair;

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

#[derive(Default)]
pub struct App {
    /// The pane that gets the keys.
    pub focus: Pane,
    /// The loop stops at the end of this pass.
    pub quit: bool,
    /// One line for the status bar, such as a warning at start.
    pub status: Option<String>,
    pub pairs: Vec<Pair>,
    pub pair_pick: Pick,
    /// The first `g` of `gg` was pressed.
    g: bool,
}

impl App {
    pub fn new(pairs: Vec<Pair>) -> Self {
        App {
            pair_pick: Pick::new(pairs.len()),
            pairs,
            ..App::default()
        }
    }

    pub fn key(&mut self, k: KeyEvent) {
        let g = std::mem::take(&mut self.g);
        let list = match self.focus {
            Pane::Pairs => Some(&mut self.pair_pick),
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

        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
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
