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

/// Upstream's number of the original world (include/world.h `WORLD_9697`).
pub(crate) const WORLD_9697: u32 = 0;

/// Upstream's number of World 2026-27 (include/world.h `WORLD_2627`).
pub(crate) const WORLD_2627: u32 = 1;

/// The world `World_Current` answers (upstream src/world.c, which reads it from the settings).
pub(crate) static mut WORLD: u32 = WORLD_9697;

/// The round robins of 2 to 24 clubs (upstream src/swos_amiga_ptr_imports.c
/// `_fixtureSchedulePtrTable`, which the oracle does not link): `FixtureRounds_Install` points
/// the odd ones at its rounds; `ffi::tables` points them all at the original's first.
#[unsafe(no_mangle)]
pub(crate) static mut _fixtureSchedulePtrTable: [*const u8; super::tables::SCHEDULE_SIZES] =
    [std::ptr::null(); super::tables::SCHEDULE_SIZES];

/// Whether `LeaguePhase_InstallCups` was called: the European cups then play league phases.
pub(crate) static mut LEAGUE_PHASES: bool = false;

/// The world played (upstream src/world.c): as [`WORLD`] says.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn World_Current() -> u32 {
    // SAFETY: a plain read of a global the lock's holder owns.
    unsafe { WORLD }
}

/// Upstream puts a world's league-phase cups in the career's contest table
/// (src/league_phase.c): the stub records it.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_InstallCups(_cups: *mut std::ffi::c_void) {
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { LEAGUE_PHASES = true };
}

/// A world's league phase sorts its own table (src/league_phase.c): none here, so upstream's
/// sort runs.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_Sort() -> i32 {
    0
}

/// A world league's split reorders its table after the sort (src/league_split.c): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_Sort() {}

/// The selected teams `VerifyTeamLoaded` gives (upstream `SELECTED_TEAMS_MAX`).
pub(crate) const SELECTED_TEAMS: usize = 160;

/// The country of the team numbers `VerifyTeamLoaded` gives as the career club.
pub(crate) const CAREER_COUNTRY: u8 = 0xFF;

/// Upstream's team loader (src/career_teams.c `VerifyTeamLoaded`): the career club for a team
/// of [`CAREER_COUNTRY`], otherwise the selected team of the number's ordinal, so the identity
/// upstream derives from the record (`diy_team_to_identity`) is the ordinal.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn VerifyTeamLoaded(
    team: u16,
) -> *mut super::bindings::diy_competition_c::TeamFileEx {
    let [country, ordinal] = team.to_be_bytes();
    if country == CAREER_COUNTRY {
        (&raw mut super::bindings::diy_competition_c::_careerTeam).cast()
    } else {
        (&raw mut super::bindings::diy_competition_c::_selectedTeams)
            .cast::<super::bindings::diy_competition_c::TeamFileEx>()
            .wrapping_add(usize::from(ordinal) % SELECTED_TEAMS)
    }
}

/// The session's pitch type (upstream `_pitchType`, the settings': -2 the season's).
#[unsafe(no_mangle)]
pub(crate) static mut _pitchType: u16 = 0;

/// The view's month (upstream `_viewRandomPhase`).
#[unsafe(no_mangle)]
pub(crate) static mut _viewRandomPhase: u8 = 0;

/// The divisions' default names (upstream `_divisionStringTable`, src/swos_amiga_ptr_imports.c).
pub(crate) const DIVISION_NAMES: usize = 4;

/// The divisions' default names: the tests point them at theirs.
#[unsafe(no_mangle)]
pub(crate) static mut _divisionStringTable: [*const u8; DIVISION_NAMES] =
    [std::ptr::null(); DIVISION_NAMES];

/// What ends upstream's lists of records: a pointer-sized 0xFFFFFFFF.
pub(crate) const LIST_END: usize = 0xFFFF_FFFF;

/// The national teams' competitions (upstream `_nationalTeamCompetitionsAndQualifications1`, a
/// list of records walked as 32-bit words): a record, or none, then the end.
#[unsafe(no_mangle)]
pub(crate) static mut _nationalTeamCompetitionsAndQualifications1: [usize; 2] = [LIST_END; 2];

/// The contests on the continental suspensions' track (upstream
/// `_europeanCupsandNationalTeamCompetitionsAndQualifications`): a record, or none, then the
/// end.
#[unsafe(no_mangle)]
pub(crate) static mut _europeanCupsandNationalTeamCompetitionsAndQualifications: [usize; 2] =
    [LIST_END; 2];

/// A world league's split (src/league_split.c `LeagueSplit_Init`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_Init(_country: u8, _division: u16, _after: i32) {}

/// Whether a world's Super Cup is the UEFA's (src/world_supercup.c `SuperCup_UefaTie`): no
/// world here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn SuperCup_UefaTie() -> i32 {
    0
}

/// Upstream's string copy (src/string_utils.c `StringCopy`): `src` and its end into `dst`, the
/// end's place returned; a null `src` copies only an end.
///
/// # Safety
///
/// `src` is null or a string ended by a 0; `dst` has room for it.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C-unwind" fn StringCopy(src: *const u8, dst: *mut u8) -> *mut u8 {
    // SAFETY: the caller's.
    unsafe {
        if src.is_null() {
            dst.write(0);
            return dst;
        }
        let mut at = 0;
        loop {
            let byte = src.add(at).read();
            dst.add(at).write(byte);
            if byte == 0 {
                return dst.add(at);
            }
            at += 1;
        }
    }
}

/// Upstream copies a country's table entries to its second team file (src/world.c): no
/// world has one at the pin, so nothing to copy.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn World_SyncSecondFiles() {}

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
        WORLD = WORLD_9697;
        LEAGUE_PHASES = false;
    }
}
