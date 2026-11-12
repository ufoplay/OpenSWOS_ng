//! The tool's commands, on the ledger file and the upstream checkout.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::check::Facts;
use crate::error::Result;
use crate::fetch::require_checkout;
use crate::git::Git;
use crate::ledger::{Ledger, is_commit_hash};
use crate::mark::{Changes, mark};
use crate::sync::{SyncReport, sync};
use crate::tracked::upstream_files;

/// Adds the new upstream files to the ledger file and drops the removed ones.
///
/// # Errors
///
/// Fails when the ledger or the checkout cannot be read, or the ledger cannot be written.
pub fn sync_ledger(ledger_path: &Path, checkout: &Path) -> Result<SyncReport> {
    let mut ledger = Ledger::load(ledger_path)?;
    require_checkout(&ledger, checkout)?;
    let upstream = upstream_files(&ledger, checkout)?;
    let report = sync(&mut ledger.files, &upstream);
    ledger.save(ledger_path)?;
    Ok(report)
}

/// Applies `changes` to the entries of the ledger file matching `patterns`.
///
/// # Errors
///
/// Fails when the ledger cannot be read or written, or `mark` refuses the changes.
pub fn mark_ledger(
    ledger_path: &Path,
    patterns: &[String],
    changes: &Changes,
) -> Result<Vec<String>> {
    let mut ledger = Ledger::load(ledger_path)?;
    let pin = ledger.upstream.pinned_commit.clone();
    let changed = mark(&mut ledger.files, patterns, changes, &pin)?;
    ledger.save(ledger_path)?;
    Ok(changed)
}

/// What `check` needs from git and the file system. `root` is the workspace root, which
/// the `rust` paths are relative to.
///
/// # Errors
///
/// Fails when the checkout is missing or git fails.
pub fn gather_facts(ledger: &Ledger, checkout: &Path, root: &Path) -> Result<Facts> {
    let git = require_checkout(ledger, checkout)?;
    let pin = &ledger.upstream.pinned_commit;
    let upstream_gitlinks = ledger
        .submodules
        .iter()
        .map(|submodule| Ok((submodule.path.clone(), git.gitlink(pin, &submodule.path)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut foreign_commits = BTreeSet::new();
    for (path, entry) in &ledger.files {
        if let Some(commit) = &entry.migrated_at
            && is_commit_hash(commit)
        {
            let place = locate(ledger, checkout, path);
            if !place
                .git
                .is_ancestor(commit, &place.pinned)
                .unwrap_or(false)
            {
                foreign_commits.insert(commit.clone());
            }
        }
    }
    let missing_paths = ledger
        .files
        .values()
        .flat_map(|entry| entry.rust.iter())
        .filter(|path| !root.join(path).exists())
        .cloned()
        .collect();
    Ok(Facts {
        upstream_files: upstream_files(ledger, checkout)?,
        upstream_gitlinks,
        foreign_commits,
        missing_paths,
    })
}

/// For each migrated file whose `migrated_at` is not the pin: the upstream commits that
/// touched it since.
///
/// # Errors
///
/// Fails when the checkout is missing or git fails.
pub fn behind_counts(ledger: &Ledger, checkout: &Path) -> Result<BTreeMap<String, usize>> {
    require_checkout(ledger, checkout)?;
    let mut counts = BTreeMap::new();
    for (path, entry) in &ledger.files {
        let Some(commit) = &entry.migrated_at else {
            continue;
        };
        let place = locate(ledger, checkout, path);
        if is_commit_hash(commit) && *commit != place.pinned {
            let commits = place
                .git
                .commits_touching(commit, &place.pinned, &place.relative)?;
            counts.insert(path.clone(), commits);
        }
    }
    Ok(counts)
}

/// Where an upstream file lives: its repository (upstream's own, or a submodule's), the
/// commit pinned there, and its path inside that repository.
struct Place {
    git: Git,
    pinned: String,
    relative: String,
}

/// Resolves a ledger path. Files inside a tracked submodule are migrated against that
/// submodule's commits.
fn locate(ledger: &Ledger, checkout: &Path, path: &str) -> Place {
    for submodule in &ledger.submodules {
        if let Some(relative) = path.strip_prefix(&format!("{}/", submodule.path)) {
            return Place {
                git: Git::new(checkout.join(PathBuf::from(&submodule.path))),
                pinned: submodule.commit.clone(),
                relative: relative.to_owned(),
            };
        }
    }
    Place {
        git: Git::new(checkout),
        pinned: ledger.upstream.pinned_commit.clone(),
        relative: path.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::test_repo::{commit, init};
    use crate::ledger::{Area, FileEntry, LEDGER_FILE_NAME, Status, Submodule, Upstream};

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
        checkout: PathBuf,
        first: String,
        pin: String,
    }

    /// An upstream checkout with two commits (`first`, then the pin) and a workspace root
    /// holding a ledger with no files.
    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().expect("temp dir");
        let checkout = dir.path().join("upstream");
        init(&checkout);
        let first = commit(&checkout, &[("src/a.c", "1"), ("src/b.c", "1")], "first");
        let pin = commit(&checkout, &[("src/a.c", "2"), ("src/new.c", "n")], "pin");
        let root = dir.path().join("workspace");
        std::fs::create_dir_all(&root).expect("workspace");
        let fixture = Fixture {
            root,
            checkout,
            first,
            pin,
            _dir: dir,
        };
        ledger(&fixture, Vec::new())
            .save(&fixture.root.join(LEDGER_FILE_NAME))
            .expect("ledger saved");
        fixture
    }

    fn ledger(fixture: &Fixture, files: Vec<(&str, FileEntry)>) -> Ledger {
        Ledger {
            upstream: Upstream {
                repository: String::new(),
                pinned_commit: fixture.pin.clone(),
                tracked: vec!["src/**".to_owned()],
                checkout: Vec::new(),
            },
            submodules: Vec::new(),
            files: files
                .into_iter()
                .map(|(path, e)| (path.to_owned(), e))
                .collect(),
        }
    }

    fn migrated(commit: &str, rust: &str) -> FileEntry {
        FileEntry {
            area: Area::Core,
            status: Status::Done,
            migrated_at: Some(commit.to_owned()),
            rust: vec![rust.to_owned()],
            ..FileEntry::untriaged()
        }
    }

    #[test]
    fn sync_then_mark_edit_the_ledger_file() {
        let fixture = fixture();
        let path = fixture.root.join(LEDGER_FILE_NAME);
        let report = sync_ledger(&path, &fixture.checkout).expect("synced");
        assert_eq!(
            report.added,
            vec![
                "src/a.c".to_owned(),
                "src/b.c".to_owned(),
                "src/new.c".to_owned()
            ]
        );

        let changes = Changes {
            area: Some(Area::View),
            ..Changes::default()
        };
        let changed = mark_ledger(&path, &["src/*.c".to_owned()], &changes).expect("marked");
        assert_eq!(changed.len(), 3);
        let saved = Ledger::load(&path).expect("loaded");
        assert!(saved.files.values().all(|entry| entry.area == Area::View));
    }

    #[test]
    fn facts_describe_the_checkout_and_the_workspace() {
        let fixture = fixture();
        std::fs::create_dir_all(fixture.root.join("src/core/src")).expect("dirs");
        std::fs::write(fixture.root.join("src/core/src/a.rs"), "").expect("port file");
        let foreign = "3333333333333333333333333333333333333333";
        let ledger = ledger(
            &fixture,
            vec![
                ("src/a.c", migrated(&fixture.first, "src/core/src/a.rs")),
                ("src/b.c", migrated(foreign, "src/core/src/missing.rs")),
            ],
        );

        let facts = gather_facts(&ledger, &fixture.checkout, &fixture.root).expect("facts");

        assert_eq!(facts.upstream_files.len(), 3);
        assert_eq!(facts.foreign_commits, BTreeSet::from([foreign.to_owned()]));
        assert_eq!(
            facts.missing_paths,
            BTreeSet::from(["src/core/src/missing.rs".to_owned()])
        );
    }

    #[test]
    fn behind_counts_cover_files_migrated_before_the_pin() {
        let fixture = fixture();
        let ledger = ledger(
            &fixture,
            vec![
                ("src/a.c", migrated(&fixture.first, "x")),
                ("src/b.c", migrated(&fixture.first, "x")),
                ("src/new.c", migrated(&fixture.pin, "x")),
            ],
        );
        let counts = behind_counts(&ledger, &fixture.checkout).expect("counts");
        assert_eq!(
            counts,
            BTreeMap::from([("src/a.c".to_owned(), 1), ("src/b.c".to_owned(), 0)])
        );
    }

    #[test]
    fn submodule_files_are_located_in_their_repository() {
        let fixture = fixture();
        let mut ledger = ledger(&fixture, Vec::new());
        ledger.submodules.push(Submodule {
            path: "third_party/engine".to_owned(),
            commit: "sub".to_owned(),
        });
        let place = locate(&ledger, &fixture.checkout, "third_party/engine/rules/a.c");
        assert_eq!(place.git.dir(), fixture.checkout.join("third_party/engine"));
        assert_eq!(
            (place.pinned.as_str(), place.relative.as_str()),
            ("sub", "rules/a.c")
        );
        let place = locate(&ledger, &fixture.checkout, "src/a.c");
        assert_eq!(place.git.dir(), fixture.checkout);
        assert_eq!(place.pinned, fixture.pin);
    }
}
