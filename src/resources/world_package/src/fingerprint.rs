//! A world's fingerprint (upstream scripts/world_import.py `world_fingerprint`): the SHA-1
//! of every file of `teams/`, `competitions/` and `data/`, folder by folder and by name,
//! each as its path, a NUL and its bytes; the first 16 hex digits. `WORLD.ini` states it,
//! the game is built with one (upstream `World_DataFingerprint`): a package and a build of
//! other data are told apart.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use thiserror::Error;

use crate::files::PackageFiles;

/// Hex digits of a fingerprint: the first 8 bytes of a SHA-1.
pub const FINGERPRINT_HEX_LEN: usize = 16;

/// The folders the fingerprint covers, in its order. Faithful to the original: `data/`, the
/// Amiga's team files, is in it although this port does not read it (D9).
pub const FINGERPRINTED_FOLDERS: [&str; 3] = ["teams", "competitions", "data"];

/// The separator between a fingerprinted file's path and its bytes.
const PATH_END: u8 = 0;

/// A world's fingerprint: 16 lower-case hex digits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Fingerprint(String);

/// A text that is not a fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("`{0}` is not a world fingerprint (16 lower-case hex digits)")]
pub struct InvalidFingerprint(pub String);

impl Fingerprint {
    /// The fingerprint of `files`: those directly in the fingerprinted folders, any name.
    #[must_use]
    pub fn of(files: &PackageFiles) -> Self {
        let mut hasher = Sha1::new();
        for folder in FINGERPRINTED_FOLDERS {
            for (path, bytes) in files.in_folder(folder) {
                hasher.update(path.as_bytes());
                hasher.update([PATH_END]);
                hasher.update(bytes);
            }
        }
        let hex: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Self(hex.chars().take(FINGERPRINT_HEX_LEN).collect())
    }

    /// The hex digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Fingerprint {
    type Error = InvalidFingerprint;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let valid = value.len() == FINGERPRINT_HEX_LEN
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidFingerprint(value))
        }
    }
}

impl From<Fingerprint> for String {
    fn from(fingerprint: Fingerprint) -> Self {
        fingerprint.0
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A package's stated fingerprint against the one its files give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintCheck {
    /// What `WORLD.ini` states.
    pub stated: Fingerprint,
    /// What the files give.
    pub computed: Fingerprint,
}

impl FingerprintCheck {
    /// Whether the files are the ones `WORLD.ini` was written for.
    #[must_use]
    pub fn is_match(&self) -> bool {
        self.stated == self.computed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(entries: &[(&str, &[u8])]) -> PackageFiles {
        let mut files = PackageFiles::default();
        for (path, bytes) in entries {
            files.insert(path, bytes.to_vec()).expect("a safe path");
        }
        files
    }

    /// The fingerprint upstream's `world_fingerprint` gives the same files (computed with
    /// it).
    #[test]
    fn is_upstreams() {
        let with_data = files(&[
            ("teams/B.json", b"bb"),
            ("teams/A.json", b"a"),
            ("teams/notes.txt", b"x"),
            ("competitions/leagues.json", b"{}"),
            ("data/TEAM.000", &[0, 1]),
            ("WORLD.ini", b"ignored"),
            ("teams/nested/C.json", b"not a file of teams/"),
        ]);
        assert_eq!(Fingerprint::of(&with_data).as_str(), "9b4a50fe40c75d5c");
        let without_data = files(&[
            ("teams/B.json", b"bb"),
            ("teams/A.json", b"a"),
            ("teams/notes.txt", b"x"),
            ("competitions/leagues.json", b"{}"),
        ]);
        assert_eq!(Fingerprint::of(&without_data).as_str(), "08b439a63d3120ea");
    }

    #[test]
    fn fingerprints_are_16_lower_case_hex_digits() {
        assert!(Fingerprint::try_from("efe1464b6d1d62b8".to_owned()).is_ok());
        assert!(Fingerprint::try_from("EFE1464B6D1D62B8".to_owned()).is_err());
        assert_eq!(
            Fingerprint::try_from("efe1".to_owned()),
            Err(InvalidFingerprint("efe1".to_owned()))
        );
        assert_eq!(
            InvalidFingerprint("x".to_owned()).to_string(),
            "`x` is not a world fingerprint (16 lower-case hex digits)"
        );
    }

    #[test]
    fn a_check_matches_when_both_agree() {
        let fingerprint = Fingerprint::of(&PackageFiles::default());
        let check = FingerprintCheck {
            stated: fingerprint.clone(),
            computed: fingerprint,
        };
        assert!(check.is_match());
        assert_eq!(check.stated.to_string(), check.computed.as_str());
    }
}
