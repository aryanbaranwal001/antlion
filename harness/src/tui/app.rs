//! All the state of the UI. Nothing here draws, so every change can be tested with no
//! terminal.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

#[derive(Default)]
pub struct App {
    /// The pane that gets the keys.
    pub focus: Pane,
    /// The loop stops at the end of this pass.
    pub quit: bool,
    /// One line for the status bar, such as a warning at start.
    pub status: Option<String>,
}

impl App {
    pub fn key(&mut self, k: KeyEvent) {
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
