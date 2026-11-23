//! A match's results given, as the core's source (the competitions' checks: `cup.rs`,
//! `play.rs`, `stages.rs`, `sim_play.rs`): each part, and which the competition asked, as the
//! oracle's cache gives them upstream.
#![allow(dead_code, reason = "each check uses a part of it")]

use openswos_core::competition::results::{MatchResults, MatchSetup, Sides, Venue};
use openswos_core::competition::running::RunningCompetition;
use openswos_core::competition::table::Score;
use openswos_core::sim::SimError;
use openswos_oracle::cup_play::{CachedResults, Taken};
use openswos_rng::Rng;

/// The simulation's mode flag for a neutral ground (upstream's 0xFF).
pub(crate) const NEUTRAL: u16 = 0xFF;

/// A match's results as the core's source: each part, and which were asked.
pub(crate) struct Given {
    /// The results.
    pub(crate) results: CachedResults,
    /// The parts asked.
    pub(crate) taken: Taken,
}

impl Given {
    /// The source of `results`, nothing asked yet.
    pub(crate) fn new(results: CachedResults) -> Self {
        Self {
            results,
            taken: Taken::default(),
        }
    }

    /// A league match's source: its normal time `goals`.
    pub(crate) fn goals(goals: (u16, u16)) -> Self {
        Self::new(CachedResults {
            normal: goals,
            extra: (0, 0),
            penalties: (0, 0),
        })
    }
}

impl MatchResults for Given {
    fn normal_time(
        &mut self,
        _competition: &RunningCompetition,
        _sides: Sides,
        setup: &MatchSetup,
        _rng: &mut Rng,
    ) -> Result<Score, SimError> {
        self.taken.normal = true;
        self.taken.venue = Some(match setup.venue {
            Venue::Home => 0,
            Venue::Neutral => NEUTRAL,
        });
        let (home, away) = self.results.normal;
        Ok(Score { home, away })
    }

    fn extra_time(&mut self, _sides: Sides, _rng: &mut Rng) -> Result<Score, SimError> {
        self.taken.extra = true;
        let (home, away) = self.results.extra;
        Ok(Score { home, away })
    }

    fn penalties(&mut self, _sides: Sides, _rng: &mut Rng) -> Result<Score, SimError> {
        self.taken.penalties = true;
        let (home, away) = self.results.penalties;
        Ok(Score { home, away })
    }
}
