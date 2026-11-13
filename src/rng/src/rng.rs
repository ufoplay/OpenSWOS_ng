//! Both generators, the second one's save stack and a recording's fixed value.

use thiserror::Error;

use crate::generator::Generator;

/// The saves the second generator's stack holds: upstream's `_savedRandState[15]`, 3 bytes a
/// save (the counter, the key's index, the key).
pub const SAVE_STACK_LEN: usize = 5;

/// The reason code upstream halts the game with when the save stack is full (src/random.c
/// `SaveRand2State`).
pub const SAVE_STACK_FULL_REASON: u16 = 0x9793;

/// The game's random numbers (upstream src/random.c): two generators, the second one's save
/// stack, and a recording's fixed value. They are part of the game's state, so every function
/// that draws a number takes them. The order of the draws is behaviour (docs/PLAN.md §5.5).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Rng {
    /// The first generator, `Rand`'s: mostly the match draws from it.
    pub primary: Generator,
    /// The second generator, `Rand2`'s: mostly the career, the competitions and simulated
    /// matches draw from it. Its state can be saved and restored, and reseeded.
    pub secondary: Generator,
    /// A recording's fixed value (upstream `_replayFixedRandom`): when set, both generators
    /// return it and do not move. The swos-port recordings were made this way.
    pub fixed: Option<u8>,
    /// The second generator's saved states, the last one on top; at most
    /// [`SAVE_STACK_LEN`].
    saved: Vec<Generator>,
}

/// The second generator's save stack is full: upstream halts the game (its reason code,
/// [`SAVE_STACK_FULL_REASON`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "the random state's save stack is full: the game halts (reason {SAVE_STACK_FULL_REASON:#06x})"
)]
pub struct SaveStackFull;

/// The second generator's save stack holds no save to restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the random state's save stack holds no save")]
pub struct NothingSaved;

impl Rng {
    /// The first generator's next value, 0 to 255, or the fixed value (upstream `Rand`).
    pub fn rand(&mut self) -> u16 {
        match self.fixed {
            Some(value) => u16::from(value),
            None => self.primary.next_value(),
        }
    }

    /// The second generator's next value, 0 to 255, or the fixed value (upstream `Rand2`).
    pub fn rand2(&mut self) -> u16 {
        match self.fixed {
            Some(value) => u16::from(value),
            None => self.secondary.next_value(),
        }
    }

    /// Two of the second generator's values as a word, the first one the low byte (upstream
    /// `GetRandomWord`).
    pub fn random_word(&mut self) -> u16 {
        let low = self.rand2();
        let high = self.rand2();
        high << u8::BITS | low
    }

    /// Reseeds the second generator from `value` and the session's `random_seed` (upstream
    /// `Randomize2`, src/game_utils.c, which reads the seed from `_randomSeed`): their XOR's
    /// high byte is the counter, its low byte the key's index.
    pub fn randomize2(&mut self, value: u16, random_seed: u16) {
        let [seed, xor_index] = (value ^ random_seed).to_be_bytes();
        self.secondary = Generator::new(seed, xor_index);
    }

    /// The second generator's saved states, the last one last.
    #[must_use]
    pub fn saved(&self) -> &[Generator] {
        &self.saved
    }

    /// Pushes the second generator's state (upstream `SaveRand2State`).
    ///
    /// # Errors
    ///
    /// [`SaveStackFull`] when this save fills the stack: faithful to the original, the save is
    /// kept, then upstream halts the game. Also when the stack was already full (upstream
    /// would write past it): nothing is saved.
    pub fn save_rand2_state(&mut self) -> Result<(), SaveStackFull> {
        if self.saved.len() == SAVE_STACK_LEN {
            return Err(SaveStackFull);
        }
        self.saved.push(self.secondary);
        if self.saved.len() == SAVE_STACK_LEN {
            Err(SaveStackFull)
        } else {
            Ok(())
        }
    }

    /// Pops the second generator's last saved state into it (upstream `RestoreRand2State`).
    ///
    /// # Errors
    ///
    /// [`NothingSaved`] when the stack is empty (upstream would read before it); the
    /// generator is left as it is.
    pub fn restore_rand2_state(&mut self) -> Result<(), NothingSaved> {
        self.secondary = self.saved.pop().ok_or(NothingSaved)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
