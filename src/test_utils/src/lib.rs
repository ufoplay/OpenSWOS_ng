//! Test helpers shared by the OpenSWOS_ng crates: where the workspace, its test assets and
//! its build output live.
//!
//! The download cache for test data that cannot be committed (docs/PLAN.md §6) is added
//! here in M1.

use std::path::{Path, PathBuf};

/// The workspace root: the nearest ancestor of this crate that holds `Cargo.lock`.
///
/// It is found from this crate's own location, so the result does not depend on the
/// directory the tests are started from.
#[must_use]
pub fn root_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| dir.join("Cargo.lock").is_file())
        .map(Path::to_path_buf)
        .expect("openswos_test_utils always lives inside the workspace, which has a Cargo.lock")
}

/// The committed test assets: `<root>/assets`.
#[must_use]
pub fn assets_path() -> PathBuf {
    root_path().join("assets")
}

/// The build output directory: `<root>/target`.
#[must_use]
pub fn target_path() -> PathBuf {
    root_path().join("target")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_path_is_the_workspace_root() {
        let manifest = std::fs::read_to_string(root_path().join("Cargo.toml"))
            .expect("the workspace root has a Cargo.toml");
        assert!(manifest.contains("[workspace]"));
    }

    #[test]
    fn assets_path_is_under_the_root() {
        assert_eq!(assets_path(), root_path().join("assets"));
        assert!(assets_path().is_dir());
    }

    #[test]
    fn target_path_is_under_the_root() {
        assert_eq!(target_path(), root_path().join("target"));
    }
}
