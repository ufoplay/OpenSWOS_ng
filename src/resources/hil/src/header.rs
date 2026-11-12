//! The header of a DOS replay, `.HIL` or `.RPL` (upstream src/dos_replay_load.c): 3626
//! bytes, little-endian.
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 4 | the signature: 8 |
//! | 4 | 4 | a byte, the game style (the playback rate), a word |
//! | 8 | 1704 | the first team ([`DosTeamGame`]) |
//! | 1712 | 1704 | the second team |
//! | 3416 | 40 | the competition's name |
//! | 3456 | 160 | zero in SWOS 2020's files, kept as written |
//! | 3616 | 2 × 5 | the scenes, the two teams' goals, the pitch, the most substitutes |

use openswos_common::{ByteReader, ByteWriter, FixedName};

use crate::error::DosReplayError;
use crate::team_game::DosTeamGame;

/// Bytes of a DOS replay's header (upstream `DOS_HDR_SIZE`).
pub const DOS_HEADER_LEN: usize = 3626;

/// A DOS replay's signature, its first long word (upstream `DosReplay_IsDosHil`).
pub const DOS_SIGNATURE: u32 = 8;

/// Bytes of the competition's name (upstream: 40 copied).
pub const GAME_NAME_LEN: usize = 40;

/// Bytes between the name and the counts.
pub const RESERVED_LEN: usize = 160;

/// A DOS replay's header.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DosReplayHeader {
    /// The byte after the signature.
    pub field_4: u8,
    /// The game style the replay was recorded in, which sets its playback rate (upstream
    /// `replay_fps_from_gamestyle`: 0 PC 70 Hz, 1 Amiga 50 Hz; SWOS 2020's `.HIL` files
    /// say 30 or 31 and more, the low bit choosing).
    pub game_style: u8,
    /// The word after it.
    pub field_6: u16,
    /// The first team.
    pub team1: DosTeamGame,
    /// The second team.
    pub team2: DosTeamGame,
    /// The competition's name.
    pub game_name: FixedName<GAME_NAME_LEN>,
    /// The bytes after it (160), kept as written.
    pub reserved: Vec<u8>,
    /// The scenes (a `.HIL`'s goals; a `.RPL` has one).
    pub num_scenes: u16,
    /// The first team's goals.
    pub team1_goals: u16,
    /// The second team's goals.
    pub team2_goals: u16,
    /// The pitch.
    pub pitch: u16,
    /// The most substitutes.
    pub max_substitutes: u16,
}

impl DosReplayHeader {
    /// Reads a header.
    ///
    /// # Errors
    ///
    /// When the data is short or the signature is not a DOS replay's.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, DosReplayError> {
        let signature = reader.read_u32_le()?;
        if signature != DOS_SIGNATURE {
            return Err(DosReplayError::Signature(signature));
        }
        Ok(Self {
            field_4: reader.read_u8()?,
            game_style: reader.read_u8()?,
            field_6: reader.read_u16_le()?,
            team1: DosTeamGame::read(reader)?,
            team2: DosTeamGame::read(reader)?,
            game_name: FixedName::from_bytes(reader.read_array()?),
            reserved: reader.read_bytes(RESERVED_LEN)?.to_vec(),
            num_scenes: reader.read_u16_le()?,
            team1_goals: reader.read_u16_le()?,
            team2_goals: reader.read_u16_le()?,
            pitch: reader.read_u16_le()?,
            max_substitutes: reader.read_u16_le()?,
        })
    }

    /// Writes the header.
    ///
    /// # Errors
    ///
    /// When a raw part (the teams' tails, the reserved bytes) is not its size.
    pub fn write(&self, writer: &mut ByteWriter) -> Result<(), DosReplayError> {
        let Self {
            field_4,
            game_style,
            field_6,
            team1,
            team2,
            game_name,
            reserved,
            num_scenes,
            team1_goals,
            team2_goals,
            pitch,
            max_substitutes,
        } = self;
        for (part, len, expected) in [
            (
                "first team's tail",
                team1.tail.len(),
                crate::team_game::TEAM_GAME_TAIL_LEN,
            ),
            (
                "second team's tail",
                team2.tail.len(),
                crate::team_game::TEAM_GAME_TAIL_LEN,
            ),
            ("reserved bytes", reserved.len(), RESERVED_LEN),
        ] {
            if len != expected {
                return Err(DosReplayError::PartLength {
                    part,
                    len,
                    expected,
                });
            }
        }
        writer.write_u32_le(DOS_SIGNATURE);
        writer.write_u8(*field_4);
        writer.write_u8(*game_style);
        writer.write_u16_le(*field_6);
        team1.write(writer);
        team2.write(writer);
        writer.write_bytes(game_name.as_bytes());
        writer.write_bytes(reserved);
        for word in [num_scenes, team1_goals, team2_goals, pitch, max_substitutes] {
            writer.write_u16_le(*word);
        }
        Ok(())
    }
}
