//! The externs the compiled units reference that the oracle does not compile, defined in Rust
//! with their upstream values (docs/PLAN.md §7.3). Each global is reset by [`reset`]. A
//! function stub records what the tests need to see, or panics as upstream halts.

use std::ffi::{c_int, c_void};
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

/// The team numbers a test assigned to the selected teams, by identity
/// ([`Oracle::assign_teams`](crate::Oracle::assign_teams)): none unless it did.
pub(crate) static mut ASSIGNED: [Option<u16>; SELECTED_TEAMS] = [None; SELECTED_TEAMS];

/// Upstream's team loader (src/career_teams.c `VerifyTeamLoaded`): the career club for a team
/// of [`CAREER_COUNTRY`], otherwise the selected team the number is assigned to
/// ([`ASSIGNED`]), else the one of the number's ordinal, so the identity upstream derives
/// from the record (`diy_team_to_identity`) is that team's.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn VerifyTeamLoaded(
    team: u16,
) -> *mut super::bindings::diy_competition_c::TeamFileEx {
    let [country, ordinal] = team.to_be_bytes();
    if country == CAREER_COUNTRY {
        return (&raw mut super::bindings::diy_competition_c::_careerTeam).cast();
    }
    // SAFETY: a plain read of a global the lock's holder owns.
    let assigned = unsafe { ASSIGNED }
        .iter()
        .position(|&number| number == Some(team));
    (&raw mut super::bindings::diy_competition_c::_selectedTeams)
        .cast::<super::bindings::diy_competition_c::TeamFileEx>()
        .wrapping_add(assigned.unwrap_or(usize::from(ordinal) % SELECTED_TEAMS))
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

/// A world's league phase at a stage of groups' end (src/league_phase.c
/// `LeaguePhase_GroupsEnd`: its play-off ties): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_GroupsEnd(_diy: *mut u8, _after: i32) {}

/// Another confederation's main round at a stage of groups' end (src/conti_season.c
/// `Conti_GroupsEnd`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn Conti_GroupsEnd(_diy: *mut u8, _after: i32) {}

/// A world's state championship at a stage of groups' end (src/states.c `States_GroupsEnd`):
/// none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn States_GroupsEnd(_diy: *mut u8, _after: i32) {}

/// A world's state championship at its knockout's end (src/states.c `States_KnockoutEnd`):
/// none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn States_KnockoutEnd() {}

/// The bytes of a tactics record (`sizeof(TeamTactics)`).
const TACTICS_LEN: usize = 370;

/// The user's tactics A to F, which the tactics table points at (src/career_settings.c): the
/// oracle's, empty (the tests point the table at their own records).
#[unsafe(no_mangle)]
pub(crate) static mut _teamTacticsUSERA: [u8; TACTICS_LEN] = [0; TACTICS_LEN];

/// See [`_teamTacticsUSERA`].
#[unsafe(no_mangle)]
pub(crate) static mut _teamTacticsUSERB: [u8; TACTICS_LEN] = [0; TACTICS_LEN];

/// See [`_teamTacticsUSERA`].
#[unsafe(no_mangle)]
pub(crate) static mut _teamTacticsUSERC: [u8; TACTICS_LEN] = [0; TACTICS_LEN];

/// See [`_teamTacticsUSERA`].
#[unsafe(no_mangle)]
pub(crate) static mut _teamTacticsUSERD: [u8; TACTICS_LEN] = [0; TACTICS_LEN];

/// See [`_teamTacticsUSERA`].
#[unsafe(no_mangle)]
pub(crate) static mut _teamTacticsUSERE: [u8; TACTICS_LEN] = [0; TACTICS_LEN];

/// See [`_teamTacticsUSERA`].
#[unsafe(no_mangle)]
pub(crate) static mut _teamTacticsUSERF: [u8; TACTICS_LEN] = [0; TACTICS_LEN];

/// The season shown (upstream `_currentSeasonShowing`, src/career_status_flags.c).
#[unsafe(no_mangle)]
pub(crate) static mut _currentSeasonShowing: u16 = 0;

/// The session's seed (upstream `_newRandomSeedCopy`, src/career_settings.c).
#[unsafe(no_mangle)]
pub(crate) static mut _newRandomSeedCopy: u8 = 0;

/// Whether the career's manager selects a national team (upstream `_isNationalSelector`).
#[unsafe(no_mangle)]
pub(crate) static mut _isNationalSelector: u16 = 0;

/// The edited team's players and line-up (upstream `_numPlayers`, `_playerPositions`): the
/// menus' current team, which no test plays.
#[unsafe(no_mangle)]
pub(crate) static mut _numPlayers: u16 = 0;

/// See [`_numPlayers`].
#[unsafe(no_mangle)]
pub(crate) static mut _playerPositions: [u8; 32] = [0; 32];

/// The career's primary competition (upstream `_primaryCompetitionBuf`): only its address is
/// compared (a world's league in zones), and no test's competition is it.
#[unsafe(no_mangle)]
pub(crate) static mut _primaryCompetitionBuf: [u8; 8192] = [0; 8192];

/// A world's league in zones: the place in its zone (src/league_split.c
/// `LeagueSplit_ZoneFind`): none here, the place itself.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_ZoneFind(_league: *const c_void, place: u16) -> u16 {
    place
}

/// A world's league in zones: a team's zone (`LeagueSplit_ZoneOfTeam`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_ZoneOfTeam(
    _league: *const c_void,
    _team: u16,
    _zones: *mut *const u8,
    _rows: *mut i32,
) -> i32 {
    -1
}

/// A world's league in zones: a count scaled to the league (`LeagueSplit_ZoneScale`): none
/// here, the count itself.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_ZoneScale(
    _zones: *const u8,
    _rows: i32,
    _row: i32,
    count: u16,
) -> u16 {
    count
}

/// A friendly club's long bench (src/long_bench.c `LongBench_FrOrder`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LongBench_FrOrder(_team: u16, _count: *mut u16) -> *const u8 {
    std::ptr::null()
}

/// A long bench's record past the team's (src/squad_extras.c `SquadExtras_BenchRecord`): none
/// here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn SquadExtras_BenchRecord(_team: u16, _index: u16) -> *mut c_void {
    std::ptr::null_mut()
}

/// The kinds of statuses a computer club's records keep (`WorldRules_CpuOpen`: its cards,
/// its injuries), as the test sets them.
pub(crate) static mut COMPUTER_STATUSES: (bool, bool) = (false, false);

/// A world's computer clubs' cards (src/world_rules.c `WRULE_CPU_CARDS`).
const CPU_CARDS: c_int = 1;

/// A world's computer clubs' injuries (`WRULE_CPU_INJURIES`).
const CPU_INJURIES: c_int = 2;

/// Whether a computer club's records keep kind `rule` (src/world_rules.c
/// `WorldRules_CpuOpen`): as [`COMPUTER_STATUSES`] says.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldRules_CpuOpen(rule: c_int) -> c_int {
    // SAFETY: a plain read of a global the lock's holder owns.
    let (cards, injuries) = unsafe { COMPUTER_STATUSES };
    c_int::from(match rule {
        CPU_CARDS => cards,
        CPU_INJURIES => injuries,
        _ => false,
    })
}

/// Whether a computer club's records keep either kind (`WorldRules_CpuAny`).
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldRules_CpuAny() -> c_int {
    c_int::from(WorldRules_CpuOpen(CPU_CARDS) != 0 || WorldRules_CpuOpen(CPU_INJURIES) != 0)
}

/// A DIY competition's game type (`GAME_TYPE_DIY_COMPETITION`).
const DIY_GAME: u16 = 1;

/// A preset competition's (`GAME_TYPE_PRESET_COMPETITION`).
const PRESET_GAME: u16 = 2;

/// Whether a computer club's statuses of kind `rule` count down before a simulated match
/// (src/world_rules.c `WorldRules_CpuCountdown`, no calendar's day running): the kind kept
/// ([`WorldRules_CpuOpen`]), in a DIY or a preset competition.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldRules_CpuCountdown(rule: c_int) -> c_int {
    // SAFETY: a plain read of a global the lock's holder owns.
    let game = unsafe { _gameType };
    c_int::from(WorldRules_CpuOpen(rule) != 0 && (game == DIY_GAME || game == PRESET_GAME))
}

/// Whether the replay corpus is armed (src/replay_test.c `ReplayTest_Armed`): no recording
/// plays here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn ReplayTest_Armed() -> c_int {
    0
}

/// Whether a recording is replayed (src/replay_test.c `ReplayTest_Running`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn ReplayTest_Running() -> c_int {
    0
}

/// Whether the rollback rig runs (src/rollback_rig.c `Rollback_Enabled`): not here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn Rollback_Enabled() -> c_int {
    0
}

/// Whether a world's league phase shows its one table (src/league_phase.c
/// `LeaguePhase_Showing`; the career's calendar walk asks, M7): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeaguePhase_Showing() -> i32 {
    0
}

/// A world's split told the career's calendar walk is on (src/league_split.c
/// `LeagueSplit_Walking`, M7): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn LeagueSplit_Walking(_on: i32) {}

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

/// The countdown of a simulated match's teams' statuses (src/career_status.c
/// `InitInGameTeams_WithMatchParam`): upstream's (renamed `upstream_*`, `units.toml`) when
/// [`UPSTREAM_SIMULATION`] says so, else none, the competitions' tests giving their results.
/// With it, the match engine's and the career's functions a match calls: none of them writes
/// what the oracle checks, so none runs (the career's own records of a match are M7's, the
/// players' match state M6's).
///
/// # Safety
///
/// `team1` and `team2` are team records upstream's countdown reads and writes.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C-unwind" fn InitInGameTeams_WithMatchParam(
    team1: *mut u8,
    team2: *mut u8,
) {
    // SAFETY: the caller's; a plain read of a global the lock's holder owns.
    unsafe {
        if UPSTREAM_SIMULATION {
            super::bindings::career_status_c::upstream_InitInGameTeams_WithMatchParam(
                team1.cast(),
                team2.cast(),
            );
        }
    }
}

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

/// The results the simulation's stubs give a cup match (normal time's, extra time's, the
/// shoot-out's), each the home side's and the away side's.
pub(crate) static mut SIMULATED: [(u16, u16); 3] = [(0, 0); 3];

/// What the simulation's stubs were asked: normal time (with its mode flag), extra time,
/// penalties.
pub(crate) static mut ASKED: (Option<u16>, bool, bool) = (None, false, false);

/// Whether the simulation's stubs hand over to upstream's simulation (renamed `upstream_*`,
/// `units.toml`), as a test of competitions simulated to their end asks.
pub(crate) static mut UPSTREAM_SIMULATION: bool = false;

/// A simulated result (src/match_sim.c `CalculateViewResult`): the oracle's
/// ([`SIMULATED`]), its mode flag (0 the home side's ground, 0xFF neutral) recorded; or
/// upstream's, when [`UPSTREAM_SIMULATION`] says so.
///
/// # Safety
///
/// `out1` and `out2` are words to write; upstream's simulation reads the competition and the
/// teams.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C-unwind" fn CalculateViewResult(
    context: *mut u8,
    mode: u16,
    goals1: u16,
    goals2: u16,
    column: u16,
    team1: *mut u8,
    team2: *mut u8,
    out1: *mut u16,
    out2: *mut u16,
) {
    // SAFETY: the caller's; plain reads and writes of globals the lock's holder owns.
    unsafe {
        ASKED.0 = Some(mode);
        if UPSTREAM_SIMULATION {
            super::bindings::match_sim_c::upstream_CalculateViewResult(
                context.cast(),
                mode,
                goals1,
                goals2,
                column,
                team1.cast(),
                team2.cast(),
                out1,
                out2,
            );
            return;
        }
        let [(home, away), _, _] = SIMULATED;
        out1.write(home);
        out2.write(away);
    }
}

/// A world's computer clubs field available players for a result (src/world_rules.c
/// `WorldRules_CpuLineupsBegin`, the SWOS 96/97 Mod's rules in another world, M5 #9): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldRules_CpuLineupsBegin(
    _which: i32,
    _team1: *mut u8,
    _team2: *mut u8,
    _mode: i16,
) {
}

/// The line-ups put back after a result (`WorldRules_CpuLineupsEnd`): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldRules_CpuLineupsEnd(_which: i32) {}

/// A world's keeper injuries after a result (`WorldRules_GkAfterSim`, M5 #9): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldRules_GkAfterSim(_team: *mut u8, _stand_ins: *const u8) {}

/// Whether a simulated match's statuses apply (src/career_status.c
/// `ShouldApplyRandomPlayerStatusAfterSim`: the career's competitions, M7), as the test sets it.
pub(crate) static mut STATUSES_APPLY: bool = true;

/// See [`STATUSES_APPLY`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn ShouldApplyRandomPlayerStatusAfterSim() -> u16 {
    // SAFETY: a plain read of a global the lock's holder owns.
    u16::from(unsafe { STATUSES_APPLY })
}

/// The career club's post-match form changes (src/career_status.c
/// `Career_ApplyPostMatchPlayerStatModsIfCareerTeam`, M7): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn Career_ApplyPostMatchPlayerStatModsIfCareerTeam(
    _team: *mut u8,
    _mods: *mut u8,
) {
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

/// A simulated extra time (src/match_sim.c `ExtraTimeSecondPhaseScoring`): the oracle's
/// ([`SIMULATED`]), recorded.
///
/// # Safety
///
/// `home` and `away` are words to write.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C-unwind" fn ExtraTimeSecondPhaseScoring(home: *mut u16, away: *mut u16) {
    // SAFETY: the caller's; plain reads and writes of globals the lock's holder owns.
    unsafe {
        if UPSTREAM_SIMULATION {
            ASKED.1 = true;
            super::bindings::match_sim_c::upstream_ExtraTimeSecondPhaseScoring(home, away);
            return;
        }
        let [_, (extra_home, extra_away), _] = SIMULATED;
        home.write(extra_home);
        away.write(extra_away);
        ASKED.1 = true;
    }
}

/// A simulated shoot-out (src/match_sim.c `PenaltyShootoutScorerPair`): the oracle's
/// ([`SIMULATED`]), recorded.
///
/// # Safety
///
/// `home` and `away` are words to write.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C-unwind" fn PenaltyShootoutScorerPair(home: *mut i16, away: *mut u16) {
    // SAFETY: the caller's; plain reads and writes of globals the lock's holder owns.
    unsafe {
        if UPSTREAM_SIMULATION {
            ASKED.2 = true;
            super::bindings::match_sim_c::upstream_PenaltyShootoutScorerPair(home, away);
            return;
        }
        let [_, _, (penalties_home, penalties_away)] = SIMULATED;
        home.write(penalties_home.cast_signed());
        away.write(penalties_away);
        ASKED.2 = true;
    }
}

/// A world's play-off decides a level tie by its rule's club (src/world_playoff.c): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldPlayoff_LevelWinner(_team1: u16, _team2: u16) -> i32 {
    0
}

/// A world's components told a tie's decision (src/world_playoff.c, src/world_cups.c,
/// src/world_cupd.c, src/world_supercup.c, src/uefa_access.c, src/conti_season.c): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldPlayoff_TieDecided(_team1: u16, _team2: u16, _won: i32) {}

/// See [`WorldPlayoff_TieDecided`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldCups_TieDecided(_team1: u16, _team2: u16, _won: i32) {}

/// See [`WorldPlayoff_TieDecided`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn CupD_TieDecided(_team1: u16, _team2: u16, _won: i32) {}

/// See [`WorldPlayoff_TieDecided`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn SuperCup_TieDecided(_team1: u16, _team2: u16, _won: i32) {}

/// See [`WorldPlayoff_TieDecided`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn UefaQual_TieDecided(_team1: u16, _team2: u16, _won: i32) {}

/// See [`WorldPlayoff_TieDecided`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn Conti_TieDecided(_team1: u16, _team2: u16, _won: i32) {}

/// A world's UEFA prize for a cup won (src/uefa_coef.c): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn UefaPrize_CupWon(_comp_id: u8, _winner: u16) {}

/// A world's own next round of a cup (src/world_playoff.c, src/world_cups.c,
/// src/world_cupd.c, src/world_europe.c): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldPlayoff_ArrangeNextRound() {}

/// See [`WorldPlayoff_ArrangeNextRound`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldCups_ArrangeNextRound() {}

/// See [`WorldPlayoff_ArrangeNextRound`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn CupD_ArrangeNextRound() {}

/// See [`WorldPlayoff_ArrangeNextRound`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldEurope_RoundPlayed() {}

/// See [`WorldPlayoff_ArrangeNextRound`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldPlayoff_RoundSize() {}

/// A world's own round names (src/world_cups.c, src/world_playoff.c): none here.
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldCups_RoundStringIndex(
    _comp_id: u8,
    _round: u16,
    _rounds: u16,
) -> i32 {
    -1
}

/// See [`WorldCups_RoundStringIndex`].
#[unsafe(no_mangle)]
pub(crate) extern "C-unwind" fn WorldPlayoff_RoundName(_round: u16, _rounds: u16) -> *const u8 {
    std::ptr::null()
}

/// The career calendar's cache (src/career_teams.c), which a cup's progress writes in a career
/// game only (`DIY_RecordCupBracketProgress`, M7): never written here, the game no career.
#[unsafe(no_mangle)]
pub(crate) static mut _teamSelectionKeyCache: [u8; 1] = [0];

/// See [`_teamSelectionKeyCache`].
#[unsafe(no_mangle)]
pub(crate) static mut _teamSelectionKeyCacheCount: u16 = 0;

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
        ASSIGNED = [None; SELECTED_TEAMS];
        UPSTREAM_SIMULATION = false;
    }
}
