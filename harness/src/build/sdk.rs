use std::{path::Path, process::Command};

use crate::error::{Error, Result};

/// Build the rust contract under `<dir>/sdk/` to `<out_dir>/sdk.wasm`.
///
/// soroban-sdk 28 refuses a plain `cargo build`: the spec it emits carries markers that
/// the build system has to shake out, and the section is only correct once that has
/// happened. `stellar contract build` runs the same `cargo rustc --release --target
/// wasm32v1-none` underneath, so the workspace release profile still applies.
pub fn build(dir: &Path, out_dir: &str) -> Result<()> {
    let manifest = dir.join("sdk/Cargo.toml");
    if !manifest.exists() {
        return Err(Error::failed(format!(
            "no manifest at {}",
            manifest.display()
        )));
    }

    let status = Command::new("stellar")
        .args(["contract", "build"])
        .arg("--manifest-path")
        .arg(&manifest)
        .args(["--out-dir", out_dir])
        .status()
        .map_err(|_| Error::failed("failed to run stellar"))?;

    if !status.success() {
        return Err(Error::failed("stellar contract build failed"));
    }

    super::take_wasm(out_dir, "sdk.wasm")
}
