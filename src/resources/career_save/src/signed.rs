//! A record byte the writer prints signed (`(int8_t)`): 200 as `-56`. Either sign reads,
//! as upstream's loader stores `(uint8_t)` of the number.
//!
//! Use with `#[serde(with = "crate::signed")]` on a `u8`.

use openswos_common::json::JsonByte;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Writes `byte` signed.
///
/// # Errors
///
/// As the serializer's.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde's `with` passes a reference
pub fn serialize<S: Serializer>(byte: &u8, serializer: S) -> Result<S::Ok, S::Error> {
    byte.cast_signed().serialize(serializer)
}

/// Reads a byte written signed or unsigned.
///
/// # Errors
///
/// When the number is outside -128..=255.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    JsonByte::deserialize(deserializer).map(JsonByte::byte)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Holder(#[serde(with = "super")] u8);

    #[test]
    fn writes_signed_and_reads_either_sign() {
        assert_eq!(
            serde_json::to_string(&Holder(200)).ok(),
            Some("-56".to_owned())
        );
        assert_eq!(serde_json::to_string(&Holder(5)).ok(), Some("5".to_owned()));
        assert_eq!(
            serde_json::from_str::<Holder>("-56").ok(),
            Some(Holder(200))
        );
        assert_eq!(
            serde_json::from_str::<Holder>("200").ok(),
            Some(Holder(200))
        );
        assert!(serde_json::from_str::<Holder>("256").is_err());
        assert!(serde_json::from_str::<Holder>("-129").is_err());
    }
}
