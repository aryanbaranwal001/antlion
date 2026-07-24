#![allow(unused)]

use std::{env, fs};

mod build;
mod compare;
mod wasm;

fn main() {
    let mut args = env::args().skip(1);

    let usage = "usage: --build <name> | --compare <name> [--sections all|size|bytes|interface|cost ...]";
    let cmd = args.next().expect(usage);
    let name = args.next().expect(usage);

    match cmd.as_str() {
        "--build" => {
            let out_dir = format!("out/{name}");
            fs::create_dir_all(&out_dir).unwrap();

            build::sdk::build(&name, &out_dir);
            build::solang::build(&name, &out_dir);
        }
        "--compare" => {
            let sections = match args.next().as_deref() {
                Some("--sections") => args.collect(),
                Some(other) => panic!("unexpected arg `{other}`; {usage}"),
                None => Vec::new(),
            };
            compare::run(&name, &sections);
        }
        _ => panic!("{usage}"),
    }
}
