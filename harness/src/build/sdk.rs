use std::{fs, process::Command};

/// Build the rust sdk contract to `out/<name>/sdk.wasm`.
///
/// soroban-sdk 28 refuses a plain `cargo build`: the spec it emits carries markers that
/// the build system has to shake out, and the section is only correct once that has
/// happened. `stellar contract build` runs the same `cargo rustc --release --target
/// wasm32v1-none` underneath, so the workspace release profile still applies.
pub fn build(name: &str, out_dir: &str) {
    let status = Command::new("stellar")
        .args(["contract", "build", "--package", name, "--out-dir", out_dir])
        .status()
        .expect("[err] failed to run stellar");

    assert!(status.success(), "[err] stellar contract build failed");

    fs::rename(
        format!("{out_dir}/{name}.wasm"),
        format!("{out_dir}/sdk.wasm"),
    )
    .expect("[err] failed to rename rust wasm");
}
