//! The externs the compiled units reference that the oracle does not compile, defined in Rust
//! with their upstream values (docs/PLAN.md §7.3). Each global is reset by [`reset`]. A
//! function stub records what the tests need to see, or panics as upstream halts.

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

/// Whether the DOS release's code runs (upstream src/dos_graphics.c `_gDosGameplay`, set by
/// `Gameplay_SetStyle`).
#[unsafe(no_mangle)]
pub(crate) static mut _gDosGameplay: c_int = 0;

/// [`FORMATIONS`] before `TacticsData_SelectRelease` is called.
pub(crate) const NO_FORMATIONS: c_int = -1;

/// The argument `TacticsData_SelectRelease` was last called with: 1 for the DOS release's
/// formations, 0 for the Amiga's, or [`NO_FORMATIONS`].
pub(crate) static mut FORMATIONS: c_int = NO_FORMATIONS;

/// The tweaks `Tweaks_On` says are on (upstream include/tweaks.h's bits).
pub(crate) static mut TWEAKS: u32 = 0;

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

/// Upstream points the formations the DOS release re-tuned at its coverage or the Amiga's
/// (src/tactics_data.c): the stub records which.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn TacticsData_SelectRelease(dos: c_int) {
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { FORMATIONS = dos };
}

/// Whether `tweak` is on (upstream src/tweaks.c, which also turns every tweak off online and
/// in the rigs): as [`TWEAKS`] says.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn Tweaks_On(tweak: u32) -> i32 {
    // SAFETY: a plain read of a global the lock's holder owns.
    i32::from(unsafe { TWEAKS } & tweak != 0)
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
        _gDosGameplay = 0;
        FORMATIONS = NO_FORMATIONS;
        TWEAKS = 0;
        _panicReasonCode = 0;
    }
}
