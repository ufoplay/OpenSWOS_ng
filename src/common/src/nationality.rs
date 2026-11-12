//! Nationality codes.

use serde::{Deserialize, Serialize};

/// A nationality code: a player's, a national team's, a manager's. Its meaning (which
/// country) is world data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Nationality(pub u8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_its_code() {
        assert_eq!(
            serde_json::to_string(&Nationality(200)).expect("serializes"),
            "200"
        );
    }
}
