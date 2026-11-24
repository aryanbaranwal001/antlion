//! Drawing. Nothing here changes the state.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    text::Span,
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use super::app::{App, OPTIONS, Pane, Pick};
use super::catalog::State;
use crate::compare::Report;

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

    let w = app
        .pairs
        .iter()
        .map(|p| p.name.chars().count())
        .max()
        .unwrap_or(0);
    let rows = app.pairs.iter().map(|p| {
        let color = match p.state {
            State::Built => Color::Green,
            State::Stale => Color::Yellow,
            State::None => Color::Red,
        };
        vec![
            Span::raw(format!("{:<w$}", p.name)),
            Span::styled(format!(" {}", p.state.label()), Style::new().fg(color)),
        ]
    });
    list(f, app, Pane::Pairs, pairs, &app.pair_pick, rows, None);

    let rows = Report::ALL.iter().map(|r| vec![Span::raw(r.name())]);
    let about = Line::from(format!(" {} ", app.report_at_cursor().about()));
    list(
        f,
        app,
        Pane::Reports,
        reports,
        &app.report_pick,
        rows,
        Some(about),
    );
    let cursor = if app.focus == Pane::Options {
        Style::new().add_modifier(Modifier::REVERSED)
    } else {
        Style::new()
    };
    let rows: Vec<ListItem> = OPTIONS
        .iter()
        .zip(app.option_values())
        .map(|(name, value)| ListItem::new(format!(" {name:<8} {value}")))
        .collect();
    let widget = List::new(rows)
        .block(frame(app, Pane::Options).title(" Options "))
        .highlight_style(cursor);
    let mut state = ListState::default().with_selected(Some(app.option));
    f.render_stateful_widget(widget, options, &mut state);
    pane(f, app, Pane::Output, output);

    let args = app.selection().args();
    let mut spans = vec![Span::raw(format!(" $ {}", super::command::line(&args)))];
    if let Some(e) = super::command::refusal(&args) {
        spans.push(Span::styled(format!("   {e}"), Style::new().fg(Color::Red)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), command);
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
        Pane::Pairs | Pane::Reports => "j/k move  Space select  a all  A none  ",
        Pane::Options => "j/k move  Space change  +/- jobs  ",
        Pane::Output => "",
    };
    format!(" {own}Tab pane  1-4 jump  q quit")
}

/// A pane that is a list: each row with its box, the count in the title, the cursor in
/// reverse when the pane has the focus, and `bottom` on the lower border.
fn list<'a>(
    f: &mut Frame,
    app: &App,
    p: Pane,
    area: Rect,
    pick: &Pick,
    rows: impl Iterator<Item = Vec<Span<'a>>>,
    bottom: Option<Line<'a>>,
) {
    let items: Vec<ListItem> = rows
        .zip(&pick.on)
        .map(|(mut spans, on)| {
            spans.insert(0, Span::raw(if *on { "[x] " } else { "[ ] " }));
            ListItem::new(Line::from(spans))
        })
        .collect();
    let title = format!(" {} ({}/{}) ", p.title(), pick.count(), pick.on.len());
    let cursor = if app.focus == p {
        Style::new().add_modifier(Modifier::REVERSED)
    } else {
        Style::new()
    };
    let mut block = frame(app, p).title(title);
    if let Some(b) = bottom {
        block = block.title_bottom(b);
    }
    let widget = List::new(items).block(block).highlight_style(cursor);
    let mut state = ListState::default().with_selected(Some(pick.cur));
    f.render_stateful_widget(widget, area, &mut state);
}

/// One pane's border, bright when the pane has the focus.
fn frame(app: &App, p: Pane) -> Block<'static> {
    let style = if app.focus == p {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().add_modifier(Modifier::DIM)
    };
    Block::bordered().border_style(style)
}

/// One pane's frame and title. The pane that has the focus has a bright border.
fn pane(f: &mut Frame, app: &App, p: Pane, area: Rect) {
    f.render_widget(frame(app, p).title(format!(" {} ", p.title())), area);
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
        for title in ["Pairs (0/0)", "Reports (0/17)", "Options", "Output"] {
            assert_eq!(
                s.iter()
                    .filter(|r| r.contains(&format!(" {title} ")))
                    .count(),
                1,
                "{title} once"
            );
        }
        assert!(s[21].starts_with(" $ antlion --compare --report   no contract given"));
        assert!(s[23].contains("Tab pane"));
        assert!(s[23].contains("q quit"));
    }

    #[test]
    fn pairs_pane() {
        use super::super::catalog::Pair;
        let p = |name: &str, state| Pair {
            name: name.to_string(),
            state,
            source: true,
        };
        let mut app = App::new(vec![p("flow", State::Built), p("ttl", State::Stale)]);
        app.pair_pick.on[1] = true;
        let s = screen(&app, 80, 24);
        assert!(s[0].contains(" Pairs (1/2) "));
        assert!(s[1].contains("[ ] flow built"));
        assert!(s[2].contains("[x] ttl  stale"));
    }

    #[test]
    fn reports_pane() {
        let mut app = App::default();
        app.report_pick.on[0] = true;
        app.report_pick.cur = 6;
        let s = screen(&app, 100, 30);
        assert!(s[0].contains(" Reports (1/17) "));
        assert!(s[1].contains("[x] size"));
        assert!(s[2].contains("[ ] sections"));
        // The description of `cost`, the report under the cursor, on the lower border.
        assert!(s.iter().any(|r| r.contains(" metered cost of calling")));
    }

    #[test]
    fn options_pane() {
        let mut app = App::default();
        app.json = true;
        app.jobs = Some(3);
        let s = screen(&app, 100, 30);
        assert!(s[1].contains(" output   json"));
        assert!(s[2].contains(" mode     totals"));
        assert!(s[3].contains(" jobs     3"));
    }

    #[test]
    fn command_line() {
        use super::super::catalog::Pair;
        let p = |name: &str| Pair {
            name: name.to_string(),
            state: State::Built,
            source: true,
        };
        let mut app = App::new(vec![p("flow"), p("numeric"), p("ttl")]);
        app.pair_pick.on[0] = true;
        app.pair_pick.on[2] = true;
        app.report_pick.on[6] = true;
        app.json = true;
        let s = screen(&app, 100, 30);
        assert!(
            s.iter()
                .any(|r| r.starts_with(" $ antlion --compare flow ttl --report cost --json "))
        );
    }

    #[test]
    fn too_small() {
        let s = screen(&App::default(), 50, 10);
        assert!(s[0].starts_with("terminal too small: 50x10"));
    }
}
