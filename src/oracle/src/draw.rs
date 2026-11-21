//! The teams' values and the draws (upstream src/diy_competition.c): `GetAveragePlayerPrice`,
//! `DIY_ComputeAverageTeamStrengthTier`, `DIY_ShuffleTeamOrderList`,
//! `DIY_ShuffleTeamGridPairs`, `DIY_ShuffleLeagueTeamOrder`, `DIY_ShuffleTeams`,
//! `DIY_SortAndShuffleTeamsByValue`. The shuffles draw from the second generator: set it with
//! [`Oracle::set_rand2_state`] first, read it with [`Oracle::rand2_state`] after.

use crate::Oracle;
use crate::ffi;

pub use crate::ffi::draw::{GRID_PAIRS, TOURNAMENT_TEAMS};

/// A team's sixteen players' prices, in its file's order.
pub type Prices = [u8; 16];

/// How `DIY_SortAndShuffleTeamsByValue` cuts the teams into pots: its slot's format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedPots {
    /// `isLeagueFormat` set: `teamsPerGroup` pots of `groups` teams.
    Groups {
        /// `groups`.
        groups: u16,
        /// `teamsPerGroup`.
        per_group: u16,
    },
    /// `isLeagueFormat` clear: four pots of a quarter each.
    Quarters,
}

impl Oracle {
    /// `GetAveragePlayerPrice` of a team with `prices`.
    #[must_use]
    pub fn average_price(&mut self, prices: &Prices) -> u16 {
        ffi::draw::average_price(prices)
    }

    /// `DIY_ComputeAverageTeamStrengthTier` of `teams` (at least one, at most 64): the
    /// average and the tier.
    #[must_use]
    pub fn strength(&mut self, teams: &[Prices]) -> (u16, u16) {
        ffi::draw::strength(teams)
    }

    /// `DIY_ShuffleTeamOrderList` of `count` places (at most 698).
    #[must_use]
    pub fn shuffled_order(&mut self, count: u16) -> Vec<u8> {
        ffi::draw::shuffled_order(count)
    }

    /// `DIY_ShuffleTeamGridPairs` of `places` (whole pairs, at most [`GRID_PAIRS`]).
    #[must_use]
    pub fn shuffled_grid(&mut self, places: &[u8]) -> Vec<u8> {
        ffi::draw::shuffled_grid(places)
    }

    /// `DIY_ShuffleLeagueTeamOrder` of a league's places (each its shuffle index and row
    /// offset, at most [`TOURNAMENT_TEAMS`]).
    #[must_use]
    pub fn shuffled_league(&mut self, places: &[(u8, u16)]) -> Vec<(u8, u16)> {
        ffi::draw::shuffled_league(places)
    }

    /// `DIY_ShuffleTeams` of a tournament whose teams are in `order` (at most
    /// [`TOURNAMENT_TEAMS`]).
    #[must_use]
    pub fn shuffled_teams(&mut self, order: &[u8]) -> Vec<u8> {
        ffi::draw::shuffled_teams(order)
    }

    /// `DIY_SortAndShuffleTeamsByValue` of a tournament whose teams are in `order`, team `i`
    /// with `teams[i]`'s prices, cut into `pots`. The pots must hold no more than the teams.
    #[must_use]
    pub fn seeded(&mut self, order: &[u8], teams: &[Prices], pots: SeedPots) -> Vec<u8> {
        ffi::draw::seeded(order, teams, pots)
    }
}
