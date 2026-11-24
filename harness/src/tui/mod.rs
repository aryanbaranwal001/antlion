//! The terminal UI of `--interactive`: pick pairs, reports and options from lists, see the
//! command they make, and run it.

use std::io::{IsTerminal, stdin, stdout};

use crate::error::{Error, Result};

/// Open the terminal UI, and return when the user quits.
pub fn run() -> Result<()> {
    // The UI draws on the screen and reads single keys, so a pipe or a file cannot hold it.
    if !stdin().is_terminal() || !stdout().is_terminal() {
        return Err(Error::usage(
            "--interactive needs a terminal; use --build or --compare in a pipe or a script",
        ));
    }
    Ok(())
}
