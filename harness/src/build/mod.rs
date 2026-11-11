use std::{fs, path::{Path, PathBuf}};

use crate::error::{Error, Result};

pub mod sdk;
pub mod solang;

/// The contract's name, which is the last component of the argument. It names the output
/// directory, so `contracts/flow` and `../elsewhere/flow` both build into `out/flow/`.
pub fn name_of(arg: &str) -> String {
    Path::new(arg.trim_end_matches(['/', '\\']))
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(arg)
        .to_string()
}

/// Rename whatever wasm a compiler just dropped in `out_dir` to `dest`.
///
/// Both compilers name their output after the crate or the solidity contract rather than
/// after the directory, and neither has to match. Taking the one new file avoids having
/// to predict the name.
pub fn take_wasm(out_dir: &str, dest: &str) -> Result<()> {
    let found: Vec<PathBuf> = fs::read_dir(out_dir)
        .map_err(|_| Error::failed("failed to read the output directory"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "wasm"))
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n != "sdk.wasm" && n != "solang.wasm"
        })
        .collect();

    let [built] = found.as_slice() else {
        return Err(Error::failed(format!(
            "expected one new wasm in {out_dir}, found {}",
            found.len()
        )));
    };

    fs::rename(built, format!("{out_dir}/{dest}"))
        .map_err(|_| Error::failed("failed to rename wasm"))
}

/// The solidity source for the pair at `dir`, which is named after the directory:
/// `<dir>/solang/<name>.sol`.
///
/// The contract name inside the file is free, only the filename is fixed.
pub fn sol_of(dir: &Path, name: &str) -> Result<PathBuf> {
    let src = dir.join("solang").join(format!("{name}.sol"));
    if !src.exists() {
        return Err(Error::failed(format!(
            "no solidity source at {}\n[err] the .sol must be named after its directory",
            src.display()
        )));
    }
    Ok(src)
}
