//! `mark`: update the entries that match glob patterns.

use std::collections::BTreeMap;

use crate::error::{Result, UpstreamError};
use crate::ledger::{Area, FileEntry, Status, is_commit_hash};
use crate::tracked::glob_set;

/// The changes `mark` applies to every matching entry. `None` and empty lists leave the
/// field as it is.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Changes {
    /// The new area.
    pub area: Option<Area>,
    /// The new status. `partial` and `done` without a `migrated_at` record the pin.
    pub status: Option<Status>,
    /// The commit the file was migrated against.
    pub migrated_at: Option<String>,
    /// The repository files holding the port (replaces the list).
    pub rust: Vec<String>,
    /// Functions migrated so far (added to the list).
    pub functions_done: Vec<String>,
    /// The new notes; an empty string clears them.
    pub notes: Option<String>,
}

/// Applies `changes` to every entry whose path matches one of `patterns`, and returns
/// the paths changed.
///
/// # Errors
///
/// Fails when a pattern does not parse or matches no entry, or `migrated_at` is not a
/// full commit hash. Nothing is changed then.
pub fn mark(
    files: &mut BTreeMap<String, FileEntry>,
    patterns: &[String],
    changes: &Changes,
    pinned_commit: &str,
) -> Result<Vec<String>> {
    if let Some(commit) = &changes.migrated_at
        && !is_commit_hash(commit)
    {
        return Err(UpstreamError::InvalidCommit {
            value: commit.clone(),
        });
    }
    for pattern in patterns {
        let set = glob_set(std::slice::from_ref(pattern))?;
        if !files.keys().any(|path| set.is_match(path)) {
            return Err(UpstreamError::NoMatch {
                pattern: pattern.clone(),
            });
        }
    }
    let set = glob_set(patterns)?;
    let mut changed = Vec::new();
    for (path, entry) in files.iter_mut().filter(|(path, _)| set.is_match(path)) {
        apply(entry, changes, pinned_commit);
        changed.push(path.clone());
    }
    Ok(changed)
}

fn apply(entry: &mut FileEntry, changes: &Changes, pinned_commit: &str) {
    let Changes {
        area,
        status,
        migrated_at,
        rust,
        functions_done,
        notes,
    } = changes;
    if let Some(area) = area {
        entry.area = *area;
    }
    if let Some(status) = status {
        entry.status = *status;
    }
    if let Some(commit) = migrated_at {
        entry.migrated_at = Some(commit.clone());
    } else if entry.status.has_migration() && entry.migrated_at.is_none() {
        entry.migrated_at = Some(pinned_commit.to_owned());
    }
    if !rust.is_empty() {
        entry.rust.clone_from(rust);
    }
    for function in functions_done {
        if !entry.functions_done.contains(function) {
            entry.functions_done.push(function.clone());
        }
    }
    if let Some(text) = notes {
        entry.notes = (!text.is_empty()).then(|| text.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "ec2ed2be5c26e510f7c7b8d7af2710e5b909ca8b";
    const OLDER: &str = "1111111111111111111111111111111111111111";

    fn ledger_files() -> BTreeMap<String, FileEntry> {
        [
            "src/netplay.c",
            "src/netplay_lobby.c",
            "src/random.c",
            "include/netplay.h",
        ]
        .into_iter()
        .map(|path| (path.to_owned(), FileEntry::untriaged()))
        .collect()
    }

    fn patterns(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn sets_area_and_status_on_every_match() {
        let mut files = ledger_files();
        let changes = Changes {
            area: Some(Area::OutOfScope),
            status: Some(Status::NotPorted),
            ..Changes::default()
        };
        let changed = mark(
            &mut files,
            &patterns(&["src/netplay*.c", "include/netplay*.h"]),
            &changes,
            PIN,
        )
        .expect("marked");
        assert_eq!(changed.len(), 3);
        let random = files.get("src/random.c").expect("present");
        assert_eq!(random.area, Area::Untriaged);
        let lobby = files.get("src/netplay_lobby.c").expect("present");
        assert_eq!(
            (lobby.area, lobby.status),
            (Area::OutOfScope, Status::NotPorted)
        );
        assert_eq!(lobby.migrated_at, None);
    }

    #[test]
    fn done_without_a_commit_records_the_pin() {
        let mut files = ledger_files();
        let changes = Changes {
            status: Some(Status::Done),
            rust: patterns(&["src/core/src/rng.rs"]),
            ..Changes::default()
        };
        mark(&mut files, &patterns(&["src/random.c"]), &changes, PIN).expect("marked");
        let entry = files.get("src/random.c").expect("present");
        assert_eq!(entry.migrated_at.as_deref(), Some(PIN));
        assert_eq!(entry.rust, patterns(&["src/core/src/rng.rs"]));
    }

    #[test]
    fn an_explicit_commit_wins_over_the_pin() {
        let mut files = ledger_files();
        let changes = Changes {
            status: Some(Status::Partial),
            migrated_at: Some(OLDER.to_owned()),
            functions_done: patterns(&["Rand", "Rand"]),
            ..Changes::default()
        };
        mark(&mut files, &patterns(&["src/random.c"]), &changes, PIN).expect("marked");
        let entry = files.get("src/random.c").expect("present");
        assert_eq!(entry.migrated_at.as_deref(), Some(OLDER));
        assert_eq!(entry.functions_done, patterns(&["Rand"]));
    }

    #[test]
    fn a_recorded_commit_is_kept_when_none_is_given() {
        let mut files = ledger_files();
        let first = Changes {
            status: Some(Status::Partial),
            migrated_at: Some(OLDER.to_owned()),
            ..Changes::default()
        };
        mark(&mut files, &patterns(&["src/random.c"]), &first, PIN).expect("marked");
        let second = Changes {
            notes: Some("halfway".to_owned()),
            ..Changes::default()
        };
        mark(&mut files, &patterns(&["src/random.c"]), &second, PIN).expect("marked");
        let entry = files.get("src/random.c").expect("present");
        assert_eq!(entry.migrated_at.as_deref(), Some(OLDER));
        assert_eq!(entry.notes.as_deref(), Some("halfway"));
    }

    #[test]
    fn an_empty_note_clears_the_notes() {
        let mut files = ledger_files();
        let set = Changes {
            notes: Some("x".to_owned()),
            ..Changes::default()
        };
        mark(&mut files, &patterns(&["src/random.c"]), &set, PIN).expect("marked");
        let clear = Changes {
            notes: Some(String::new()),
            ..Changes::default()
        };
        mark(&mut files, &patterns(&["src/random.c"]), &clear, PIN).expect("marked");
        assert_eq!(files.get("src/random.c").expect("present").notes, None);
    }

    #[test]
    fn a_pattern_matching_nothing_fails_and_changes_nothing() {
        let mut files = ledger_files();
        let changes = Changes {
            area: Some(Area::Core),
            ..Changes::default()
        };
        let error = mark(
            &mut files,
            &patterns(&["src/random.c", "src/nope*.c"]),
            &changes,
            PIN,
        )
        .expect_err("no match");
        assert!(matches!(error, UpstreamError::NoMatch { .. }), "{error}");
        assert_eq!(files, ledger_files());
    }

    #[test]
    fn an_abbreviated_commit_is_rejected() {
        let mut files = ledger_files();
        let changes = Changes {
            migrated_at: Some("ec2ed2b".to_owned()),
            ..Changes::default()
        };
        let error =
            mark(&mut files, &patterns(&["src/random.c"]), &changes, PIN).expect_err("short hash");
        assert!(
            matches!(error, UpstreamError::InvalidCommit { .. }),
            "{error}"
        );
    }
}
