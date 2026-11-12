//! `status`: how far the migration is, against the pinned commit.

use std::collections::BTreeMap;
use std::fmt;

use crate::ledger::{Area, Ledger, Status};

/// Digits of a commit hash shown in the report.
const SHORT_HASH_LEN: usize = 12;

/// A migrated file that upstream changed after its migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Behind {
    /// The upstream file.
    pub path: String,
    /// The commit it was migrated against.
    pub migrated_at: String,
    /// The upstream commits touching it since then, up to the pin.
    pub commits: usize,
}

/// The migration's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// The pinned commit.
    pub pinned_commit: String,
    /// Entries per area and status.
    pub counts: BTreeMap<(Area, Status), usize>,
    /// Tracked upstream files the ledger lacks.
    pub not_in_ledger: usize,
    /// Migrated files upstream changed since their migration.
    pub behind: Vec<Behind>,
    /// In-scope entries still to migrate (not done/not ported at the pin), plus the
    /// untriaged ones.
    pub remaining: usize,
}

impl Summary {
    /// Whether every in-scope file is migrated at the pinned commit.
    #[must_use]
    pub fn fully_migrated(&self) -> bool {
        self.remaining == 0 && self.not_in_ledger == 0
    }
}

/// Summarises `ledger`. `behind` holds, for each migrated file whose `migrated_at` is not
/// the pin, the number of upstream commits touching it since; `not_in_ledger` counts the
/// tracked upstream files the ledger lacks.
#[must_use]
pub fn summarize(
    ledger: &Ledger,
    behind: &BTreeMap<String, usize>,
    not_in_ledger: usize,
) -> Summary {
    let pin = &ledger.upstream.pinned_commit;
    let mut counts = BTreeMap::new();
    let mut remaining = 0;
    let mut behind_files = Vec::new();
    for (path, entry) in &ledger.files {
        *counts.entry((entry.area, entry.status)).or_insert(0) += 1;
        let at_pin = entry.migrated_at.as_ref() == Some(pin);
        let finished = match entry.status {
            Status::NotPorted => true,
            Status::Done => at_pin,
            Status::Todo | Status::Partial => false,
        };
        if entry.area == Area::Untriaged || (entry.area.in_scope() && !finished) {
            remaining += 1;
        }
        if let (Some(commit), Some(&commits)) = (&entry.migrated_at, behind.get(path))
            && commits > 0
        {
            behind_files.push(Behind {
                path: path.clone(),
                migrated_at: commit.clone(),
                commits,
            });
        }
    }
    Summary {
        pinned_commit: pin.clone(),
        counts,
        not_in_ledger,
        behind: behind_files,
        remaining,
    }
}

/// Shortens a commit hash for display.
fn short(commit: &str) -> &str {
    commit.get(..SHORT_HASH_LEN).unwrap_or(commit)
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "pinned at {}", self.pinned_commit)?;
        writeln!(f)?;
        write!(f, "{:<14}", "area")?;
        for status in Status::ALL {
            write!(f, "{:>12}", status.as_str())?;
        }
        writeln!(f, "{:>8}", "total")?;
        for area in Area::ALL {
            let row: Vec<usize> = Status::ALL
                .iter()
                .map(|status| {
                    self.counts
                        .get(&(*area, *status))
                        .copied()
                        .unwrap_or_default()
                })
                .collect();
            let total: usize = row.iter().sum();
            if total == 0 {
                continue;
            }
            write!(f, "{:<14}", area.as_str())?;
            for count in &row {
                write!(f, "{count:>12}")?;
            }
            writeln!(f, "{total:>8}")?;
        }
        writeln!(f)?;
        if self.not_in_ledger > 0 {
            writeln!(
                f,
                "{} upstream files are not in the ledger: run `just upstream_sync`",
                self.not_in_ledger
            )?;
        }
        if self.behind.is_empty() {
            writeln!(f, "no migrated file is behind the pin")?;
        } else {
            writeln!(f, "migrated files upstream changed since:")?;
            for behind in &self.behind {
                writeln!(
                    f,
                    "  {} (migrated at {}, {} commits since)",
                    behind.path,
                    short(&behind.migrated_at),
                    behind.commits
                )?;
            }
        }
        if self.fully_migrated() {
            writeln!(f, "fully migrated at {}", self.pinned_commit)
        } else {
            writeln!(
                f,
                "not fully migrated at {}: {} in-scope files to go",
                short(&self.pinned_commit),
                self.remaining
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{FileEntry, Upstream};

    const PIN: &str = "ec2ed2be5c26e510f7c7b8d7af2710e5b909ca8b";
    const OLDER: &str = "1111111111111111111111111111111111111111";

    fn entry(area: Area, status: Status, migrated_at: Option<&str>) -> FileEntry {
        FileEntry {
            area,
            status,
            migrated_at: migrated_at.map(str::to_owned),
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
            submodules: Vec::new(),
            files: files
                .into_iter()
                .map(|(path, e)| (path.to_owned(), e))
                .collect(),
        }
    }

    #[test]
    fn counts_entries_per_area_and_status() {
        let ledger = ledger(vec![
            ("a.c", entry(Area::Core, Status::Todo, None)),
            ("b.c", entry(Area::Core, Status::Todo, None)),
            ("c.c", entry(Area::View, Status::Todo, None)),
        ]);
        let summary = summarize(&ledger, &BTreeMap::new(), 0);
        assert_eq!(summary.counts.get(&(Area::Core, Status::Todo)), Some(&2));
        assert_eq!(summary.counts.get(&(Area::View, Status::Todo)), Some(&1));
        assert_eq!(summary.remaining, 2);
    }

    #[test]
    fn done_at_the_pin_and_not_ported_are_finished() {
        let ledger = ledger(vec![
            ("a.c", entry(Area::Core, Status::Done, Some(PIN))),
            ("b.c", entry(Area::Resource, Status::NotPorted, None)),
            ("c.c", entry(Area::OutOfScope, Status::NotPorted, None)),
            ("d.c", entry(Area::View, Status::Todo, None)),
        ]);
        let summary = summarize(&ledger, &BTreeMap::new(), 0);
        assert!(summary.fully_migrated(), "{summary}");
        assert!(summary.to_string().contains("fully migrated at"));
    }

    #[test]
    fn done_at_an_older_commit_is_not_finished() {
        let ledger = ledger(vec![("a.c", entry(Area::Core, Status::Done, Some(OLDER)))]);
        let behind = BTreeMap::from([("a.c".to_owned(), 3)]);
        let summary = summarize(&ledger, &behind, 0);
        assert!(!summary.fully_migrated());
        assert_eq!(
            summary.behind,
            vec![Behind {
                path: "a.c".to_owned(),
                migrated_at: OLDER.to_owned(),
                commits: 3,
            }]
        );
        assert!(summary.to_string().contains("3 commits since"));
    }

    #[test]
    fn an_older_commit_untouched_since_is_not_listed_as_behind() {
        let ledger = ledger(vec![("a.c", entry(Area::Core, Status::Done, Some(OLDER)))]);
        let behind = BTreeMap::from([("a.c".to_owned(), 0)]);
        assert!(summarize(&ledger, &behind, 0).behind.is_empty());
    }

    #[test]
    fn untriaged_and_unlisted_files_block_completion() {
        let untriaged = ledger(vec![("a.c", FileEntry::untriaged())]);
        assert!(!summarize(&untriaged, &BTreeMap::new(), 0).fully_migrated());
        let empty = ledger(Vec::new());
        let summary = summarize(&empty, &BTreeMap::new(), 2);
        assert!(!summary.fully_migrated());
        assert!(
            summary
                .to_string()
                .contains("2 upstream files are not in the ledger")
        );
    }

    #[test]
    fn hashes_are_shortened_for_display() {
        assert_eq!(short(PIN), "ec2ed2be5c26");
        assert_eq!(short("abc"), "abc");
    }
}
