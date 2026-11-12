//! The team database file: the PC game's `DATA/TEAM.nnn` (one file per country, `nnn`
//! being its number) and `DATA/TEAM.CUS` (the custom teams).
//!
//! A big-endian `u16` count, then that many 684-byte team records ([`Team`]), and nothing
//! after them. The PC game's files are not compressed (D9).

use openswos_common::{ByteReader, ByteWriter, Exporter, Importer, ReadError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::team::{RecordError, TEAM_RECORD_LEN, Team};

/// Bytes of the team count at the start of the file.
const TEAM_COUNT_LEN: usize = size_of::<u16>();

/// A team database file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamFile {
    /// The file's teams, in order.
    pub teams: Vec<Team>,
}

/// Why a team file does not decode or encode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TeamFileError {
    /// The count is missing, or bytes follow the last team.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// A team record does not decode.
    #[error("team {index}: {source}")]
    Team {
        /// The team's index in the file.
        index: usize,
        /// Why.
        source: RecordError,
    },
    /// More teams than the file's 16-bit count can hold.
    #[error("{count} teams do not fit a team file, which counts them in 16 bits")]
    TooManyTeams {
        /// The teams given.
        count: usize,
    },
}

/// The team-file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamFileFormat;

impl Importer for TeamFileFormat {
    type Model = TeamFile;
    type Error = TeamFileError;

    fn import(bytes: &[u8]) -> Result<TeamFile, TeamFileError> {
        let mut reader = ByteReader::new(bytes);
        let count = reader.read_u16_be()?;
        let teams = (0..usize::from(count))
            .map(|index| {
                Team::read(&mut reader).map_err(|source| TeamFileError::Team { index, source })
            })
            .collect::<Result<_, _>>()?;
        reader.finish()?;
        Ok(TeamFile { teams })
    }
}

impl Exporter for TeamFileFormat {
    type Model = TeamFile;
    type Error = TeamFileError;

    fn export(model: &TeamFile) -> Result<Vec<u8>, TeamFileError> {
        let count = team_count(model.teams.len())?;
        let mut writer =
            ByteWriter::with_capacity(TEAM_COUNT_LEN + model.teams.len() * TEAM_RECORD_LEN);
        writer.write_u16_be(count);
        for team in &model.teams {
            team.write(&mut writer);
        }
        Ok(writer.into_bytes())
    }
}

/// The count field for `len` teams.
fn team_count(len: usize) -> Result<u16, TeamFileError> {
    u16::try_from(len).map_err(|_| TeamFileError::TooManyTeams { count: len })
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size test records
mod tests {
    use super::*;
    use crate::team::InvalidShirtType;
    use proptest::prelude::*;

    /// A file of `count` teams named TEAM 0, TEAM 1...
    fn file(count: u8) -> Vec<u8> {
        let mut bytes = vec![0, count];
        for index in 0..count {
            let mut record = vec![0u8; TEAM_RECORD_LEN];
            record[1] = index;
            record[5..10].copy_from_slice(b"TEAM ");
            record[10] = b'0' + index;
            bytes.extend(record);
        }
        bytes
    }

    #[test]
    fn decodes_the_teams_in_order() {
        let teams = TeamFileFormat::import(&file(3))
            .expect("a valid file")
            .teams;
        let names: Vec<String> = teams.iter().map(|team| team.name.text()).collect();
        assert_eq!(names, vec!["TEAM 0", "TEAM 1", "TEAM 2"]);
    }

    #[test]
    fn round_trips() {
        let bytes = file(2);
        let model = TeamFileFormat::import(&bytes).expect("a valid file");
        assert_eq!(TeamFileFormat::export(&model), Ok(bytes));
    }

    #[test]
    fn an_empty_file_has_no_teams() {
        let model = TeamFileFormat::import(&[0, 0]).expect("a valid file");
        assert_eq!(model, TeamFile::default());
        assert_eq!(TeamFileFormat::export(&model), Ok(vec![0, 0]));
    }

    #[test]
    fn a_missing_count_fails() {
        assert!(matches!(
            TeamFileFormat::import(&[1]),
            Err(TeamFileError::Read(ReadError::UnexpectedEnd { .. }))
        ));
    }

    #[test]
    fn a_count_beyond_the_data_names_the_team() {
        let mut bytes = file(2);
        bytes[1] = 3;
        let error = TeamFileFormat::import(&bytes).expect_err("one team short");
        assert!(
            matches!(error, TeamFileError::Team { index: 2, .. }),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .starts_with("team 2: unexpected end of data"),
            "{error}"
        );
    }

    #[test]
    fn bytes_after_the_last_team_fail() {
        let mut bytes = file(1);
        bytes.push(0);
        assert!(matches!(
            TeamFileFormat::import(&bytes),
            Err(TeamFileError::Read(ReadError::TrailingBytes {
                remaining: 1,
                ..
            }))
        ));
    }

    #[test]
    fn an_invalid_team_names_its_index() {
        let mut bytes = file(2);
        bytes[2 + TEAM_RECORD_LEN + 26] = 9;
        assert_eq!(
            TeamFileFormat::import(&bytes),
            Err(TeamFileError::Team {
                index: 1,
                source: RecordError::ShirtType(InvalidShirtType(9)),
            })
        );
    }

    #[test]
    fn the_count_holds_16_bits() {
        assert_eq!(team_count(usize::from(u16::MAX)), Ok(u16::MAX));
        assert_eq!(
            team_count(usize::from(u16::MAX) + 1),
            Err(TeamFileError::TooManyTeams {
                count: usize::from(u16::MAX) + 1
            })
        );
    }

    proptest! {
        #[test]
        fn importing_any_bytes_never_panics(bytes: Vec<u8>) {
            let _ = TeamFileFormat::import(&bytes);
        }
    }
}
