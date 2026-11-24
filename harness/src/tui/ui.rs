//! Drawing. Nothing here changes the state.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Paragraph},
};

use super::app::{App, Pane};

/// The smallest screen the layout fits in.
const MIN_W: u16 = 60;
const MIN_H: u16 = 16;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    if area.width < MIN_W || area.height < MIN_H {
        let msg = format!(
            "terminal too small: {}x{}, needs {MIN_W}x{MIN_H}",
            area.width, area.height
        );
        f.render_widget(Paragraph::new(msg), area);
        return;
    }

    let [top, output, command, status, keys] = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    let [pairs, reports, options] = Layout::horizontal([
        Constraint::Percentage(35),
        Constraint::Percentage(35),
        Constraint::Percentage(30),
    ])
    .areas(top);

    pane(f, app, Pane::Pairs, pairs);
    pane(f, app, Pane::Reports, reports);
    pane(f, app, Pane::Options, options);
    pane(f, app, Pane::Output, output);

    f.render_widget(Paragraph::new(" $ antlion"), command);
    if let Some(s) = &app.status {
        f.render_widget(
            Paragraph::new(format!(" {s}")).style(Style::new().fg(Color::Yellow)),
            status,
        );
    }
    f.render_widget(
        Paragraph::new(Line::from(hints(app.focus)))
            .style(Style::new().add_modifier(Modifier::DIM)),
        keys,
    );
}

/// The keys of the pane that has the focus, then the keys that work everywhere.
fn hints(p: Pane) -> String {
    let own = match p {
        Pane::Pairs | Pane::Reports | Pane::Options | Pane::Output => "",
    };
    format!(" {own}Tab pane  1-4 jump  q quit")
}

/// One pane's frame and title. The pane that has the focus has a bright border.
fn pane(f: &mut Frame, app: &App, p: Pane, area: Rect) {
    let style = if app.focus == p {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().add_modifier(Modifier::DIM)
    };
    let block = Block::bordered()
        .title(format!(" {} ", p.title()))
        .border_style(style);
    f.render_widget(block, area);
}

#[cfg(test)]
pub(super) mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    /// The screen drawn at `w` by `h`, one string for each row.
    pub fn screen(app: &App, w: u16, h: u16) -> Vec<String> {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| draw(f, app)).unwrap();
        let buf = t.backend().buffer();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect()
    }

    #[test]
    fn layout_fits_80_by_24() {
        let s = screen(&App::default(), 80, 24);
        for title in ["Pairs", "Reports", "Options", "Output"] {
            assert_eq!(
                s.iter()
                    .filter(|r| r.contains(&format!(" {title} ")))
                    .count(),
                1,
                "{title} once"
            );
        }
        assert!(s[21].starts_with(" $ antlion"));
        assert!(s[23].contains("Tab pane"));
        assert!(s[23].contains("q quit"));
    }

    #[test]
    fn too_small() {
        let s = screen(&App::default(), 50, 10);
        assert!(s[0].starts_with("terminal too small: 50x10"));
    }
}
