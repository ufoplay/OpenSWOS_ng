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

/// What ends upstream's lists of records: a pointer-sized 0xFFFFFFFF.
pub(crate) const LIST_END: usize = 0xFFFF_FFFF;

/// A world league's split (src/league_split.c `LeagueSplit_Init`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_Init(_country: u8, _division: u16, _after: i32) {}

/// A world's league phase counts its stage's own matches (src/league_phase.c
/// `LeaguePhase_StageDays`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_StageDays(
    _comp_id: u8,
    _groups: u16,
    _per_group: u16,
) -> i32 {
    0
}

/// A world's state championship counts its stage's own matches (src/states.c
/// `States_StageDays`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn States_StageDays(
    _comp_id: u8,
    _groups: u16,
    _per_group: u16,
) -> i32 {
    0
}

/// A world's league phase starts its stage of groups (src/league_phase.c
/// `LeaguePhase_StageStart`: its draw): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_StageStart() {}

/// A world's league phase draws its knockout its own way (src/league_phase.c
/// `LeaguePhase_KnockoutDraw`): none here, so upstream's draw runs.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_KnockoutDraw(_diy: *mut u8, _teams: u16) -> i32 {
    0
}

/// A world's state championship starts its stage of groups (src/states.c
/// `States_StageStart`: its calendar): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn States_StageStart() {}

/// A world's state championship starts its knockout (src/states.c `States_KnockoutStart`: its
/// fixed draw): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn States_KnockoutStart() {}

/// The match engine's and the career's globals a league match reads and writes
/// (src/diy_competition.c `DIY_PlayOrSimulateLeagueFixture` and the career updates after it):
/// none of the oracle's checks reads them.
#[unsafe(no_mangle)]
pub(crate) static mut _careerLoadedFlag: u16 = 0;
/// See [`_careerLoadedFlag`].
#[unsafe(no_mangle)]
pub(crate) static mut _careerMatchSimulatedFlag: u16 = 0;
/// See [`_careerLoadedFlag`].
#[unsafe(no_mangle)]
pub(crate) static mut _careerTeamLostMatch: u16 = 0;
/// See [`_careerLoadedFlag`]: 0, no career game, so the career's statistics are not updated.
#[unsafe(no_mangle)]
pub(crate) static mut _gameType: u16 = 0;
/// See [`_careerLoadedFlag`].
#[unsafe(no_mangle)]
pub(crate) static mut _isCareer: u16 = 0;
/// See [`_careerLoadedFlag`].
#[unsafe(no_mangle)]
pub(crate) static mut _numSelectedTeams: u16 = 0;
/// See [`_careerLoadedFlag`].
#[unsafe(no_mangle)]
pub(crate) static mut _selTeamNumber: u16 = 0;
/// The career club's promotion, which the season's moves flag
/// (`DIY_UpdatePromotionRelegationScratchCounts`).
#[unsafe(no_mangle)]
pub(crate) static mut _playPromotedAnimation: u16 = 0;
/// The career club's relegation, likewise.
#[unsafe(no_mangle)]
pub(crate) static mut _playRelegatedAnimation: u16 = 0;

/// The match engine's and the career's functions a league match calls: none of them writes
/// what the oracle checks, so none runs (the career's own records of a match are M7's, the
/// players' match state M6's).
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn InitInGameTeams_WithMatchParam(_team1: *mut u8, _team2: *mut u8) {}

/// See [`InitInGameTeams_WithMatchParam`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn SetupPlayers(
    _team1: *mut u8,
    _team2: *mut u8,
    _max_subs: i16,
    _exclude: i16,
    _changes: i16,
    _mode: i16,
) {
}

/// See [`InitInGameTeams_WithMatchParam`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn CupRecord_Match(
    _team1: *const u8,
    _team2: *const u8,
    _goals1: u16,
    _goals2: u16,
    _flags: u16,
    _penalties1: u16,
    _penalties2: u16,
    _league: i32,
) {
}

/// See [`InitInGameTeams_WithMatchParam`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn UpdateCareerPlayersFormAfterMatch(
    _competition: *mut u8,
    _team1: *mut u8,
    _team2: *mut u8,
    _goals1: u16,
    _goals2: u16,
) {
}

/// See [`InitInGameTeams_WithMatchParam`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn BeginContractNegotiationSequence(
    _competition: *mut u8,
    _team: *mut u8,
    _opponent: *mut u8,
) {
}

/// See [`InitInGameTeams_WithMatchParam`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn Career_UpdateStatsAfterMatch(
    _team1: *mut u8,
    _team2: *mut u8,
    _kind: u16,
    _goals: u16,
    _aggregate: u16,
    _penalty: u16,
    _flags: u16,
) {
}

/// See [`InitInGameTeams_WithMatchParam`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn ModHooks_CareerEvent(
    _kind: u8,
    _goal_difference: i16,
    _message: u16,
    _flags: u8,
) {
}

/// See [`InitInGameTeams_WithMatchParam`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LongBench_LeagueMatch(_on: i32) {}

/// A simulated result (src/match_sim.c `CalculateViewResult`): never reached, the oracle
/// giving each league match's result cached.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn CalculateViewResult(
    _context: *mut u8,
    _mode: u16,
    _goals1: u16,
    _goals2: u16,
    _column: u16,
    _team1: *mut u8,
    _team2: *mut u8,
    _out1: *mut u16,
    _out2: *mut u16,
) {
    unreachable!("the oracle gives each league match's result cached");
}

/// A played match (src/game_init.c `InitializeInGameTeamsAndStartGame`): never reached, the
/// oracle's league matches simulated.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn InitializeInGameTeamsAndStartGame(
    _randomize_pitch: i16,
    _min_subs: i16,
    _max_subs: i16,
    _d3: i16,
    _d7: i16,
    _team1: *mut u8,
    _team2: *mut u8,
) {
    unreachable!("the oracle's league matches are simulated");
}

/// A world's league split, league phase and state championships pair, count and record their
/// own matches (src/league_split.c, src/league_phase.c, src/states.c): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_Pair(
    _team1: *mut *mut u8,
    _team2: *mut *mut u8,
) -> i32 {
    0
}

/// See [`LeagueSplit_Pair`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_Pair(
    _team1: *mut *mut u8,
    _team2: *mut *mut u8,
) -> i32 {
    0
}

/// See [`LeagueSplit_Pair`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn States_Pair(_team1: *mut *mut u8, _team2: *mut *mut u8) -> i32 {
    0
}

/// See [`LeagueSplit_Pair`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_Count() -> i32 {
    0
}

/// See [`LeagueSplit_Pair`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_AfterResult(
    _team1: *mut u8,
    _team2: *mut u8,
    _goals1: u16,
    _goals2: u16,
) {
}

/// See [`LeagueSplit_Pair`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_Result(
    _team1: *mut u8,
    _team2: *mut u8,
    _goals1: u16,
    _goals2: u16,
) {
}

/// A world's national cup just made (src/world_cups.c `WorldCups_CupMade`: its byes, a later
/// season's draw): no world here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldCups_CupMade(_record: *const u8) {}

/// A classic world's European cup just drawn (src/world_europe.c `WorldEurope_DrawMade`: no tie
/// of one country's clubs): no world here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldEurope_DrawMade(_comp_id: u8) {}

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
