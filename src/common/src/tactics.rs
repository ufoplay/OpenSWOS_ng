//! The engine's tactic slots.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A tactic slot: one of the engine's 18 (upstream `TACTIC_*`, include/game_types.h). A
/// team record names its tactics by slot, and a tactic its ball-out-of-play tactics. What
/// formation each slot plays is data (upstream `bin/data_json/tactics/`); the slots are the
/// engine's. The PC game's team files use the first ten, World 2026-27's only the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Tactics {
    /// 4-4-2 (upstream `TACTIC_DEFAULT`).
    FourFourTwo = 0,
    /// 5-4-1.
    FiveFourOne = 1,
    /// 4-5-1.
    FourFiveOne = 2,
    /// 5-3-2.
    FiveThreeTwo = 3,
    /// 3-5-2.
    ThreeFiveTwo = 4,
    /// 4-3-3.
    FourThreeThree = 5,
    /// 4-2-4.
    FourTwoFour = 6,
    /// 3-4-3.
    ThreeFourThree = 7,
    /// Sweep.
    Sweep = 8,
    /// 5-2-3.
    FiveTwoThree = 9,
    /// Attack.
    Attack = 10,
    /// Defend.
    Defend = 11,
    /// The user's tactic A.
    UserA = 12,
    /// The user's tactic B.
    UserB = 13,
    /// The user's tactic C.
    UserC = 14,
    /// The user's tactic D.
    UserD = 15,
    /// The user's tactic E.
    UserE = 16,
    /// The user's tactic F.
    UserF = 17,
}

/// A code that is not a tactic slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a tactic slot")]
pub struct InvalidTactics(pub u8);

crate::byte_enum!(
    Tactics,
    InvalidTactics,
    [
        FourFourTwo,
        FiveFourOne,
        FourFiveOne,
        FiveThreeTwo,
        ThreeFiveTwo,
        FourThreeThree,
        FourTwoFour,
        ThreeFourThree,
        Sweep,
        FiveTwoThree,
        Attack,
        Defend,
        UserA,
        UserB,
        UserC,
        UserD,
        UserE,
        UserF,
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_follow_upstream() {
        for (code, tactics) in (0u8..).zip(Tactics::ALL.iter().copied()) {
            assert_eq!(u8::from(tactics), code);
            assert_eq!(Tactics::try_from(code), Ok(tactics));
        }
        assert_eq!(u8::from(Tactics::UserF), 17, "upstream TACTIC_USER_F");
        assert_eq!(Tactics::try_from(18), Err(InvalidTactics(18)));
        assert_eq!(InvalidTactics(18).to_string(), "18 is not a tactic slot");
    }

    #[test]
    fn serializes_by_name() {
        assert_eq!(
            serde_json::to_value(Tactics::FourFourTwo).ok(),
            Some(serde_json::json!("four_four_two"))
        );
    }
}
