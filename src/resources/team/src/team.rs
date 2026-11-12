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

use bitflags::bitflags;
use openswos_common::{
    ByteReader, ByteWriter, FixedName, InvalidTactics, ReadError, Tactics, TeamId,
};
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
///
/// The game does not read it from a file: it sets it when it loads one, to its team file's
/// base plus the team's ordinal (upstream `SetTeamGlobalNumbers`). So it is kept as stored.
/// Faithful to the original: the PC game's `.EDT` files store it little-endian (the DOS
/// game's own byte order), unlike every other team file, so read big-endian it is no
/// index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GlobalTeamNumber(pub i16);

/// Which of its country's divisions the team plays in, 0 being the top one (upstream
/// `league`; 0..=4 in the PC game's files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Division(pub u8);

/// A kit colour (upstream `KIT_COLOURS`, include/team_colors.h; the names are upstream
/// docs/CUSTOM_TEAMS.md's). The game's kit-clash rules tell them apart; how each one is
/// drawn belongs to the view. The PC game's files use the first ten; purple comes from the
/// SWOS 96/97 Mod (World 2026-27).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum KitColour {
    /// Grey.
    Grey = 0,
    /// White.
    White = 1,
    /// Black.
    Black = 2,
    /// Orange.
    Orange = 3,
    /// Red.
    Red = 4,
    /// Blue.
    Blue = 5,
    /// Brown (upstream `KIT_MAROON`).
    Brown = 6,
    /// Light blue.
    LightBlue = 7,
    /// Green.
    Green = 8,
    /// Yellow.
    Yellow = 9,
    /// Purple (upstream `KIT_PURPLE`).
    Purple = 10,
}

/// A code that is not a kit colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a kit colour")]
pub struct InvalidKitColour(pub u8);

openswos_common::byte_enum!(
    KitColour,
    InvalidKitColour,
    [
        Grey, White, Black, Orange, Red, Blue, Brown, LightBlue, Green, Yellow, Purple,
    ]
);

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

openswos_common::byte_enum!(
    ShirtType,
    InvalidShirtType,
    [Plain, ColouredSleeves, VerticalStripes, HorizontalStripes]
);

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
            stripes: KitColour::try_from(stripes)?,
            basic: KitColour::try_from(basic)?,
            shorts: KitColour::try_from(shorts)?,
            socks: KitColour::try_from(socks)?,
        })
    }

    fn write(&self, writer: &mut ByteWriter) {
        let Self {
            shirt_type,
            stripes,
            basic,
            shorts,
            socks,
        } = *self;
        writer.write_bytes(&[
            u8::from(shirt_type),
            u8::from(stripes),
            u8::from(basic),
            u8::from(shorts),
            u8::from(socks),
        ]);
    }
}

/// Who controls a team in the competition being played (upstream `teamControls`:
/// `TEAM_NOT_SELECTED` in src/teams.c, `TEAM_COMPUTER`..`TEAM_COACH` in
/// include/play_match_menu.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum TeamControls {
    /// Not playing: the value of every team file.
    NotSelected = 0,
    /// The computer.
    Computer = 1,
    /// A human, as player and coach.
    PlayerCoach = 2,
    /// A human, as coach only.
    Coach = 3,
}

/// A code that is not a team control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a team control")]
pub struct InvalidTeamControls(pub u8);

openswos_common::byte_enum!(
    TeamControls,
    InvalidTeamControls,
    [NotSelected, Computer, PlayerCoach, Coach]
);

/// Whether a team's cached player match values are up to date (upstream
/// `playerValuesDirty`, `TEAM_VALUES_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum PlayerValues {
    /// Up to date (upstream `TEAM_VALUES_CLEAN`).
    Clean = 0,
    /// The squad changed: recompute them before use (upstream `TEAM_VALUES_DIRTY`).
    Dirty = 1,
}

/// A code that is not a player-values state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is neither clean (0) nor dirty (1) player values")]
pub struct InvalidPlayerValues(pub u8);

openswos_common::byte_enum!(PlayerValues, InvalidPlayerValues, [Clean, Dirty]);

bitflags! {
    /// A team's match flags (upstream `andWith0xFE`). Bits without a name are kept as
    /// read.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct MatchFlags: u8 {
        /// The team has played a training match: set for both sides (upstream
        /// src/game_init.c), cleared when the team is initialised (src/match_sim.c) and
        /// when a competition result is computed (src/diy_competition.c). The squad menu
        /// disables an entry while it is set (src/menus.c).
        const TRAINED = 0b1;
        // The other bits are unnamed but kept.
        const _ = !0;
    }
}

impl From<u8> for MatchFlags {
    fn from(bits: u8) -> Self {
        Self::from_bits_retain(bits)
    }
}

impl From<MatchFlags> for u8 {
    fn from(flags: MatchFlags) -> Self {
        flags.bits()
    }
}

/// The header fields the game writes while it runs: unset in the PC game's files, in use
/// in career saves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamRuntime {
    /// Who controls the team.
    pub controls: TeamControls,
    /// Own goals conceded in the last match (upstream `writesZeroHere`).
    pub own_goals: u8,
    /// Match flags.
    pub match_flags: MatchFlags,
    /// Whether the players' cached match values are up to date.
    pub player_values: PlayerValues,
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
    /// A kit colour is not one.
    #[error(transparent)]
    KitColour(#[from] InvalidKitColour),
    /// The tactics are not a slot.
    #[error(transparent)]
    Tactics(#[from] InvalidTactics),
    /// The team control is not one.
    #[error(transparent)]
    TeamControls(#[from] InvalidTeamControls),
    /// The player-values state is neither clean nor dirty.
    #[error(transparent)]
    PlayerValues(#[from] InvalidPlayerValues),
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
    pub tactics: Tactics,
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
    /// When the data is too short or a code (shirt type, kit colour, tactics, team control,
    /// player-values state) is not one.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, RecordError> {
        let id = TeamId::from_bytes(reader.read_array()?);
        let global_number = GlobalTeamNumber(reader.read_i16_be()?);
        let controls = TeamControls::try_from(reader.read_u8()?)?;
        let name = FixedName::from_bytes(reader.read_array()?);
        let own_goals = reader.read_u8()?;
        let match_flags = MatchFlags::from(reader.read_u8()?);
        let tactics = Tactics::try_from(reader.read_u8()?)?;
        let division = Division(reader.read_u8()?);
        let primary_kit = Kit::read(reader)?;
        let secondary_kit = Kit::read(reader)?;
        let coach = FixedName::from_bytes(reader.read_array()?);
        let player_values = PlayerValues::try_from(reader.read_u8()?)?;
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
                player_values,
            },
        })
    }

    /// Writes the team record.
    pub fn write(&self, writer: &mut ByteWriter) {
        let Self {
            id,
            global_number: GlobalTeamNumber(global_number),
            name,
            tactics,
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
                    player_values,
                },
        } = self;
        writer.write_bytes(&id.to_bytes());
        writer.write_i16_be(*global_number);
        writer.write_u8(u8::from(*controls));
        writer.write_bytes(name.as_bytes());
        writer.write_u8(*own_goals);
        writer.write_u8(u8::from(*match_flags));
        writer.write_u8(u8::from(*tactics));
        writer.write_u8(*division);
        primary_kit.write(writer);
        secondary_kit.write(writer);
        writer.write_bytes(coach.as_bytes());
        writer.write_u8(u8::from(*player_values));
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

    /// Offset of the team control in a record.
    const CONTROLS_AT: usize = 4;

    /// Offset of the tactics in a record.
    const TACTICS_AT: usize = 24;

    /// Offsets of the kits' shirt types in a record.
    const SHIRT_TYPES_AT: [usize; 2] = [26, 31];

    /// Offsets of the kits' colours in a record.
    const KIT_COLOURS_AT: [usize; 8] = [27, 28, 29, 30, 32, 33, 34, 35];

    /// Offset of the player-values state in a record.
    const PLAYER_VALUES_AT: usize = 59;

    /// A team record whose every header byte is told apart, with valid codes.
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
        assert_eq!(team.tactics, Tactics::FourThreeThree);
        assert_eq!(team.division, Division(3));
        assert_eq!(
            team.primary_kit,
            Kit {
                shirt_type: ShirtType::HorizontalStripes,
                stripes: KitColour::Red,
                basic: KitColour::White,
                shorts: KitColour::Black,
                socks: KitColour::Yellow,
            }
        );
        assert_eq!(
            team.secondary_kit,
            Kit {
                shirt_type: ShirtType::Plain,
                stripes: KitColour::Blue,
                basic: KitColour::Blue,
                shorts: KitColour::Blue,
                socks: KitColour::Blue,
            }
        );
        assert_eq!(team.coach.text(), "ROY AITKEN");
        assert_eq!(team.player_order[2], 5);
        assert_eq!(
            team.runtime,
            TeamRuntime {
                controls: TeamControls::PlayerCoach,
                own_goals: 1,
                match_flags: MatchFlags::TRAINED,
                player_values: PlayerValues::Dirty,
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
    fn an_invalid_code_is_reported() {
        let cases = [
            (
                SHIRT_TYPES_AT[1],
                4,
                RecordError::ShirtType(InvalidShirtType(4)),
                "4 is not a shirt type",
            ),
            (
                KIT_COLOURS_AT[7],
                11,
                RecordError::KitColour(InvalidKitColour(11)),
                "11 is not a kit colour",
            ),
            (
                TACTICS_AT,
                18,
                RecordError::Tactics(InvalidTactics(18)),
                "18 is not a tactic slot",
            ),
            (
                CONTROLS_AT,
                4,
                RecordError::TeamControls(InvalidTeamControls(4)),
                "4 is not a team control",
            ),
            (
                PLAYER_VALUES_AT,
                2,
                RecordError::PlayerValues(InvalidPlayerValues(2)),
                "2 is neither clean (0) nor dirty (1) player values",
            ),
        ];
        for (offset, value, expected, message) in cases {
            let mut bytes = record();
            bytes[offset] = value;
            let error = Team::read(&mut ByteReader::new(&bytes)).expect_err(message);
            assert_eq!(error, expected);
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn match_flags_keep_their_unnamed_bits() {
        let flags = MatchFlags::from(0b1000_0001);
        assert!(flags.contains(MatchFlags::TRAINED));
        assert_eq!(u8::from(flags), 0b1000_0001);
        assert_eq!(u8::from(MatchFlags::empty()), 0);
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

    /// Every variant of `all` has its index as its code.
    fn assert_codes_in_order<T>(all: &[T])
    where
        T: Copy + PartialEq + std::fmt::Debug + Into<u8> + TryFrom<u8>,
        <T as TryFrom<u8>>::Error: std::fmt::Debug,
    {
        for (code, value) in (0u8..).zip(all.iter().copied()) {
            assert_eq!(value.into(), code);
            assert_eq!(T::try_from(code).ok(), Some(value));
        }
    }

    #[test]
    fn codes_follow_upstream() {
        assert_codes_in_order(ShirtType::ALL);
        assert_codes_in_order(KitColour::ALL);
        assert_codes_in_order(Tactics::ALL);
        assert_codes_in_order(TeamControls::ALL);
        assert_codes_in_order(PlayerValues::ALL);
        assert_eq!(KitColour::ALL.len(), 11, "upstream KIT_COLOURS");
        assert_eq!(u8::from(KitColour::Brown), 6, "upstream KIT_MAROON");
        assert_eq!(u8::from(KitColour::Purple), 10, "upstream KIT_PURPLE");
        assert_eq!(u8::from(Tactics::UserF), 17, "upstream TACTIC_USER_F");
        assert_eq!(u8::from(TeamControls::Coach), 3, "upstream TEAM_COACH");
        assert_eq!(ShirtType::try_from(4), Err(InvalidShirtType(4)));
    }

    #[test]
    fn codes_serialize_by_name() {
        let kit = Kit {
            shirt_type: ShirtType::VerticalStripes,
            stripes: KitColour::LightBlue,
            basic: KitColour::White,
            shorts: KitColour::Black,
            socks: KitColour::Purple,
        };
        let json = serde_json::to_value(kit).expect("serializes");
        assert_eq!(json["stripes"], "light_blue");
        assert_eq!(json["socks"], "purple");
        assert_eq!(
            serde_json::to_value(Tactics::FourFourTwo).expect("serializes"),
            "four_four_two"
        );
        assert_eq!(
            serde_json::to_value(TeamControls::PlayerCoach).expect("serializes"),
            "player_coach"
        );
        let back: Kit = serde_json::from_value(json).expect("deserializes");
        assert_eq!(back, kit);
    }

    #[test]
    fn the_record_is_684_bytes() {
        assert_eq!(TEAM_RECORD_LEN, 684);
    }

    /// The number of codes of a code enum.
    fn code_count<T>(all: &[T]) -> u8 {
        u8::try_from(all.len()).expect("fewer than 256 codes")
    }

    /// Any record whose codes are valid.
    fn any_record() -> impl Strategy<Value = Vec<u8>> {
        (
            proptest::collection::vec(any::<u8>(), TEAM_RECORD_LEN),
            0..code_count(TeamControls::ALL),
            0..code_count(Tactics::ALL),
            proptest::array::uniform2(0..code_count(ShirtType::ALL)),
            proptest::array::uniform8(0..code_count(KitColour::ALL)),
            0..code_count(PlayerValues::ALL),
        )
            .prop_map(
                |(mut bytes, controls, tactics, shirt_types, colours, values)| {
                    bytes[CONTROLS_AT] = controls;
                    bytes[TACTICS_AT] = tactics;
                    for (offset, shirt_type) in SHIRT_TYPES_AT.into_iter().zip(shirt_types) {
                        bytes[offset] = shirt_type;
                    }
                    for (offset, colour) in KIT_COLOURS_AT.into_iter().zip(colours) {
                        bytes[offset] = colour;
                    }
                    bytes[PLAYER_VALUES_AT] = values;
                    bytes
                },
            )
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
