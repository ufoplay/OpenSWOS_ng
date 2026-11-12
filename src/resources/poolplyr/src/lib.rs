#![doc = include_str!("../readme.md")]
// This crate parses untrusted bytes: every access is checked.
#![deny(clippy::indexing_slicing)]

use openswos_common::{ByteReader, ByteWriter, Exporter, FixedName, Importer, ReadError, TeamId};
use openswos_team_resource::{PLAYER_RECORD_LEN, Player, TEAM_NAME_LEN};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Bytes of a pool record's team id.
const TEAM_ID_LEN: usize = size_of::<[u8; 2]>();

/// Bytes of a pool record's player ordinal.
const PLAYER_ORDINAL_LEN: usize = size_of::<u8>();

/// Bytes of a pool record (upstream `PoolPlayer`): team id, player ordinal, team name and
/// the player record.
pub const POOL_PLAYER_RECORD_LEN: usize =
    TEAM_ID_LEN + PLAYER_ORDINAL_LEN + TEAM_NAME_LEN + PLAYER_RECORD_LEN;

/// A player of the transfer pool, with the club he plays for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoolPlayer {
    /// The player's club.
    pub team: TeamId,
    /// The player's place in his club's squad: the index of his record in the club's team
    /// record (upstream `playerOrdinal`).
    pub player_ordinal: u8,
    /// The club's name.
    pub team_name: FixedName<TEAM_NAME_LEN>,
    /// The player.
    pub player: Player,
}

impl PoolPlayer {
    /// Reads a pool record.
    ///
    /// # Errors
    ///
    /// When the data is too short.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, ReadError> {
        Ok(Self {
            team: TeamId::from_bytes(reader.read_array()?),
            player_ordinal: reader.read_u8()?,
            team_name: FixedName::from_bytes(reader.read_array()?),
            player: Player::read(reader)?,
        })
    }

    /// Writes the pool record.
    pub fn write(&self, writer: &mut ByteWriter) {
        let Self {
            team,
            player_ordinal,
            team_name,
            player,
        } = self;
        writer.write_bytes(&team.to_bytes());
        writer.write_u8(*player_ordinal);
        writer.write_bytes(team_name.as_bytes());
        player.write(writer);
    }
}

/// The transfer pool file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoolFile {
    /// The pool's players, in order.
    pub players: Vec<PoolPlayer>,
}

/// Why a pool file does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PoolFileError {
    /// The file's length is not a whole number of pool records.
    #[error("{len} bytes are not a whole number of {POOL_PLAYER_RECORD_LEN}-byte pool records")]
    PartialRecord {
        /// The file's length.
        len: usize,
    },
    /// The data ended inside a record.
    #[error(transparent)]
    Read(#[from] ReadError),
}

/// The transfer pool format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolFileFormat;

impl Importer for PoolFileFormat {
    type Model = PoolFile;
    type Error = PoolFileError;

    fn import(bytes: &[u8]) -> Result<PoolFile, PoolFileError> {
        if !bytes.len().is_multiple_of(POOL_PLAYER_RECORD_LEN) {
            return Err(PoolFileError::PartialRecord { len: bytes.len() });
        }
        let mut reader = ByteReader::new(bytes);
        let players = (0..bytes.len() / POOL_PLAYER_RECORD_LEN)
            .map(|_| PoolPlayer::read(&mut reader))
            .collect::<Result<_, _>>()?;
        Ok(PoolFile { players })
    }
}

impl Exporter for PoolFileFormat {
    type Model = PoolFile;
    type Error = PoolFileError;

    fn export(model: &PoolFile) -> Result<Vec<u8>, PoolFileError> {
        let mut writer = ByteWriter::with_capacity(model.players.len() * POOL_PLAYER_RECORD_LEN);
        for player in &model.players {
            player.write(&mut writer);
        }
        Ok(writer.into_bytes())
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size test records
mod tests {
    use super::*;
    use openswos_common::{TeamFileNumber, TeamOrdinal};
    use proptest::prelude::*;

    /// ERIC DEFLANDRE of CLUB BRUGES, the PC pool's first record (skills left at 0).
    fn record() -> Vec<u8> {
        let mut bytes = vec![0u8; POOL_PLAYER_RECORD_LEN];
        bytes[0..2].copy_from_slice(&[0x02, 0x04]); // team id: file 2, ordinal 4
        bytes[2] = 2; // player ordinal
        bytes[3..14].copy_from_slice(b"CLUB BRUGES");
        bytes[20 + 2] = 7; // shirt number
        bytes[20 + 3..20 + 17].copy_from_slice(b"ERIC DEFLANDRE");
        bytes
    }

    #[test]
    fn the_record_is_58_bytes() {
        assert_eq!(POOL_PLAYER_RECORD_LEN, 58);
    }

    #[test]
    fn decodes_every_field() {
        let pool = PoolFileFormat::import(&record()).expect("a valid pool");
        let [player] = pool.players.as_slice() else {
            panic!("one player expected");
        };
        assert_eq!(
            player.team,
            TeamId {
                file: TeamFileNumber(2),
                ordinal: TeamOrdinal(4)
            }
        );
        assert_eq!(player.player_ordinal, 2);
        assert_eq!(player.team_name.text(), "CLUB BRUGES");
        assert_eq!(player.player.name.text(), "ERIC DEFLANDRE");
        assert_eq!(player.player.shirt_number, 7);
    }

    #[test]
    fn round_trips() {
        let bytes = [record(), record()].concat();
        let model = PoolFileFormat::import(&bytes).expect("a valid pool");
        assert_eq!(model.players.len(), 2);
        assert_eq!(PoolFileFormat::export(&model), Ok(bytes));
    }

    #[test]
    fn an_empty_file_is_an_empty_pool() {
        assert_eq!(PoolFileFormat::import(&[]), Ok(PoolFile::default()));
        assert_eq!(PoolFileFormat::export(&PoolFile::default()), Ok(Vec::new()));
    }

    #[test]
    fn a_partial_record_fails() {
        let error = PoolFileFormat::import(&record()[1..]).expect_err("one byte short");
        assert_eq!(error, PoolFileError::PartialRecord { len: 57 });
        assert_eq!(
            error.to_string(),
            "57 bytes are not a whole number of 58-byte pool records"
        );
    }

    proptest! {
        #[test]
        fn any_whole_records_round_trip(records in proptest::collection::vec(any::<[u8; 58]>(), 0..8)) {
            let bytes = records.concat();
            let model = PoolFileFormat::import(&bytes).expect("whole records always decode");
            prop_assert_eq!(PoolFileFormat::export(&model), Ok(bytes));
        }

        #[test]
        fn importing_any_bytes_never_panics(bytes: Vec<u8>) {
            let _ = PoolFileFormat::import(&bytes);
        }
    }
}
