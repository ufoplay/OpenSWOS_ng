//! Fieldless enums stored as a byte code.

/// For a fieldless `#[repr(u8)]` enum: `ALL`, its variants in code order, and the
/// conversions to and from its byte code. `$error` is a tuple struct of the byte: the error
/// of a byte that is no variant's code. Every variant must be listed, in code order (a
/// missing one does not compile).
#[macro_export]
macro_rules! byte_enum {
    ($name:ident, $error:ident, [$($variant:ident),+ $(,)?]) => {
        impl $name {
            /// Every variant, in code order.
            pub const ALL: &[Self] = &[$(Self::$variant),+];
        }

        impl From<$name> for u8 {
            fn from(value: $name) -> Self {
                // Exhaustive: a variant missing from the list does not compile.
                match value {
                    $($name::$variant)|+ => value as Self,
                }
            }
        }

        impl TryFrom<u8> for $name {
            type Error = $error;

            fn try_from(code: u8) -> Result<Self, Self::Error> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|value| u8::from(*value) == code)
                    .ok_or($error(code))
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum Sample {
        Zero = 0,
        Two = 2,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct InvalidSample(u8);

    crate::byte_enum!(Sample, InvalidSample, [Zero, Two]);

    #[test]
    fn codes_convert_both_ways() {
        assert_eq!(Sample::ALL, &[Sample::Zero, Sample::Two]);
        assert_eq!(u8::from(Sample::Two), 2);
        assert_eq!(Sample::try_from(2), Ok(Sample::Two));
        assert_eq!(Sample::try_from(0), Ok(Sample::Zero));
    }

    #[test]
    fn a_byte_that_is_no_code_is_an_error() {
        assert_eq!(Sample::try_from(1), Err(InvalidSample(1)));
    }
}
