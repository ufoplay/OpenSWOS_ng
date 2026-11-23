//! src/career_teams.c's stage advance over `_diyFileBufferCopy` and `_diyCompetitionStart`.

use super::bindings::career_teams_c as career;
use super::bindings::swos_data_bulk_imports_c as bulk;
use super::cup::set_cup_session;
use super::league::image;
use super::play::set_teams;
use super::stubs;
use super::tournament::{read, write};
use crate::stage::{SCRATCH, StageSetup};
use crate::tournament::Started;

/// `DIY_AdvanceToNextCompetitionSlot` of `setup`, no career game, the draws' scratch filled.
pub(crate) fn next_stage(setup: &StageSetup) -> Result<Started, u16> {
    set_cup_session(&setup.competition, 0, &setup.teams, &setup.career_team);
    set_teams(&setup.names);
    write(&setup.state);
    // SAFETY: plain writes to globals the lock's holder owns: the draws' scratch, the
    // career's flags the stubs own.
    unsafe {
        (*(&raw mut bulk::_diyTeamShuffleScratch)).fill(SCRATCH);
        stubs::_gameType = 0;
        stubs::_careerLoadedFlag = 0;
    }
    // SAFETY: the function reads and writes the globals just set; its externs are the compiled
    // data's and the stubs.
    super::halting(|| unsafe { career::DIY_AdvanceToNextCompetitionSlot() })?;
    Ok((read(), image()))
}
