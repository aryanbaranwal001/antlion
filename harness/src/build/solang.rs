use std::{fs, path::Path, process::Command};

/// Compile `<dir>/solang/<name>.sol` to `<out_dir>/solang.wasm`.
pub fn build(dir: &Path, name: &str, out_dir: &str) {
    let src = super::sol_of(dir, name);

    let status = Command::new("solang")
        .args(["compile", "--target", "soroban", "--output", out_dir])
        .arg(&src)
        .status()
        .expect("[err] failed to run solang");

    assert!(status.success(), "[err] solang compile failed");

    // solang writes an abi beside the wasm, named after the contract rather than the file.
    for entry in fs::read_dir(out_dir).expect("[err] failed to read the output directory") {
        let path = entry.expect("[err] failed to read a directory entry").path();
        if path.extension().is_some_and(|e| e == "abi") {
            fs::remove_file(&path).expect("[err] failed to remove abi");
        }
    }

    super::take_wasm(out_dir, "solang.wasm");
}
