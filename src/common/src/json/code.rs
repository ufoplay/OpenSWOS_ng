//! A code (an enum, flags, a nibble) written as its byte, as upstream writes it:
//! `#[serde(with = "code")]`. A number that is not one of the code's values does
//! not read.

use std::fmt::Display;

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serializer};

/// Writes `value` as its byte.
///
/// # Errors
///
/// When the serializer fails.
pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Copy + Into<u8>,
    S: Serializer,
{
    serializer.serialize_u8((*value).into())
}

/// Reads a byte as a `T`.
///
/// # Errors
///
/// When the number is not a byte, or no `T`.
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: TryFrom<u8>,
    T::Error: Display,
    D: Deserializer<'de>,
{
    let byte = u8::deserialize(deserializer)?;
    T::try_from(byte).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use crate::{Nibble, Position, Tactics};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Codes {
        #[serde(with = "super")]
        position: Position,
        #[serde(with = "super")]
        tactics: Tactics,
        #[serde(with = "super")]
        nibble: Nibble,
    }

    #[test]
    fn codes_are_written_as_their_bytes() {
        let codes = Codes {
            position: Position::Striker,
            tactics: Tactics::UserF,
            nibble: Nibble::from_low_bits(9),
        };
        let text = serde_json::to_string(&codes).expect("serializes");
        assert_eq!(text, r#"{"position":7,"tactics":17,"nibble":9}"#);
        assert_eq!(
            serde_json::from_str::<Codes>(&text).expect("deserializes"),
            codes
        );
    }

    #[test]
    fn a_number_that_is_no_code_does_not_read() {
        for (text, message) in [
            (
                r#"{"position":8,"tactics":0,"nibble":0}"#,
                "8 is not a player position",
            ),
            (
                r#"{"position":0,"tactics":18,"nibble":0}"#,
                "18 is not a tactic slot",
            ),
            (
                r#"{"position":0,"tactics":0,"nibble":16}"#,
                "16 does not fit a nibble (0..=15)",
            ),
            (
                r#"{"position":0,"tactics":0,"nibble":256}"#,
                "invalid value: integer `256`",
            ),
        ] {
            let error = serde_json::from_str::<Codes>(text).expect_err(text);
            assert!(error.to_string().starts_with(message), "{text}: {error}");
        }
    }
}
