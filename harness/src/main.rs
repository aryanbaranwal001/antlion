#![allow(unused)]

use std::{env, fs};

mod build;
mod compare;
mod wasm;

fn main() {
    let mut args = env::args().skip(1);
    let usage =
        "[err] usage: --build <contract> | --compare <contract>... --report <report>... | --help";
    let cmd = args.next().expect(usage);

    match cmd.as_str() {
        "--help" => print!("{}", include_str!("../../help.md")),
        "--build" => {
            let arg = args.next().expect(usage);
            let dir = build::dir_of(&arg);
            let name = build::name_of(&arg);
            let out_dir = format!("out/{name}");
            fs::create_dir_all(&out_dir).expect("[err] failed to create the output directory");

            build::sdk::build(&dir, &out_dir);
            build::solang::build(&dir, &name, &out_dir);
        }
        "--compare" => compare::run(&args.collect::<Vec<_>>()),
        _ => panic!("{usage}"),
    }
}
