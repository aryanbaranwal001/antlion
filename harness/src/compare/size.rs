// total size
pub fn report(name: &str, sdk: &[u8], solang: &[u8]) {
    println!("== {name}: total size ==");

    println!("  {:<7} {:<7} {:<7}", "sdk", "solang", "diff");
    println!(
        "  {:<7} {:<7} {:<7}",
        sdk.len(),
        solang.len(),
        super::signed(sdk.len() as i64 - solang.len() as i64)
    );
}
