//! src/match_sim.c's simulated result over the oracle's sides' records, src/menus.c's
//! formations' orders.

use super::bindings::career_status_c as status;
use super::bindings::career_team_scalars_c as scalars;
use super::bindings::diy_competition_c as diy;
use super::bindings::match_sim_c as sim;
use super::bindings::menus_c as menus;
use super::bindings::swos_amiga_data_imports_c as data;
use super::bindings::swos_data_bulk_imports_c as bulk;
use super::bindings::tactics_data_c::{self as tactics, TeamTactics};
use super::bindings::test_rules_c as rules;
use super::cup::set_cup_session;
use super::stubs;
use crate::sim::{CAREER_LINE_UP, TACTICS_LEN, TACTICS_SLOTS};
use crate::sim_result::{CountdownSetup, ORDER_LEN, Outcome, ResultSetup, Side};

/// The bytes of a selected team's record.
const RECORD_LEN: usize = 684;

/// The formations whose orders upstream reckons (`_playersPositionsByTactics[18]`).
const FORMATIONS: usize = 18;

/// A side's record, aligned as upstream's are.
#[repr(C, align(4))]
struct Record([u8; RECORD_LEN]);

/// The sides' records the calls simulate.
static mut SIDES: [Record; 2] = [Record([0; RECORD_LEN]), Record([0; RECORD_LEN])];

/// The tactics records a call points the table at.
static mut TACTICS: [[u8; TACTICS_LEN]; TACTICS_SLOTS] = [[0; TACTICS_LEN]; TACTICS_SLOTS];

/// The tactics table pointed at `records` (the compiled table kept and returned).
pub(super) fn point_tactics(records: &[[u8; TACTICS_LEN]]) -> [*mut TeamTactics; TACTICS_SLOTS] {
    assert_eq!(records.len(), TACTICS_SLOTS, "a slot's tactics each");
    // SAFETY: plain writes to globals the lock's holder owns; the slots point at the oracle's
    // own records, which outlive the calls.
    unsafe {
        let compiled = tactics::_teamTacticsTable;
        let own = &raw mut TACTICS;
        for (slot, record) in records.iter().enumerate() {
            (*own)[slot] = *record;
            tactics::_teamTacticsTable[slot] = (&raw mut (*own)[slot]).cast::<TeamTactics>();
        }
        compiled
    }
}

/// `InitializeTacticsPositions` over `records`: the formations' orders.
pub(crate) fn defensive_orders(records: &[[u8; TACTICS_LEN]]) -> Vec<[u8; ORDER_LEN]> {
    let compiled = point_tactics(records);
    // SAFETY: the function reads the table just pointed and writes each formation's order,
    // read back by value; the table is put back.
    unsafe {
        menus::InitializeTacticsPositions();
        let orders = (&raw const menus::_playersPositionsByTactics)
            .read()
            .iter()
            .take(FORMATIONS)
            .map(|&order| order.cast::<[u8; ORDER_LEN]>().read())
            .collect();
        tactics::_teamTacticsTable = compiled;
        orders
    }
}

/// Side `index`'s record in place: `_careerTeam` for the career club, else the oracle's own.
fn side_address(side: &Side, index: usize) -> *mut u8 {
    if side.career.is_some() {
        (&raw mut diy::_careerTeam).cast()
    } else {
        // SAFETY: a pointer to the oracle's own record, no reference taken.
        unsafe { (&raw mut SIDES[index]).cast() }
    }
}

/// Side `index`'s record written.
fn write_side(side: &Side, index: usize) -> *mut u8 {
    let address = side_address(side, index);
    // SAFETY: plain writes to globals the lock's holder owns, within their storage.
    unsafe {
        address.copy_from_nonoverlapping(side.record.as_ptr(), side.record.len());
        if let Some((line_up, players)) = side.career {
            sim::_currentMatchPlayers = line_up;
            scalars::_careerNumPlayers = players;
        }
    }
    address
}

/// The records of `sides`, written at `addresses`, read back.
fn read_sides(sides: &[Side; 2], addresses: [*mut u8; 2]) -> [Vec<u8>; 2] {
    let read = |side: &Side, address: *mut u8| {
        let mut record = vec![0; side.record.len()];
        // SAFETY: a plain read of the record written and used.
        unsafe { address.copy_to_nonoverlapping(record.as_mut_ptr(), record.len()) };
        record
    };
    [read(&sides[0], addresses[0]), read(&sides[1], addresses[1])]
}

/// `InitInGameTeams_WithMatchParam` of `sides` under `setup`.
pub(crate) fn countdown(sides: &[Side; 2], setup: &CountdownSetup) -> [Vec<u8>; 2] {
    assert!(
        sides.iter().filter(|side| side.career.is_some()).count() <= 1,
        "one career club at most"
    );
    let homes = write_side(&sides[0], 0);
    let aways = write_side(&sides[1], 1);
    // SAFETY: plain writes to globals the lock's holder owns; the function reads them and the
    // two records, the caller checking every line-up place names a record within them.
    unsafe {
        diy::_diyCompetitionStart
            .base
            .__bindgen_anon_4
            .substitutesCount = setup.mode.cast_signed();
        data::_trainingGame = [u16::from(setup.training)];
        stubs::_gameType = setup.game_type;
        stubs::COMPUTER_STATUSES = setup.computer;
        status::upstream_InitInGameTeams_WithMatchParam(homes.cast(), aways.cast());
    }
    read_sides(sides, [homes, aways])
}

/// A simulated match of `setup`, then its extra time and its shoot-out as it asks.
pub(crate) fn simulate(setup: &ResultSetup) -> Result<Outcome, u16> {
    assert!(
        setup
            .sides
            .iter()
            .filter(|side| side.career.is_some())
            .count()
            <= 1,
        "one career club at most"
    );
    set_cup_session(&setup.competition, 0, &setup.teams, &setup.career_team);
    let compiled = point_tactics(&setup.tactics);
    let homes = write_side(&setup.sides[0], 0);
    let aways = write_side(&setup.sides[1], 1);
    // SAFETY: plain writes to globals the lock's holder owns.
    unsafe {
        menus::InitializeTacticsPositions();
        data::_trainingGame = [u16::from(setup.status.training)];
        rules::TestRules_Force(i32::from(setup.status.testers_rule));
        stubs::_gameType = setup.game_type;
        stubs::COMPUTER_STATUSES = setup.computer;
        stubs::STATUSES_APPLY = setup.statuses_apply;
        stubs::_isNationalSelector = u16::from(setup.national_selector);
        stubs::_currentSeasonShowing = setup.day.season;
        stubs::_newRandomSeedCopy = setup.day.seed;
        stubs::_viewRandomPhase = setup.day.phase;
    }
    let (mut home, mut away) = (0, 0);
    let (fewest, most) = setup.substitutes;
    // SAFETY: the competition, the sides, the tactics and the globals the call reads are set;
    // the caller checks every record it reads is within its storage.
    super::halting(|| unsafe {
        sim::upstream_CalculateViewResult(
            (&raw mut diy::_diyCompetitionStart).cast(),
            setup.mode,
            fewest,
            most,
            setup.column,
            homes.cast(),
            aways.cast(),
            &raw mut home,
            &raw mut away,
        );
    })?;
    let (with_extra_time, with_penalties) = setup.then;
    let extra_time = with_extra_time.then(|| {
        let (mut home, mut away) = (0, 0);
        // SAFETY: the result's state is the call's just made.
        unsafe { sim::upstream_ExtraTimeSecondPhaseScoring(&raw mut home, &raw mut away) };
        (home, away)
    });
    let penalties = with_penalties.then(|| {
        let (mut home, mut away) = (0, 0);
        // SAFETY: a draw of the second generator and a read of upstream's table.
        unsafe { sim::upstream_PenaltyShootoutScorerPair(&raw mut home, &raw mut away) };
        (home, away)
    });
    let records = read_sides(&setup.sides, [homes, aways]);
    // SAFETY: plain reads of upstream's counters, of their lengths (the first 32 of the
    // second's 64); the tactics table put back.
    let scorers = unsafe {
        tactics::_teamTacticsTable = compiled;
        let second = (&raw const bulk::_team2GoalCounterByPlayer).read();
        let mut away_scorers = [0; CAREER_LINE_UP];
        away_scorers.copy_from_slice(&second[..CAREER_LINE_UP]);
        [
            (&raw const bulk::_team1GoalCounterByPlayer).read(),
            away_scorers,
        ]
    };
    Ok(Outcome {
        goals: (home, away),
        extra_time,
        penalties,
        records,
        scorers,
    })
}
