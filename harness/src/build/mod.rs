use std::{
    fs,
    path::{Path, PathBuf},
};

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

/// Build every pair in `paths`, in order. A pair that fails is reported and skipped, so the
/// others still build.
pub fn all(paths: &[String]) -> Result<()> {
    let mut failed = Vec::new();
    for path in paths {
        match pair(path) {
            Ok(()) => {}
            Err(Error::Failed(msg)) => {
                eprintln!("[err] {path}: {msg}");
                failed.push(path.as_str());
            }
            Err(e) => return Err(e),
        }
    }

    if !failed.is_empty() {
        return Err(Error::failed(format!(
            "{} of {} pairs failed: {}",
            failed.len(),
            paths.len(),
            failed.join(", ")
        )));
    }
    Ok(())
}

/// Build both sides of the pair at `path` into `out/<name>/`.
///
/// Both compilers write into a temporary directory first. The two files move into place
/// only when both builds succeed, so `out/<name>/` never holds one new file next to one old
/// file, and a failed build of a new pair leaves no directory behind.
fn pair(path: &str) -> Result<()> {
    let name = name_of(path);
    let tmp = PathBuf::from(format!("out/.{name}.build"));

    let _ = fs::remove_dir_all(&tmp);
    let built = build_into(Path::new(path), &name, &tmp);
    let _ = fs::remove_dir_all(&tmp);
    built
}

/// Both builds into `tmp`, then both files into `out/<name>/`.
fn build_into(dir: &Path, name: &str, tmp: &Path) -> Result<()> {
    let mkdir = |d: &Path| {
        fs::create_dir_all(d).map_err(|_| Error::failed("failed to create the output directory"))
    };
    let (sdk_dir, solang_dir) = (tmp.join("sdk"), tmp.join("solang"));
    mkdir(&sdk_dir)?;
    mkdir(&solang_dir)?;

    let sdk = sdk::build(dir, &sdk_dir)?;
    let solang = solang::build(dir, name, &solang_dir)?;

    let out = PathBuf::from(format!("out/{name}"));
    mkdir(&out)?;
    let rename = |from: &Path, to: &str| {
        fs::rename(from, out.join(to)).map_err(|_| Error::failed("failed to rename wasm"))
    };
    rename(&sdk, "sdk.wasm")?;
    rename(&solang, "solang.wasm")
}

/// The one wasm file a compiler wrote into `dir`. Both compilers name their output after
/// the crate or the solidity contract, so the name cannot be predicted.
fn only_wasm(dir: &Path) -> Result<PathBuf> {
    let found: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|_| Error::failed("failed to read the output directory"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "wasm"))
        .collect();

    match <[PathBuf; 1]>::try_from(found) {
        Ok([built]) => Ok(built),
        Err(found) => Err(Error::failed(format!(
            "expected one wasm in {}, found {}",
            dir.display(),
            found.len()
        ))),
    }
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
