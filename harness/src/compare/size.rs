use crate::wasm::Contract;

/// Compare the two modules' total wasm byte size.
pub fn report(sdk: &Contract, solang: &Contract) {
    super::banner("total size");

    let (sdk, solang) = (sdk.wasm.len(), solang.wasm.len());

    println!("{:<7} {:<7} {:<7}", "sdk", "solang", "diff");
    println!(
        "{:<7} {:<7} {:<7}",
        sdk,
        solang,
        super::signed(sdk as i64 - solang as i64)
    );
    println!();
}
