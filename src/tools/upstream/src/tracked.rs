//! Which upstream files the ledger covers.

use std::collections::BTreeSet;
use std::path::Path;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::error::{Result, UpstreamError};
use crate::git::{EntryKind, Git};
use crate::ledger::Ledger;

/// Compiles glob patterns over upstream paths: `*` stays within one directory, `**`
/// crosses directories.
///
/// # Errors
///
/// Fails when a pattern does not parse.
pub fn glob_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .map_err(|source| UpstreamError::Pattern {
                pattern: pattern.clone(),
                source,
            })?;
        builder.add(glob);
    }
    builder.build().map_err(|source| UpstreamError::Pattern {
        pattern: patterns.join(", "),
        source,
    })
}

/// The paths that match `set`.
#[must_use]
pub fn select(paths: impl IntoIterator<Item = String>, set: &GlobSet) -> BTreeSet<String> {
    paths
        .into_iter()
        .filter(|path| set.is_match(path))
        .collect()
}

/// Every upstream file at the pinned commit that the ledger must cover: the repository's
/// own files and its tracked submodules' files (prefixed with the submodule path), that
/// match `upstream.tracked`.
///
/// # Errors
///
/// Fails when git fails or a pattern does not parse.
pub fn upstream_files(ledger: &Ledger, checkout: &Path) -> Result<BTreeSet<String>> {
    let set = glob_set(&ledger.upstream.tracked)?;
    let main = Git::new(checkout);
    let mut paths = blob_paths(&main, &ledger.upstream.pinned_commit, "")?;
    for submodule in &ledger.submodules {
        let repo = Git::new(checkout.join(&submodule.path));
        let prefix = format!("{}/", submodule.path);
        paths.extend(blob_paths(&repo, &submodule.commit, &prefix)?);
    }
    Ok(select(paths, &set))
}

/// The files (not gitlinks) of the tree at `rev`, each path prefixed with `prefix`.
fn blob_paths(git: &Git, rev: &str, prefix: &str) -> Result<Vec<String>> {
    Ok(git
        .list_tree(rev)?
        .into_iter()
        .filter_map(|entry| match entry.kind {
            EntryKind::Blob => Some(format!("{prefix}{}", entry.path)),
            EntryKind::Commit => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::test_repo::{commit, init};
    use crate::ledger::Upstream;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn single_star_stays_within_a_directory() {
        let set = glob_set(&strings(&["src/*.c"])).expect("valid pattern");
        let selected = select(strings(&["src/a.c", "src/sub/b.c", "src/a.h"]), &set);
        assert_eq!(selected, BTreeSet::from(["src/a.c".to_owned()]));
    }

    #[test]
    fn double_star_crosses_directories() {
        let set = glob_set(&strings(&["src/**"])).expect("valid pattern");
        let selected = select(strings(&["src/a.c", "src/sub/b.c", "include/a.h"]), &set);
        assert_eq!(
            selected,
            BTreeSet::from(["src/a.c".to_owned(), "src/sub/b.c".to_owned()])
        );
    }

    #[test]
    fn any_pattern_may_match() {
        let set = glob_set(&strings(&["src/netplay*.c", "include/netplay*.h"])).expect("valid");
        let selected = select(
            strings(&["src/netplay.c", "include/netplay_lobby.h", "src/net.c"]),
            &set,
        );
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn invalid_patterns_are_reported() {
        let error = glob_set(&strings(&["src/[a.c"])).expect_err("unclosed class");
        assert!(matches!(error, UpstreamError::Pattern { .. }), "{error}");
    }

    #[test]
    fn upstream_files_cover_the_tracked_paths_at_the_pin() {
        let dir = tempfile::tempdir().expect("temp dir");
        init(dir.path());
        let pinned = commit(
            dir.path(),
            &[("src/a.c", "a"), ("docs/x.md", "x"), ("include/a.h", "h")],
            "first",
        );
        commit(dir.path(), &[("src/later.c", "l")], "after the pin");
        let ledger = Ledger {
            upstream: Upstream {
                repository: String::new(),
                pinned_commit: pinned,
                tracked: strings(&["src/**", "include/**"]),
                checkout: Vec::new(),
            },
            submodules: Vec::new(),
            files: std::collections::BTreeMap::new(),
        };
        let files = upstream_files(&ledger, dir.path()).expect("listing");
        assert_eq!(
            files,
            BTreeSet::from(["include/a.h".to_owned(), "src/a.c".to_owned()])
        );
    }
}
