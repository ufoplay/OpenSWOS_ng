//! Record bytes and words as the JSON writes them.
//!
//! The writers disagree on signedness. OpenSWOS Setup (`tools/swosdata/lib/team_json.c`)
//! signs some fields as the C structs do (`nationality -56`); the World 2026-27 import, and
//! this port, write them unsigned (`nationality 200`). Both mean the same byte. These types
//! keep the number exactly as written, so every file round-trips byte for byte, and give the
//! byte.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The lowest number a byte may be written as: a signed byte's minimum.
const BYTE_MIN: i16 = -128;

/// The highest number a byte may be written as: an unsigned byte's maximum.
const BYTE_MAX: i16 = 255;

/// The lowest number a word may be written as: a signed word's minimum.
const WORD_MIN: i32 = -32_768;

/// The highest number a word may be written as: an unsigned word's maximum.
const WORD_MAX: i32 = 65_535;

/// A number outside what a byte or word may be written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{value} is neither a signed nor an unsigned {kind}")]
pub struct OutOfRange {
    /// The number.
    pub value: i32,
    /// "byte" or "word".
    pub kind: &'static str,
}

/// A record byte as the JSON writes it: `-128..=255`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "i16", into = "i16")]
pub struct JsonByte(i16);

impl JsonByte {
    /// The byte.
    #[must_use]
    pub fn byte(self) -> u8 {
        u8::try_from(self.0)
            .or_else(|_| i8::try_from(self.0).map(i8::cast_unsigned))
            .expect("a JsonByte is always -128..=255")
    }

    /// The number as written.
    #[must_use]
    pub fn written(self) -> i16 {
        self.0
    }
}

impl From<u8> for JsonByte {
    /// The byte, written unsigned.
    fn from(byte: u8) -> Self {
        Self(i16::from(byte))
    }
}

impl TryFrom<i16> for JsonByte {
    type Error = OutOfRange;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        if (BYTE_MIN..=BYTE_MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(OutOfRange {
                value: i32::from(value),
                kind: "byte",
            })
        }
    }
}

impl From<JsonByte> for i16 {
    fn from(byte: JsonByte) -> Self {
        byte.0
    }
}

/// A record word as the JSON writes it: `-32768..=65535`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "i32", into = "i32")]
pub struct JsonWord(i32);

impl JsonWord {
    /// The word.
    #[must_use]
    pub fn word(self) -> u16 {
        u16::try_from(self.0)
            .or_else(|_| i16::try_from(self.0).map(i16::cast_unsigned))
            .expect("a JsonWord is always -32768..=65535")
    }

    /// The number as written.
    #[must_use]
    pub fn written(self) -> i32 {
        self.0
    }
}

impl From<u16> for JsonWord {
    /// The word, written unsigned.
    fn from(word: u16) -> Self {
        Self(i32::from(word))
    }
}

impl TryFrom<i32> for JsonWord {
    type Error = OutOfRange;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if (WORD_MIN..=WORD_MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(OutOfRange {
                value,
                kind: "word",
            })
        }
    }
}

impl From<JsonWord> for i32 {
    fn from(word: JsonWord) -> Self {
        word.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn a_byte_reads_the_same_signed_or_unsigned() {
        assert_eq!(JsonByte::try_from(-56_i16).map(JsonByte::byte), Ok(200));
        assert_eq!(JsonByte::try_from(200_i16).map(JsonByte::byte), Ok(200));
        assert_eq!(JsonByte::try_from(-1_i16).map(JsonByte::byte), Ok(255));
        assert_eq!(JsonByte::try_from(0_i16).map(JsonByte::byte), Ok(0));
    }

    #[test]
    fn a_byte_is_written_unsigned() {
        assert_eq!(JsonByte::from(200).written(), 200);
        assert_eq!(JsonByte::from(0).written(), 0);
    }

    #[test]
    fn numbers_beyond_a_byte_are_rejected() {
        assert_eq!(
            JsonByte::try_from(256_i16),
            Err(OutOfRange {
                value: 256,
                kind: "byte"
            })
        );
        assert_eq!(
            JsonByte::try_from(-129_i16),
            Err(OutOfRange {
                value: -129,
                kind: "byte"
            })
        );
        assert_eq!(
            OutOfRange {
                value: 256,
                kind: "byte"
            }
            .to_string(),
            "256 is neither a signed nor an unsigned byte"
        );
    }

    #[test]
    fn words_work_like_bytes() {
        assert_eq!(JsonWord::try_from(-1_i32).map(JsonWord::word), Ok(u16::MAX));
        assert_eq!(JsonWord::try_from(1360_i32).map(JsonWord::word), Ok(1360));
        assert_eq!(JsonWord::from(u16::MAX).written(), 65_535);
        assert_eq!(
            JsonWord::try_from(65_536_i32),
            Err(OutOfRange {
                value: 65_536,
                kind: "word"
            })
        );
    }

    #[test]
    fn json_keeps_the_number_as_written() {
        let byte: JsonByte = serde_json::from_str("-56").expect("a byte");
        assert_eq!(serde_json::to_string(&byte).expect("serializes"), "-56");
        assert!(serde_json::from_str::<JsonByte>("300").is_err());
    }

    proptest! {
        #[test]
        fn bytes_survive_being_written(byte: u8) {
            prop_assert_eq!(JsonByte::from(byte).byte(), byte);
        }

        #[test]
        fn signed_writings_read_as_their_byte(byte: u8) {
            let signed = JsonByte::try_from(i16::from(byte.cast_signed())).expect("in range");
            prop_assert_eq!(signed.byte(), byte);
        }

        #[test]
        fn words_survive_being_written(word: u16) {
            prop_assert_eq!(JsonWord::from(word).word(), word);
            let signed = JsonWord::try_from(i32::from(word.cast_signed())).expect("in range");
            prop_assert_eq!(signed.word(), word);
        }
    }
}
