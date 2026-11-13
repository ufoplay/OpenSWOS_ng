//! The 68000's arithmetic where it differs from Rust's operators.

/// `lsl.l`: `value`'s bits shifted left by `count`, those shifted out lost (upstream
/// include/math68k.h `M68kShiftLeft32`, which shifts the unsigned pattern because a signed
/// left shift of a negative value is undefined in C). A count of 32 or more leaves 0; upstream
/// is undefined there, and every caller shifts by a constant from 1 to 5.
#[must_use]
pub const fn shift_left_32(value: i32, count: u32) -> i32 {
    match value.checked_shl(count) {
        Some(shifted) => shifted,
        None => 0,
    }
}

/// `lsl.w`: as [`shift_left_32`], on a word (upstream `M68kShiftLeft16`). A count of 16 or
/// more leaves 0, the word's bits of upstream's result where it is defined.
#[must_use]
pub const fn shift_left_16(value: i16, count: u32) -> i16 {
    match value.checked_shl(count) {
        Some(shifted) => shifted,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{shift_left_16, shift_left_32};

    /// A word with its top bit set.
    const TOP_BIT_16: i16 = i16::MIN;

    proptest! {
        #[test]
        fn the_32_bit_shift_moves_the_pattern(value: i32, count in 0..i32::BITS) {
            let pattern = value.cast_unsigned() << count;
            prop_assert_eq!(shift_left_32(value, count), pattern.cast_signed());
        }

        #[test]
        fn the_16_bit_shift_moves_the_pattern(value: i16, count in 0..i16::BITS) {
            let pattern = value.cast_unsigned() << count;
            prop_assert_eq!(shift_left_16(value, count), pattern.cast_signed());
        }

        #[test]
        fn a_count_past_the_long_leaves_zero(value: i32, count in i32::BITS..=u32::MAX) {
            prop_assert_eq!(shift_left_32(value, count), 0);
        }

        #[test]
        fn a_count_past_the_word_leaves_zero(value: i16, count in i16::BITS..=u32::MAX) {
            prop_assert_eq!(shift_left_16(value, count), 0);
        }
    }

    #[test]
    fn a_negative_value_shifts_without_overflow() {
        assert_eq!(shift_left_32(-1, 1), -2);
        assert_eq!(shift_left_32(i32::MIN, 1), 0);
        assert_eq!(shift_left_16(-1, 1), -2);
        assert_eq!(shift_left_16(TOP_BIT_16, 1), 0);
    }
}
