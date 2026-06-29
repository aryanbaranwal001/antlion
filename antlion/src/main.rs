use std::{env, fs, process::Command};

fn main() {
    let name = env::args().nth(1).expect("usage: antlion <contract-name>");

    let out_dir = format!("out/{name}");
    fs::create_dir_all(&out_dir).unwrap();

    // Rust contract -> wasm
    let status = Command::new("cargo")
        .args(["build", "-p", &name, "--target", "wasm32v1-none"])
        .status()
        .expect("failed to run cargo");
    assert!(status.success(), "cargo build failed");

    fs::copy(
        format!("target/wasm32v1-none/debug/{name}.wasm"),
        format!("{out_dir}/rust.wasm"),
    )
    .expect("failed to copy rust wasm");

    // Solidity contract -> wasm
    let status = Command::new("solang")
        .args([
            "compile",
            "--target",
            "soroban",
            "--output",
            &out_dir,
            &format!("contracts/{name}/{name}.sol"),
        ])
        .status()
        .expect("failed to run solang");
    assert!(status.success(), "solang compile failed");

    // solang names output after the contract name (capitalized)
    let capitalized = {
        let mut c = name.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    };

    // remvoe abi file
    fs::remove_file(format!("{out_dir}/{capitalized}.abi")).expect("failed to remove abi");
    fs::rename(
        format!("{out_dir}/{capitalized}.wasm"),
        format!("{out_dir}/solang.wasm"),
    )
    .expect("failed to rename solang wasm");
}
