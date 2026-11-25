//! Drawing. Nothing here changes the state.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    text::Span,
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use super::app::{App, Ask, Focus};
use super::complete;
use super::search;
#[cfg(test)]
use std::path::Path;

/// The smallest screen the layout fits in.
const MIN_W: u16 = 60;
const MIN_H: u16 = 16;

/// The word the command line starts with, which the user cannot change.
const PROGRAM: &str = "antlion ";

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

    let [command, output, suggestions, details, status, keys] = areas(area);
    command_line(f, app, command);
    output_pane(f, app, output);
    suggestions_pane(f, app, suggestions);
    let (title, lines) = &app.details;
    let lines: Vec<Line> = lines.iter().map(|l| Line::raw(l.as_str())).collect();
    f.render_widget(
        Paragraph::new(lines)
            .block(frame(app.focus == Focus::Details).title(format!(" {title} ")))
            .wrap(Wrap { trim: false })
            .scroll((u16::try_from(app.details_top).unwrap_or(u16::MAX), 0)),
        details,
    );

    if let Some(p) = &app.prompt {
        let label = match p.ask {
            Ask::Search => "/",
            Ask::Palette => ":",
            Ask::History => "history: ",
            Ask::Presets => "preset: ",
            Ask::PresetName => "save the command as the preset: ",
            Ask::Save => "save to: ",
            Ask::Overwrite => "file is there, y to write over it: ",
            Ask::Confirm(_) => "y to go on: ",
        };
        f.render_widget(Paragraph::new(format!(" {label}{}", p.text)), status);
        let x = status.x + 1 + (label.chars().count() + p.text.chars().count()) as u16;
        f.set_cursor_position((x.min(status.right().saturating_sub(1)), status.y));
    } else if let Some(s) = &app.status {
        f.render_widget(
            Paragraph::new(format!(" {s}")).style(Style::new().fg(Color::Yellow)),
            status,
        );
    } else if let Some(t) = app.current() {
        let tabs = format!("  tab {} of {}", app.tab + 1, app.tabs.len());
        f.render_widget(Paragraph::new(run_state(&t.run) + &tabs), status);
    }
    f.render_widget(
        Paragraph::new(Line::from(HINTS)).style(Style::new().add_modifier(Modifier::DIM)),
        keys,
    );
    if app
        .prompt
        .as_ref()
        .is_some_and(|p| matches!(p.ask, Ask::Palette | Ask::History | Ask::Presets))
    {
        palette(f, app);
    }
    if let Some(top) = app.help {
        help(f, top);
    }
    if let Some((title, lines, top)) = &app.info {
        popup(f, title, lines, *top);
    }
}

/// The command line: `antlion`, then what the user typed, and on the lower border why the
/// CLI refuses it. It scrolls to the side to keep the cursor in view.
fn command_line(f: &mut Frame, app: &App, area: Rect) {
    // The command line gets the letters whatever has the focus, so it has a plain border.
    let mut block = Block::bordered().title(" Command ");
    let args = app.args();
    if !args.is_empty()
        && let Some(e) = super::command::refusal(&args)
    {
        block = block.title_bottom(Line::styled(format!(" {e} "), Style::new().fg(Color::Red)));
    }
    let inner = block.inner(area);
    let at = PROGRAM.len() + app.before().chars().count();
    let left = (at + 1).saturating_sub(inner.width as usize);
    let text = Line::from(vec![
        Span::styled(PROGRAM, Style::new().add_modifier(Modifier::BOLD)),
        Span::raw(app.line.as_str()),
    ]);
    let left16 = u16::try_from(left).unwrap_or(u16::MAX);
    f.render_widget(Paragraph::new(text).block(block).scroll((0, left16)), area);
    let open = app.prompt.is_some() || app.help.is_some() || app.info.is_some();
    if !open {
        f.set_cursor_position((inner.x + (at - left) as u16, inner.y));
    }
}

/// The words that can come at the cursor. When there is none, why.
fn suggestions_pane(f: &mut Frame, app: &App, area: Rect) {
    let block = frame(app.focus == Focus::Suggestions)
        .title(format!(" Suggestions ({}) ", app.suggestions.len()));
    if app.suggestions.is_empty() {
        let before = app.before();
        let why = if before.trim_end().ends_with("--jobs") && before.ends_with(' ') {
            "type a number, 1 or more"
        } else if !complete::word(before).is_empty() {
            "nothing matches the word typed"
        } else {
            "nothing more to add: Ctrl+R runs the command"
        };
        let text = Paragraph::new(format!(" {why}"))
            .style(Style::new().add_modifier(Modifier::DIM))
            .wrap(Wrap { trim: false })
            .block(block);
        f.render_widget(text, area);
        return;
    }
    let rows: Vec<ListItem> = app
        .suggestions
        .iter()
        .map(|s| ListItem::new(format!(" {}", s.text)))
        .collect();
    // Enter puts in the suggestion under the cursor from any panel, so it always shows.
    let cursor = Style::new().add_modifier(Modifier::REVERSED);
    let widget = List::new(rows).block(block).highlight_style(cursor);
    let mut state = ListState::default().with_selected(Some(app.pick));
    f.render_stateful_widget(widget, area, &mut state);
}

/// Every key of the screen, by where it works.
const KEYS: &str = "\
COMMAND LINE

    type             the command, after antlion; letters always go here
    Enter            put in the suggestion under the cursor
    Ctrl+R           run the command
    Left Right       move the cursor; Home and End go to the start and the end
    Backspace Del    delete the character before the cursor, or at it
    Ctrl+W Ctrl+U    delete the word before the cursor, or all text before it

PANELS

    Tab  Shift+Tab   next panel, panel before: suggestions, output, details
    Up Down          move one item or line in the panel
    PgUp PgDn        move a page in the panel
    Ctrl+Home        go to the start of the panel
    Ctrl+End         go to the end; in the output, follow new output

OUTPUT

    Shift+Left       scroll to the left; Shift+Right scrolls to the right
    Ctrl+F           search; case counts only with a capital
    Ctrl+N Ctrl+B    next match, match before
    Ctrl+PgUp        tab before; Ctrl+PgDn goes to the next tab
    Ctrl+S           save the output to a file
    Ctrl+Y           copy the command

EVERYWHERE

    Ctrl+P           command palette, with the history of runs and more
    F1               this help
    Ctrl+C           cancel the run; with no run, quit
    Ctrl+Q           quit at once, and stop the run

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
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    popup(f, "Help", &lines, top);
}

/// A box over most of the screen with `lines`, from line `top`, and the keys to scroll it.
fn popup(f: &mut Frame, title: &str, lines: &[String], top: usize) {
    let lines: Vec<Line> = lines.iter().map(|l| Line::raw(l.as_str())).collect();
    let area = f.area();
    let r = Rect::new(
        2,
        1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    let top = top.min(lines.len().saturating_sub(1));
    let block = Block::bordered()
        .title(format!(" {title} "))
        .title_bottom(Line::from(" j/k scroll  Esc close ").right_aligned())
        .border_style(Style::new().fg(Color::Cyan));
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
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
        .title(match &app.prompt {
            Some(p) if p.ask == Ask::History => format!(" history: {} ", p.text),
            Some(p) if p.ask == Ask::Presets => format!(" presets: {} ", p.text),
            p => format!(" : {} ", p.as_ref().map_or("", |p| p.text.as_str())),
        })
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
    let block = frame(app.focus == Focus::Output);
    let Some(t) = app.current() else {
        f.render_widget(block.title(" Output "), area);
        return;
    };
    let r = &t.run;
    let block = block.title(format!(" Output: {} ", t.title()));
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
    let [_, output, ..] = areas(area);
    output.height.saturating_sub(2) as usize
}

/// The parts of the screen: on the left the command line and the output under it, on the
/// right the suggestions and their details under them, and at the bottom the status and
/// the key hints.
fn areas(area: Rect) -> [Rect; 6] {
    let [main, status, keys] = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(main);
    let [command, output] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).areas(left);
    let [suggestions, details] =
        Layout::vertical([Constraint::Percentage(40), Constraint::Min(3)]).areas(right);
    [command, output, suggestions, details, status, keys]
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

/// The most used keys, on the last row.
const HINTS: &str = " Enter add  ^R run  Tab panel  ↑↓ move  ^F find  ^P palette  F1 help  ^C quit";

/// A pane's border, bright when it has the focus.
fn frame(focused: bool) -> Block<'static> {
    let style = if focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().add_modifier(Modifier::DIM)
    };
    Block::bordered().border_style(style)
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

    fn typed(line: &str) -> App {
        let mut app = App::default();
        for c in line.chars() {
            app.key(ratatui::crossterm::event::KeyEvent::from(
                ratatui::crossterm::event::KeyCode::Char(c),
            ));
        }
        app
    }

    #[test]
    fn layout_fits_80_by_24() {
        let s = screen(&App::default(), 80, 24);
        // The command line and the suggestions share the top row, the output is under the
        // command line.
        assert!(s[0].contains(" Command ") && s[0].contains(" Suggestions (4) "));
        assert!(s[1].starts_with("│antlion "));
        assert!(s[1].contains(" --compare"));
        assert!(s[3].contains(" Output "));
        // The details of `--compare`, the suggestion under the cursor, under the
        // suggestions.
        let details = s.iter().position(|r| r.contains(" --compare ─")).unwrap();
        assert!(details > 3);
        assert!(s[details + 1].contains("diff both builds"));
        assert!(s[23].contains("^C quit"));
    }

    #[test]
    fn command_line_and_refusal() {
        let s = screen(&typed("--compare flow --report cost --json"), 100, 30);
        assert!(s[1].starts_with("│antlion --compare flow --report cost --json "));
        // The CLI takes it, so no reason shows on the border.
        assert!(s[2].starts_with("└──────"));

        let s = screen(&typed("--compare flow"), 100, 30);
        assert!(s[2].contains(" no report requested"));
    }

    #[test]
    fn long_line_scrolls() {
        let long = format!("--compare {}", "x".repeat(100));
        let s = screen(&typed(&long), 80, 24);
        // The end of the line, where the cursor is, stays in view.
        assert!(s[1].contains("xxxx │"));
        assert!(!s[1].contains("antlion"));
    }

    #[test]
    fn suggestions_and_why_none() {
        let s = screen(&typed("--compare flow --report si"), 100, 30);
        assert!(s[1].contains(" size"));
        let s = screen(&typed("--compare flow --report size --jobs "), 100, 30);
        assert!(s[0].contains(" Suggestions (0) "));
        assert!(s[1].contains("type a number, 1 or more"));
        let s = screen(&typed("--help "), 100, 30);
        assert!(s[1].contains("nothing more to add"));
        let s = screen(&typed("--zz"), 100, 30);
        assert!(s[1].contains("nothing matches"));
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
        assert!(s[3].contains(" Output: antlion -c "));
        // The end is on screen, the start is not.
        assert!(s.iter().any(|r| r.contains("line100")));
        assert!(s.iter().any(|r| r.contains("oops")));
        assert!(s.iter().all(|r| !r.contains("line1 ")));
        assert!(s[22].contains("done") && s[22].contains("101 lines  tab 1 of 1"));
    }

    #[test]
    fn help_overlay() {
        let mut app = App {
            help: Some(0),
            ..App::default()
        };
        let s = screen(&app, 100, 60);
        assert!(s[1].contains(" Help "));
        assert!(s[2].contains("COMMAND LINE"));
        assert!(s.iter().any(|r| r.contains("put in the suggestion")));
        // The inputs part of help.md comes after the keys.
        assert!((0..120).any(|top| {
            app.help = Some(top);
            screen(&app, 100, 60)
                .iter()
                .any(|r| r.contains("small       values that ride inline"))
        }));
    }

    #[test]
    fn too_small() {
        let s = screen(&App::default(), 50, 10);
        assert!(s[0].starts_with("terminal too small: 50x10"));
    }
}
