//! The league table (upstream src/diy_competition.c): `DIY_SortLeagueTableByStandings`.

use crate::Oracle;
use crate::ffi;

/// The rows upstream's table holds (`leagueStandings[64]`).
pub const STANDINGS_ROWS: usize = 64;

/// A team's standings row (upstream `DIYTeamStats`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StandingsRow {
    /// The team (`teamSelOffset`): its index among the selected teams, or -1 the career club.
    pub team: i32,
    /// `gamesPlayed`.
    pub played: u16,
    /// `wins`.
    pub won: u16,
    /// `draws`.
    pub drawn: u16,
    /// `losses`.
    pub lost: u16,
    /// `goalsFor`.
    pub goals_for: u16,
    /// `goalsAgainst`.
    pub goals_against: u16,
    /// `points`.
    pub points: u16,
}

impl Oracle {
    /// `DIY_SortLeagueTableByStandings` over `rows`, in `groups` groups of `per_group` places
    /// each, the selected teams named `names` (by index; a name past 16 bytes is cut, as the
    /// record's field holds 16 and its end): each place's row, group after group.
    ///
    /// # Panics
    ///
    /// When `rows` holds more than upstream's 64, or fewer than the places.
    #[must_use]
    pub fn sort_league_table(
        &mut self,
        rows: &[StandingsRow],
        names: &[Vec<u8>],
        groups: u16,
        per_group: u16,
    ) -> Vec<usize> {
        ffi::table::sort(rows, names, groups, per_group)
    }
}
