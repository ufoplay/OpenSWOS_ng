//! How a package's files differ from the ones kept for it (docs/PLAN.md M4 #5e): the committed
//! World 1996-97 against its extraction again, World 2026-27's completion against its committed
//! digests. At an upstream bump the extraction runs again, and what changed is named file by
//! file.

use std::collections::BTreeMap;
use std::fmt;

use openswos_world_package_resource::PackageFiles;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Each file's SHA-256 (lower-case hex), by its path: what a package holds, without its data.
/// World 2026-27's data may not be committed; its digests may.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Digests(pub BTreeMap<String, String>);

impl Digests {
    /// The digests of `files`.
    #[must_use]
    pub fn of(files: &PackageFiles) -> Self {
        Self(
            files
                .iter()
                .map(|(path, bytes)| {
                    let hex: String = Sha256::digest(bytes)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect();
                    (path.to_owned(), hex)
                })
                .collect(),
        )
    }

    /// The digests as their file holds them: JSON, two spaces, a trailing newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a map of strings encodes");
        text.push('\n');
        text
    }

    /// The digests their file `text` holds.
    ///
    /// # Errors
    ///
    /// When `text` is not a JSON object of strings.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

/// A file kept with other bytes than the fresh one's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    /// Its path.
    pub path: String,
    /// The first line that differs (from 1), when both are text and a line does.
    pub line: Option<usize>,
}

/// How the files kept differ from the fresh ones.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilesDiff {
    /// In both, with other bytes.
    pub changed: Vec<Changed>,
    /// Only among the fresh ones.
    pub new: Vec<String>,
    /// Only among the kept ones.
    pub gone: Vec<String>,
}

impl FilesDiff {
    /// How the files `kept` differ from the `fresh` ones, a changed text file with its first
    /// line that differs.
    #[must_use]
    pub fn between(kept: &PackageFiles, fresh: &PackageFiles) -> Self {
        let kept: BTreeMap<&str, &[u8]> = kept.iter().collect();
        let fresh: BTreeMap<&str, &[u8]> = fresh.iter().collect();
        compare(&kept, &fresh, |kept, fresh| first_line(kept, fresh))
    }

    /// How the files whose digests are `kept` differ from those whose digests are `fresh`.
    #[must_use]
    pub fn between_digests(kept: &Digests, fresh: &Digests) -> Self {
        let kept: BTreeMap<&str, &String> = kept
            .0
            .iter()
            .map(|(path, digest)| (path.as_str(), digest))
            .collect();
        let fresh: BTreeMap<&str, &String> = fresh
            .0
            .iter()
            .map(|(path, digest)| (path.as_str(), digest))
            .collect();
        compare(&kept, &fresh, |_, _| None)
    }

    /// Whether the files are the same.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.new.is_empty() && self.gone.is_empty()
    }
}

impl fmt::Display for FilesDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("the same files");
        }
        let changed: Vec<String> = self
            .changed
            .iter()
            .map(|changed| match changed.line {
                Some(line) => format!("{} (line {line})", changed.path),
                None => changed.path.clone(),
            })
            .collect();
        let parts: Vec<String> = [
            ("changed", changed),
            ("new", self.new.clone()),
            ("gone", self.gone.clone()),
        ]
        .into_iter()
        .filter(|(_, paths)| !paths.is_empty())
        .map(|(what, paths)| format!("{what}: {}", paths.join(", ")))
        .collect();
        f.write_str(&parts.join("; "))
    }
}

/// How `kept` differs from `fresh`, a changed entry's line given by `line`.
fn compare<V: PartialEq>(
    kept: &BTreeMap<&str, V>,
    fresh: &BTreeMap<&str, V>,
    line: impl Fn(&V, &V) -> Option<usize>,
) -> FilesDiff {
    let changed = kept
        .iter()
        .filter_map(|(path, old)| {
            let new = fresh.get(path)?;
            (old != new).then(|| Changed {
                path: (*path).to_owned(),
                line: line(old, new),
            })
        })
        .collect();
    let only = |of: &BTreeMap<&str, V>, not_in: &BTreeMap<&str, V>| -> Vec<String> {
        of.keys()
            .filter(|path| !not_in.contains_key(*path))
            .map(|path| (*path).to_owned())
            .collect()
    };
    FilesDiff {
        changed,
        new: only(fresh, kept),
        gone: only(kept, fresh),
    }
}

/// The first line (from 1) where the texts `kept` and `fresh` differ: none when either is not
/// text, or their lines are the same.
fn first_line(kept: &[u8], fresh: &[u8]) -> Option<usize> {
    let kept: Vec<&str> = std::str::from_utf8(kept).ok()?.lines().collect();
    let fresh: Vec<&str> = std::str::from_utf8(fresh).ok()?.lines().collect();
    let index = kept
        .iter()
        .zip(&fresh)
        .position(|(old, new)| old != new)
        .or_else(|| (kept.len() != fresh.len()).then(|| kept.len().min(fresh.len())))?;
    Some(index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(entries: &[(&str, &[u8])]) -> PackageFiles {
        let mut files = PackageFiles::default();
        for (path, bytes) in entries {
            files.insert(path, bytes.to_vec()).expect("a file");
        }
        files
    }

    #[test]
    fn a_digest_is_a_file_s_sha_256() {
        let digests = Digests::of(&files(&[("WORLD.ini", b"abc")]));
        assert_eq!(
            digests.0.get("WORLD.ini").map(String::as_str),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
    }

    #[test]
    fn digests_write_and_read_back() {
        let digests = Digests::of(&files(&[("a.json", b"1"), ("rules/b.json", b"2")]));
        let text = digests.to_json();
        assert!(text.starts_with("{\n  \"a.json\": \""), "{text}");
        assert!(text.ends_with("}\n"));
        assert_eq!(Digests::from_json(&text).ok(), Some(digests));
        assert!(Digests::from_json("[1]").is_err());
    }

    #[test]
    fn files_differ_by_name_and_by_line() {
        let kept = files(&[
            ("a.json", b"{\n  \"x\": 1\n}\n"),
            ("b.dat", &[0xFF, 0]),
            ("c.json", b"1\n2\n"),
            ("gone.json", b"{}"),
        ]);
        let fresh = files(&[
            ("a.json", b"{\n  \"x\": 2\n}\n"),
            ("b.dat", &[0xFF, 1]),
            ("c.json", b"1\n2\n3\n"),
            ("new.json", b"{}"),
        ]);
        let diff = FilesDiff::between(&kept, &fresh);
        assert_eq!(
            diff.changed,
            [
                Changed {
                    path: "a.json".to_owned(),
                    line: Some(2)
                },
                Changed {
                    path: "b.dat".to_owned(),
                    line: None
                },
                Changed {
                    path: "c.json".to_owned(),
                    line: Some(3)
                },
            ]
        );
        assert_eq!(diff.new, ["new.json"]);
        assert_eq!(diff.gone, ["gone.json"]);
        assert_eq!(
            diff.to_string(),
            "changed: a.json (line 2), b.dat, c.json (line 3); new: new.json; gone: gone.json"
        );
        assert!(!diff.is_empty());
        let same = FilesDiff::between(&kept, &kept);
        assert!(same.is_empty());
        assert_eq!(same.to_string(), "the same files");
    }

    #[test]
    fn digests_differ_by_name_only() {
        let kept = Digests::of(&files(&[("a.json", b"1"), ("gone.json", b"")]));
        let fresh = Digests::of(&files(&[("a.json", b"2")]));
        let diff = FilesDiff::between_digests(&kept, &fresh);
        assert_eq!(diff.to_string(), "changed: a.json; gone: gone.json");
        assert_eq!(first_line(b"x", b"x\r\n"), None, "the same lines");
    }
}
