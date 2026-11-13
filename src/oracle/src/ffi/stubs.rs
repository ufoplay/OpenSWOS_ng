//! The externs the compiled units reference that the oracle does not compile, defined in Rust
//! with their upstream values (docs/PLAN.md §7.3). Each global is reset by [`reset`]; the
//! functions are never called by the functions the tests call.

use std::ffi::c_int;
use std::panic::panic_any;

/// `_replayFixedRandom`'s value when no recording pins the generators (upstream
/// src/replay_test.c).
pub(crate) const NO_FIXED_RANDOM: c_int = -1;

/// The second generator's counter (upstream src/career_settings.c `_seed2`).
#[unsafe(no_mangle)]
pub(crate) static mut _seed2: u8 = 0;

/// The second generator's key index (`_randXorIndex2`).
#[unsafe(no_mangle)]
pub(crate) static mut _randXorIndex2: u8 = 0;

/// The second generator's key (`_randXorKey2`).
#[unsafe(no_mangle)]
pub(crate) static mut _randXorKey2: u8 = 0;

/// The session's random seed, which `Randomize2` XORs its value with (upstream
/// src/career_settings.c `_randomSeed`).
#[unsafe(no_mangle)]
pub(crate) static mut _randomSeed: u16 = 0;

/// A recording's fixed random value pinning both generators, or [`NO_FIXED_RANDOM`]
/// (upstream src/replay_test.c `_replayFixedRandom`).
#[unsafe(no_mangle)]
pub(crate) static mut _replayFixedRandom: c_int = NO_FIXED_RANDOM;

/// The movement scale's numerator (upstream src/dos_gameplay.c `_gMoveDeltaScaleNum`: 1, the
/// Amiga's, until `ApplyGameStyle`).
#[unsafe(no_mangle)]
pub(crate) static mut _gMoveDeltaScaleNum: i16 = 1;

/// The movement scale's denominator (`_gMoveDeltaScaleDen`).
#[unsafe(no_mangle)]
pub(crate) static mut _gMoveDeltaScaleDen: i16 = 1;

/// Why the game halted (upstream `_panicReasonCode`, read by `PanicHaltLoop`).
#[unsafe(no_mangle)]
pub(crate) static mut _panicReasonCode: u16 = 0;

/// What [`PanicHaltLoop`] panics with: upstream's reason code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UpstreamPanic(pub(crate) u16);

/// Upstream halts the game here (src/panic.c): the stub panics with the reason code, and the
/// panic unwinds through the C frames (compiled with `-fexceptions`) to the safe caller.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn PanicHaltLoop() -> ! {
    // SAFETY: a plain read of a global the lock's holder owns.
    panic_any(UpstreamPanic(unsafe { _panicReasonCode }))
}

/// Resets the stubbed globals to upstream's values.
pub(crate) fn reset() {
    // SAFETY: plain writes to globals the lock's holder owns.
    unsafe {
        _seed2 = 0;
        _randXorIndex2 = 0;
        _randXorKey2 = 0;
        _randomSeed = 0;
        _replayFixedRandom = NO_FIXED_RANDOM;
        _gMoveDeltaScaleNum = 1;
        _gMoveDeltaScaleDen = 1;
        _panicReasonCode = 0;
    }
}
