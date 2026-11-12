//! A code (an enum or flags of `openswos_team_resource`) written as its byte, as upstream
//! writes it: `#[serde(with = "code")]`. A number that is not one of the code's values does
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
    use openswos_common::Position;
    use openswos_team_resource::{KitColour, MatchFlags, PlayerFlags};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Codes {
        #[serde(with = "super")]
        position: Position,
        #[serde(with = "super")]
        colour: KitColour,
        #[serde(with = "super")]
        flags: PlayerFlags,
        #[serde(with = "super")]
        match_flags: MatchFlags,
    }

    #[test]
    fn codes_are_written_as_their_bytes() {
        let codes = Codes {
            position: Position::Striker,
            colour: KitColour::Purple,
            flags: PlayerFlags::IN_FORM | PlayerFlags::TRIALIST,
            match_flags: MatchFlags::from(0b1000_0001),
        };
        let text = serde_json::to_string(&codes).expect("serializes");
        assert_eq!(
            text,
            r#"{"position":7,"colour":10,"flags":5,"match_flags":129}"#
        );
        assert_eq!(
            serde_json::from_str::<Codes>(&text).expect("deserializes"),
            codes
        );
    }

    #[test]
    fn a_number_that_is_no_code_does_not_read() {
        for (text, message) in [
            (
                r#"{"position":8,"colour":0,"flags":0,"match_flags":0}"#,
                "8 is not a player position",
            ),
            (
                r#"{"position":0,"colour":11,"flags":0,"match_flags":0}"#,
                "11 is not a kit colour",
            ),
            (
                r#"{"position":0,"colour":0,"flags":8,"match_flags":0}"#,
                "0x08 holds bits that are not player flags",
            ),
            (
                r#"{"position":0,"colour":0,"flags":0,"match_flags":256}"#,
                "invalid value: integer `256`",
            ),
        ] {
            let error = serde_json::from_str::<Codes>(text).expect_err(text);
            assert!(error.to_string().starts_with(message), "{text}: {error}");
        }
    }
}
