//! A match's result as a competition asks for it (docs/PLAN.md M5 #8d): a league's match its
//! normal time, a cup's tie each part as it needs it (upstream diy_competition.c
//! `getCachedOrCalcViewResult`, `getCachedOrCalcExtraTimeResult`, `getCachedOrCalcPenaltyResult`),
//! from a source: the simulation ([`Simulation`](crate::sim::Simulation), M5 #8) or the match
//! engine (M6). A match's setup is the competition's: where it is played, its benches, the
//! players' goals' column.

use openswos_rng::Rng;

use super::running::RunningCompetition;
use super::table::Score;
use crate::sim::SimError;

/// Where a match is played (upstream's mode flag: 0 the home side's ground, 0xFF neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Venue {
    /// The home side's ground: its advantage counts.
    Home,
    /// Neutral: a single-leg final's, a league's played once each way round.
    Neutral,
}

/// A match's two sides, the home side first, each its identity: -1 (any below 0) the career
/// club, otherwise its index among the selected teams (upstream `diy_resolve_team`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sides {
    /// The home side's identity.
    pub home: i32,
    /// The away side's identity.
    pub away: i32,
}

/// A match's setup: where it is played, its benches, its goals' column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatchSetup {
    /// Where it is played.
    pub venue: Venue,
    /// The fewest substitutes and the most (upstream passes a league's
    /// `leagueMinSubstitutes`, `leagueMaxSubstitutes`, a cup's `minSubstitutes`,
    /// `pointsForAWin`).
    pub substitutes: (u16, u16),
    /// The players' goals' column its goals count in (upstream's word of the competition's
    /// `compSlot` and `slotId`).
    pub column: u16,
}

impl MatchSetup {
    /// The setup of a match of `competition` at `venue`, its benches `substitutes`: the goals'
    /// column the competition's.
    #[must_use]
    pub fn of(competition: &RunningCompetition, venue: Venue, substitutes: (u16, u16)) -> Self {
        Self {
            venue,
            substitutes,
            column: u16::from_le_bytes([competition.comp_slot(), competition.slot_id()]),
        }
    }
}

/// What a match's result is made of, each asked for only when the competition needs it, in
/// upstream's order: normal time, then extra time, then penalties; each drawn by `rng` as a
/// source draws.
pub trait MatchResults {
    /// The goals of normal time of the match between `sides` in `competition`, set up as
    /// `setup`.
    ///
    /// # Errors
    ///
    /// [`SimError`] when the source cannot give them.
    fn normal_time(
        &mut self,
        competition: &RunningCompetition,
        sides: Sides,
        setup: &MatchSetup,
        rng: &mut Rng,
    ) -> Result<Score, SimError>;

    /// The goals of the extra time alone of the match between `sides` (its normal time the
    /// last asked).
    ///
    /// # Errors
    ///
    /// [`SimError`] when the source cannot give them.
    fn extra_time(&mut self, sides: Sides, rng: &mut Rng) -> Result<Score, SimError>;

    /// The shoot-out's penalties of the match between `sides`.
    ///
    /// # Errors
    ///
    /// [`SimError`] when the source cannot give them.
    fn penalties(&mut self, sides: Sides, rng: &mut Rng) -> Result<Score, SimError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_goals_column_is_the_word_of_the_slots() {
        let mut competition = RunningCompetition::default();
        competition.set_comp_slot(2);
        competition.set_slot_id(1);
        let setup = MatchSetup::of(&competition, Venue::Neutral, (3, 5));
        assert_eq!(
            setup,
            MatchSetup {
                venue: Venue::Neutral,
                substitutes: (3, 5),
                column: 0x0102,
            }
        );
    }
}
