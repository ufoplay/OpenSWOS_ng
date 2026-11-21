//! `rules/team_files.json`: how the world's clubs are numbered across its team files.

use openswos_common::TeamFileNumber;
use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/team_files.json`.
///
/// ```json
/// {
///   "bases": [0, 16, 26, …],
///   "second_files": [],
///   "squad_numbers": false
/// }
/// ```
///
/// (written one number a line).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamFiles {
    /// Each team file's first global team number, by file number (256 of them, never going
    /// down): a club's global number is its file's base plus its place in the file (upstream
    /// `_teamsCountryNumbers`, src/swos_amiga_data_imports.c; `SetTeamGlobalNumbers`). The
    /// league tables of a career are kept by global number.
    pub bases: Vec<u16>,
    /// The countries whose clubs run over two team files (upstream src/world.c
    /// `World_SecondFile`): none in 96/97.
    pub second_files: Vec<SecondFile>,
    /// Whether careers and matches keep the players' own squad numbers (upstream
    /// `World_SquadNumbers`): not in 96/97.
    pub squad_numbers: bool,
}

/// A country's second team file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecondFile {
    /// The country.
    pub country: TeamFileNumber,
    /// Its second file's number: its clubs play for the country.
    pub file: TeamFileNumber,
}

impl RulesFile for TeamFiles {
    const FILE_NAME: &'static str = "team_files.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    #[test]
    fn it_reads_and_writes_back() {
        let text = "{\n  \"bases\": [\n    0,\n    16\n  ],\n  \"second_files\": [\n    {\n      \
                    \"country\": 20,\n      \"file\": 197\n    }\n  ],\n  \"squad_numbers\": \
                    true\n}\n";
        let files: TeamFiles = read(text.as_bytes()).expect("team files");
        assert_eq!(
            files,
            TeamFiles {
                bases: vec![0, 16],
                second_files: vec![SecondFile {
                    country: TeamFileNumber(20),
                    file: TeamFileNumber(197)
                }],
                squad_numbers: true
            }
        );
        assert_eq!(write(&files), text.as_bytes());
    }

    #[test]
    fn a_base_is_a_word() {
        assert!(
            read::<TeamFiles>(br#"{"bases": [65536], "second_files": [], "squad_numbers": false}"#)
                .is_err()
        );
    }
}
