//! What upstream's JSON files share: their text layout, bytes as written, codes as numbers,
//! and keys that may be absent or `null`.

pub mod code;
pub mod numbers;
pub mod text;

use serde::{Deserialize, Deserializer};

pub use numbers::{JsonByte, JsonWord, OutOfRange};
pub use text::{Indent, NonAscii, TrailingNewline, to_text};

/// For an `Option<Option<T>>` field (with `#[serde(default)]` and
/// `skip_serializing_if = "Option::is_none"`): an absent key reads `None`, `null` reads
/// `Some(None)`, a value `Some(Some(value))`. So both survive a round trip.
///
/// # Errors
///
/// When the value is not a `T` or `null`.
pub fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Holder {
        #[serde(
            default,
            deserialize_with = "super::present",
            skip_serializing_if = "Option::is_none"
        )]
        key: Option<Option<u8>>,
    }

    #[test]
    fn absent_null_and_given_keys_survive() {
        for (text, key) in [
            ("{}", None),
            (r#"{"key":null}"#, Some(None)),
            (r#"{"key":3}"#, Some(Some(3))),
        ] {
            let holder: Holder = serde_json::from_str(text).expect("deserializes");
            assert_eq!(holder.key, key);
            assert_eq!(serde_json::to_string(&holder).ok(), Some(text.to_owned()));
        }
    }
}
