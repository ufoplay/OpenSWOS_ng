//! `META`: the match (upstream src/replay_v2.c `build_meta`), taken at kick-off but the
//! final score, which is filled in at the end. Big-endian.
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 2 | version: 1 |
//! | 2 | 2 | the screen width it was recorded on |
//! | 4 | 2 | gameplay flags ([`GameplayFlags`]) |
//! | 6 | 2 | the view height it was simulated in (200 DOS, 272 Amiga; 0 unknown) |
//! | 8 | 4 | the UTC time |
//! | 12 | 32 | the build stamp |
//! | 44 | 2 + 2 | the final score |
//! | 48 | 2 | the pitch's number and type |
//! | 50 | 2 | the most substitutes |
//! | 52 | 2 | 0 |
//! | 54 | 40 + 40 | the competition's name and round |
//! | 134 | 2 × 1018 | the two teams (`TeamGame`, big-endian words) |

use bitflags::bitflags;
use openswos_common::{ByteReader, ByteWriter, FixedName};
use openswos_hil_resource::{TeamGame, WordOrder};

use crate::error::ChunkError;

/// Bytes of the build stamp.
pub const BUILD_LEN: usize = 32;

/// Bytes of the competition's name, and of its round.
pub const GAME_NAME_LEN: usize = 40;

bitflags! {
    /// How the match was played and shown.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct GameplayFlags: u16 {
        /// The DOS gameplay.
        const DOS_GAMEPLAY = 1;
        /// The DOS graphics.
        const DOS_GRAPHICS = 2;
        /// The arcade mode.
        const ARCADE = 4;
        /// The DOS view (320 × 200).
        const DOS_VIEW = 8;
        /// The view is recorded (a file without it predates it).
        const VIEW_RECORDED = 16;
        /// The XBOX gameplay style.
        const XBOX = 32;
        /// The Amiga 60 Hz style.
        const AMIGA60 = 64;
        /// The Moon style.
        const MOON = 128;
        // Flags of later builds are kept as read.
        const _ = !0;
    }
}

/// The match a recording is of.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SrpMeta {
    /// The chunk's version (1).
    pub version: u16,
    /// The screen width it was recorded on.
    pub screen_width: u16,
    /// How it was played and shown.
    pub gameplay: GameplayFlags,
    /// The view height it was simulated in; 0 in older files.
    pub sim_view_height: u16,
    /// When it was played: seconds since 1970, UTC.
    pub time: u32,
    /// The build that recorded it.
    pub build: FixedName<BUILD_LEN>,
    /// The first team's final goals.
    pub team1_goals: u16,
    /// The second team's final goals.
    pub team2_goals: u16,
    /// The pitch's number and type.
    pub pitch_number_and_type: u16,
    /// The most substitutes.
    pub max_substitutes: u16,
    /// The word after them, kept as written (0).
    pub reserved: u16,
    /// The competition's name.
    pub game_name: FixedName<GAME_NAME_LEN>,
    /// The competition's round.
    pub game_round: FixedName<GAME_NAME_LEN>,
    /// The first team.
    pub team1: TeamGame,
    /// The second team.
    pub team2: TeamGame,
    /// Bytes after the teams (a later build's), kept as written.
    pub trailing: Vec<u8>,
}

impl SrpMeta {
    /// The match in a `META` chunk's content.
    ///
    /// # Errors
    ///
    /// When the content is shorter than the match.
    pub fn decode(data: &[u8]) -> Result<Self, ChunkError> {
        let mut reader = ByteReader::new(data);
        Ok(Self {
            version: reader.read_u16_be()?,
            screen_width: reader.read_u16_be()?,
            gameplay: GameplayFlags::from_bits_retain(reader.read_u16_be()?),
            sim_view_height: reader.read_u16_be()?,
            time: reader.read_u32_be()?,
            build: FixedName::from_bytes(reader.read_array()?),
            team1_goals: reader.read_u16_be()?,
            team2_goals: reader.read_u16_be()?,
            pitch_number_and_type: reader.read_u16_be()?,
            max_substitutes: reader.read_u16_be()?,
            reserved: reader.read_u16_be()?,
            game_name: FixedName::from_bytes(reader.read_array()?),
            game_round: FixedName::from_bytes(reader.read_array()?),
            team1: TeamGame::read(&mut reader, WordOrder::Big)?,
            team2: TeamGame::read(&mut reader, WordOrder::Big)?,
            trailing: reader.read_bytes(reader.remaining())?.to_vec(),
        })
    }

    /// The `META` chunk's content.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let Self {
            version,
            screen_width,
            gameplay,
            sim_view_height,
            time,
            build,
            team1_goals,
            team2_goals,
            pitch_number_and_type,
            max_substitutes,
            reserved,
            game_name,
            game_round,
            team1,
            team2,
            trailing,
        } = self;
        let mut writer = ByteWriter::new();
        writer.write_u16_be(*version);
        writer.write_u16_be(*screen_width);
        writer.write_u16_be(gameplay.bits());
        writer.write_u16_be(*sim_view_height);
        writer.write_u32_be(*time);
        writer.write_bytes(build.as_bytes());
        for word in [
            team1_goals,
            team2_goals,
            pitch_number_and_type,
            max_substitutes,
            reserved,
        ] {
            writer.write_u16_be(*word);
        }
        writer.write_bytes(game_name.as_bytes());
        writer.write_bytes(game_round.as_bytes());
        team1.write(&mut writer, WordOrder::Big);
        team2.write(&mut writer, WordOrder::Big);
        writer.write_bytes(trailing);
        writer.into_bytes()
    }
}
