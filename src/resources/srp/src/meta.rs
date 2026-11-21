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
//! | 2170 | 4 + 1222 | optional: `LBNC`, then a long bench's places at kick-off ([`LongBench`]) |
//!
//! A file without the long bench is a match of the game's 16 places.

use bitflags::bitflags;
use openswos_common::{ByteReader, ByteWriter, FixedName};
use openswos_hil_resource::{DosPlayerGame, PLAYER_GAME_LEN, TeamGame, WordOrder};

use crate::error::ChunkError;

/// Bytes of the build stamp.
pub const BUILD_LEN: usize = 32;

/// Bytes of the competition's name, and of its round.
pub const GAME_NAME_LEN: usize = 40;

/// The tag before a long bench's places (upstream `META_BENCH`).
pub const LONG_BENCH_TAG: [u8; 4] = *b"LBNC";

/// The places a team plays with in the game (upstream `LB_GAME_PLACES`).
pub const GAME_PLACES: u16 = 16;

/// The most places a long bench gives a team: 11 and 15 substitutes (upstream
/// `LB_PLACES_MAX`).
pub const LONG_BENCH_PLACES_MAX: u16 = 26;

/// The places past the game's 16 a recording keeps per team: 17..26 (upstream
/// `LB_EXTRA_PLACES`).
pub const LONG_BENCH_EXTRA_PLACES: usize = 10;

/// Bytes of a long bench's places: their count, both teams' (upstream `LB_REPLAY_BYTES`).
const LONG_BENCH_BYTES: usize = 2 + 2 * LONG_BENCH_EXTRA_PLACES * PLAYER_GAME_LEN;

/// A team's places past the game's 16.
pub type ExtraPlaces = [DosPlayerGame; LONG_BENCH_EXTRA_PLACES];

/// A World 2026-27 match's long bench (upstream long_bench.c `LongBench_ReplayWrite`): its
/// places at kick-off and both teams' players 17..26, for their names in the viewer. The
/// records are copied as they lie in memory (all bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LongBench {
    /// The places a team has (17..=26 for a long bench), as written.
    pub places: u16,
    /// Each team's places 17..26, the first team's first.
    pub teams: [ExtraPlaces; 2],
}

impl LongBench {
    /// The places the viewer takes (`LongBench_ReplayRead`): `None`, the game's 16, unless
    /// past them and at most [`LONG_BENCH_PLACES_MAX`].
    #[must_use]
    pub fn places_taken(&self) -> Option<u16> {
        (self.places > GAME_PLACES && self.places <= LONG_BENCH_PLACES_MAX).then_some(self.places)
    }

    fn read(reader: &mut ByteReader<'_>) -> Result<Self, ChunkError> {
        let places = reader.read_u16_be()?;
        let mut team = || -> Result<ExtraPlaces, ChunkError> {
            let players = (0..LONG_BENCH_EXTRA_PLACES)
                .map(|_| DosPlayerGame::read(reader))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(players
                .try_into()
                .expect("as many players as places were read"))
        };
        let first = team()?;
        let second = team()?;
        Ok(Self {
            places,
            teams: [first, second],
        })
    }

    fn write(&self, writer: &mut ByteWriter) {
        writer.write_bytes(&LONG_BENCH_TAG);
        writer.write_u16_be(self.places);
        for player in self.teams.iter().flatten() {
            player.write(writer);
        }
    }
}

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
    /// A long bench's places, when the match had one (and the chunk holds them whole).
    pub long_bench: Option<LongBench>,
    /// Bytes after the teams and the long bench (a later build's), kept as written.
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
        let mut meta = Self {
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
            long_bench: None,
            trailing: Vec::new(),
        };
        // As the viewer reads it (`ReplayV2_Load`): the tag, and the places whole after it.
        let rest = reader.read_bytes(reader.remaining())?;
        let bench = rest
            .strip_prefix(&LONG_BENCH_TAG)
            .filter(|bench| bench.len() >= LONG_BENCH_BYTES);
        meta.trailing = match bench {
            Some(bench) => {
                let mut bench_reader = ByteReader::new(bench);
                meta.long_bench = Some(LongBench::read(&mut bench_reader)?);
                bench_reader.read_bytes(bench_reader.remaining())?.to_vec()
            }
            None => rest.to_vec(),
        };
        Ok(meta)
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
            long_bench,
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
        if let Some(bench) = long_bench {
            bench.write(&mut writer);
        }
        writer.write_bytes(trailing);
        writer.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openswos_hil_resource::TEAM_GAME_LEN;

    /// Where the teams start in the chunk (the table above).
    const TEAMS_AT: usize = 134;

    /// Bytes of the chunk without a long bench (upstream `META_BYTES`).
    const META_BYTES: usize = TEAMS_AT + 2 * TEAM_GAME_LEN;

    /// A long bench's places: 11 and 9 substitutes.
    const PLACES: u16 = 20;

    /// A player 17's shirt number.
    const SHIRT: u8 = 17;

    fn bench(places: u16) -> Vec<u8> {
        let mut bytes = LONG_BENCH_TAG.to_vec();
        bytes.extend(places.to_be_bytes());
        let mut player = vec![0; PLAYER_GAME_LEN];
        if let Some(shirt) = player.get_mut(3) {
            *shirt = SHIRT;
        }
        for _ in 0..2 * LONG_BENCH_EXTRA_PLACES {
            bytes.extend(&player);
        }
        bytes
    }

    #[test]
    fn a_match_of_16_places_has_no_long_bench() {
        let bytes = vec![0; META_BYTES];
        let meta = SrpMeta::decode(&bytes).expect("a match");
        assert_eq!(meta.long_bench, None);
        assert!(meta.trailing.is_empty());
        assert_eq!(meta.encode(), bytes);
    }

    #[test]
    fn a_long_bench_follows_the_teams() {
        let mut bytes = vec![0; META_BYTES];
        bytes.extend(bench(PLACES));
        let meta = SrpMeta::decode(&bytes).expect("a match");
        let long_bench = meta.long_bench.as_ref().expect("a long bench");
        assert_eq!(long_bench.places_taken(), Some(PLACES));
        assert!(
            long_bench
                .teams
                .iter()
                .flatten()
                .all(|player| player.shirt_number.cast_unsigned() == SHIRT)
        );
        assert_eq!(meta.encode(), bytes);
        // Bytes after it are kept.
        bytes.extend([7, 8]);
        let meta = SrpMeta::decode(&bytes).expect("a match");
        assert_eq!(meta.trailing, vec![7, 8]);
        assert_eq!(meta.encode(), bytes);
    }

    #[test]
    fn places_the_viewer_does_not_take_and_a_short_tail_are_kept() {
        let mut bytes = vec![0; META_BYTES];
        bytes.extend(bench(GAME_PLACES));
        let meta = SrpMeta::decode(&bytes).expect("a match");
        assert_eq!(
            meta.long_bench.as_ref().and_then(LongBench::places_taken),
            None
        );
        assert_eq!(meta.encode(), bytes);
        // Too short for the places: not a long bench, bytes as written.
        bytes.truncate(META_BYTES + LONG_BENCH_TAG.len() + 2);
        let meta = SrpMeta::decode(&bytes).expect("a match");
        assert_eq!(meta.long_bench, None);
        assert_eq!(meta.trailing.len(), LONG_BENCH_TAG.len() + 2);
        assert_eq!(meta.encode(), bytes);
    }
}
