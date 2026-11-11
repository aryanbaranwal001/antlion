use std::{fs, path::Path, process::Command};

use crate::error::{Error, Result};

/// Compile `<dir>/solang/<name>.sol` to `<out_dir>/solang.wasm`.
pub fn build(dir: &Path, name: &str, out_dir: &str) -> Result<()> {
    let src = super::sol_of(dir, name)?;

    let status = Command::new("solang")
        .args(["compile", "--target", "soroban", "--output", out_dir])
        .arg(&src)
        .status()
        .map_err(|_| Error::failed("failed to run solang"))?;

    if !status.success() {
        return Err(Error::failed("solang compile failed"));
    }

    // solang writes an abi beside the wasm, named after the contract rather than the file.
    let entries = fs::read_dir(out_dir)
        .map_err(|_| Error::failed("failed to read the output directory"))?;
    for entry in entries {
        let path = entry
            .map_err(|_| Error::failed("failed to read a directory entry"))?
            .path();
        if path.extension().is_some_and(|e| e == "abi") {
            fs::remove_file(&path).map_err(|_| Error::failed("failed to remove abi"))?;
        }
    }

    super::take_wasm(out_dir, "solang.wasm")
}
