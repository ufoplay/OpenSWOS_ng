//! A cup played (upstream src/diy_competition.c): a tie decided
//! (`ProcessDIYKnockoutMatchResultAndCareerUpdates`), a match played and kept in the bracket
//! (`DIY_PlayCupMatchAndAdvance`, simulated), the next round arranged
//! (`DIY_UpdateCompetitionProgress`), the rounds' names (`GetNextCupRoundStringIndex`,
//! `GetCupRoundStringIndex`). Each match's results are the simulation's stubs' (normal time,
//! extra time, penalties), so no simulation runs; upstream asks each part it needs, and the
//! oracle tells which it asked (normal time's with its mode). A next round's draw comes from
//! the second generator: set it with [`Oracle::set_rand2_state`] first.

use crate::Oracle;
use crate::cup::SelectedTeam;
use crate::ffi;
use crate::running::IMAGE_LEN;
use crate::tournament::STATE_LEN;

/// A match's results as the simulation's stubs give them: each its home side's and away
/// side's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedResults {
    /// Normal time's goals.
    pub normal: (u16, u16),
    /// Extra time's goals.
    pub extra: (u16, u16),
    /// The shoot-out's penalties.
    pub penalties: (u16, u16),
}

/// Which parts of the results upstream asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Taken {
    /// Normal time's.
    pub normal: bool,
    /// Normal time's mode flag (0 the home side's ground, 0xFF neutral), when asked.
    pub venue: Option<u16>,
    /// Extra time's.
    pub extra: bool,
    /// The shoot-out's.
    pub penalties: bool,
}

/// A tie's decision as upstream packs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decided {
    /// The match's goals.
    pub goals: u16,
    /// The aggregate.
    pub aggregate: u16,
    /// The penalties.
    pub penalties: u16,
    /// The flags.
    pub flags: u16,
    /// The parts it took.
    pub taken: Taken,
}

/// What a cup's functions read besides the competition: the tournament's state, the teams
/// (their controls), the match's results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CupMatchSetup {
    /// The competition under way before.
    pub competition: Box<[u8; IMAGE_LEN]>,
    /// The tournament's state before.
    pub state: Box<[u8; STATE_LEN]>,
    /// The selected teams, by ordinal.
    pub teams: Vec<SelectedTeam>,
    /// The career club.
    pub career_team: SelectedTeam,
    /// The match's results.
    pub results: CachedResults,
}

impl Oracle {
    /// `ProcessDIYKnockoutMatchResultAndCareerUpdates` in `setup`'s competition, between the
    /// selected teams 0 and 1, their first leg's goals `first_leg`.
    #[must_use]
    pub fn decide_tie(&mut self, setup: &CupMatchSetup, first_leg: (u16, u16)) -> Decided {
        ffi::cup_play::decide(setup, first_leg)
    }

    /// `DIY_PlayCupMatchAndAdvance` of `setup`, simulated: the competition and the tournament's
    /// state after, and the results' parts it took.
    #[must_use]
    pub fn play_cup_match(
        &mut self,
        setup: &CupMatchSetup,
    ) -> (Box<[u8; IMAGE_LEN]>, Box<[u8; STATE_LEN]>, Taken) {
        ffi::cup_play::play(setup)
    }

    /// `DIY_UpdateCompetitionProgress` of `competition`, the tournament's state `state`: the
    /// competition after.
    #[must_use]
    pub fn next_round(
        &mut self,
        competition: &[u8; IMAGE_LEN],
        state: &[u8; STATE_LEN],
    ) -> Box<[u8; IMAGE_LEN]> {
        ffi::cup_play::next_round(competition, state)
    }

    /// `GetNextCupRoundStringIndex(round, rounds, base)`.
    #[must_use]
    pub fn round_name(&mut self, round: u16, rounds: u16, base: u16) -> u16 {
        ffi::cup_play::round_name(round, rounds, base)
    }

    /// `GetCupRoundStringIndex(stage, groups, stages, teams, third_place)`.
    #[must_use]
    pub fn stage_name(
        &mut self,
        stage: u16,
        groups: u16,
        stages: u16,
        teams: u16,
        third_place: u16,
    ) -> u16 {
        ffi::cup_play::stage_name(stage, groups, stages, teams, third_place)
    }
}
