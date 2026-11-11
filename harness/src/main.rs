use std::{env, fs};

use cli::Command;
use error::{Error, Result};

mod build;
mod cli;
mod compare;
mod error;
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
        Command::Build(arg) => {
            let dir = std::path::PathBuf::from(&arg);
            let name = build::name_of(&arg);
            let out_dir = format!("out/{name}");
            fs::create_dir_all(&out_dir)
                .map_err(|_| Error::failed("failed to create the output directory"))?;

            build::sdk::build(&dir, &out_dir)?;
            build::solang::build(&dir, &name, &out_dir)?;
        }
        Command::Compare(c) => compare::run(&c)?,
    }
    Ok(())
}
