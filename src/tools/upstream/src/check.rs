//! `check`: the CI gate. The ledger must cover exactly the tracked upstream files, and
//! every entry must be consistent.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ledger::{Area, Ledger, Status, is_commit_hash};

/// What `check` needs to know beyond the ledger, gathered from git and the file system.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The tracked upstream files at the pinned commit.
    pub upstream_files: BTreeSet<String>,
    /// For each ledger submodule: the commit upstream pins it to at the pinned commit
    /// (`None`: there is no submodule at that path).
    pub upstream_gitlinks: BTreeMap<String, Option<String>>,
    /// The `migrated_at` commits that are not the pinned commit or one of its ancestors
    /// (including commits git does not know).
    pub foreign_commits: BTreeSet<String>,
    /// The `rust` paths that do not exist in this repository.
    pub missing_paths: BTreeSet<String>,
}

/// One inconsistency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The pinned commit is not a full commit hash.
    InvalidPin(String),
    /// A tracked upstream file has no entry.
    NotInLedger(String),
    /// An entry names a file upstream does not have at the pin.
    NotUpstream(String),
    /// An entry is not classified.
    Untriaged(String),
    /// An out-of-scope entry is not `not_ported`.
    OutOfScopePorted {
        /// The upstream file.
        path: String,
        /// Its status.
        status: Status,
    },
    /// A `partial`/`done` entry has no `migrated_at`.
    MissingMigratedAt(String),
    /// A `partial`/`done` entry lists no files of this repository.
    MissingPort(String),
    /// `migrated_at` is not a full commit hash.
    InvalidCommit {
        /// The upstream file.
        path: String,
        /// The value.
        commit: String,
    },
    /// `migrated_at` is not the pinned commit or one of its ancestors.
    ForeignCommit {
        /// The upstream file.
        path: String,
        /// The commit.
        commit: String,
    },
    /// A `rust` path does not exist.
    MissingPath {
        /// The upstream file.
        path: String,
        /// The missing repository path.
        missing: String,
    },
    /// A submodule's recorded commit differs from the one upstream pins.
    SubmoduleMismatch {
        /// The submodule path.
        path: String,
        /// The commit the ledger records.
        ledger: String,
        /// The commit upstream pins, if any.
        upstream: Option<String>,
    },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPin(pin) => write!(f, "pinned_commit `{pin}` is not a full commit hash"),
            Self::NotInLedger(path) => write!(
                f,
                "{path}: upstream file missing from the ledger (run `just upstream_sync`)"
            ),
            Self::NotUpstream(path) => {
                write!(f, "{path}: not an upstream file at the pinned commit")
            }
            Self::Untriaged(path) => write!(
                f,
                "{path}: untriaged (give it an area with `just upstream_mark`)"
            ),
            Self::OutOfScopePorted { path, status } => {
                write!(
                    f,
                    "{path}: out_of_scope but `{status}` (must be `not_ported`)"
                )
            }
            Self::MissingMigratedAt(path) => write!(f, "{path}: partial/done without migrated_at"),
            Self::MissingPort(path) => write!(
                f,
                "{path}: partial/done without the repository files holding the port (rust)"
            ),
            Self::InvalidCommit { path, commit } => {
                write!(
                    f,
                    "{path}: migrated_at `{commit}` is not a full commit hash"
                )
            }
            Self::ForeignCommit { path, commit } => {
                write!(
                    f,
                    "{path}: migrated_at {commit} is not the pinned commit or an ancestor of it"
                )
            }
            Self::MissingPath { path, missing } => write!(f, "{path}: {missing} does not exist"),
            Self::SubmoduleMismatch {
                path,
                ledger,
                upstream,
            } => match upstream {
                Some(commit) => write!(
                    f,
                    "submodule {path}: the ledger says {ledger}, upstream pins {commit}"
                ),
                None => write!(
                    f,
                    "submodule {path}: upstream has no submodule there at the pinned commit"
                ),
            },
        }
    }
}

/// Every inconsistency of `ledger`, given the `facts` around it.
#[must_use]
pub fn check(ledger: &Ledger, facts: &Facts) -> Vec<Problem> {
    let mut problems = Vec::new();
    if !is_commit_hash(&ledger.upstream.pinned_commit) {
        problems.push(Problem::InvalidPin(ledger.upstream.pinned_commit.clone()));
    }
    for submodule in &ledger.submodules {
        let upstream = facts
            .upstream_gitlinks
            .get(&submodule.path)
            .cloned()
            .flatten();
        if upstream.as_deref() != Some(submodule.commit.as_str()) {
            problems.push(Problem::SubmoduleMismatch {
                path: submodule.path.clone(),
                ledger: submodule.commit.clone(),
                upstream,
            });
        }
    }
    problems.extend(
        facts
            .upstream_files
            .iter()
            .filter(|path| !ledger.files.contains_key(*path))
            .map(|path| Problem::NotInLedger(path.clone())),
    );
    for (path, entry) in &ledger.files {
        if !facts.upstream_files.contains(path) {
            problems.push(Problem::NotUpstream(path.clone()));
        }
        match entry.area {
            Area::Untriaged => problems.push(Problem::Untriaged(path.clone())),
            Area::OutOfScope if entry.status != Status::NotPorted => {
                problems.push(Problem::OutOfScopePorted {
                    path: path.clone(),
                    status: entry.status,
                });
            }
            Area::OutOfScope
            | Area::Core
            | Area::Resource
            | Area::WorldData
            | Area::TestData
            | Area::View => {}
        }
        if entry.status.has_migration() {
            if entry.migrated_at.is_none() {
                problems.push(Problem::MissingMigratedAt(path.clone()));
            }
            if entry.rust.is_empty() {
                problems.push(Problem::MissingPort(path.clone()));
            }
        }
        if let Some(commit) = &entry.migrated_at {
            if !is_commit_hash(commit) {
                problems.push(Problem::InvalidCommit {
                    path: path.clone(),
                    commit: commit.clone(),
                });
            } else if facts.foreign_commits.contains(commit) {
                problems.push(Problem::ForeignCommit {
                    path: path.clone(),
                    commit: commit.clone(),
                });
            }
        }
        problems.extend(
            entry
                .rust
                .iter()
                .filter(|missing| facts.missing_paths.contains(*missing))
                .map(|missing| Problem::MissingPath {
                    path: path.clone(),
                    missing: missing.clone(),
                }),
        );
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{FileEntry, Submodule, Upstream};

    const PIN: &str = "ec2ed2be5c26e510f7c7b8d7af2710e5b909ca8b";
    const SUB: &str = "87b65278a15007223ec4f387f1027eb4d5cd7625";
    const OTHER: &str = "2222222222222222222222222222222222222222";

    fn entry(area: Area, status: Status) -> FileEntry {
        FileEntry {
            area,
            status,
            ..FileEntry::untriaged()
        }
    }

    fn ledger(files: Vec<(&str, FileEntry)>) -> Ledger {
        Ledger {
            upstream: Upstream {
                repository: String::new(),
                pinned_commit: PIN.to_owned(),
                tracked: Vec::new(),
                checkout: Vec::new(),
            },
            submodules: vec![Submodule {
                path: "third_party/engine".to_owned(),
                commit: SUB.to_owned(),
            }],
            files: files
                .into_iter()
                .map(|(path, e)| (path.to_owned(), e))
                .collect(),
        }
    }

    fn facts(upstream_files: &[&str]) -> Facts {
        Facts {
            upstream_files: upstream_files
                .iter()
                .map(|path| (*path).to_owned())
                .collect(),
            upstream_gitlinks: BTreeMap::from([(
                "third_party/engine".to_owned(),
                Some(SUB.to_owned()),
            )]),
            ..Facts::default()
        }
    }

    fn done(rust: &str) -> FileEntry {
        FileEntry {
            migrated_at: Some(PIN.to_owned()),
            rust: vec![rust.to_owned()],
            ..entry(Area::Core, Status::Done)
        }
    }

    #[test]
    fn a_consistent_ledger_passes() {
        let ledger = ledger(vec![
            ("a.c", done("src/core/src/a.rs")),
            ("b.c", entry(Area::OutOfScope, Status::NotPorted)),
            ("c.c", entry(Area::View, Status::Todo)),
        ]);
        assert_eq!(check(&ledger, &facts(&["a.c", "b.c", "c.c"])), Vec::new());
    }

    #[test]
    fn the_file_lists_must_agree() {
        let ledger = ledger(vec![("gone.c", entry(Area::Core, Status::Todo))]);
        assert_eq!(
            check(&ledger, &facts(&["new.c"])),
            vec![
                Problem::NotInLedger("new.c".to_owned()),
                Problem::NotUpstream("gone.c".to_owned()),
            ]
        );
    }

    #[test]
    fn untriaged_entries_fail() {
        let ledger = ledger(vec![("a.c", FileEntry::untriaged())]);
        assert_eq!(
            check(&ledger, &facts(&["a.c"])),
            vec![Problem::Untriaged("a.c".to_owned())]
        );
    }

    #[test]
    fn out_of_scope_entries_must_be_not_ported() {
        let ledger = ledger(vec![("a.c", entry(Area::OutOfScope, Status::Todo))]);
        assert_eq!(
            check(&ledger, &facts(&["a.c"])),
            vec![Problem::OutOfScopePorted {
                path: "a.c".to_owned(),
                status: Status::Todo,
            }]
        );
    }

    #[test]
    fn migrations_need_a_commit_and_a_port() {
        let ledger = ledger(vec![("a.c", entry(Area::Core, Status::Partial))]);
        assert_eq!(
            check(&ledger, &facts(&["a.c"])),
            vec![
                Problem::MissingMigratedAt("a.c".to_owned()),
                Problem::MissingPort("a.c".to_owned()),
            ]
        );
    }

    #[test]
    fn commits_must_be_full_hashes_on_the_pinned_history() {
        let short = FileEntry {
            migrated_at: Some("ec2ed2b".to_owned()),
            ..done("src/core/src/a.rs")
        };
        let foreign = FileEntry {
            migrated_at: Some(OTHER.to_owned()),
            ..done("src/core/src/b.rs")
        };
        let ledger = ledger(vec![("a.c", short), ("b.c", foreign)]);
        let facts = Facts {
            foreign_commits: BTreeSet::from([OTHER.to_owned()]),
            ..facts(&["a.c", "b.c"])
        };
        assert_eq!(
            check(&ledger, &facts),
            vec![
                Problem::InvalidCommit {
                    path: "a.c".to_owned(),
                    commit: "ec2ed2b".to_owned(),
                },
                Problem::ForeignCommit {
                    path: "b.c".to_owned(),
                    commit: OTHER.to_owned(),
                },
            ]
        );
    }

    #[test]
    fn port_paths_must_exist() {
        let ledger = ledger(vec![("a.c", done("src/core/src/gone.rs"))]);
        let facts = Facts {
            missing_paths: BTreeSet::from(["src/core/src/gone.rs".to_owned()]),
            ..facts(&["a.c"])
        };
        assert_eq!(
            check(&ledger, &facts),
            vec![Problem::MissingPath {
                path: "a.c".to_owned(),
                missing: "src/core/src/gone.rs".to_owned(),
            }]
        );
    }

    #[test]
    fn submodules_must_match_upstream() {
        let ledger = ledger(Vec::new());
        let moved = Facts {
            upstream_gitlinks: BTreeMap::from([(
                "third_party/engine".to_owned(),
                Some(OTHER.to_owned()),
            )]),
            ..Facts::default()
        };
        assert_eq!(
            check(&ledger, &moved),
            vec![Problem::SubmoduleMismatch {
                path: "third_party/engine".to_owned(),
                ledger: SUB.to_owned(),
                upstream: Some(OTHER.to_owned()),
            }]
        );
        let gone = Facts::default();
        assert_eq!(
            check(&ledger, &gone),
            vec![Problem::SubmoduleMismatch {
                path: "third_party/engine".to_owned(),
                ledger: SUB.to_owned(),
                upstream: None,
            }]
        );
    }

    #[test]
    fn the_pin_must_be_a_full_hash() {
        let mut ledger = ledger(Vec::new());
        ledger.upstream.pinned_commit = "main".to_owned();
        assert_eq!(
            check(&ledger, &facts(&[])),
            vec![Problem::InvalidPin("main".to_owned())]
        );
    }

    #[test]
    fn problems_explain_themselves() {
        let text = Problem::Untriaged("src/a.c".to_owned()).to_string();
        assert!(
            text.contains("src/a.c") && text.contains("upstream_mark"),
            "{text}"
        );
        let text = Problem::SubmoduleMismatch {
            path: "p".to_owned(),
            ledger: SUB.to_owned(),
            upstream: None,
        }
        .to_string();
        assert!(text.contains("no submodule"), "{text}");
    }
}
