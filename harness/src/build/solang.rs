use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::error::{Error, Result};

/// Compile `<dir>/solang/<name>.sol` into `out_dir`, and return the wasm it made. The abi
/// solang writes beside it stays in `out_dir`.
pub fn build(dir: &Path, name: &str, out_dir: &Path) -> Result<PathBuf> {
    let src = super::sol_of(dir, name)?;

    let status = Command::new("solang")
        .args(["compile", "--target", "soroban", "--output"])
        .arg(out_dir)
        .arg(&src)
        .status()
        .map_err(|e| {
            Error::failed(format!(
                "`solang` could not be started ({e}). The binary on PATH must be named exactly `solang`."
            ))
        })?;

    if !status.success() {
        return Err(Error::failed("solang compile failed"));
    }

    super::only_wasm(out_dir)
}
