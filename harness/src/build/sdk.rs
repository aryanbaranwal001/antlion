use std::{fs, process::Command};

/// Build the Rust soroban-sdk contract to wasm and copy it to `out/<name>/sdk.wasm`.
pub fn build(name: &str, out_dir: &str) {
    let status = Command::new("cargo")
        .args(["build", "-p", name, "--release", "--target", "wasm32v1-none"])
        .status()
        .expect("failed to run cargo");
    assert!(status.success(), "cargo build failed");

    fs::copy(
        format!("target/wasm32v1-none/release/{name}.wasm"),
        format!("{out_dir}/sdk.wasm"),
    )
    .expect("failed to copy rust wasm");
}
