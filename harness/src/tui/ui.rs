//! Drawing. Nothing here changes the state.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph},
};

use super::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    let [main, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(f.area());

    let block = Block::bordered().title(" antlion ");
    f.render_widget(Paragraph::new("q quit").block(block), main);

    if let Some(s) = &app.status {
        f.render_widget(
            Paragraph::new(s.as_str()).style(Style::new().fg(Color::Yellow)),
            status,
        );
    }
}
