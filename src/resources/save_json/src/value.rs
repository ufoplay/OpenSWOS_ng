//! A save's values as written: byte strings, integers, and objects whose members keep
//! their order.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

/// A string of a save: its bytes, as written (names in the game's character set, which
/// need not be UTF-8).
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SaveString(pub Vec<u8>);

impl SaveString {
    /// Its bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The bytes of `text`'s characters, if each is at most U+00FF.
    fn from_chars(text: &str) -> Option<Self> {
        text.chars()
            .map(|character| u8::try_from(u32::from(character)).ok())
            .collect::<Option<Vec<u8>>>()
            .map(Self)
    }
}

impl From<&str> for SaveString {
    /// The UTF-8 bytes of `text`.
    fn from(text: &str) -> Self {
        Self(text.as_bytes().to_vec())
    }
}

impl Serialize for SaveString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Each byte as the character of its code: the formatter writes it back as the byte.
        let text: String = self.0.iter().copied().map(char::from).collect();
        serializer.serialize_str(&text)
    }
}

impl<'de> Deserialize<'de> for SaveString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::from_chars(&text).ok_or_else(|| {
            de::Error::custom(format!(
                "{text:?} holds a character past U+00FF: no byte of a save's string"
            ))
        })
    }
}

/// A value of a save, as written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SaveValue {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer (the writer prints no other number).
    Integer(i64),
    /// A string.
    String(SaveString),
    /// An array.
    Array(Vec<SaveValue>),
    /// An object: its members in the order written.
    Object(Vec<(String, SaveValue)>),
}

impl SaveValue {
    /// The value of the first member named `key`, if this is an object with one.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members
                .iter()
                .find_map(|(name, value)| (name == key).then_some(value)),
            Self::Null | Self::Bool(_) | Self::Integer(_) | Self::String(_) | Self::Array(_) => {
                None
            }
        }
    }
}

impl Serialize for SaveValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Integer(value) => serializer.serialize_i64(*value),
            Self::String(value) => value.serialize(serializer),
            Self::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Self::Object(members) => {
                let mut map = serializer.serialize_map(Some(members.len()))?;
                for (key, value) in members {
                    map.serialize_entry(&SaveString::from(key.as_str()), value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for SaveValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(SaveValueVisitor)
    }
}

struct SaveValueVisitor;

impl<'de> Visitor<'de> for SaveValueVisitor {
    type Value = SaveValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "a save's value: null, a boolean, an integer, a string, an array or an object",
        )
    }

    fn visit_unit<E: de::Error>(self) -> Result<SaveValue, E> {
        Ok(SaveValue::Null)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<SaveValue, E> {
        Ok(SaveValue::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<SaveValue, E> {
        Ok(SaveValue::Integer(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<SaveValue, E> {
        i64::try_from(value)
            .map(SaveValue::Integer)
            .map_err(|_| E::custom(format!("{value} is past the integers a save holds")))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<SaveValue, E> {
        Err(E::custom(format!(
            "{value} is not an integer: saves hold integers"
        )))
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<SaveValue, E> {
        SaveString::from_chars(text)
            .map(SaveValue::String)
            .ok_or_else(|| E::custom(format!("{text:?} holds a character past U+00FF")))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<SaveValue, A::Error> {
        let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(value) = seq.next_element()? {
            values.push(value);
        }
        Ok(SaveValue::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<SaveValue, A::Error> {
        let mut members = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, value)) = map.next_entry::<SaveString, SaveValue>()? {
            let key = String::from_utf8(key.0)
                .map_err(|_| de::Error::custom("a key that is not UTF-8"))?;
            members.push((key, value));
        }
        Ok(SaveValue::Object(members))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatter::to_save_bytes;
    use crate::layout::Layout;
    use crate::text::from_save_bytes;
    use proptest::prelude::*;

    #[test]
    fn objects_keep_their_order() {
        let value: SaveValue =
            from_save_bytes(br#"{"z":1,"a":[true,null,"x"],"m":{"b":-2,"a":3}}"#).expect("reads");
        let SaveValue::Object(members) = &value else {
            panic!("{value:?} is not an object");
        };
        let keys: Vec<&str> = members.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(keys, vec!["z", "a", "m"]);
        assert_eq!(
            value.get("m").and_then(|m| m.get("b")),
            Some(&SaveValue::Integer(-2))
        );
        let bytes = to_save_bytes(&value, &Layout::upstream(3, 0)).expect("writes");
        assert_eq!(
            bytes,
            b"{\n  \"z\":1,\n  \"a\":[true,null,\"x\"],\n  \"m\":{\"b\":-2,\"a\":3}\n}\n".to_vec()
        );
    }

    #[test]
    fn strings_keep_their_bytes() {
        let value: SaveValue = from_save_bytes(b"[\"\xc9IRE\x7f\"]").expect("reads");
        assert_eq!(
            value,
            SaveValue::Array(vec![SaveValue::String(SaveString(b"\xc9IRE\x7f".to_vec()))])
        );
    }

    #[test]
    fn fractions_and_wide_characters_do_not_read() {
        assert!(from_save_bytes::<SaveValue>(b"[1.5]").is_err());
        assert!(from_save_bytes::<SaveValue>(b"[18446744073709551615]").is_err());
        assert!(from_save_bytes::<SaveValue>(b"[\"\\u0100\"]").is_err());
    }

    fn any_value() -> impl Strategy<Value = SaveValue> {
        let leaf = prop_oneof![
            Just(SaveValue::Null),
            any::<bool>().prop_map(SaveValue::Bool),
            any::<i64>().prop_map(SaveValue::Integer),
            proptest::collection::vec(any::<u8>(), 0..8)
                .prop_map(|bytes| SaveValue::String(SaveString(bytes))),
        ];
        leaf.prop_recursive(3, 32, 4, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..4).prop_map(SaveValue::Array),
                proptest::collection::vec(("[a-z_]{1,6}", inner), 0..4).prop_map(SaveValue::Object),
            ]
        })
    }

    proptest! {
        #[test]
        fn every_value_round_trips(value in any_value(), spaced in 0usize..3) {
            let layout = match &value {
                SaveValue::Object(members) => Layout::upstream(members.len(), spaced),
                SaveValue::Null
                | SaveValue::Bool(_)
                | SaveValue::Integer(_)
                | SaveValue::String(_)
                | SaveValue::Array(_) => Layout::default(),
            };
            let bytes = to_save_bytes(&value, &layout).expect("writes");
            let read = from_save_bytes::<SaveValue>(&bytes).expect("reads back");
            prop_assert_eq!(&read, &value);
            prop_assert_eq!(to_save_bytes(&read, &layout).expect("writes"), bytes.clone());
            if let SaveValue::Object(members) = &value && !members.is_empty() {
                prop_assert_eq!(Layout::of(&bytes), Some(layout));
            }
        }
    }
}
