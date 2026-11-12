//! Team identity.
//!
//! A team is named by the team file holding it and its place in that file. What a file
//! number means (in World 1996-97 it is the country's code) is world data (docs/PLAN.md
//! §5.2), so these are plain numbers.

use serde::{Deserialize, Serialize};

/// A team file's number: `TEAM.nnn` is file `nnn`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TeamFileNumber(pub u8);

/// A team's place in its team file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TeamOrdinal(pub u8);

/// A team's identity across the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TeamId {
    /// The team file holding the team.
    pub file: TeamFileNumber,
    /// The team's place in that file.
    pub ordinal: TeamOrdinal,
}

impl TeamId {
    /// The identity as the records store it: the file's byte, then the ordinal's
    /// (upstream: `PACK_BE_U16(country, ordinal)`).
    #[must_use]
    pub fn from_bytes(bytes: [u8; 2]) -> Self {
        let [file, ordinal] = bytes;
        Self {
            file: TeamFileNumber(file),
            ordinal: TeamOrdinal(ordinal),
        }
    }

    /// The identity's two bytes, as the records store it.
    #[must_use]
    pub fn to_bytes(self) -> [u8; 2] {
        let Self {
            file: TeamFileNumber(file),
            ordinal: TeamOrdinal(ordinal),
        } = self;
        [file, ordinal]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn the_file_byte_comes_first() {
        let rangers = TeamId::from_bytes([33, 7]);
        assert_eq!(rangers.file, TeamFileNumber(33));
        assert_eq!(rangers.ordinal, TeamOrdinal(7));
        assert_eq!(rangers.to_bytes(), [33, 7]);
    }

    #[test]
    fn serializes_as_file_and_ordinal() {
        let json = serde_json::to_string(&TeamId::from_bytes([8, 2])).expect("serializes");
        assert_eq!(json, "{\"file\":8,\"ordinal\":2}");
    }

    #[test]
    fn orders_by_file_then_ordinal() {
        assert!(TeamId::from_bytes([1, 9]) < TeamId::from_bytes([2, 0]));
        assert!(TeamId::from_bytes([2, 0]) < TeamId::from_bytes([2, 1]));
    }

    proptest! {
        #[test]
        fn bytes_round_trip(bytes: [u8; 2]) {
            prop_assert_eq!(TeamId::from_bytes(bytes).to_bytes(), bytes);
        }
    }
}
