// total size
pub fn report(name: &str, sdk: &[u8], solang: &[u8]) {
    println!("== {name}: total size ==");
    println!("  solang {:>6} bytes", solang.len());
    println!("  sdk    {:>6} bytes", sdk.len());
    println!(
        "  diff   {:>6} (solang - sdk)\n",
        super::signed(solang.len() as i64 - sdk.len() as i64)
    );
}
