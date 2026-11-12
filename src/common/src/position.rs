//! A player's role.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A player's role, as the player records store it in 3 bits (upstream:
/// `positionAndFace >> 5`, docs/DATA_JSON.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Position {
    /// GK.
    Goalkeeper = 0,
    /// RB.
    RightBack = 1,
    /// LB.
    LeftBack = 2,
    /// CD.
    CentreDefender = 3,
    /// RW.
    RightWing = 4,
    /// LW.
    LeftWing = 5,
    /// MF.
    Midfielder = 6,
    /// ST.
    Striker = 7,
}

/// A byte that is not a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a player position")]
pub struct InvalidPosition(pub u8);

impl Position {
    /// Every position, in code order.
    pub const ALL: &[Self] = &[
        Self::Goalkeeper,
        Self::RightBack,
        Self::LeftBack,
        Self::CentreDefender,
        Self::RightWing,
        Self::LeftWing,
        Self::Midfielder,
        Self::Striker,
    ];
}

impl From<Position> for u8 {
    fn from(position: Position) -> Self {
        position as Self
    }
}

impl TryFrom<u8> for Position {
    type Error = InvalidPosition;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        Self::ALL
            .iter()
            .copied()
            .find(|position| u8::from(*position) == code)
            .ok_or(InvalidPosition(code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_follow_the_records() {
        assert_eq!(u8::from(Position::Goalkeeper), 0);
        assert_eq!(u8::from(Position::CentreDefender), 3);
        assert_eq!(u8::from(Position::Striker), 7);
    }

    #[test]
    fn every_code_round_trips() {
        for (code, position) in (0u8..).zip(Position::ALL.iter().copied()) {
            assert_eq!(u8::from(position), code);
            assert_eq!(Position::try_from(code), Ok(position));
        }
    }

    #[test]
    fn other_codes_are_rejected() {
        assert_eq!(Position::try_from(8), Err(InvalidPosition(8)));
        assert_eq!(Position::try_from(u8::MAX), Err(InvalidPosition(u8::MAX)));
        assert_eq!(InvalidPosition(8).to_string(), "8 is not a player position");
    }

    #[test]
    fn serializes_by_name() {
        let json = serde_json::to_string(&Position::CentreDefender).expect("serializes");
        assert_eq!(json, "\"centre_defender\"");
        assert_eq!(
            serde_json::from_str::<Position>("\"striker\"").expect("deserializes"),
            Position::Striker
        );
    }
}
