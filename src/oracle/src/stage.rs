//! A tournament's stage ended and the next loaded (upstream src/career_teams.c
//! `DIY_AdvanceToNextCompetitionSlot`, its stage ends and the load in src/diy_competition.c):
//! the tournament's state and the competition under way after, from images of both. The
//! world's stage-end hooks are the stubs' (none), and no career game is played, so the career's
//! records of the new stage (`DIY_PostLoadCompetitionDispatch`) are not run. Before the call
//! the draws' scratch (`_diyTeamShuffleScratch`) is filled with [`SCRATCH`], so an order read
//! from what no draw wrote shows. A next stage's draw comes from the second generator: set it
//! with [`Oracle::set_rand2_state`] first.

use crate::Oracle;
use crate::cup::SelectedTeam;
use crate::ffi;
use crate::play::Teams;
use crate::running::IMAGE_LEN;
use crate::tournament::{STATE_LEN, Started};

/// The byte the draws' scratch holds before each call: no team's index a draw gives.
pub const SCRATCH: u8 = 0xEE;

/// What `DIY_AdvanceToNextCompetitionSlot` reads: the tournament's state, the competition
/// under way, the teams (their prices and controls for the next stage's draw, their names for
/// the groups' ranking).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSetup {
    /// The tournament's state before.
    pub state: Box<[u8; STATE_LEN]>,
    /// The competition under way before.
    pub competition: Box<[u8; IMAGE_LEN]>,
    /// The selected teams' prices and controls, by ordinal.
    pub teams: Vec<SelectedTeam>,
    /// The career club's.
    pub career_team: SelectedTeam,
    /// The teams' names and numbers.
    pub names: Teams,
}

impl Oracle {
    /// `DIY_AdvanceToNextCompetitionSlot` of `setup`: the tournament's state and the
    /// competition under way after, or the reason code upstream halts with.
    ///
    /// # Errors
    ///
    /// Upstream's halt (0x8034: no pattern of the World Cup's own draw for its best placed).
    pub fn next_stage(&mut self, setup: &StageSetup) -> Result<Started, u16> {
        ffi::stage::next_stage(setup)
    }
}
