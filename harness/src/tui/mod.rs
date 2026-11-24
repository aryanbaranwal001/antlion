//! The terminal UI of `--interactive`: pick pairs, reports and options from lists, see the
//! command they make, and run it.

mod app;
mod catalog;
mod command;
mod runner;
mod search;
mod ui;

use std::{
    io::{IsTerminal, stdin, stdout},
    time::Duration,
};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyEventKind},
};

use crate::error::{Error, Result};
use app::App;

/// How long the loop waits for a key before it draws again.
const TICK: Duration = Duration::from_millis(100);

/// Open the terminal UI, and return when the user quits.
pub fn run() -> Result<()> {
    // The UI draws on the screen and reads single keys, so a pipe or a file cannot hold it.
    if !stdin().is_terminal() || !stdout().is_terminal() {
        return Err(Error::usage(
            "--interactive needs a terminal; use --build or --compare in a pipe or a script",
        ));
    }

    let dir = std::env::current_dir()
        .map_err(|e| Error::failed(format!("cannot read the working directory: {e}")))?;
    let mut app = App::new(catalog::pairs(&dir));
    app.root = dir.clone();
    // A run is antlion itself again, so the command shown is the command that runs.
    app.program = std::env::current_exe()
        .map_err(|e| Error::failed(format!("cannot find the antlion binary: {e}")))?
        .to_string_lossy()
        .into_owned();
    app.status = catalog::root_warning(&dir);

    // `init` also puts the terminal back when a panic happens, so the shell stays usable.
    let mut terminal = ratatui::init();
    let out = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    out
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    let io = |e: std::io::Error| Error::failed(format!("terminal: {e}"));
    while !app.quit {
        app.tick();
        app.view_height = ui::output_height(terminal.size().map_err(io)?);
        terminal.draw(|f| ui::draw(f, app)).map_err(io)?;
        if event::poll(TICK).map_err(io)? {
            // A key is reported on press and, on some terminals, on release too.
            if let Event::Key(k) = event::read().map_err(io)?
                && k.kind == KeyEventKind::Press
            {
                app.key(k);
            }
        }
    }
    Ok(())
}
