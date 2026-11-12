//! The 38-byte player record (upstream `PlayerFile`).
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 1 | nationality |
//! | 1 | 1 | runtime: cached match value (signed) |
//! | 2 | 1 | shirt number |
//! | 3 | 23 | name (22 characters + an always-NUL byte) |
//! | 26 | 1 | position (bits 5-7), face (bits 3-4), runtime flags (bits 0-2) |
//! | 27 | 1 | runtime: cards and injuries |
//! | 28 | 1 | passing (low nibble; the high nibble is unused) |
//! | 29 | 1 | velocity (high nibble), heading (low nibble) |
//! | 30 | 1 | tackling (high nibble), control (low nibble) |
//! | 31 | 1 | speed (high nibble), finishing (low nibble) |
//! | 32 | 1 | price id |
//! | 33 | 4 | runtime: goals in league, domestic cup, league cup, continental cup |
//! | 37 | 1 | runtime: form (signed) |
//!
//! The runtime fields are 0 in every file of the PC game; a career save stores the same
//! record with them in use.

use bitflags::bitflags;
use openswos_common::{ByteReader, ByteWriter, FixedName, Position, ReadError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Bytes of a player record.
pub const PLAYER_RECORD_LEN: usize = 38;

/// Bytes of the player-name field: 22 characters and a byte that is always NUL (upstream
/// folds the old `field_19` terminator into the name).
pub const PLAYER_NAME_LEN: usize = 23;

/// Where the position starts in the position-and-face byte: bits 5-7 (upstream
/// `PF_POSITION_SHIFT`).
const POSITION_SHIFT: u32 = 5;

/// Where the face starts in the position-and-face byte: bits 3-4 (upstream
/// `PF_FACE_SHIFT`).
const FACE_SHIFT: u32 = 3;

/// The face's bits once shifted down (upstream `PF_FACE_MASK` = `0x18`).
const FACE_MASK: u8 = 0b11;

/// The runtime flags' bits of the position-and-face byte: bits 0-2.
const FLAGS_MASK: u8 = 0b111;

/// Bits in a nibble: a skill byte holds two skills.
const NIBBLE_BITS: u32 = 4;

/// A nibble's bits.
const NIBBLE_MASK: u8 = 0x0F;

/// The bits of a skill nibble the engine reads: bits 0-2. Many original players have bit
/// 3 set, which the engine ignores (upstream docs/DATA_PIPELINE.md).
const EFFECTIVE_SKILL_MASK: u8 = 0b0111;

/// A player's nationality code. Its meaning (which country) is world data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Nationality(pub u8);

/// A player's base price: an index into the game's price table (0..=49 in the PC game's
/// files; upstream: valid range `0x00..=0x31`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PriceId(pub u8);

/// A player's face and hair (upstream `include/mod_api.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Face {
    /// White.
    White = 0,
    /// Ginger.
    Ginger = 1,
    /// Black.
    Black = 2,
    /// Black with blond hair (World 2026-27 only).
    BlackBlond = 3,
}

/// A code that is not a face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a face")]
pub struct InvalidFace(pub u8);

impl Face {
    /// Every face, in code order.
    pub const ALL: &[Self] = &[Self::White, Self::Ginger, Self::Black, Self::BlackBlond];
}

impl From<Face> for u8 {
    fn from(face: Face) -> Self {
        face as Self
    }
}

impl TryFrom<u8> for Face {
    type Error = InvalidFace;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        Self::ALL
            .iter()
            .copied()
            .find(|face| u8::from(*face) == code)
            .ok_or(InvalidFace(code))
    }
}

bitflags! {
    /// A player's runtime flags (upstream `PF_INFORM`, `PF_LOAN`, `PF_TRIALIST`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct PlayerFlags: u8 {
        /// In form: form gains are doubled.
        const IN_FORM = 0b001;
        /// On loan (status "LOAN").
        const ON_LOAN = 0b010;
        /// Signed as a trialist.
        const TRIALIST = 0b100;
    }
}

/// A 4-bit skill value, as stored: 0..=15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct RawSkill(u8);

/// A value too big for a skill nibble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} does not fit a skill nibble (0..=15)")]
pub struct InvalidSkill(pub u8);

impl RawSkill {
    /// The stored value, 0..=15.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }

    /// The value the engine plays with, 0..=7: bit 3 is ignored.
    #[must_use]
    pub fn effective(self) -> u8 {
        self.0 & EFFECTIVE_SKILL_MASK
    }

    /// The high and low nibbles of a skill byte.
    fn split(byte: u8) -> (Self, Self) {
        (Self(byte >> NIBBLE_BITS), Self(byte & NIBBLE_MASK))
    }

    /// The skill byte holding `high` and `low`.
    fn join(high: Self, low: Self) -> u8 {
        (high.0 << NIBBLE_BITS) | low.0
    }
}

impl TryFrom<u8> for RawSkill {
    type Error = InvalidSkill;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= NIBBLE_MASK {
            Ok(Self(value))
        } else {
            Err(InvalidSkill(value))
        }
    }
}

impl From<RawSkill> for u8 {
    fn from(skill: RawSkill) -> Self {
        skill.0
    }
}

/// A player's seven skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Skills {
    /// Passing.
    pub passing: RawSkill,
    /// Velocity: shot power (the player card's "Ve").
    pub velocity: RawSkill,
    /// Heading.
    pub heading: RawSkill,
    /// Tackling.
    pub tackling: RawSkill,
    /// Ball control.
    pub control: RawSkill,
    /// Speed.
    pub speed: RawSkill,
    /// Finishing.
    pub finishing: RawSkill,
    /// The passing byte's high nibble: unused by the game, 0 in every original file, kept
    /// so the record round-trips.
    pub passing_high: RawSkill,
}

/// Goals scored, per competition slot (upstream `goalStats[4]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GoalCounts {
    /// In the domestic league.
    pub league: u8,
    /// In the domestic cup.
    pub domestic_cup: u8,
    /// In the league cup.
    pub league_cup: u8,
    /// In the continental cup.
    pub continental_cup: u8,
}

/// The fields the game writes while it runs: 0 in the PC game's files, in use in career
/// saves. Their meaning belongs to the game logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerRuntime {
    /// The player's last computed match value (upstream `cachedMatchValue`).
    pub cached_match_value: i8,
    /// In form, on loan, trialist.
    pub flags: PlayerFlags,
    /// Cards (bits 2-4) and injury (bits 5-7).
    pub cards_injuries: u8,
    /// Goals scored this season.
    pub goals: GoalCounts,
    /// Form, -100..=100, or a status sentinel (upstream `PLAYER_FORM_TRIALIST` /
    /// `PLAYER_FORM_RESERVE`).
    pub form: i8,
}

/// One player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Player {
    /// Nationality.
    pub nationality: Nationality,
    /// Shirt number (1..=16 in the PC game's files).
    pub shirt_number: u8,
    /// Name.
    pub name: FixedName<PLAYER_NAME_LEN>,
    /// Role.
    pub position: Position,
    /// Face and hair.
    pub face: Face,
    /// Skills.
    pub skills: Skills,
    /// Base price.
    pub price: PriceId,
    /// Fields the game writes while it runs.
    pub runtime: PlayerRuntime,
}

impl Player {
    /// Reads a player record.
    ///
    /// # Errors
    ///
    /// When the data is too short.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, ReadError> {
        let nationality = Nationality(reader.read_u8()?);
        let cached_match_value = reader.read_i8()?;
        let shirt_number = reader.read_u8()?;
        let name = FixedName::from_bytes(reader.read_array()?);
        let position_and_face = reader.read_u8()?;
        let cards_injuries = reader.read_u8()?;
        let (passing_high, passing) = RawSkill::split(reader.read_u8()?);
        let (velocity, heading) = RawSkill::split(reader.read_u8()?);
        let (tackling, control) = RawSkill::split(reader.read_u8()?);
        let (speed, finishing) = RawSkill::split(reader.read_u8()?);
        let price = PriceId(reader.read_u8()?);
        let [league, domestic_cup, league_cup, continental_cup] = reader.read_array()?;
        let form = reader.read_i8()?;
        Ok(Self {
            nationality,
            shirt_number,
            name,
            position: Position::try_from(position_and_face >> POSITION_SHIFT)
                .expect("the top 3 bits always hold a position"),
            face: Face::try_from((position_and_face >> FACE_SHIFT) & FACE_MASK)
                .expect("2 bits always hold a face"),
            skills: Skills {
                passing,
                velocity,
                heading,
                tackling,
                control,
                speed,
                finishing,
                passing_high,
            },
            price,
            runtime: PlayerRuntime {
                cached_match_value,
                flags: PlayerFlags::from_bits_retain(position_and_face & FLAGS_MASK),
                cards_injuries,
                goals: GoalCounts {
                    league,
                    domestic_cup,
                    league_cup,
                    continental_cup,
                },
                form,
            },
        })
    }

    /// Writes the player record.
    pub fn write(&self, writer: &mut ByteWriter) {
        let Self {
            nationality: Nationality(nationality),
            shirt_number,
            name,
            position,
            face,
            skills:
                Skills {
                    passing,
                    velocity,
                    heading,
                    tackling,
                    control,
                    speed,
                    finishing,
                    passing_high,
                },
            price: PriceId(price),
            runtime:
                PlayerRuntime {
                    cached_match_value,
                    flags,
                    cards_injuries,
                    goals:
                        GoalCounts {
                            league,
                            domestic_cup,
                            league_cup,
                            continental_cup,
                        },
                    form,
                },
        } = *self;
        writer.write_u8(nationality);
        writer.write_i8(cached_match_value);
        writer.write_u8(shirt_number);
        writer.write_bytes(name.as_bytes());
        writer.write_u8(
            (u8::from(position) << POSITION_SHIFT)
                | (u8::from(face) << FACE_SHIFT)
                | (flags.bits() & FLAGS_MASK),
        );
        writer.write_u8(cards_injuries);
        writer.write_u8(RawSkill::join(passing_high, passing));
        writer.write_u8(RawSkill::join(velocity, heading));
        writer.write_u8(RawSkill::join(tackling, control));
        writer.write_u8(RawSkill::join(speed, finishing));
        writer.write_u8(price);
        writer.write_bytes(&[league, domestic_cup, league_cup, continental_cup]);
        writer.write_i8(form);
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size test records
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// LEONARD HASA, the first player of the PC game's TEAM.000, plus runtime values.
    fn record() -> [u8; PLAYER_RECORD_LEN] {
        let mut bytes = [0u8; PLAYER_RECORD_LEN];
        bytes[0] = 12; // nationality
        bytes[1] = 0xFE; // cached match value -2
        bytes[2] = 9; // shirt
        bytes[3..15].copy_from_slice(b"LEONARD HASA");
        bytes[26] = (7 << 5) | (2 << 3) | 0b101; // striker, black, in form + trialist
        bytes[27] = 0x24; // cards and injuries
        bytes[28] = 0x0B; // passing 11
        bytes[29] = 0x3C; // velocity 3, heading 12
        bytes[30] = 0x7F; // tackling 7, control 15
        bytes[31] = 0x81; // speed 8, finishing 1
        bytes[32] = 49; // price id
        bytes[33..37].copy_from_slice(&[1, 2, 3, 4]);
        bytes[37] = 0x9B; // form: the trialist sentinel
        bytes
    }

    fn skill(value: u8) -> RawSkill {
        RawSkill::try_from(value).expect("a nibble")
    }

    #[test]
    fn decodes_every_field() {
        let bytes = record();
        let player = Player::read(&mut ByteReader::new(&bytes)).expect("a whole record");
        assert_eq!(player.nationality, Nationality(12));
        assert_eq!(player.shirt_number, 9);
        assert_eq!(player.name.text(), "LEONARD HASA");
        assert_eq!(player.position, Position::Striker);
        assert_eq!(player.face, Face::Black);
        assert_eq!(
            player.skills,
            Skills {
                passing: skill(11),
                velocity: skill(3),
                heading: skill(12),
                tackling: skill(7),
                control: skill(15),
                speed: skill(8),
                finishing: skill(1),
                passing_high: skill(0),
            }
        );
        assert_eq!(player.price, PriceId(49));
        assert_eq!(
            player.runtime,
            PlayerRuntime {
                cached_match_value: -2,
                flags: PlayerFlags::IN_FORM | PlayerFlags::TRIALIST,
                cards_injuries: 0x24,
                goals: GoalCounts {
                    league: 1,
                    domestic_cup: 2,
                    league_cup: 3,
                    continental_cup: 4,
                },
                form: -101,
            }
        );
    }

    #[test]
    fn encodes_back_the_same_bytes() {
        let bytes = record();
        let player = Player::read(&mut ByteReader::new(&bytes)).expect("a whole record");
        let mut writer = ByteWriter::new();
        player.write(&mut writer);
        assert_eq!(writer.into_bytes(), bytes.to_vec());
    }

    #[test]
    fn a_short_record_fails() {
        let bytes = record();
        let error = Player::read(&mut ByteReader::new(&bytes[..37])).expect_err("one byte short");
        assert!(
            matches!(error, ReadError::UnexpectedEnd { offset: 37, .. }),
            "{error}"
        );
    }

    #[test]
    fn skills_keep_bit_3_but_the_engine_ignores_it() {
        assert_eq!(skill(12).value(), 12);
        assert_eq!(skill(12).effective(), 4);
        assert_eq!(skill(7).effective(), 7);
        assert_eq!(RawSkill::try_from(16), Err(InvalidSkill(16)));
        assert_eq!(u8::from(skill(15)), 15);
    }

    #[test]
    fn faces_follow_their_codes() {
        for (code, face) in (0u8..).zip(Face::ALL.iter().copied()) {
            assert_eq!(Face::try_from(code), Ok(face));
            assert_eq!(u8::from(face), code);
        }
        assert_eq!(Face::try_from(4), Err(InvalidFace(4)));
    }

    #[test]
    fn serializes_readably() {
        let bytes = record();
        let player = Player::read(&mut ByteReader::new(&bytes)).expect("a whole record");
        let json = serde_json::to_value(player).expect("serializes");
        assert_eq!(json["name"], "LEONARD HASA");
        assert_eq!(json["position"], "striker");
        assert_eq!(json["face"], "black");
        assert_eq!(json["skills"]["heading"], 12);
        assert_eq!(json["runtime"]["flags"], "IN_FORM | TRIALIST");
        let back: Player = serde_json::from_value(json).expect("deserializes");
        assert_eq!(back, player);
    }

    #[test]
    fn json_rejects_skills_beyond_a_nibble() {
        assert!(serde_json::from_str::<RawSkill>("16").is_err());
        assert_eq!(serde_json::from_str::<RawSkill>("15").ok(), Some(skill(15)));
    }

    proptest! {
        #[test]
        fn any_record_round_trips(bytes: [u8; PLAYER_RECORD_LEN]) {
            let player = Player::read(&mut ByteReader::new(&bytes)).expect("a whole record");
            let mut writer = ByteWriter::new();
            player.write(&mut writer);
            prop_assert_eq!(writer.into_bytes(), bytes.to_vec());
        }
    }
}
