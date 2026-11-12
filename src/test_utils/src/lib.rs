//! Test helpers shared by the OpenSWOS_ng crates:
//!
//! - where the workspace, its committed test assets and its build output live;
//! - [`fixtures`]: listing a format's fixtures, JSON sidecars, byte-for-byte round trips;
//! - [`data`]: your own copy of the game, outside the repository;
//! - [`cache`]: test data downloaded once into `target/cache/` (D8);
//! - [`skip`] and [`require!`]: tests whose data is missing say so and return.

use std::path::{Path, PathBuf};

pub mod cache;
pub mod data;
pub mod fixtures;
pub mod skip;

/// The folder for temporary data, under `target/`: every cache, download and checkout.
const CACHE_DIR: &str = "cache";

/// The workspace root: the nearest ancestor of this crate that holds `Cargo.lock`.
///
/// It is found from this crate's own location, so the result does not depend on the
/// directory the tests are started from.
///
/// # Panics
///
/// When no ancestor holds a `Cargo.lock`, which cannot happen inside the workspace.
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

/// The folder for temporary data: `<root>/target/cache`.
#[must_use]
pub fn cache_path() -> PathBuf {
    target_path().join(CACHE_DIR)
}

/// Unwraps a `Result<T, Skip>`, or reports the skip ([`skip::report`]) and returns from
/// the test.
///
/// ```ignore
/// let team_file = openswos_test_utils::require!(openswos_test_utils::data::game_file("DATA/TEAM.008"));
/// ```
#[macro_export]
macro_rules! require {
    ($value:expr) => {
        match $value {
            ::std::result::Result::Ok(value) => value,
            ::std::result::Result::Err(skip) => {
                $crate::skip::report(&skip);
                return;
            }
        }
    };
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
    fn target_and_cache_paths_are_under_the_root() {
        assert_eq!(target_path(), root_path().join("target"));
        assert_eq!(cache_path(), root_path().join("target").join("cache"));
    }
}
