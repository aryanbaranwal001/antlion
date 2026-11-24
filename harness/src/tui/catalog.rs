//! What the lists show: the pairs and their build state, the reports, the options.

use std::path::Path;

/// A warning when `dir` is not the project root. antlion reads `out/` and `contracts/` from
/// the working directory, so outside the root no pair can be found.
pub fn root_warning(dir: &Path) -> Option<String> {
    if dir.join("out").is_dir() || dir.join("contracts").is_dir() {
        return None;
    }
    Some(format!(
        "no out/ or contracts/ in {}: start antlion from the project root",
        dir.display()
    ))
}

#[cfg(test)]
pub(super) mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    /// A new empty folder for one test, removed first if an old run left it.
    pub fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("antlion-tui-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn root() {
        let d = scratch("root");
        assert!(root_warning(&d).is_some());
        fs::create_dir(d.join("contracts")).unwrap();
        assert!(root_warning(&d).is_none());

        let d = scratch("root-out");
        fs::create_dir(d.join("out")).unwrap();
        assert!(root_warning(&d).is_none());
    }
}
