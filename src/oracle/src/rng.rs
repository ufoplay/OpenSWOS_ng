//! The random generators (upstream src/random.c): `Rand`, `Rand2`, `GetRandomWord`, the
//! second generator's save stack, and `Randomize2` (src/game_utils.c).

use thiserror::Error;

use crate::Oracle;
use crate::ffi;

pub use crate::ffi::rng::{RANDOM_TABLE_LEN, SAVED_RAND_STATE_LEN};

/// A generator's state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RandState {
    /// The counter: the table's index of the next value.
    pub seed: u8,
    /// The key's index in the table.
    pub xor_index: u8,
    /// The key the values are XORed with.
    pub xor_key: u8,
}

/// Why the second generator's state was not saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Rand2SaveError {
    /// Saved, then upstream halted the game: the stack is full (its reason code, `0x9793`).
    #[error("upstream halted the game: reason {0:#06x}")]
    Halted(u16),
    /// The stack has no room for a save: upstream would write past it, so it is not called.
    #[error("the save stack has no room for a save")]
    NoRoom,
}

/// The second generator's save stack holds no save: upstream would read before it, so it is
/// not called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the save stack holds no save")]
pub struct NothingSaved;

impl Oracle {
    /// The random table (`_kRandomTable`).
    #[must_use]
    pub fn random_table(&self) -> [u8; RANDOM_TABLE_LEN] {
        ffi::rng::random_table()
    }

    /// The first generator's state.
    #[must_use]
    pub fn rand_state(&self) -> RandState {
        ffi::rng::rand_state()
    }

    /// Sets the first generator's state.
    pub fn set_rand_state(&mut self, state: RandState) {
        ffi::rng::set_rand_state(state);
    }

    /// The second generator's state.
    #[must_use]
    pub fn rand2_state(&self) -> RandState {
        ffi::rng::rand2_state()
    }

    /// Sets the second generator's state.
    pub fn set_rand2_state(&mut self, state: RandState) {
        ffi::rng::set_rand2_state(state);
    }

    /// Pins both generators to `value`, as a recording does (`_replayFixedRandom`), or frees
    /// them.
    pub fn set_fixed_random(&mut self, value: Option<u8>) {
        ffi::rng::set_fixed_random(value);
    }

    /// `Rand()`: the first generator's next value.
    pub fn rand(&mut self) -> u16 {
        ffi::rng::rand()
    }

    /// `Rand2()`: the second generator's next value.
    pub fn rand2(&mut self) -> u16 {
        ffi::rng::rand2()
    }

    /// `GetRandomWord()`: two of the second generator's values, as a word.
    pub fn random_word(&mut self) -> u16 {
        ffi::rng::random_word()
    }

    /// `Randomize2(value)`, with `random_seed` as the session's seed (`_randomSeed`): reseeds
    /// the second generator.
    pub fn randomize2(&mut self, value: u16, random_seed: u16) {
        ffi::rng::randomize2(value, random_seed);
    }

    /// The bytes on the second generator's save stack.
    #[must_use]
    pub fn saved_rand2_bytes(&self) -> usize {
        ffi::rng::saved_rand2_bytes()
    }

    /// `SaveRand2State()`: pushes the second generator's state.
    ///
    /// # Errors
    ///
    /// When upstream halts the game (the stack now full), or the stack has no room.
    pub fn save_rand2_state(&mut self) -> Result<(), Rand2SaveError> {
        ffi::rng::save_rand2_state()
    }

    /// `RestoreRand2State()`: pops the second generator's state.
    ///
    /// # Errors
    ///
    /// When the stack holds no save.
    pub fn restore_rand2_state(&mut self) -> Result<(), NothingSaved> {
        ffi::rng::restore_rand2_state()
    }
}
