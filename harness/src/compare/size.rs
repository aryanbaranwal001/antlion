// total size
pub fn report(sdk: &[u8], solang: &[u8]) {
    super::banner("total size");

    println!("{:<7} {:<7} {:<7}", "sdk", "solang", "diff");
    println!(
        "{:<7} {:<7} {:<7}",
        sdk.len(),
        solang.len(),
        super::signed(sdk.len() as i64 - solang.len() as i64)
    );
    println!();
}
