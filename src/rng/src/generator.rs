//! One generator.

use crate::table::random_byte;

/// One of the game's two generators (upstream src/random.c): a counter walking the random
/// table, each byte XORed with a key. Each time the counter comes back to 0, the key moves on
/// to the table's next byte. Every state is valid, the key included: it need not be the
/// table's byte at its index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Generator {
    /// The counter: the table's index of the next value (upstream `_seed`, `_seed2`).
    pub seed: u8,
    /// The key's index in the table (`_randXorIndex`, `_randXorIndex2`).
    pub xor_index: u8,
    /// The key the values are XORed with (`_randXorKey`, `_randXorKey2`).
    pub xor_key: u8,
}

impl Generator {
    /// A generator at `seed` whose key is the table's byte at `xor_index`, as the game seeds
    /// one (upstream `Randomize2`; the autopilot and the rigs too).
    #[must_use]
    pub const fn new(seed: u8, xor_index: u8) -> Self {
        Self {
            seed,
            xor_index,
            xor_key: random_byte(xor_index),
        }
    }

    /// The next value, 0 to 255 (the body of upstream's `Rand` and `Rand2`). At counter 0 the
    /// key first moves on.
    pub fn next_value(&mut self) -> u16 {
        if self.seed == 0 {
            self.xor_index = self.xor_index.wrapping_add(1);
            self.xor_key = random_byte(self.xor_index);
        }
        let value = random_byte(self.seed) ^ self.xor_key;
        self.seed = self.seed.wrapping_add(1);
        u16::from(value)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::Generator;
    use crate::table::random_byte;

    /// The table's first byte.
    const FIRST_BYTE: u8 = 124;

    /// The table's second byte.
    const SECOND_BYTE: u8 = 154;

    #[test]
    fn new_takes_the_key_from_the_table() {
        assert_eq!(
            Generator::new(u8::MAX, 1),
            Generator {
                seed: u8::MAX,
                xor_index: 1,
                xor_key: SECOND_BYTE,
            }
        );
    }

    #[test]
    fn the_key_moves_on_at_counter_zero() {
        let mut generator = Generator::default();
        // The key moves to the table's second byte, the value is the first byte under it.
        assert_eq!(generator.next_value(), u16::from(FIRST_BYTE ^ SECOND_BYTE));
        assert_eq!(
            generator,
            Generator {
                seed: 1,
                xor_index: 1,
                xor_key: SECOND_BYTE,
            }
        );
    }

    #[test]
    fn the_key_index_wraps() {
        let mut generator = Generator {
            seed: 0,
            xor_index: u8::MAX,
            xor_key: 0,
        };
        generator.next_value();
        assert_eq!((generator.xor_index, generator.xor_key), (0, FIRST_BYTE));
    }

    proptest! {
        #[test]
        fn away_from_zero_the_key_stays(seed in 1..=u8::MAX, xor_index: u8, xor_key: u8) {
            let mut generator = Generator { seed, xor_index, xor_key };
            prop_assert_eq!(generator.next_value(), u16::from(random_byte(seed) ^ xor_key));
            prop_assert_eq!(
                generator,
                Generator { seed: seed.wrapping_add(1), xor_index, xor_key }
            );
        }

        #[test]
        fn a_full_round_moves_the_key_once(seed: u8, xor_index: u8, xor_key: u8) {
            let mut generator = Generator { seed, xor_index, xor_key };
            for _ in 0..=u8::MAX {
                generator.next_value();
            }
            let moved = xor_index.wrapping_add(1);
            prop_assert_eq!(
                generator,
                Generator { seed, xor_index: moved, xor_key: random_byte(moved) }
            );
        }
    }
}
