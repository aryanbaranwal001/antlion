//! Drawing. Nothing here changes the state.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    text::Span,
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};

use super::app::{App, Ask, OPTIONS, Pane, Pick};
use super::catalog::State;
use super::search;
use crate::compare::Report;
#[cfg(test)]
use std::path::Path;

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

    let [top, output, command, status, keys] = rows(area);
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
    output_pane(f, app, output);

    if let Some(p) = &app.prompt {
        let label = match p.ask {
            Ask::Search | Ask::Filter(_) => "/",
            Ask::Palette => ":",
            Ask::Save => "save to: ",
            Ask::Overwrite => "file is there, y to write over it: ",
        };
        f.render_widget(Paragraph::new(format!(" {label}{}", p.text)), command);
        let x = command.x + 1 + (label.chars().count() + p.text.chars().count()) as u16;
        f.set_cursor_position((x.min(command.right().saturating_sub(1)), command.y));
    } else {
        let args = app.selection().args();
        let mut spans = vec![Span::raw(format!(" $ {}", super::command::line(&args)))];
        if let Some(e) = super::command::refusal(&args) {
            spans.push(Span::styled(format!("   {e}"), Style::new().fg(Color::Red)));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), command);
    }
    if let Some(s) = &app.status {
        f.render_widget(
            Paragraph::new(format!(" {s}")).style(Style::new().fg(Color::Yellow)),
            status,
        );
    } else if let Some(t) = app.current() {
        let tabs = format!("  tab {} of {}", app.tab + 1, app.tabs.len());
        f.render_widget(Paragraph::new(run_state(&t.run) + &tabs), status);
    }
    f.render_widget(
        Paragraph::new(Line::from(hints(app.focus)))
            .style(Style::new().add_modifier(Modifier::DIM)),
        keys,
    );
    if app.prompt.as_ref().is_some_and(|p| p.ask == Ask::Palette) {
        palette(f, app);
    }
    if let Some(top) = app.help {
        help(f, top);
    }
}

/// Every key of the screen, by where it works.
const KEYS: &str = "\
EVERYWHERE

    Tab  Shift+Tab   next pane, pane before
    1 2 3 4          pairs, reports, options, output
    r                run the command
    b                build the selected pairs
    c                cancel the run
    :                command palette
    ?                this help
    q                quit; during a run, press it twice
    Ctrl+C           quit at once, and stop the run

PAIRS AND REPORTS

    j k              down, up
    gg G             first, last
    Space            select or clear the item
    a A              select all, clear all (of the items shown)
    /                filter; Esc clears it, the selection stays

OPTIONS

    j k              down, up
    Space  Enter     change the value
    + -              more or fewer jobs; below 1 is auto

OUTPUT

    j k              one line down, up
    Ctrl+D Ctrl+U    half a page down, up
    PgDn PgUp        a page down, up
    gg G             top; end, which follows new output
    h l  0           left, right, back to the start of the line
    /                search; case counts only with a capital
    n N              next match, match before
    [ ]              tab before, next tab
    s                save the output to a file

PALETTE AND PROMPTS

    type             filter the palette, or type the text
    Up Down          move in the palette (also Tab, Ctrl+N, Ctrl+P)
    Enter            do it
    Esc              close
";

/// The help overlay: every key, then the inputs part of `help.md`.
fn help(f: &mut Frame, top: usize) {
    let help_md = include_str!("../../../help.md");
    let inputs = help_md
        .split("\nINPUTS\n")
        .nth(1)
        .and_then(|s| s.split("\nBUNDLED CONTRACTS\n").next())
        .unwrap_or("");
    let text = format!("{KEYS}\nINPUTS\n{inputs}");
    let lines: Vec<Line> = text.lines().map(Line::raw).collect();

    let area = f.area();
    let r = Rect::new(
        2,
        1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    let top = top.min(lines.len().saturating_sub(1));
    let block = Block::bordered()
        .title(" Help ")
        .title_bottom(Line::from(" j/k scroll  Esc close ").right_aligned())
        .border_style(Style::new().fg(Color::Cyan));
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((u16::try_from(top).unwrap_or(u16::MAX), 0)),
        r,
    );
}

/// The command palette: a box in the middle of the screen, with the entries that match.
fn palette(f: &mut Frame, app: &App) {
    let area = f.area();
    let w = (area.width * 3 / 5).max(40).min(area.width);
    let h = (area.height * 3 / 5).max(8).min(area.height);
    let r = Rect::new((area.width - w) / 2, (area.height - h) / 4, w, h);
    let hits = app.palette_hits();
    let width = r.width.saturating_sub(4) as usize;
    let rows: Vec<ListItem> = hits
        .iter()
        .map(|e| {
            let pad = width.saturating_sub(e.label.chars().count() + e.key.len());
            ListItem::new(format!("{}{}{}", e.label, " ".repeat(pad), e.key))
        })
        .collect();
    let block = Block::bordered()
        .title(format!(
            " : {} ",
            app.prompt.as_ref().map_or("", |p| p.text.as_str())
        ))
        .title_bottom(Line::from(" Enter do  Up/Down move  Esc close ").right_aligned())
        .border_style(Style::new().fg(Color::Cyan));
    let widget = List::new(rows)
        .block(block)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state =
        ListState::default().with_selected((!hits.is_empty()).then_some(app.palette_cur));
    f.render_widget(Clear, r);
    f.render_stateful_widget(widget, r, &mut state);
}

/// The lines of the run that fit, from the end: the pane follows the output as it comes.
fn output_pane(f: &mut Frame, app: &App, area: Rect) {
    let Some(t) = app.current() else {
        pane(f, app, Pane::Output, area);
        return;
    };
    let r = &t.run;
    let block =
        frame(app, Pane::Output).title(format!(" Output: {} ", super::command::line(&r.args)));
    let height = block.inner(area).height as usize;
    let last = r.lines.len().saturating_sub(height);
    let top = t.top.map_or(last, |x| x.min(last));
    let end = (top + height).min(r.lines.len());
    let mut shown = if end > top {
        format!(" lines {}-{} of {} ", top + 1, end, r.lines.len())
    } else {
        " no output ".to_string()
    };
    if let Some(s) = &t.search {
        let at = if s.hits.is_empty() { 0 } else { s.at + 1 };
        shown = format!(" /{}  {at}/{} {shown}", s.query, s.hits.len());
    }
    let block = block.title(Line::from(shown).right_aligned());
    let lines: Vec<Line> = r.lines[top..end]
        .iter()
        .map(|l| {
            let mut style = if l.err {
                Style::new().fg(Color::Red)
            } else {
                Style::new()
            };
            if t.search
                .as_ref()
                .is_some_and(|s| search::holds(&l.text, &s.query))
            {
                style = style.bg(Color::Yellow).fg(Color::Black);
            }
            Line::styled(l.text.as_str(), style)
        })
        .collect();
    let left = u16::try_from(t.left).unwrap_or(u16::MAX);
    f.render_widget(Paragraph::new(lines).block(block).scroll((0, left)), area);
}

/// The rows inside the output pane for a screen of `size`, as `draw` lays it out.
pub fn output_height(size: ratatui::layout::Size) -> usize {
    let area = Rect::new(0, 0, size.width, size.height);
    let [_, output, ..] = rows(area);
    output.height.saturating_sub(2) as usize
}

/// The five rows of the screen: the three lists, the output, the command, the status and
/// the key hints.
fn rows(area: Rect) -> [Rect; 5] {
    Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area)
}

/// The state of a run on one line: running or how it ended, its time and its lines.
fn run_state(r: &super::runner::Run) -> String {
    use super::runner::End;
    let (state, time) = match r.end {
        None => ("running".to_string(), r.started.elapsed()),
        Some((End::Exited(Some(0)), t)) => ("done".to_string(), t),
        Some((End::Exited(Some(c)), t)) => (format!("failed, exit {c}"), t),
        Some((End::Exited(None), t)) => ("stopped by a signal".to_string(), t),
        Some((End::Cancelled, t)) => ("cancelled".to_string(), t),
    };
    format!(
        " {state}  {:.1} s  {} lines",
        time.as_secs_f64(),
        r.lines.len()
    )
}

/// The keys of the pane that has the focus, then the keys that work everywhere.
fn hints(p: Pane) -> String {
    let own = match p {
        Pane::Pairs | Pane::Reports => "j/k  Space select  a/A all/none  / filter  ",
        Pane::Options => "j/k move  Space change  +/- jobs  ",
        Pane::Output => "j/k ^D/^U gg/G h/l scroll  / n N search  [ ] tab  s save  ",
    };
    format!(" {own}Tab  r run  b build  ? help  q quit")
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
    let mut rows: Vec<Option<Vec<Span>>> = rows.map(Some).collect();
    let items: Vec<ListItem> = pick
        .rows()
        .into_iter()
        .filter_map(|i| {
            let mut spans = rows.get_mut(i)?.take()?;
            spans.insert(0, Span::raw(if pick.on[i] { "[x] " } else { "[ ] " }));
            Some(ListItem::new(Line::from(spans)))
        })
        .collect();
    let mut title = format!(" {} ({}/{}) ", p.title(), pick.count(), pick.on.len());
    if !pick.filter.is_empty() {
        title.push_str(&format!("/{} ", pick.filter));
    }
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
        assert!(s[23].contains("Tab  r run"));
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
    fn output_follows_the_end() {
        let mut app = App::default();
        let script = "for i in $(seq 1 100); do echo line$i; done; echo oops >&2";
        let mut r = super::super::runner::Run::start(
            "sh",
            vec!["-c".into(), script.into()],
            Path::new("."),
        )
        .unwrap();
        super::super::runner::tests::finish(&mut r);
        app.tabs.push(super::super::app::Tab::new(r));
        let s = screen(&app, 80, 24);
        assert!(s[10].contains(" Output: antlion -c "));
        // The end is on screen, the start is not.
        assert!(s.iter().any(|r| r.contains("line100")));
        assert!(s.iter().any(|r| r.contains("oops")));
        assert!(s.iter().all(|r| !r.contains("line1 ")));
        assert!(s[22].contains("done") && s[22].contains("101 lines  tab 1 of 1"));
    }

    #[test]
    fn help_overlay() {
        let mut app = App::default();
        app.help = Some(0);
        let s = screen(&app, 100, 60);
        assert!(s[1].contains(" Help "));
        assert!(s[2].contains("EVERYWHERE"));
        assert!(s.iter().any(|r| r.contains("filter; Esc clears it")));
        app.help = Some(48);
        let s = screen(&app, 100, 60);
        assert!(
            s.iter()
                .any(|r| r.contains("small       values that ride inline"))
        );
    }

    #[test]
    fn too_small() {
        let s = screen(&App::default(), 50, 10);
        assert!(s[0].starts_with("terminal too small: 50x10"));
    }
}
