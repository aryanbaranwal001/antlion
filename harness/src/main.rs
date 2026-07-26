#![allow(unused)]

use std::{env, fs};

mod build;
mod compare;
mod wasm;

fn main() {
    let mut args = env::args().skip(1);
    let usage = "usage: --build <name> | --compare <name> [--report sections|interface|...]";
    let cmd = args.next().expect(usage);
    let name = args.next().expect(usage);

    match cmd.as_str() {
        "--build" => {
            let out_dir = format!("out/{name}");
            fs::create_dir_all(&out_dir).unwrap();

            build::sdk::build(&name, &out_dir);
            build::solang::build(&name, &out_dir);
        }
        "--compare" => compare::run(&name, args),
        _ => panic!("{usage}"),
    }
}
