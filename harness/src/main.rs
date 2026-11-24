use std::env;

use cli::Command;
use error::{Error, Result};

mod build;
mod cli;
mod compare;
mod error;
#[cfg(feature = "interactive")]
mod tui;
mod wasm;

const VERSION: &str = "antlion version v0.1.1";

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(match e {
            Error::Usage(_) => 2,
            Error::Failed(_) => 1,
        });
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    match cli::parse(&args)? {
        Command::Help => print!("{}", include_str!("../../help.md")),
        Command::Version => println!("{VERSION}"),
        Command::Build(paths) => build::all(&paths)?,
        Command::Compare(c) => compare::run(&c)?,
        #[cfg(feature = "interactive")]
        Command::Interactive => tui::run()?,
        #[cfg(not(feature = "interactive"))]
        Command::Interactive => {
            return Err(Error::failed(
                "this antlion was built without the `interactive` feature",
            ));
        }
    }
    Ok(())
}
