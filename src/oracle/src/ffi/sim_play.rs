//! Upstream's simulation over the squads it keeps: the session set once, each match's call
//! armed (no result cached, the stubs handing over).

use super::bindings::career_team_scalars_c as scalars;
use super::bindings::diy_competition_c as diy;
use super::bindings::match_sim_c as sim;
use super::bindings::menus_c as menus;
use super::bindings::swos_amiga_data_imports_c as data;
use super::bindings::tactics_data_c::{self as tactics, TeamTactics};
use super::bindings::test_rules_c as rules;
use super::draw::loaded;
use super::sim_result::point_tactics;
use super::stubs::{self, SELECTED_TEAMS};
use crate::sim::TACTICS_SLOTS;
use crate::sim_play::SimSession;

/// The bytes of a selected team's record.
const RECORD_LEN: usize = 684;

/// The cache's word for no result.
const NONE: u16 = u16::MAX;

/// The session's game type, set again for each simulated call (the competitions' calls set
/// theirs).
static mut GAME_TYPE: u16 = 0;

/// The compiled tactics table, kept while a session points it at its own records.
static mut COMPILED: Option<[*mut TeamTactics; TACTICS_SLOTS]> = None;

/// The compiled tactics table put back, when a session pointed it away.
pub(super) fn reset() {
    // SAFETY: plain reads and writes of globals the lock's holder owns.
    unsafe {
        if let Some(compiled) = COMPILED {
            tactics::_teamTacticsTable = compiled;
            COMPILED = None;
        }
    }
}

/// `teams` assigned to the selected teams in turn, the others' assignments cleared.
pub(crate) fn assign(teams: &[[u8; 2]]) {
    assert!(teams.len() <= SELECTED_TEAMS, "the teams the stub holds");
    let mut assigned = [None; SELECTED_TEAMS];
    for (slot, &team) in assigned.iter_mut().zip(teams) {
        *slot = Some(u16::from_be_bytes(team));
    }
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { stubs::ASSIGNED = assigned };
}

/// The squads of `session` written, its globals set.
pub(crate) fn set(session: &SimSession) {
    assert!(
        session.selected.len() <= SELECTED_TEAMS,
        "the teams the stub holds"
    );
    for (ordinal, record) in session.selected.iter().enumerate() {
        assert_eq!(record.len(), RECORD_LEN, "a selected team's record");
        // SAFETY: a plain write of a record's size to a selected team of a global the lock's
        // holder owns.
        unsafe {
            loaded(ordinal)
                .cast::<u8>()
                .copy_from_nonoverlapping(record.as_ptr(), RECORD_LEN);
        }
    }
    // The table pointed at the session's records until the next lock: the simulated calls
    // read it.
    let compiled = point_tactics(&session.tactics);
    // SAFETY: plain reads and writes of a global the lock's holder owns.
    unsafe {
        let kept = COMPILED;
        if kept.is_none() {
            COMPILED = Some(compiled);
        }
    }
    // SAFETY: plain writes to globals the lock's holder owns, the career club's record within
    // its buffer; the formations' orders reckoned from the table just pointed.
    unsafe {
        if let Some(career) = &session.career {
            let (line_up, players) = career.career.expect("the career club's line-up");
            (&raw mut diy::_careerTeam)
                .cast::<u8>()
                .copy_from_nonoverlapping(career.record.as_ptr(), career.record.len());
            sim::_currentMatchPlayers = line_up;
            scalars::_careerNumPlayers = players;
        }
        menus::InitializeTacticsPositions();
        data::_trainingGame = [u16::from(session.status.training)];
        rules::TestRules_Force(i32::from(session.status.testers_rule));
        GAME_TYPE = session.game_type;
        stubs::COMPUTER_STATUSES = session.computer;
        stubs::STATUSES_APPLY = session.statuses_apply;
        stubs::_isNationalSelector = u16::from(session.national_selector);
        stubs::_currentSeasonShowing = session.season;
        stubs::_newRandomSeedCopy = session.seed;
    }
}

/// Upstream's simulation armed for a call: no result cached, the stubs handing over, the
/// session's game type.
pub(super) fn arm() {
    // SAFETY: plain writes to globals the lock's holder owns.
    unsafe {
        data::_cachedViewResultTeam1Goals = [NONE];
        data::_cachedExtraTimeTeam1Goals = [NONE];
        data::_cachedPenaltyShootoutTeam1Score = [NONE];
        stubs::_gameType = GAME_TYPE;
        stubs::UPSTREAM_SIMULATION = true;
    }
}

/// Upstream's simulation disarmed after a call.
pub(super) fn disarm() {
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { stubs::UPSTREAM_SIMULATION = false };
}

/// The first `selected` selected teams' records, and the career club's `career_len` bytes.
pub(crate) fn squads(
    selected: usize,
    career_len: Option<usize>,
) -> (Vec<Vec<u8>>, Option<Vec<u8>>) {
    assert!(selected <= SELECTED_TEAMS, "the teams the stub holds");
    let read = |address: *const u8, len: usize| {
        let mut record = vec![0; len];
        // SAFETY: a plain read of a record of a global the lock's holder owns, within it.
        unsafe { address.copy_to_nonoverlapping(record.as_mut_ptr(), len) };
        record
    };
    let records = (0..selected)
        .map(|ordinal| read(loaded(ordinal).cast::<u8>(), RECORD_LEN))
        .collect();
    let career = career_len.map(|len| read((&raw const diy::_careerTeam).cast::<u8>(), len));
    (records, career)
}
