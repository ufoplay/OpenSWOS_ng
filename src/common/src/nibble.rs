//! Half a byte, as records pack two values into one.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Bits in a nibble.
const NIBBLE_BITS: u32 = 4;

/// A nibble's bits.
const NIBBLE_MASK: u8 = 0x0F;

/// Four bits as a record stores them: half a byte.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(try_from = "u8", into = "u8")]
pub struct Nibble(u8);

/// A value too big for a nibble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} does not fit a nibble (0..=15)")]
pub struct InvalidNibble(pub u8);

impl Nibble {
    /// All bits clear.
    pub const ZERO: Self = Self(0);

    /// The value, 0..=15.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }

    /// The low 4 bits of `byte`.
    #[must_use]
    pub fn from_low_bits(byte: u8) -> Self {
        Self(byte & NIBBLE_MASK)
    }

    /// The high and low nibbles of a byte.
    #[must_use]
    pub fn split(byte: u8) -> (Self, Self) {
        (Self(byte >> NIBBLE_BITS), Self(byte & NIBBLE_MASK))
    }

    /// The byte holding `high` and `low`.
    #[must_use]
    pub fn join(high: Self, low: Self) -> u8 {
        (high.0 << NIBBLE_BITS) | low.0
    }
}

impl TryFrom<u8> for Nibble {
    type Error = InvalidNibble;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= NIBBLE_MASK {
            Ok(Self(value))
        } else {
            Err(InvalidNibble(value))
        }
    }
}

impl From<Nibble> for u8 {
    fn from(nibble: Nibble) -> Self {
        nibble.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn a_byte_holds_two_nibbles() {
        assert_eq!(
            Nibble::split(0x3E),
            (Nibble::from_low_bits(3), Nibble::from_low_bits(14))
        );
        assert_eq!(
            Nibble::join(Nibble::from_low_bits(3), Nibble::from_low_bits(14)),
            0x3E
        );
        assert_eq!(Nibble::from_low_bits(0xA7).value(), 7);
        assert_eq!(Nibble::ZERO, Nibble::default());
    }

    #[test]
    fn values_beyond_a_nibble_are_rejected() {
        assert_eq!(Nibble::try_from(15).map(u8::from), Ok(15));
        assert_eq!(Nibble::try_from(16), Err(InvalidNibble(16)));
        assert_eq!(
            InvalidNibble(16).to_string(),
            "16 does not fit a nibble (0..=15)"
        );
        assert!(serde_json::from_str::<Nibble>("16").is_err());
        assert_eq!(
            serde_json::to_string(&Nibble::ZERO).ok(),
            Some("0".to_owned())
        );
    }

    proptest! {
        #[test]
        fn any_byte_survives_as_two_nibbles(byte: u8) {
            let (high, low) = Nibble::split(byte);
            prop_assert_eq!(Nibble::join(high, low), byte);
        }
    }
}
