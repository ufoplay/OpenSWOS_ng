//! Competitions' matches simulated by upstream (M5 #8d): the squads written once as upstream's
//! loaded teams (`_selectedTeams`, `_careerTeam` with its line-up) and kept by its matches; the
//! session's globals set (the tactics table pointed at the session's records, the formations'
//! orders reckoned, the rules, the day's season and seed); then a league's or a cup's match
//! played with no result given ([`ResultSetup::goals`](crate::play::ResultSetup),
//! [`CupMatchSetup::results`](crate::cup_play::CupMatchSetup) none): its teams' countdown and
//! its result upstream's (`InitInGameTeams_WithMatchParam`, `CalculateViewResult` and the rest,
//! as their stubs hand over), its draws the second generator's.

use crate::Oracle;
use crate::ffi;
use crate::sim::{Status, TACTICS_LEN};
use crate::sim_result::Side;

/// The squads and the session a competition's matches are simulated in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimSession {
    /// The selected teams' records (684 bytes each), by identity.
    pub selected: Vec<Vec<u8>>,
    /// The career club's record and line-up, when there is one.
    pub career: Option<Side>,
    /// The session's tactics, each slot's record.
    pub tactics: Vec<[u8; TACTICS_LEN]>,
    /// The status rules (their mode the competition's).
    pub status: Status,
    /// `_gameType`: 1 a DIY competition, 3 a season, 4 a career.
    pub game_type: u16,
    /// Whether a world opens a computer club's cards, its injuries (`WorldRules_CpuOpen`).
    pub computer: (bool, bool),
    /// `ShouldApplyRandomPlayerStatusAfterSim`'s answer.
    pub statuses_apply: bool,
    /// `_isNationalSelector`.
    pub national_selector: bool,
    /// `_currentSeasonShowing`.
    pub season: u16,
    /// `_newRandomSeedCopy`.
    pub seed: u8,
}

impl Oracle {
    /// The squads of `session` written as upstream's loaded teams, its globals set: the league's
    /// and the cup's matches with no result given are simulated in it.
    pub fn set_simulation(&mut self, session: &SimSession) {
        ffi::sim_play::set(session);
    }

    /// `teams` assigned to the selected teams in turn, so the stubbed loader gives a team of
    /// such a number that selected team (and the others by their numbers' ordinals) until the
    /// next lock.
    pub fn assign_teams(&mut self, teams: &[[u8; 2]]) {
        ffi::sim_play::assign(teams);
    }

    /// The first `selected` selected teams' records as upstream's matches leave them, and the
    /// career club's (its `career_len` bytes) when asked.
    #[must_use]
    pub fn squads(
        &mut self,
        selected: usize,
        career_len: Option<usize>,
    ) -> (Vec<Vec<u8>>, Option<Vec<u8>>) {
        ffi::sim_play::squads(selected, career_len)
    }
}
