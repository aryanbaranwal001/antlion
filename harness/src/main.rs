use std::{env, fs};

use error::{Error, Result};

mod build;
mod compare;
mod error;
mod wasm;

const VERSION: &str = "antlion version v0.1.1";

fn main() {
    if let Err(e) = run() {
        panic!("{e}");
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let usage = "usage: --build <contract> | --compare <contract>... --report <report>... | --help | --version";
    let cmd = args.next().ok_or_else(|| Error::usage(usage))?;

    match cmd.as_str() {
        "--help" => print!("{}", include_str!("../../help.md")),
        "--version" => println!("{VERSION}"),
        "--build" => {
            let arg = args.next().ok_or_else(|| Error::usage(usage))?;
            let dir = std::path::PathBuf::from(&arg);
            let name = build::name_of(&arg);
            let out_dir = format!("out/{name}");
            fs::create_dir_all(&out_dir)
                .map_err(|_| Error::failed("failed to create the output directory"))?;

            build::sdk::build(&dir, &out_dir);
            build::solang::build(&dir, &name, &out_dir);
        }
        "--compare" => compare::run(&args.collect::<Vec<_>>()),
        _ => return Err(Error::usage(usage)),
    }
    Ok(())
}
