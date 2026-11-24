//! Drawing. Nothing here changes the state.

use ratatui::{
    Frame,
    widgets::{Block, Paragraph},
};

use super::app::App;

pub fn draw(f: &mut Frame, _app: &App) {
    let block = Block::bordered().title(" antlion ");
    f.render_widget(Paragraph::new("q quit").block(block), f.area());
}
