use std::{env, fs};

mod build;

fn main() {
    let name = env::args().nth(1).expect("usage: antlion <contract-name>");

    let out_dir = format!("out/{name}");
    fs::create_dir_all(&out_dir).unwrap();

    build::sdk::build(&name, &out_dir);
    build::solang::build(&name, &out_dir);
}
