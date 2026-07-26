use std::{fs, process::Command};

/// Compile the solidity contract to wasm and normalize output to `out/<name>/solang.wasm`.
pub fn build(name: &str, out_dir: &str) {
    let status = Command::new("solang")
        .args([
            "compile",
            "--target",
            "soroban",
            "--output",
            out_dir,
            &format!("contracts/{name}/solang/{name}.sol"),
        ])
        .status()
        .expect("failed to run solang");

    assert!(status.success(), "solang compile failed");

    let capitalized = {
        let mut c = name.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => unreachable!("name was already validated to exist"),
        }
    };

    fs::remove_file(format!("{out_dir}/{capitalized}.abi")).expect("failed to remove abi");
    fs::rename(
        format!("{out_dir}/{capitalized}.wasm"),
        format!("{out_dir}/solang.wasm"),
    )
    .expect("failed to rename solang wasm");
}
