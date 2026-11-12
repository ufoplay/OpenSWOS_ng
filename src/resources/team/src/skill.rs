//! A player's skills. A record stores each one in a nibble (4 bits): a level the match
//! plays with (bits 0-2) and a marker (bit 3).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Bits in a nibble: a skill byte holds two.
const NIBBLE_BITS: u32 = 4;

/// A nibble's bits.
const NIBBLE_MASK: u8 = 0x0F;

/// The level's bits in a skill nibble: bits 0-2, all the match engine reads (upstream
/// `match_sim.c` masks the four skill bytes with `0x07777777`).
const LEVEL_MASK: u8 = 0b0111;

/// The marker's bit in a skill nibble: bit 3 (upstream `transfer_market.c`,
/// `playerMarketPlayerMatchesFilters`).
const MARKER_BIT: u8 = 0b1000;

/// Four bits as a record stores them: half a byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Nibble(u8);

/// A value too big for a nibble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} does not fit a nibble (0..=15)")]
pub struct InvalidNibble(pub u8);

impl Nibble {
    /// All bits clear.
    pub const ZERO: Self = Self(0);

    /// The value, 0..=15.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }

    /// The high and low nibbles of a byte.
    #[must_use]
    pub fn split(byte: u8) -> (Self, Self) {
        (Self(byte >> NIBBLE_BITS), Self(byte & NIBBLE_MASK))
    }

    /// The byte holding `high` and `low`.
    #[must_use]
    pub fn join(high: Self, low: Self) -> u8 {
        (high.0 << NIBBLE_BITS) | low.0
    }
}

impl TryFrom<u8> for Nibble {
    type Error = InvalidNibble;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= NIBBLE_MASK {
            Ok(Self(value))
        } else {
            Err(InvalidNibble(value))
        }
    }
}

impl From<Nibble> for u8 {
    fn from(nibble: Nibble) -> Self {
        nibble.0
    }
}

/// A skill level: 0..=7, what the match plays with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct SkillLevel(u8);

/// A value that is not a skill level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a skill level (0..=7)")]
pub struct InvalidSkillLevel(pub u8);

impl SkillLevel {
    /// The level, 0..=7.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for SkillLevel {
    type Error = InvalidSkillLevel;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= LEVEL_MASK {
            Ok(Self(value))
        } else {
            Err(InvalidSkillLevel(value))
        }
    }
}

impl From<SkillLevel> for u8 {
    fn from(level: SkillLevel) -> Self {
        level.0
    }
}

/// A skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Skill {
    /// The level the match plays with (bits 0-2).
    pub level: SkillLevel,
    /// The marker (bit 3). The match ignores it. The transfer market's skill search lists
    /// only players whose searched skill is marked, and the trialists the game generates
    /// are marked on their position's skills (upstream `_trialistSkillBitMasks`). Most
    /// outfield players of the PC game's files have two marked skills.
    pub marked: bool,
}

/// A number that is not a skill's nibble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a skill (0..=15: a level 0..=7, plus 8 when marked)")]
pub struct InvalidSkill(pub u8);

impl Skill {
    /// The skills a byte holds: (high nibble, low nibble).
    #[must_use]
    pub fn split(byte: u8) -> (Self, Self) {
        let (high, low) = Nibble::split(byte);
        (Self::from(high), Self::from(low))
    }

    /// The byte holding `high` and `low`.
    #[must_use]
    pub fn join(high: Self, low: Self) -> u8 {
        Nibble::join(Nibble::from(high), Nibble::from(low))
    }
}

impl From<Nibble> for Skill {
    fn from(nibble: Nibble) -> Self {
        Self {
            level: SkillLevel(nibble.0 & LEVEL_MASK),
            marked: nibble.0 & MARKER_BIT != 0,
        }
    }
}

impl From<Skill> for Nibble {
    fn from(skill: Skill) -> Self {
        let Skill { level, marked } = skill;
        Self(if marked {
            level.0 | MARKER_BIT
        } else {
            level.0
        })
    }
}

impl TryFrom<u8> for Skill {
    type Error = InvalidSkill;

    /// A skill from its nibble's value, 0..=15.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Nibble::try_from(value)
            .map(Self::from)
            .map_err(|_| InvalidSkill(value))
    }
}

impl From<Skill> for u8 {
    /// The skill's nibble value, 0..=15.
    fn from(skill: Skill) -> Self {
        Nibble::from(skill).0
    }
}

/// A player's seven skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Skills {
    /// Passing.
    pub passing: Skill,
    /// Velocity: shot power (the player card's "Ve").
    pub velocity: Skill,
    /// Heading.
    pub heading: Skill,
    /// Tackling.
    pub tackling: Skill,
    /// Ball control.
    pub control: Skill,
    /// Speed.
    pub speed: Skill,
    /// Finishing.
    pub finishing: Skill,
    /// The passing byte's high nibble: unused by the game, 0 in every original file, kept
    /// so the record round-trips.
    pub passing_high: Nibble,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn skill(level: u8, marked: bool) -> Skill {
        Skill {
            level: SkillLevel::try_from(level).expect("a level"),
            marked,
        }
    }

    #[test]
    fn a_nibble_is_a_level_and_a_marker() {
        assert_eq!(Skill::try_from(15), Ok(skill(7, true)));
        assert_eq!(Skill::try_from(7), Ok(skill(7, false)));
        assert_eq!(Skill::try_from(14), Ok(skill(6, true)));
        assert_eq!(Skill::try_from(0), Ok(skill(0, false)));
        assert_eq!(u8::from(skill(4, true)), 12);
        assert_eq!(u8::from(skill(4, false)), 4);
    }

    #[test]
    fn values_beyond_their_range_are_rejected() {
        assert_eq!(Skill::try_from(16), Err(InvalidSkill(16)));
        assert_eq!(
            InvalidSkill(16).to_string(),
            "16 is not a skill (0..=15: a level 0..=7, plus 8 when marked)"
        );
        assert_eq!(SkillLevel::try_from(8), Err(InvalidSkillLevel(8)));
        assert_eq!(
            InvalidSkillLevel(8).to_string(),
            "8 is not a skill level (0..=7)"
        );
        assert_eq!(Nibble::try_from(16), Err(InvalidNibble(16)));
        assert_eq!(
            InvalidNibble(16).to_string(),
            "16 does not fit a nibble (0..=15)"
        );
    }

    #[test]
    fn a_byte_holds_two_skills() {
        // PAUL OKON's tackling-and-control byte (assets/TEAM/TEAM.085).
        assert_eq!(Skill::split(0x3E), (skill(3, false), skill(6, true)));
        assert_eq!(Skill::join(skill(3, false), skill(6, true)), 0x3E);
        assert_eq!(Nibble::split(0x3E), (Nibble(3), Nibble(14)));
        assert_eq!(Nibble::ZERO.value(), 0);
        assert_eq!(SkillLevel::try_from(5).map(SkillLevel::value), Ok(5));
    }

    #[test]
    fn serializes_as_level_and_marker() {
        let json = serde_json::to_value(skill(6, true)).expect("serializes");
        assert_eq!(json, serde_json::json!({ "level": 6, "marked": true }));
        assert_eq!(
            serde_json::from_value::<Skill>(json).expect("deserializes"),
            skill(6, true)
        );
        assert!(serde_json::from_str::<Skill>(r#"{"level": 8, "marked": false}"#).is_err());
        assert!(serde_json::from_str::<Nibble>("16").is_err());
    }

    proptest! {
        #[test]
        fn any_byte_survives_as_two_skills(byte: u8) {
            let (high, low) = Skill::split(byte);
            prop_assert_eq!(Skill::join(high, low), byte);
        }

        #[test]
        fn any_nibble_survives_as_a_skill(value in 0u8..=15) {
            let skill = Skill::try_from(value).expect("a nibble");
            prop_assert_eq!(u8::from(skill), value);
        }
    }
}
