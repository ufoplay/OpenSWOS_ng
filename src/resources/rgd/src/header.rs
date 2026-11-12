//! A recording's header (swos-port `Header`, `HeaderV1p2`; upstream docs/TESTS.md §2):
//! the match's settings, its two teams and the user's tactics, as they were when the
//! recording began.
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 3 | version major, minor, the fixed random value |
//! | 3 | 5 | pitch type or season, pitch type, game length, replays, equal teams |
//! | 8 | 684 | the top team's record |
//! | 692 | 684 | the bottom team's record |
//! | 1376 | 4 | the top and bottom teams' coach and player numbers |
//! | 1380 | 2 | extra time, penalty shoot-out |
//! | 1382 | 2 | the top and bottom teams' marked players |
//! | 1384 | 6 × 370 | the user's tactics |
//! | 3604 | 2 | the input controls |
//! | 3606 | 4 | the initial frame |
//! | 3610 | 7 | 1.2+: the training game's career size, season, fewest and most substitutes |

use openswos_common::{ByteReader, ByteWriter};
use openswos_tac_resource::TeamTactics;
use openswos_team_resource::Team;

use crate::error::RgdError;
use crate::version::RgdVersion;

/// The user's tactics: A to F.
pub const USER_TACTICS: usize = 6;

/// A 1.2+ header's additions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct HeaderV1p2 {
    /// The training game's career size.
    pub training_game_career_size: u32,
    /// The season.
    pub season: i8,
    /// The fewest substitutes.
    pub min_substitutes: i8,
    /// The most substitutes.
    pub max_substitutes: i8,
}

/// A recording's header.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RgdHeader {
    /// The version.
    pub version: RgdVersion,
    /// The random value the match was played with (swos-port pins it).
    pub fixed_random_value: u8,
    /// The pitch type or the season.
    pub pitch_type_or_season: i8,
    /// The pitch type.
    pub pitch_type: i8,
    /// The game's length.
    pub game_length: i8,
    /// Automatic replays.
    pub auto_replays: i8,
    /// All the player's teams equal.
    pub all_player_teams_equal: i8,
    /// The top team. Faithful to the original: its global number is little-endian in
    /// memory, which the team crate reads byte-swapped.
    pub top_team: Team,
    /// The bottom team.
    pub bottom_team: Team,
    /// The top team's coach number.
    pub top_team_coach_no: u8,
    /// The top team's player number.
    pub top_team_player_no: u8,
    /// The bottom team's coach number.
    pub bottom_team_coach_no: u8,
    /// The bottom team's player number.
    pub bottom_team_player_no: u8,
    /// Extra time.
    pub extra_time: u8,
    /// A penalty shoot-out.
    pub penalty_shootout: u8,
    /// The top team's marked player.
    pub top_team_marked_player: i8,
    /// The bottom team's marked player.
    pub bottom_team_marked_player: i8,
    /// The user's tactics A to F.
    pub user_tactics: [TeamTactics; USER_TACTICS],
    /// The input controls.
    pub input_controls: u16,
    /// The initial frame.
    pub initial_frame: u32,
    /// The 1.2+ additions: present exactly when the version has them.
    pub v1p2: Option<HeaderV1p2>,
}

impl RgdHeader {
    /// Reads a header.
    ///
    /// # Errors
    ///
    /// When the data is short, the version is not one, or a team or tactic does not read.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, RgdError> {
        let [major, minor] = reader.read_array()?;
        let version = RgdVersion::new(major, minor)?;
        let fixed_random_value = reader.read_u8()?;
        let [
            pitch_type_or_season,
            pitch_type,
            game_length,
            auto_replays,
            all_player_teams_equal,
        ] = reader.read_array()?.map(u8::cast_signed);
        let top_team = Team::read(reader)?;
        let bottom_team = Team::read(reader)?;
        let [
            top_team_coach_no,
            top_team_player_no,
            bottom_team_coach_no,
            bottom_team_player_no,
            extra_time,
            penalty_shootout,
        ] = reader.read_array()?;
        let top_team_marked_player = reader.read_i8()?;
        let bottom_team_marked_player = reader.read_i8()?;
        let mut user_tactics = Vec::with_capacity(USER_TACTICS);
        for _ in 0..USER_TACTICS {
            user_tactics.push(TeamTactics::read(reader)?);
        }
        let input_controls = reader.read_u16_le()?;
        let initial_frame = reader.read_u32_le()?;
        let v1p2 = if version.has_header_v1p2() {
            Some(HeaderV1p2 {
                training_game_career_size: reader.read_u32_le()?,
                season: reader.read_i8()?,
                min_substitutes: reader.read_i8()?,
                max_substitutes: reader.read_i8()?,
            })
        } else {
            None
        };
        Ok(Self {
            version,
            fixed_random_value,
            pitch_type_or_season,
            pitch_type,
            game_length,
            auto_replays,
            all_player_teams_equal,
            top_team,
            bottom_team,
            top_team_coach_no,
            top_team_player_no,
            bottom_team_coach_no,
            bottom_team_player_no,
            extra_time,
            penalty_shootout,
            top_team_marked_player,
            bottom_team_marked_player,
            user_tactics: user_tactics.try_into().expect("six tactics were read"),
            input_controls,
            initial_frame,
            v1p2,
        })
    }

    /// Writes the header.
    ///
    /// # Errors
    ///
    /// When the 1.2 additions are present without the version having them, or the
    /// other way round.
    pub fn write(&self, writer: &mut ByteWriter) -> Result<(), RgdError> {
        let Self {
            version,
            fixed_random_value,
            pitch_type_or_season,
            pitch_type,
            game_length,
            auto_replays,
            all_player_teams_equal,
            top_team,
            bottom_team,
            top_team_coach_no,
            top_team_player_no,
            bottom_team_coach_no,
            bottom_team_player_no,
            extra_time,
            penalty_shootout,
            top_team_marked_player,
            bottom_team_marked_player,
            user_tactics,
            input_controls,
            initial_frame,
            v1p2,
        } = self;
        if v1p2.is_some() != version.has_header_v1p2() {
            return Err(RgdError::HeaderVersion(*version));
        }
        writer.write_bytes(&[version.major, version.minor, *fixed_random_value]);
        writer.write_bytes(
            &[
                *pitch_type_or_season,
                *pitch_type,
                *game_length,
                *auto_replays,
                *all_player_teams_equal,
            ]
            .map(i8::cast_unsigned),
        );
        top_team.write(writer);
        bottom_team.write(writer);
        writer.write_bytes(&[
            *top_team_coach_no,
            *top_team_player_no,
            *bottom_team_coach_no,
            *bottom_team_player_no,
            *extra_time,
            *penalty_shootout,
        ]);
        writer.write_i8(*top_team_marked_player);
        writer.write_i8(*bottom_team_marked_player);
        for tactics in user_tactics {
            tactics.write(writer);
        }
        writer.write_u16_le(*input_controls);
        writer.write_u32_le(*initial_frame);
        if let Some(HeaderV1p2 {
            training_game_career_size,
            season,
            min_substitutes,
            max_substitutes,
        }) = v1p2
        {
            writer.write_u32_le(*training_game_career_size);
            writer.write_i8(*season);
            writer.write_i8(*min_substitutes);
            writer.write_i8(*max_substitutes);
        }
        Ok(())
    }
}
