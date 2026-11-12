#![doc = include_str!("../readme.md")]
// This crate parses untrusted bytes: every access is checked.
#![deny(clippy::indexing_slicing)]

use openswos_common::{ByteReader, ByteWriter, Exporter, Importer};
use openswos_team_resource::{RecordError, TEAM_RECORD_LEN, Team};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A European-cup team pool.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TmdFile {
    /// The pool's teams, in order.
    pub teams: Vec<Team>,
}

/// Why a pool file does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TmdFileError {
    /// The file's length is not a whole number of team records.
    #[error("{len} bytes are not a whole number of {TEAM_RECORD_LEN}-byte team records")]
    PartialRecord {
        /// The file's length.
        len: usize,
    },
    /// A team record does not decode.
    #[error("team {index}: {source}")]
    Team {
        /// The team's index in the file.
        index: usize,
        /// Why.
        source: RecordError,
    },
}

/// The European-cup pool format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TmdFileFormat;

impl Importer for TmdFileFormat {
    type Model = TmdFile;
    type Error = TmdFileError;

    fn import(bytes: &[u8]) -> Result<TmdFile, TmdFileError> {
        if !bytes.len().is_multiple_of(TEAM_RECORD_LEN) {
            return Err(TmdFileError::PartialRecord { len: bytes.len() });
        }
        let mut reader = ByteReader::new(bytes);
        let teams = (0..bytes.len() / TEAM_RECORD_LEN)
            .map(|index| {
                Team::read(&mut reader).map_err(|source| TmdFileError::Team { index, source })
            })
            .collect::<Result<_, _>>()?;
        Ok(TmdFile { teams })
    }
}

impl Exporter for TmdFileFormat {
    type Model = TmdFile;
    type Error = TmdFileError;

    fn export(model: &TmdFile) -> Result<Vec<u8>, TmdFileError> {
        let mut writer = ByteWriter::with_capacity(model.teams.len() * TEAM_RECORD_LEN);
        for team in &model.teams {
            team.write(&mut writer);
        }
        Ok(writer.into_bytes())
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size test records
mod tests {
    use super::*;
    use openswos_team_resource::InvalidShirtType;
    use proptest::prelude::*;

    /// A pool of `count` teams named CLUB 0, CLUB 1...
    fn pool(count: u8) -> Vec<u8> {
        let mut bytes = Vec::new();
        for index in 0..count {
            let mut record = vec![0u8; TEAM_RECORD_LEN];
            record[0] = 13; // file
            record[1] = index; // ordinal
            record[5..10].copy_from_slice(b"CLUB ");
            record[10] = b'0' + index;
            bytes.extend(record);
        }
        bytes
    }

    #[test]
    fn decodes_the_teams_in_order() {
        let teams = TmdFileFormat::import(&pool(3)).expect("a valid pool").teams;
        let names: Vec<String> = teams.iter().map(|team| team.name.text()).collect();
        assert_eq!(names, vec!["CLUB 0", "CLUB 1", "CLUB 2"]);
    }

    #[test]
    fn round_trips() {
        let bytes = pool(2);
        let model = TmdFileFormat::import(&bytes).expect("a valid pool");
        assert_eq!(TmdFileFormat::export(&model), Ok(bytes));
    }

    #[test]
    fn an_empty_file_is_an_empty_pool() {
        assert_eq!(TmdFileFormat::import(&[]), Ok(TmdFile::default()));
        assert_eq!(TmdFileFormat::export(&TmdFile::default()), Ok(Vec::new()));
    }

    #[test]
    fn a_partial_record_fails() {
        let mut bytes = pool(1);
        bytes.pop();
        let error = TmdFileFormat::import(&bytes).expect_err("one byte short");
        assert_eq!(
            error,
            TmdFileError::PartialRecord {
                len: TEAM_RECORD_LEN - 1
            }
        );
        assert_eq!(
            error.to_string(),
            "683 bytes are not a whole number of 684-byte team records"
        );
    }

    #[test]
    fn an_invalid_team_names_its_index() {
        let mut bytes = pool(3);
        bytes[2 * TEAM_RECORD_LEN + 31] = 7;
        assert_eq!(
            TmdFileFormat::import(&bytes),
            Err(TmdFileError::Team {
                index: 2,
                source: RecordError::ShirtType(InvalidShirtType(7)),
            })
        );
    }

    proptest! {
        #[test]
        fn importing_any_bytes_never_panics(bytes: Vec<u8>) {
            let _ = TmdFileFormat::import(&bytes);
        }
    }
}
