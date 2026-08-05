#![allow(unused)]

use std::{env, fs};

mod build;
mod compare;
mod wasm;

fn main() {
    let mut args = env::args().skip(1);
    let usage =
        "[err] usage: --build <name> | --compare <name>... --report sections|interface|...|all";
    let cmd = args.next().expect(usage);

    match cmd.as_str() {
        "--build" => {
            let name = args.next().expect(usage);
            let out_dir = format!("out/{name}");
            fs::create_dir_all(&out_dir).unwrap();

            build::sdk::build(&name, &out_dir);
            build::solang::build(&name, &out_dir);
        }
        "--compare" => compare::run(&args.collect::<Vec<_>>()),
        _ => panic!("{usage}"),
    }
}
