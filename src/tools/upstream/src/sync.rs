//! `sync`: bring the ledger's file list in line with upstream at the pinned commit.

use std::collections::{BTreeMap, BTreeSet};

use crate::ledger::FileEntry;

/// What `sync` changed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncReport {
    /// Upstream files new to the ledger, added as untriaged.
    pub added: Vec<String>,
    /// Entries of files upstream no longer has, removed because nothing was ported.
    pub dropped: Vec<String>,
    /// Entries of files upstream no longer has, kept because they record ported work:
    /// the port has to follow the removal, then the entry is removed by hand.
    pub orphaned: Vec<String>,
}

/// Adds every upstream file the ledger lacks, and removes the entries of files upstream
/// no longer has, unless they record ported work.
pub fn sync(files: &mut BTreeMap<String, FileEntry>, upstream: &BTreeSet<String>) -> SyncReport {
    let mut report = SyncReport::default();
    for path in upstream {
        if !files.contains_key(path) {
            files.insert(path.clone(), FileEntry::untriaged());
            report.added.push(path.clone());
        }
    }
    let gone: Vec<String> = files
        .keys()
        .filter(|path| !upstream.contains(*path))
        .cloned()
        .collect();
    for path in gone {
        let holds_work = files
            .get(&path)
            .is_some_and(|entry| entry.status.has_migration() || !entry.rust.is_empty());
        if holds_work {
            report.orphaned.push(path);
        } else {
            files.remove(&path);
            report.dropped.push(path);
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{Area, Status};

    fn entry(area: Area, status: Status) -> FileEntry {
        FileEntry {
            area,
            status,
            ..FileEntry::untriaged()
        }
    }

    #[test]
    fn adds_new_upstream_files_as_untriaged() {
        let mut files = BTreeMap::from([("a.c".to_owned(), entry(Area::Core, Status::Todo))]);
        let upstream = BTreeSet::from(["a.c".to_owned(), "b.c".to_owned()]);
        let report = sync(&mut files, &upstream);
        assert_eq!(report.added, vec!["b.c".to_owned()]);
        assert_eq!(files.get("b.c"), Some(&FileEntry::untriaged()));
        assert_eq!(files.get("a.c"), Some(&entry(Area::Core, Status::Todo)));
    }

    #[test]
    fn drops_removed_files_without_ported_work() {
        let mut files = BTreeMap::from([
            ("todo.c".to_owned(), entry(Area::Core, Status::Todo)),
            (
                "skip.c".to_owned(),
                entry(Area::OutOfScope, Status::NotPorted),
            ),
        ]);
        let report = sync(&mut files, &BTreeSet::new());
        assert_eq!(
            report.dropped,
            vec!["skip.c".to_owned(), "todo.c".to_owned()]
        );
        assert!(files.is_empty());
    }

    #[test]
    fn keeps_removed_files_with_ported_work() {
        let mut files = BTreeMap::from([
            ("done.c".to_owned(), entry(Area::Core, Status::Done)),
            ("partial.c".to_owned(), entry(Area::Core, Status::Partial)),
            (
                "linked.c".to_owned(),
                FileEntry {
                    rust: vec!["src/core/src/x.rs".to_owned()],
                    ..entry(Area::Core, Status::Todo)
                },
            ),
        ]);
        let report = sync(&mut files, &BTreeSet::new());
        assert_eq!(report.orphaned.len(), 3);
        assert!(report.dropped.is_empty());
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn a_synced_ledger_is_left_alone() {
        let mut files = BTreeMap::from([("a.c".to_owned(), entry(Area::Core, Status::Todo))]);
        let report = sync(&mut files, &BTreeSet::from(["a.c".to_owned()]));
        assert_eq!(report, SyncReport::default());
    }
}
