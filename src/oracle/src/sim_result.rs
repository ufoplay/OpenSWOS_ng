//! A simulated result (upstream src/match_sim.c, M5 #8c): the formations' defensive orders
//! (src/menus.c `InitializeTacticsPositions`), a match's result (`CalculateViewResult`, renamed
//! `upstream_CalculateViewResult`), its extra time (`upstream_ExtraTimeSecondPhaseScoring`) and
//! its shoot-out (`upstream_PenaltyShootoutScorerPair`), called in turn as upstream keeps the
//! result's state between them. The sides' records are the oracle's own (the career club's in
//! `_careerTeam`), the competition in `_diyCompetitionStart`, the teams the stubbed loader gives
//! the selected teams'. Its draws come from the second generator: set it with
//! [`Oracle::set_rand2_state`] first. The countdown of a match's teams' statuses before it
//! (src/career_status.c `InitInGameTeams_WithMatchParam`, renamed
//! `upstream_InitInGameTeams_WithMatchParam`, M5 #8d) is called over the same records.

use crate::Oracle;
use crate::cup::SelectedTeam;
use crate::ffi;
use crate::running::IMAGE_LEN;
use crate::sim::{CAREER_LINE_UP, DayGlobals, Status, TACTICS_LEN};

/// The places of a formation's order (`TacticsPosition`).
pub const ORDER_LEN: usize = 11;

/// A side of a simulated match: its record, and the career club's line-up when it is the
/// career club.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    /// The record: a selected team's 684 bytes, the career club's 1,292.
    pub record: Vec<u8>,
    /// The career club's line-up and players: `Some` for the career club.
    pub career: Option<([u8; CAREER_LINE_UP], u16)>,
}

/// What a simulated match reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSetup {
    /// The competition under way (`ctx`).
    pub competition: Box<[u8; IMAGE_LEN]>,
    /// The mode flag (0 the home side's ground).
    pub mode: u16,
    /// The fewest substitutes and the most.
    pub substitutes: (u16, u16),
    /// The goals' column.
    pub column: u16,
    /// The home side and the away side.
    pub sides: [Side; 2],
    /// The session's tactics, each slot's record.
    pub tactics: Vec<[u8; TACTICS_LEN]>,
    /// The selected teams' prices and controls, by ordinal.
    pub teams: Vec<SelectedTeam>,
    /// The career club's.
    pub career_team: SelectedTeam,
    /// The status rules (their mode the competition's, set by the call).
    pub status: Status,
    /// `_gameType`: 1 a DIY competition, 3 a season, 4 a career.
    pub game_type: u16,
    /// Whether a computer club's records keep its cards, its injuries.
    pub computer: (bool, bool),
    /// `ShouldApplyRandomPlayerStatusAfterSim`'s answer.
    pub statuses_apply: bool,
    /// `_isNationalSelector`.
    pub national_selector: bool,
    /// The day's globals.
    pub day: DayGlobals,
    /// Whether extra time is played after, and a shoot-out after that.
    pub then: (bool, bool),
}

/// What the countdown of a match's teams' statuses reads besides the teams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountdownSetup {
    /// The competition's track (`_diyCompetitionStart.substitutesCount`).
    pub mode: u16,
    /// `_trainingGame`.
    pub training: bool,
    /// `_gameType`: 1 a DIY competition, 3 a season, 4 a career.
    pub game_type: u16,
    /// Whether a world opens a computer club's cards, its injuries (`WorldRules_CpuOpen`).
    pub computer: (bool, bool),
}

/// What a simulated match leaves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Normal time's goals.
    pub goals: (u16, u16),
    /// Extra time's, when played.
    pub extra_time: Option<(u16, u16)>,
    /// The shoot-out's, when played.
    pub penalties: Option<(i16, u16)>,
    /// The sides' records after.
    pub records: [Vec<u8>; 2],
    /// Each side's goals by record (`_team1GoalCounterByPlayer`, `_team2GoalCounterByPlayer`).
    pub scorers: [[u8; CAREER_LINE_UP]; 2],
}

impl Oracle {
    /// `InitializeTacticsPositions` over the tactics `records`: each of the 18 formations'
    /// orders.
    #[must_use]
    pub fn defensive_orders(&mut self, records: &[[u8; TACTICS_LEN]]) -> Vec<[u8; ORDER_LEN]> {
        ffi::sim_result::defensive_orders(records)
    }

    /// A simulated match of `setup`, its extra time and its shoot-out after as it asks, or the
    /// reason code upstream halts with.
    ///
    /// # Errors
    ///
    /// Upstream's halt (0x6006: a side not in its league).
    pub fn simulate(&mut self, setup: &ResultSetup) -> Result<Outcome, u16> {
        ffi::sim_result::simulate(setup)
    }

    /// `InitInGameTeams_WithMatchParam` of `sides` (the home side first) under `setup`: the
    /// sides' records after.
    #[must_use]
    pub fn countdown(&mut self, sides: &[Side; 2], setup: &CountdownSetup) -> [Vec<u8>; 2] {
        ffi::sim_result::countdown(sides, setup)
    }
}
