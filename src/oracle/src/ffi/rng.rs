//! src/random.c, and `Randomize2` from src/game_utils.c.

use super::bindings::game_utils_c::Randomize2;
use super::bindings::random_c;
use super::halting;
use super::stubs::{self, NO_FIXED_RANDOM};
use crate::rng::{NothingSaved, Rand2SaveError, RandState};

/// The bytes of the random table (upstream `_kRandomTable[257]`).
pub const RANDOM_TABLE_LEN: usize = 257;

/// The bytes of the second generator's save stack (`_savedRandState[15]`).
pub const SAVED_RAND_STATE_LEN: usize = 15;

/// The bytes a save of the second generator takes: its counter, key index and key.
const RAND_STATE_BYTES: usize = 3;

/// The start of the save stack.
fn saved_rand_state() -> *mut u8 {
    (&raw mut random_c::_savedRandState).cast::<u8>()
}

/// Resets the unit's globals to upstream's values; the save stack's pointer to its start, as
/// upstream defines it.
pub(super) fn reset() {
    // SAFETY: plain writes to globals the lock's holder owns, the stack's within its bytes.
    unsafe {
        random_c::_seed = 0;
        random_c::_randXorIndex = 0;
        random_c::_randXorKey = 0;
        saved_rand_state().write_bytes(0, SAVED_RAND_STATE_LEN);
        random_c::_savedRandStatePtr = saved_rand_state();
    }
}

/// The random table.
pub(crate) fn random_table() -> [u8; RANDOM_TABLE_LEN] {
    // SAFETY: upstream defines the table with its 257 bytes (src/swos_data_bulk_imports.c);
    // bindgen declares it unsized.
    unsafe {
        (&raw const random_c::_kRandomTable)
            .cast::<[u8; RANDOM_TABLE_LEN]>()
            .read()
    }
}

/// The first generator's state.
pub(crate) fn rand_state() -> RandState {
    // SAFETY: plain reads of globals the lock's holder owns.
    unsafe {
        RandState {
            seed: random_c::_seed,
            xor_index: random_c::_randXorIndex,
            xor_key: random_c::_randXorKey,
        }
    }
}

/// Sets the first generator's state.
pub(crate) fn set_rand_state(state: RandState) {
    // SAFETY: plain writes to globals the lock's holder owns.
    unsafe {
        random_c::_seed = state.seed;
        random_c::_randXorIndex = state.xor_index;
        random_c::_randXorKey = state.xor_key;
    }
}

/// The second generator's state.
pub(crate) fn rand2_state() -> RandState {
    // SAFETY: plain reads of globals the lock's holder owns.
    unsafe {
        RandState {
            seed: stubs::_seed2,
            xor_index: stubs::_randXorIndex2,
            xor_key: stubs::_randXorKey2,
        }
    }
}

/// Sets the second generator's state.
pub(crate) fn set_rand2_state(state: RandState) {
    // SAFETY: plain writes to globals the lock's holder owns.
    unsafe {
        stubs::_seed2 = state.seed;
        stubs::_randXorIndex2 = state.xor_index;
        stubs::_randXorKey2 = state.xor_key;
    }
}

/// Pins both generators to `value`, as a recording does, or frees them.
pub(crate) fn set_fixed_random(value: Option<u8>) {
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe {
        stubs::_replayFixedRandom = value.map_or(NO_FIXED_RANDOM, i32::from);
    }
}

/// `Rand()`.
pub(crate) fn rand() -> u16 {
    // SAFETY: reads and writes globals the lock's holder owns, within the table.
    unsafe { random_c::Rand() }
}

/// `Rand2()`.
pub(crate) fn rand2() -> u16 {
    // SAFETY: as `rand`.
    unsafe { random_c::Rand2() }
}

/// `GetRandomWord()`.
pub(crate) fn random_word() -> u16 {
    // SAFETY: as `rand`.
    unsafe { random_c::GetRandomWord() }
}

/// `Randomize2(value)`, with `random_seed` in `_randomSeed`.
pub(crate) fn randomize2(value: u16, random_seed: u16) {
    // SAFETY: writes the seed and reads the table, globals the lock's holder owns.
    unsafe {
        stubs::_randomSeed = random_seed;
        Randomize2(value);
    }
}

/// The bytes on the save stack.
pub(crate) fn saved_rand2_bytes() -> usize {
    // SAFETY: the pointer stays within the stack: only `save_rand2_state` and
    // `restore_rand2_state` move it, each checking its room first.
    let offset = unsafe { random_c::_savedRandStatePtr.offset_from(saved_rand_state()) };
    usize::try_from(offset).expect("the pointer at or after the stack's start")
}

/// `SaveRand2State()`. Upstream writes the save, then halts the game when the stack is full;
/// with no room for the save it would write past the stack, so it is not called.
pub(crate) fn save_rand2_state() -> Result<(), Rand2SaveError> {
    if saved_rand2_bytes() + RAND_STATE_BYTES > SAVED_RAND_STATE_LEN {
        return Err(Rand2SaveError::NoRoom);
    }
    // SAFETY: the stack has room for the three bytes it writes; its halt unwinds through it
    // (`-fexceptions`).
    halting(|| unsafe { random_c::SaveRand2State() }).map_err(Rand2SaveError::Halted)
}

/// `RestoreRand2State()`. With no save on the stack upstream would read before it, so it is
/// not called.
pub(crate) fn restore_rand2_state() -> Result<(), NothingSaved> {
    if saved_rand2_bytes() < RAND_STATE_BYTES {
        return Err(NothingSaved);
    }
    // SAFETY: the stack holds the three bytes it reads.
    unsafe { random_c::RestoreRand2State() };
    Ok(())
}
