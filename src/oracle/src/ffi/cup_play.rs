//! src/diy_competition.c's cup play over `_diyCompetitionStart`, its match results given by
//! the simulation's stubs.

use super::bindings::diy_competition_c as diy;
use super::bindings::swos_amiga_data_imports_c as data;
use super::cup::set_cup_session;
use super::draw::loaded;
use super::league::image;
use super::running::IMAGE_LEN;
use super::stubs;
use super::tournament::{STATE_LEN, read, write};
use crate::cup_play::{CachedResults, CupMatchSetup, Decided, Taken};

/// The cache's word for no result.
const NONE: u16 = u16::MAX;

/// The session set: the competition, the tournament's state, the teams, no career game.
fn set(setup: &CupMatchSetup) {
    set_cup_session(&setup.competition, 0, &setup.teams, &setup.career_team);
    write(&setup.state);
    // SAFETY: plain writes to globals the stubs own.
    unsafe {
        stubs::_gameType = 0;
        stubs::_careerLoadedFlag = 0;
    }
}

/// The results given the simulation's stubs, the caches empty so upstream asks them; or, with
/// none, upstream's simulation armed.
fn cache(results: Option<&CachedResults>) {
    // SAFETY: plain writes to globals the lock's holder owns.
    unsafe {
        data::_cachedViewResultTeam1Goals = [NONE];
        data::_cachedExtraTimeTeam1Goals = [NONE];
        data::_cachedPenaltyShootoutTeam1Score = [NONE];
        if let Some(results) = results {
            stubs::SIMULATED = [results.normal, results.extra, results.penalties];
        }
        stubs::ASKED = (None, false, false);
    }
    if results.is_none() {
        super::sim_play::arm();
    }
}

/// The parts of the results upstream asked the simulation's stubs.
fn taken() -> Taken {
    // SAFETY: a plain read of a global the lock's holder owns.
    let (normal, extra, penalties) = unsafe { stubs::ASKED };
    Taken {
        normal: normal.is_some(),
        venue: normal,
        extra,
        penalties,
    }
}

/// `ProcessDIYKnockoutMatchResultAndCareerUpdates` between the selected teams 0 and 1.
pub(crate) fn decide(setup: &CupMatchSetup, first_leg: (u16, u16)) -> Decided {
    set(setup);
    cache(Some(setup.results.as_ref().expect("a tie's results given")));
    let (mut goals, mut aggregate, mut penalties, mut flags) = (0, 0, 0, 0);
    // SAFETY: the function reads the globals just set and the two selected teams' records; it
    // writes its four outputs.
    unsafe {
        diy::ProcessDIYKnockoutMatchResultAndCareerUpdates(
            loaded(0),
            loaded(1),
            first_leg.0,
            first_leg.1,
            &raw mut goals,
            &raw mut aggregate,
            &raw mut penalties,
            &raw mut flags,
        );
    }
    Decided {
        goals,
        aggregate,
        penalties,
        flags,
        taken: taken(),
    }
}

/// `DIY_PlayCupMatchAndAdvance` of `setup`, simulated (mode 0).
pub(crate) fn play(setup: &CupMatchSetup) -> (Box<[u8; IMAGE_LEN]>, Box<[u8; STATE_LEN]>, Taken) {
    set(setup);
    cache(setup.results.as_ref());
    // SAFETY: the function reads and writes the globals just set; its externs are the compiled
    // data's and the stubs.
    let played = super::halting(|| unsafe {
        diy::DIY_PlayCupMatchAndAdvance(0, 0);
    });
    super::sim_play::disarm();
    played.expect("a cup match does not halt");
    let taken = taken();
    (image(), read(), taken)
}

/// `DIY_UpdateCompetitionProgress` of `competition`, the tournament's state `state`.
pub(crate) fn next_round(
    competition: &[u8; IMAGE_LEN],
    state: &[u8; STATE_LEN],
) -> Box<[u8; IMAGE_LEN]> {
    // SAFETY: a plain write of the struct's size to a global the lock's holder owns.
    unsafe {
        (&raw mut diy::_diyCompetitionStart)
            .cast::<u8>()
            .copy_from_nonoverlapping(competition.as_ptr(), IMAGE_LEN);
        stubs::_gameType = 0;
        stubs::_careerLoadedFlag = 0;
    }
    write(state);
    // SAFETY: the function reads and writes the globals just set.
    unsafe { diy::DIY_UpdateCompetitionProgress() };
    image()
}

/// `GetNextCupRoundStringIndex`.
pub(crate) fn round_name(round: u16, rounds: u16, base: u16) -> u16 {
    // SAFETY: a pure function of its arguments.
    unsafe { diy::GetNextCupRoundStringIndex(round, rounds, base) }
}

/// `GetCupRoundStringIndex`.
pub(crate) fn stage_name(
    stage: u16,
    groups: u16,
    stages: u16,
    teams: u16,
    third_place: u16,
) -> u16 {
    // SAFETY: a pure function of its arguments.
    unsafe { diy::GetCupRoundStringIndex(stage, groups, stages, teams, third_place) }
}
