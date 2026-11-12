//! Where the ledger and the upstream checkout live.

use std::path::{Path, PathBuf};

use crate::error::{Result, UpstreamError};
use crate::ledger::LEDGER_FILE_NAME;

/// Environment variable that overrides the upstream checkout directory.
pub const CHECKOUT_DIR_ENV: &str = "OPENSWOS_UPSTREAM_DIR";

/// The default upstream checkout directory, relative to the workspace root: inside
/// `target/`, with every other generated or downloaded file (`cargo clean` removes it).
pub const DEFAULT_CHECKOUT_DIR: &str = "target/cache/upstream/OpenSWOS";

/// The workspace this tool works on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// The nearest directory, from `start` upwards, that holds the ledger.
    ///
    /// # Errors
    ///
    /// Fails when no directory does.
    pub fn discover(start: &Path) -> Result<Self> {
        start
            .ancestors()
            .find(|dir| dir.join(LEDGER_FILE_NAME).is_file())
            .map(|root| Self {
                root: root.to_path_buf(),
            })
            .ok_or_else(|| UpstreamError::NoLedger {
                start: start.to_path_buf(),
            })
    }

    /// The workspace root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The ledger file.
    #[must_use]
    pub fn ledger_path(&self) -> PathBuf {
        self.root.join(LEDGER_FILE_NAME)
    }

    /// The upstream checkout: `override_dir` when given (relative paths are taken from
    /// the root), else the default directory under the root.
    #[must_use]
    pub fn checkout_dir(&self, override_dir: Option<&Path>) -> PathBuf {
        match override_dir {
            Some(dir) => self.root.join(dir),
            None => self.root.join(DEFAULT_CHECKOUT_DIR),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_ledger_in_a_parent_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join(LEDGER_FILE_NAME), "").expect("ledger");
        let nested = dir.path().join("src/tools");
        std::fs::create_dir_all(&nested).expect("nested dir");
        let workspace = Workspace::discover(&nested).expect("found");
        assert_eq!(workspace.root(), dir.path());
        assert_eq!(workspace.ledger_path(), dir.path().join(LEDGER_FILE_NAME));
    }

    #[test]
    fn fails_without_a_ledger() {
        let dir = tempfile::tempdir().expect("temp dir");
        let error = Workspace::discover(dir.path()).expect_err("no ledger");
        assert!(matches!(error, UpstreamError::NoLedger { .. }), "{error}");
    }

    #[test]
    fn the_checkout_defaults_under_the_root_and_can_be_overridden() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join(LEDGER_FILE_NAME), "").expect("ledger");
        let workspace = Workspace::discover(dir.path()).expect("found");
        assert_eq!(
            workspace.checkout_dir(None),
            dir.path().join(DEFAULT_CHECKOUT_DIR)
        );
        assert_eq!(
            workspace.checkout_dir(Some(Path::new("other"))),
            dir.path().join("other")
        );
        let absolute = dir.path().join("abs");
        assert_eq!(workspace.checkout_dir(Some(&absolute)), absolute);
    }
}
