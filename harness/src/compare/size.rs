use serde_json::{Value, json};

use crate::wasm::Contract;

/// The two modules' total wasm byte size.
pub struct Data {
    sdk: usize,
    solang: usize,
}

pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    Data {
        sdk: sdk.wasm.len(),
        solang: solang.wasm.len(),
    }
}

/// Compare the two modules' total wasm byte size.
pub fn text(d: &Data) {
    super::banner("total size");

    println!("{:<7} {:<7} {:<7}", "sdk", "solang", "diff");
    println!(
        "{:<7} {:<7} {:<7}",
        d.sdk,
        d.solang,
        super::signed(d.sdk as i64 - d.solang as i64)
    );
    println!();
}

pub fn json(d: &Data) -> Value {
    json!({ "sdk": d.sdk, "solang": d.solang, "diff": d.sdk as i64 - d.solang as i64 })
}
