//! `fetch`: clone or update the upstream checkout at the pinned commit.

use std::path::Path;

use crate::error::{Result, UpstreamError};
use crate::git::Git;
use crate::ledger::Ledger;

/// Clones upstream into `dir` (or reuses the clone there), checks out the pinned commit
/// restricted to `upstream.checkout`, and checks out the ledger's submodules.
///
/// # Errors
///
/// Fails when git fails, or a submodule is pinned upstream at a commit other than the
/// one the ledger records.
pub fn fetch(ledger: &Ledger, dir: &Path) -> Result<()> {
    let pinned = &ledger.upstream.pinned_commit;
    let git = if dir.join(".git").exists() {
        Git::new(dir)
    } else {
        if let Some(parent) = dir.parent() {
            std::fs::create_dir_all(parent).map_err(|source| UpstreamError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        Git::clone_blobless(&ledger.upstream.repository, dir)?
    };
    if !git.has_commit(pinned) {
        git.fetch_origin()?;
    }
    git.sparse_checkout(&ledger.upstream.checkout)?;
    git.checkout_detached(pinned)?;
    for submodule in &ledger.submodules {
        git.update_submodule(&submodule.path)?;
        let actual = Git::new(dir.join(&submodule.path)).head()?;
        if actual != submodule.commit {
            return Err(UpstreamError::SubmoduleMismatch {
                path: submodule.path.clone(),
                expected: submodule.commit.clone(),
                actual,
            });
        }
    }
    Ok(())
}

/// Fails unless `dir` is a checkout that has the pinned commit.
///
/// # Errors
///
/// [`UpstreamError::NoCheckout`] when it is not.
pub fn require_checkout(ledger: &Ledger, dir: &Path) -> Result<Git> {
    let git = Git::new(dir);
    if dir.join(".git").exists() && git.has_commit(&ledger.upstream.pinned_commit) {
        Ok(git)
    } else {
        Err(UpstreamError::NoCheckout {
            dir: dir.to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::test_repo::{commit, init};
    use crate::ledger::Upstream;

    /// A ledger cloning from a local repository. A plain path works on every OS (git
    /// ignores the blob filter for local clones, with a warning).
    fn ledger_for(repository: &Path, pinned: &str) -> Ledger {
        Ledger {
            upstream: Upstream {
                repository: repository.display().to_string(),
                pinned_commit: pinned.to_owned(),
                tracked: vec!["src/**".to_owned()],
                checkout: vec!["src".to_owned()],
            },
            submodules: Vec::new(),
            files: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn clones_and_checks_out_the_pin_sparsely() {
        let dir = tempfile::tempdir().expect("temp dir");
        let origin = dir.path().join("origin");
        init(&origin);
        let pinned = commit(&origin, &[("src/a.c", "a"), ("docs/x.md", "x")], "first");
        commit(&origin, &[("src/a.c", "later")], "after the pin");
        let checkout = dir.path().join("work/upstream");

        fetch(&ledger_for(&origin, &pinned), &checkout).expect("fetched");

        assert_eq!(Git::new(&checkout).head().expect("head"), pinned);
        assert_eq!(
            std::fs::read_to_string(checkout.join("src/a.c")).expect("checked out"),
            "a"
        );
        assert!(
            !checkout.join("docs/x.md").exists(),
            "docs/ is outside the sparse checkout"
        );
    }

    #[test]
    fn an_existing_clone_follows_a_new_pin() {
        let dir = tempfile::tempdir().expect("temp dir");
        let origin = dir.path().join("origin");
        init(&origin);
        let first = commit(&origin, &[("src/a.c", "1")], "first");
        let checkout = dir.path().join("upstream");
        fetch(&ledger_for(&origin, &first), &checkout).expect("first fetch");

        let second = commit(&origin, &[("src/a.c", "2")], "second");
        fetch(&ledger_for(&origin, &second), &checkout).expect("second fetch");

        assert_eq!(Git::new(&checkout).head().expect("head"), second);
        assert_eq!(
            std::fs::read_to_string(checkout.join("src/a.c")).expect("checked out"),
            "2"
        );
    }

    #[test]
    fn a_checkout_is_required_for_the_other_commands() {
        let dir = tempfile::tempdir().expect("temp dir");
        let origin = dir.path().join("origin");
        init(&origin);
        let pinned = commit(&origin, &[("src/a.c", "a")], "first");
        let ledger = ledger_for(&origin, &pinned);
        let checkout = dir.path().join("upstream");

        let error = require_checkout(&ledger, &checkout).expect_err("no checkout yet");
        assert!(matches!(error, UpstreamError::NoCheckout { .. }), "{error}");

        fetch(&ledger, &checkout).expect("fetched");
        assert!(require_checkout(&ledger, &checkout).is_ok());
    }
}
