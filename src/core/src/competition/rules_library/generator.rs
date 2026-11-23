//! The Mod's generator (dplay.c `dp_below`, gplay.c's `below`): a linear congruential
//! generator, a draw below a bound by the high half of its state.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

/// The generator's multiplier.
const MULTIPLIER: u32 = 1_103_515_245;

/// Its increment.
const INCREMENT: u32 = 12_345;

/// How far its state is shifted, and a draw's product.
const DRAW_SHIFT: u32 = 16;

/// The next draw below `bound`, `state` stepped. Faithful to the original: the product wraps
/// past 32 bits.
pub(super) fn below(state: &mut u32, bound: u32) -> u32 {
    *state = state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
    ((*state >> DRAW_SHIFT).wrapping_mul(bound)) >> DRAW_SHIFT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draw_is_the_high_half_scaled() {
        // From 0: the state the increment, its high half 0, a draw of 0.
        let mut state = 0;
        assert_eq!(below(&mut state, 100), 0);
        assert_eq!(state, 12_345);
        let mut state = 1;
        let draw = below(&mut state, 256);
        assert!(draw < 256);
        assert_eq!(state, 1_103_515_245 + 12_345);
    }
}
