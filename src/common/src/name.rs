//! Fixed-size text fields of the game's records.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// The byte that ends a name shorter than its field.
const NUL: u8 = 0;

/// A text field of exactly `N` bytes, as a record stores it.
///
/// The game reads a name up to its first NUL, or the whole field when it holds none (a
/// name filling its field is ended by the next field of the record). The field keeps
/// every byte, so a record round-trips exactly, bytes after the NUL included. Trailing
/// spaces are part of the name.
///
/// Text converts byte for byte (Latin-1): which glyph the game draws for a byte above
/// 0x7F is the view's business.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixedName<const N: usize> {
    bytes: [u8; N],
}

/// Why a text does not fit a name field.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NameError {
    /// The text has more characters than the field has bytes.
    #[error("`{text}` has {len} characters; the field holds {max}")]
    TooLong {
        /// The text.
        text: String,
        /// Its length in characters.
        len: usize,
        /// The field's size.
        max: usize,
    },
    /// A character has no byte: NUL, or beyond U+00FF.
    #[error("{character:?} cannot be stored in a name field")]
    Unrepresentable {
        /// The character.
        character: char,
    },
    /// Raw bytes of the wrong length.
    #[error("{len} bytes given for a field of {expected}")]
    WrongLength {
        /// The bytes given.
        len: usize,
        /// The field's size.
        expected: usize,
    },
}

impl<const N: usize> FixedName<N> {
    /// The field holding exactly `bytes`.
    #[must_use]
    pub fn from_bytes(bytes: [u8; N]) -> Self {
        Self { bytes }
    }

    /// The field's bytes, all `N` of them.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.bytes
    }

    /// The name's bytes: up to the first NUL, or the whole field.
    #[must_use]
    pub fn text_bytes(&self) -> &[u8] {
        self.bytes
            .split(|&byte| byte == NUL)
            .next()
            .unwrap_or_default()
    }

    /// The name, one character per byte.
    #[must_use]
    pub fn text(&self) -> String {
        self.text_bytes().iter().copied().map(char::from).collect()
    }

    /// The field holding `text`, NUL-padded.
    ///
    /// # Errors
    ///
    /// When a character has no byte, or the text is longer than the field.
    pub fn from_text(text: &str) -> Result<Self, NameError> {
        let mut bytes = [NUL; N];
        let mut slots = bytes.iter_mut();
        for character in text.chars() {
            let byte = u8::try_from(character)
                .ok()
                .filter(|&byte| byte != NUL)
                .ok_or(NameError::Unrepresentable { character })?;
            let slot = slots.next().ok_or_else(|| NameError::TooLong {
                text: text.to_owned(),
                len: text.chars().count(),
                max: N,
            })?;
            *slot = byte;
        }
        Ok(Self { bytes })
    }

    /// Whether the field is exactly its text NUL-padded: nothing but NULs after the first
    /// NUL. Only such a field is described by its text alone.
    #[must_use]
    pub fn is_canonical(&self) -> bool {
        let tail = self
            .bytes
            .get(self.text_bytes().len()..)
            .unwrap_or_default();
        tail.iter().all(|&byte| byte == NUL)
    }
}

impl<const N: usize> Default for FixedName<N> {
    /// The empty name: all NULs.
    fn default() -> Self {
        Self { bytes: [NUL; N] }
    }
}

impl<const N: usize> fmt::Display for FixedName<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text())
    }
}

impl<const N: usize> fmt::Debug for FixedName<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_canonical() {
            f.debug_tuple("FixedName").field(&self.text()).finish()
        } else {
            f.debug_struct("FixedName")
                .field("bytes", &self.bytes)
                .finish()
        }
    }
}

/// How a name field is written in JSON: its text when that describes it whole, else its
/// raw bytes.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum NameRepr {
    Text(String),
    Raw { bytes: Vec<u8> },
}

impl<const N: usize> Serialize for FixedName<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let repr = if self.is_canonical() {
            NameRepr::Text(self.text())
        } else {
            NameRepr::Raw {
                bytes: self.bytes.to_vec(),
            }
        };
        repr.serialize(serializer)
    }
}

impl<'de, const N: usize> Deserialize<'de> for FixedName<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = match NameRepr::deserialize(deserializer)? {
            NameRepr::Text(text) => Self::from_text(&text),
            NameRepr::Raw { bytes } => {
                let len = bytes.len();
                <[u8; N]>::try_from(bytes)
                    .map(Self::from_bytes)
                    .map_err(|_| NameError::WrongLength { len, expected: N })
            }
        };
        name.map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    type TeamName = FixedName<17>;

    fn name(bytes: &[u8]) -> TeamName {
        let mut field = [0; 17];
        field
            .get_mut(..bytes.len())
            .expect("fits the field")
            .copy_from_slice(bytes);
        TeamName::from_bytes(field)
    }

    #[test]
    fn text_ends_at_the_first_nul() {
        let field = name(b"RANGERS\0JUNK");
        assert_eq!(field.text(), "RANGERS");
        assert_eq!(field.text_bytes(), b"RANGERS");
    }

    #[test]
    fn a_full_field_has_no_nul() {
        let field = name(b"BORUSSIA MOENCHEN");
        assert_eq!(field.text(), "BORUSSIA MOENCHEN");
        assert!(field.is_canonical());
    }

    #[test]
    fn trailing_spaces_are_part_of_the_name() {
        assert_eq!(name(b"AJAX  ").text(), "AJAX  ");
    }

    #[test]
    fn from_text_pads_with_nuls() {
        let field = TeamName::from_text("JUVENTUS").expect("fits");
        assert_eq!(field, name(b"JUVENTUS"));
        assert!(field.is_canonical());
        assert_eq!(field.as_bytes().len(), 17);
    }

    #[test]
    fn latin1_characters_are_one_byte() {
        let field = TeamName::from_text("ÉIRE").expect("Latin-1 fits");
        assert_eq!(field.text_bytes(), &[0xC9, b'I', b'R', b'E']);
        assert_eq!(field.text(), "ÉIRE");
    }

    #[test]
    fn rejects_text_that_does_not_fit() {
        assert_eq!(
            TeamName::from_text("FORTUNA DUSSELDORF"),
            Err(NameError::TooLong {
                text: "FORTUNA DUSSELDORF".to_owned(),
                len: 18,
                max: 17,
            })
        );
        assert_eq!(
            TeamName::from_text("ŁÓDŹ"),
            Err(NameError::Unrepresentable { character: 'Ł' })
        );
        assert_eq!(
            TeamName::from_text("A\0B"),
            Err(NameError::Unrepresentable { character: '\0' })
        );
    }

    #[test]
    fn bytes_after_the_nul_make_a_field_non_canonical() {
        assert!(!name(b"RANGERS\0JUNK").is_canonical());
        assert!(name(b"RANGERS").is_canonical());
        assert!(TeamName::default().is_canonical());
        assert_eq!(TeamName::default().text(), "");
    }

    #[test]
    fn display_and_debug() {
        assert_eq!(name(b"AC MILAN").to_string(), "AC MILAN");
        assert_eq!(
            format!("{:?}", name(b"AC MILAN")),
            "FixedName(\"AC MILAN\")"
        );
        assert!(format!("{:?}", name(b"A\0B")).starts_with("FixedName { bytes: [65, 0, 66"));
    }

    #[test]
    fn canonical_fields_serialize_as_text() {
        let json = serde_json::to_string(&name(b"AC MILAN")).expect("serializes");
        assert_eq!(json, "\"AC MILAN\"");
        let back: TeamName = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, name(b"AC MILAN"));
    }

    #[test]
    fn other_fields_serialize_as_raw_bytes() {
        let field = name(b"A\0B");
        let json = serde_json::to_string(&field).expect("serializes");
        assert_eq!(json, "{\"bytes\":[65,0,66,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}");
        let back: TeamName = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, field);
    }

    #[test]
    fn deserializing_checks_the_field_size() {
        assert!(serde_json::from_str::<TeamName>("{\"bytes\":[65,66]}").is_err());
        assert!(serde_json::from_str::<TeamName>("\"FORTUNA DUSSELDORF\"").is_err());
        assert!(serde_json::from_str::<TeamName>("42").is_err());
    }

    proptest! {
        #[test]
        fn any_field_survives_json(bytes: [u8; 17]) {
            let field = TeamName::from_bytes(bytes);
            let json = serde_json::to_string(&field).expect("serializes");
            let back: TeamName = serde_json::from_str(&json).expect("deserializes");
            prop_assert_eq!(back, field);
        }

        #[test]
        fn canonical_fields_round_trip_through_text(bytes: [u8; 17]) {
            let field = TeamName::from_bytes(bytes);
            prop_assume!(field.is_canonical());
            prop_assert_eq!(TeamName::from_text(&field.text()), Ok(field));
        }
    }
}
