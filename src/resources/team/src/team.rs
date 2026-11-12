//! The 684-byte team record (upstream `TeamFileEx`): a 76-byte header (upstream
//! `TeamFile`) and 16 player records.
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 2 | team id: file number, ordinal |
//! | 2 | 2 | global team number (big-endian, signed) |
//! | 4 | 1 | runtime: who controls the team |
//! | 5 | 17 | name |
//! | 22 | 1 | runtime: own goals (upstream `writesZeroHere`) |
//! | 23 | 1 | runtime: match flags (upstream `andWith0xFE`) |
//! | 24 | 1 | tactics |
//! | 25 | 1 | division (upstream `league`) |
//! | 26 | 5 | primary kit: shirt type, stripes, basic, shorts, socks |
//! | 31 | 5 | secondary kit |
//! | 36 | 23 | coach name |
//! | 59 | 1 | runtime: player values dirty (upstream `playerValuesDirty`) |
//! | 60 | 16 | player order |
//! | 76 | 16 × 38 | players ([`Player`]) |

use openswos_common::{ByteReader, ByteWriter, FixedName, ReadError, TeamId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::player::{PLAYER_RECORD_LEN, Player};

/// Players in a team record.
pub const PLAYERS_PER_TEAM: usize = 16;

/// Bytes of a team record's header (upstream `TeamFile`).
pub const TEAM_HEADER_LEN: usize = 76;

/// Bytes of a team record: the header and its players (upstream `TeamFileEx`).
pub const TEAM_RECORD_LEN: usize = TEAM_HEADER_LEN + PLAYERS_PER_TEAM * PLAYER_RECORD_LEN;

/// Bytes of the team-name field.
pub const TEAM_NAME_LEN: usize = 17;

/// Bytes of the coach-name field.
pub const COACH_NAME_LEN: usize = 23;

/// A team's index across every team file of the game (upstream `globalTeamNumber`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GlobalTeamNumber(pub i16);

/// A team's tactics: an index into the game's tactics (0..=9 in the PC game's files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TacticsIndex(pub u8);

/// Which of its country's divisions the team plays in, 0 being the top one (upstream
/// `league`; 0..=4 in the PC game's files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Division(pub u8);

/// A kit colour: an index into the game's kit colours (0..=9 in the PC game's files; World
/// 2026-27 adds 10, purple). Which colour each one is belongs to the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KitColour(pub u8);

/// A shirt's pattern (upstream `SHIRT_*`, include/team_colors.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ShirtType {
    /// One colour.
    Plain = 0,
    /// Sleeves in the second colour.
    ColouredSleeves = 1,
    /// Vertical stripes.
    VerticalStripes = 2,
    /// Hoops.
    HorizontalStripes = 3,
}

/// A code that is not a shirt type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a shirt type")]
pub struct InvalidShirtType(pub u8);

impl ShirtType {
    /// Every shirt type, in code order.
    pub const ALL: &[Self] = &[
        Self::Plain,
        Self::ColouredSleeves,
        Self::VerticalStripes,
        Self::HorizontalStripes,
    ];
}

impl From<ShirtType> for u8 {
    fn from(shirt_type: ShirtType) -> Self {
        shirt_type as Self
    }
}

impl TryFrom<u8> for ShirtType {
    type Error = InvalidShirtType;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        Self::ALL
            .iter()
            .copied()
            .find(|shirt_type| u8::from(*shirt_type) == code)
            .ok_or(InvalidShirtType(code))
    }
}

/// A kit, as a team record stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Kit {
    /// The shirt's pattern.
    pub shirt_type: ShirtType,
    /// The shirt's second colour: stripes, sleeves or hoops.
    pub stripes: KitColour,
    /// The shirt's main colour.
    pub basic: KitColour,
    /// The shorts' colour.
    pub shorts: KitColour,
    /// The socks' colour.
    pub socks: KitColour,
}

impl Kit {
    fn read(reader: &mut ByteReader<'_>) -> Result<Self, RecordError> {
        let [shirt_type, stripes, basic, shorts, socks] = reader.read_array()?;
        Ok(Self {
            shirt_type: ShirtType::try_from(shirt_type)?,
            stripes: KitColour(stripes),
            basic: KitColour(basic),
            shorts: KitColour(shorts),
            socks: KitColour(socks),
        })
    }

    fn write(&self, writer: &mut ByteWriter) {
        let Self {
            shirt_type,
            stripes: KitColour(stripes),
            basic: KitColour(basic),
            shorts: KitColour(shorts),
            socks: KitColour(socks),
        } = *self;
        writer.write_bytes(&[u8::from(shirt_type), stripes, basic, shorts, socks]);
    }
}

/// The header fields the game writes while it runs: 0 in the PC game's files, in use in
/// career saves. Their meaning belongs to the game logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamRuntime {
    /// Who controls the team (upstream `teamControls`; include/play_match_menu.h names 1
    /// computer, 2 player-coach, 3 coach).
    pub controls: u8,
    /// Own goals conceded in the last match (upstream `writesZeroHere`).
    pub own_goals: u8,
    /// Match flags: bit 0 is set for both sides of a played match (upstream
    /// `andWith0xFE`).
    pub match_flags: u8,
    /// Whether the players' cached match values must be recomputed: 0 clean, 1 dirty
    /// (upstream `playerValuesDirty`).
    pub values_dirty: u8,
}

/// Why a team record does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecordError {
    /// The data ended inside the record.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// A kit's shirt type is not one.
    #[error(transparent)]
    ShirtType(#[from] InvalidShirtType),
}

/// One team and its squad.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Team {
    /// The team's identity: its file and its place in it.
    pub id: TeamId,
    /// Its index across every team file.
    pub global_number: GlobalTeamNumber,
    /// Its name.
    pub name: FixedName<TEAM_NAME_LEN>,
    /// Its tactics.
    pub tactics: TacticsIndex,
    /// Its division.
    pub division: Division,
    /// Its first kit.
    pub primary_kit: Kit,
    /// Its second kit.
    pub secondary_kit: Kit,
    /// Its coach.
    pub coach: FixedName<COACH_NAME_LEN>,
    /// The order of the squad: each entry is the index of a player record (a permutation
    /// of 0..16 in every original file; upstream `playerNumbers`).
    pub player_order: [u8; PLAYERS_PER_TEAM],
    /// The squad.
    pub players: [Player; PLAYERS_PER_TEAM],
    /// Fields the game writes while it runs.
    pub runtime: TeamRuntime,
}

impl Team {
    /// Reads a team record.
    ///
    /// # Errors
    ///
    /// When the data is too short or a shirt type is invalid.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, RecordError> {
        let id = TeamId::from_bytes(reader.read_array()?);
        let global_number = GlobalTeamNumber(reader.read_i16_be()?);
        let controls = reader.read_u8()?;
        let name = FixedName::from_bytes(reader.read_array()?);
        let own_goals = reader.read_u8()?;
        let match_flags = reader.read_u8()?;
        let tactics = TacticsIndex(reader.read_u8()?);
        let division = Division(reader.read_u8()?);
        let primary_kit = Kit::read(reader)?;
        let secondary_kit = Kit::read(reader)?;
        let coach = FixedName::from_bytes(reader.read_array()?);
        let values_dirty = reader.read_u8()?;
        let player_order = reader.read_array()?;
        let players: Vec<Player> = (0..PLAYERS_PER_TEAM)
            .map(|_| Player::read(reader))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            id,
            global_number,
            name,
            tactics,
            division,
            primary_kit,
            secondary_kit,
            coach,
            player_order,
            players: players
                .try_into()
                .expect("exactly PLAYERS_PER_TEAM players were read"),
            runtime: TeamRuntime {
                controls,
                own_goals,
                match_flags,
                values_dirty,
            },
        })
    }

    /// Writes the team record.
    pub fn write(&self, writer: &mut ByteWriter) {
        let Self {
            id,
            global_number: GlobalTeamNumber(global_number),
            name,
            tactics: TacticsIndex(tactics),
            division: Division(division),
            primary_kit,
            secondary_kit,
            coach,
            player_order,
            players,
            runtime:
                TeamRuntime {
                    controls,
                    own_goals,
                    match_flags,
                    values_dirty,
                },
        } = self;
        writer.write_bytes(&id.to_bytes());
        writer.write_i16_be(*global_number);
        writer.write_u8(*controls);
        writer.write_bytes(name.as_bytes());
        writer.write_u8(*own_goals);
        writer.write_u8(*match_flags);
        writer.write_u8(*tactics);
        writer.write_u8(*division);
        primary_kit.write(writer);
        secondary_kit.write(writer);
        writer.write_bytes(coach.as_bytes());
        writer.write_u8(*values_dirty);
        writer.write_bytes(player_order);
        for player in players {
            player.write(writer);
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size test records
mod tests {
    use super::*;
    use openswos_common::{TeamFileNumber, TeamOrdinal};
    use proptest::prelude::*;

    /// A team record whose every header byte is told apart, with valid shirt types.
    fn record() -> Vec<u8> {
        let mut bytes = vec![0u8; TEAM_RECORD_LEN];
        bytes[0] = 33; // file
        bytes[1] = 7; // ordinal
        bytes[2..4].copy_from_slice(&1360i16.to_be_bytes());
        bytes[4] = 2; // controls
        bytes[5..22].copy_from_slice(b"BORUSSIA MOENCHEN"); // a full 17-byte name
        bytes[22] = 1; // own goals
        bytes[23] = 1; // match flags
        bytes[24] = 5; // tactics
        bytes[25] = 3; // division
        bytes[26..31].copy_from_slice(&[3, 4, 1, 2, 9]);
        bytes[31..36].copy_from_slice(&[0, 5, 5, 5, 5]);
        bytes[36..46].copy_from_slice(b"ROY AITKEN");
        bytes[59] = 1; // values dirty
        bytes[60..76].copy_from_slice(&[0, 2, 5, 6, 3, 8, 10, 11, 9, 14, 15, 1, 4, 7, 12, 13]);
        for (index, player) in bytes[TEAM_HEADER_LEN..]
            .chunks_mut(PLAYER_RECORD_LEN)
            .enumerate()
        {
            player[2] = u8::try_from(index + 1).expect("16 shirts"); // shirt number
        }
        bytes
    }

    #[test]
    fn decodes_every_header_field() {
        let bytes = record();
        let team = Team::read(&mut ByteReader::new(&bytes)).expect("a whole record");
        assert_eq!(
            team.id,
            TeamId {
                file: TeamFileNumber(33),
                ordinal: TeamOrdinal(7)
            }
        );
        assert_eq!(team.global_number, GlobalTeamNumber(1360));
        assert_eq!(team.name.text(), "BORUSSIA MOENCHEN");
        assert_eq!(team.tactics, TacticsIndex(5));
        assert_eq!(team.division, Division(3));
        assert_eq!(
            team.primary_kit,
            Kit {
                shirt_type: ShirtType::HorizontalStripes,
                stripes: KitColour(4),
                basic: KitColour(1),
                shorts: KitColour(2),
                socks: KitColour(9),
            }
        );
        assert_eq!(team.secondary_kit.shirt_type, ShirtType::Plain);
        assert_eq!(team.coach.text(), "ROY AITKEN");
        assert_eq!(team.player_order[2], 5);
        assert_eq!(
            team.runtime,
            TeamRuntime {
                controls: 2,
                own_goals: 1,
                match_flags: 1,
                values_dirty: 1,
            }
        );
        let shirts: Vec<u8> = team
            .players
            .iter()
            .map(|player| player.shirt_number)
            .collect();
        assert_eq!(shirts, (1..=16).collect::<Vec<u8>>());
    }

    #[test]
    fn encodes_back_the_same_bytes() {
        let bytes = record();
        let team = Team::read(&mut ByteReader::new(&bytes)).expect("a whole record");
        let mut writer = ByteWriter::with_capacity(TEAM_RECORD_LEN);
        team.write(&mut writer);
        assert_eq!(writer.into_bytes(), bytes);
    }

    #[test]
    fn an_invalid_shirt_type_is_reported() {
        let mut bytes = record();
        bytes[31] = 4;
        let error = Team::read(&mut ByteReader::new(&bytes)).expect_err("shirt type 4");
        assert_eq!(error, RecordError::ShirtType(InvalidShirtType(4)));
        assert_eq!(error.to_string(), "4 is not a shirt type");
    }

    #[test]
    fn a_short_record_fails() {
        let bytes = record();
        let error =
            Team::read(&mut ByteReader::new(&bytes[..TEAM_RECORD_LEN - 1])).expect_err("short");
        assert!(
            matches!(error, RecordError::Read(ReadError::UnexpectedEnd { .. })),
            "{error}"
        );
    }

    #[test]
    fn shirt_types_follow_their_codes() {
        for (code, shirt_type) in (0u8..).zip(ShirtType::ALL.iter().copied()) {
            assert_eq!(ShirtType::try_from(code), Ok(shirt_type));
            assert_eq!(u8::from(shirt_type), code);
        }
        assert_eq!(ShirtType::try_from(4), Err(InvalidShirtType(4)));
    }

    #[test]
    fn the_record_is_684_bytes() {
        assert_eq!(TEAM_RECORD_LEN, 684);
    }

    /// Any record whose shirt types are valid.
    fn any_record() -> impl Strategy<Value = Vec<u8>> {
        (
            proptest::collection::vec(any::<u8>(), TEAM_RECORD_LEN),
            0u8..4,
            0u8..4,
        )
            .prop_map(|(mut bytes, primary, secondary)| {
                bytes[26] = primary;
                bytes[31] = secondary;
                bytes
            })
    }

    proptest! {
        #[test]
        fn any_valid_record_round_trips(bytes in any_record()) {
            let team = Team::read(&mut ByteReader::new(&bytes)).expect("a valid record");
            let mut writer = ByteWriter::new();
            team.write(&mut writer);
            prop_assert_eq!(writer.into_bytes(), bytes);
        }
    }
}
