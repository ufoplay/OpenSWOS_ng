//! A league's rules as the rules library reads them (upstream src/rules/swosrules.h
//! `sr_league`): its divisions and their counts by place, its second teams, its fixed tiers'
//! brackets, its knockouts' spec.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::season::PromedioSeed;

/// A league's second team (`sr_league.second`): its global number and its first team's, each
/// less the league's base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SecondTeam {
    /// Its global number less the base.
    pub team: u8,
    /// Its first team's, none outside the league.
    pub first: Option<u8>,
}

/// A league's rules for its second teams (`sr_league`'s `second`, `second_top`,
/// `second_flags`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SecondTeams {
    /// The second teams.
    pub teams: Vec<SecondTeam>,
    /// The highest level one may reach, none for any.
    pub top: Option<u8>,
    /// They may go up through the play-offs (flag 1).
    pub play_offs: bool,
    /// They may leave the league from its lowest division (flag 2).
    pub leave_lowest: bool,
    /// Each second team's team word (`second_team`), 0 none: a row is found by it as well as
    /// by its global number; none for none.
    pub words: Option<Vec<u16>>,
}

/// A league's fixed tiers' brackets (`sr_league`'s `italy`, `serie_b`, `group_first`,
/// `groups`: the Mod's dplay.c): an upper tier's play-off and play-out, and a lower tier of
/// groups whose play-off sends one club up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixedTiers {
    /// The upper tier's division.
    pub upper: u8,
    /// The lower tier's first group's division.
    pub lower_first: u8,
    /// The lower tier's groups (2 to 4).
    pub lower_groups: u8,
}

/// A league's rules (`sr_league`): its divisions, each division's promoted and relegated by
/// place (none, read as upstream's null counts: no move by place), its second teams, its fixed
/// tiers' brackets, its knockouts' spec (the bytes the Mod's `compile_spec` makes, its length
/// the vector's), its promedio's history at a new career, how the game lays out a team word.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct League {
    /// Its divisions.
    pub divisions: u8,
    /// Each division's clubs promoted by place (the first division's none).
    pub promoted: Vec<u8>,
    /// Each division's clubs relegated by place.
    pub relegated: Vec<u8>,
    /// Its second teams.
    pub second_teams: SecondTeams,
    /// Its fixed tiers' brackets, none without.
    pub fixed_tiers: Option<FixedTiers>,
    /// Its knockouts' spec, none without.
    pub spec: Option<Vec<u8>>,
    /// Its promedio's history at a new career (`promedio_seed`).
    pub promedio_seeds: Vec<PromedioSeed>,
    /// A team word's layout (`team_file_high`): the file in the high byte (OpenSWOS's), else
    /// in the low (the Mod's).
    pub team_file_high: bool,
}
