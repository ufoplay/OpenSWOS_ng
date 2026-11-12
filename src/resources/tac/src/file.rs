//! The tactics file (`.TAC`): one tactics record ([`TeamTactics`]), which the game's tactics
//! editor saves and loads (upstream `WriteTacticsFile` / `ReadTacticsFile`,
//! src/file_io.c).
//!
//! The game reads the record and ignores what follows it (`ReadTacticsFile` copies 370
//! bytes). Those bytes are kept as read, so every file round-trips: SWOS 2020's CPU
//! tactics end with six zero bytes.

use std::convert::Infallible;

use openswos_common::{ByteReader, ByteWriter, Exporter, Importer};
use serde::{Deserialize, Serialize};

use crate::tactics::{TACTICS_RECORD_LEN, TacticsError, TeamTactics};

/// A tactics file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TacFile {
    /// The tactic.
    pub tactics: TeamTactics,
    /// The bytes after the record, which the game does not read.
    pub trailer: Vec<u8>,
}

/// The tactics-file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacFileFormat;

impl Importer for TacFileFormat {
    type Model = TacFile;
    type Error = TacticsError;

    fn import(bytes: &[u8]) -> Result<TacFile, TacticsError> {
        let mut reader = ByteReader::new(bytes);
        let tactics = TeamTactics::read(&mut reader)?;
        let trailer = reader.read_bytes(reader.remaining())?.to_vec();
        reader.finish()?;
        Ok(TacFile { tactics, trailer })
    }
}

impl Exporter for TacFileFormat {
    type Model = TacFile;
    type Error = Infallible;

    fn export(model: &TacFile) -> Result<Vec<u8>, Infallible> {
        let TacFile { tactics, trailer } = model;
        let mut writer = ByteWriter::with_capacity(TACTICS_RECORD_LEN + trailer.len());
        tactics.write(&mut writer);
        writer.write_bytes(trailer);
        Ok(writer.into_bytes())
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size test records
mod tests {
    use super::*;
    use openswos_common::ReadError;
    use proptest::prelude::*;

    /// Offset of the ball-out-of-play tactics in a record.
    const BALL_OUT_OF_PLAY_AT: usize = 369;

    /// SWOS 2020's trailer.
    const ZEROS: [u8; 6] = [0; 6];

    fn file(trailer: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8; TACTICS_RECORD_LEN];
        bytes[..3].copy_from_slice(b"532");
        bytes[BALL_OUT_OF_PLAY_AT] = 3;
        bytes.extend_from_slice(trailer);
        bytes
    }

    #[test]
    fn a_bare_record_has_no_trailer() {
        let tac = TacFileFormat::import(&file(&[])).expect("a valid file");
        assert_eq!(tac.tactics.name.text(), "532");
        assert!(tac.trailer.is_empty());
    }

    #[test]
    fn bytes_after_the_record_are_kept() {
        let bytes = file(&ZEROS);
        let tac = TacFileFormat::import(&bytes).expect("a valid file");
        assert_eq!(tac.trailer, ZEROS.to_vec());
        assert_eq!(TacFileFormat::export(&tac), Ok(bytes));
    }

    #[test]
    fn a_short_file_fails() {
        let bytes = file(&[]);
        assert!(matches!(
            TacFileFormat::import(&bytes[..TACTICS_RECORD_LEN - 1]),
            Err(TacticsError::Read(ReadError::UnexpectedEnd { .. }))
        ));
    }

    proptest! {
        #[test]
        fn importing_any_bytes_never_panics(bytes: Vec<u8>) {
            let _ = TacFileFormat::import(&bytes);
        }

        #[test]
        fn any_trailer_round_trips(trailer: Vec<u8>) {
            let bytes = file(&trailer);
            let tac = TacFileFormat::import(&bytes).expect("a valid file");
            prop_assert_eq!(TacFileFormat::export(&tac), Ok(bytes));
        }
    }
}
