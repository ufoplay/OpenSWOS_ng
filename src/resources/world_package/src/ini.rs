//! `WORLD.ini`: a world's tag, name and fingerprint, as upstream scripts/world_import.py
//! writes it (`; ` comment lines, then `key = value` lines). The game reads its fingerprint
//! (upstream src/main.c).

use std::fmt::Write as _;

use openswos_common::{Exporter, Importer};
use thiserror::Error;

use crate::fingerprint::{Fingerprint, InvalidFingerprint};

/// The key of the world's tag.
const TAG_KEY: &str = "tag";

/// The key of the world's name.
const NAME_KEY: &str = "name";

/// The key of the world's fingerprint.
const FINGERPRINT_KEY: &str = "fingerprint";

/// What starts a comment line.
const COMMENT_START: char = ';';

/// What separates a key from its value.
const ASSIGN: char = '=';

/// A world's tag: its short name in paths and settings (`2627`; World 1996-97 is `9697`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldTag(pub String);

/// A `key = value` line other than the three known ones, kept as read.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IniEntry {
    /// Its key.
    pub key: String,
    /// Its value.
    pub value: String,
}

/// Who a world is: everything `WORLD.ini` states but its fingerprint, which its files give.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorldIdentity {
    /// The comment lines, after their `;` (a leading space included), written first.
    pub comments: Vec<String>,
    /// Its tag.
    pub tag: WorldTag,
    /// Its name (`World 2026-27`).
    pub name: String,
    /// Other entries, written after the known ones.
    pub other: Vec<IniEntry>,
}

/// `WORLD.ini`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorldIni {
    /// Who the world is.
    pub identity: WorldIdentity,
    /// The fingerprint of the files it was written for.
    pub fingerprint: Fingerprint,
}

/// Why a `WORLD.ini` does not read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IniError {
    /// Not UTF-8.
    #[error("WORLD.ini is not UTF-8")]
    NotUtf8,
    /// A line is neither a comment nor `key = value`.
    #[error("WORLD.ini line {line}: neither a comment nor `key = value`")]
    Malformed {
        /// The line, from 1.
        line: usize,
    },
    /// A key is given twice.
    #[error("WORLD.ini: `{0}` is given twice")]
    Duplicate(String),
    /// A known key is missing.
    #[error("WORLD.ini has no `{0}`")]
    Missing(&'static str),
    /// The fingerprint is not one.
    #[error(transparent)]
    Fingerprint(#[from] InvalidFingerprint),
}

/// The `WORLD.ini` format. Blank lines are not kept; upstream writes none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldIniFormat;

impl Importer for WorldIniFormat {
    type Model = WorldIni;
    type Error = IniError;

    fn import(bytes: &[u8]) -> Result<WorldIni, IniError> {
        let text = std::str::from_utf8(bytes).map_err(|_| IniError::NotUtf8)?;
        let mut comments = Vec::new();
        let mut entries: Vec<IniEntry> = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                continue;
            }
            if let Some(comment) = line.strip_prefix(COMMENT_START) {
                comments.push(comment.to_owned());
                continue;
            }
            let (key, value) = line
                .split_once(ASSIGN)
                .ok_or(IniError::Malformed { line: index + 1 })?;
            let key = key.trim().to_owned();
            if entries.iter().any(|entry| entry.key == key) {
                return Err(IniError::Duplicate(key));
            }
            entries.push(IniEntry {
                key,
                value: value.trim().to_owned(),
            });
        }
        let mut take = |key: &'static str| {
            let index = entries
                .iter()
                .position(|entry| entry.key == key)
                .ok_or(IniError::Missing(key))?;
            Ok::<String, IniError>(entries.remove(index).value)
        };
        let tag = WorldTag(take(TAG_KEY)?);
        let name = take(NAME_KEY)?;
        let fingerprint = Fingerprint::try_from(take(FINGERPRINT_KEY)?)?;
        Ok(WorldIni {
            identity: WorldIdentity {
                comments,
                tag,
                name,
                other: entries,
            },
            fingerprint,
        })
    }
}

impl Exporter for WorldIniFormat {
    type Model = WorldIni;
    type Error = std::convert::Infallible;

    fn export(model: &WorldIni) -> Result<Vec<u8>, std::convert::Infallible> {
        let WorldIni {
            identity:
                WorldIdentity {
                    comments,
                    tag,
                    name,
                    other,
                },
            fingerprint,
        } = model;
        let mut text = String::new();
        for comment in comments {
            writeln!(text, "{COMMENT_START}{comment}").expect("writing to a String cannot fail");
        }
        let known = [
            (TAG_KEY, tag.0.as_str()),
            (NAME_KEY, name.as_str()),
            (FINGERPRINT_KEY, fingerprint.as_str()),
        ];
        let others = other
            .iter()
            .map(|entry| (entry.key.as_str(), entry.value.as_str()));
        for (key, value) in known.into_iter().chain(others) {
            writeln!(text, "{key} {ASSIGN} {value}").expect("writing to a String cannot fail");
        }
        Ok(text.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// World 2026-27's `WORLD.ini`, as upstream writes it.
    const WORLD_2627_INI: &str = "; A world's data for OpenSWOS -- written by scripts/world_import.py; the package\n\
        ; (scripts/make_world_package.sh) carries it, OpenSWOS Setup installs it.\n\
        tag = 2627\nname = World 2026-27\nfingerprint = efe1464b6d1d62b8\n";

    #[test]
    fn upstreams_ini_round_trips() {
        let ini = WorldIniFormat::import(WORLD_2627_INI.as_bytes()).expect("valid");
        assert_eq!(ini.identity.tag, WorldTag("2627".to_owned()));
        assert_eq!(ini.identity.name, "World 2026-27");
        assert_eq!(ini.fingerprint.as_str(), "efe1464b6d1d62b8");
        assert_eq!(ini.identity.comments.len(), 2);
        assert_eq!(
            WorldIniFormat::export(&ini).ok(),
            Some(WORLD_2627_INI.as_bytes().to_vec())
        );
    }

    #[test]
    fn other_entries_are_kept_after_the_known_ones() {
        let text = "tag = 9697\nname = World 1996-97\nfingerprint = 0123456789abcdef\nauthor = x\n";
        let ini = WorldIniFormat::import(text.as_bytes()).expect("valid");
        assert_eq!(
            ini.identity.other,
            vec![IniEntry {
                key: "author".to_owned(),
                value: "x".to_owned()
            }]
        );
        assert_eq!(
            WorldIniFormat::export(&ini).ok(),
            Some(text.as_bytes().to_vec())
        );
    }

    #[test]
    fn broken_files_are_named() {
        let cases: [(&[u8], IniError); 5] = [
            (b"tag = 1\nname = x\n", IniError::Missing(FINGERPRINT_KEY)),
            (b"tag = 1\nnonsense\n", IniError::Malformed { line: 2 }),
            (
                b"tag = 1\ntag = 2\n",
                IniError::Duplicate(TAG_KEY.to_owned()),
            ),
            (&[0xFF], IniError::NotUtf8),
            (
                b"tag = 1\nname = x\nfingerprint = nope\n",
                IniError::Fingerprint(InvalidFingerprint("nope".to_owned())),
            ),
        ];
        for (bytes, error) in cases {
            assert_eq!(WorldIniFormat::import(bytes), Err(error));
        }
        assert_eq!(
            IniError::Missing(NAME_KEY).to_string(),
            "WORLD.ini has no `name`"
        );
    }
}
