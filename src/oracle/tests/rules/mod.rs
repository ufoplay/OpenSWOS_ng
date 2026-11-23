//! The core's rules library's inputs of the oracle's setups (the rules library's checks:
//! `rules_library.rs`, `knockouts.rs`).
#![allow(dead_code, reason = "each check uses a part of it")]

use openswos_core::competition::rules_library::{
    FixedTiers, League, PromedioSeed, Row, SecondTeam, SecondTeams, Tables,
};
use openswos_oracle::rules_library::{CLUBS, DIVISIONS, LeagueSetup, RulesTables};

/// A second team's or a highest level's "none".
pub(crate) const NONE: u8 = 0xFF;

/// The second teams' flags: play-offs allowed, the lowest division left.
const PLAY_OFFS: u8 = 1;
/// See [`PLAY_OFFS`].
const LEAVE_LOWEST: u8 = 2;

/// The core's tables of `tables`.
pub(crate) fn core_tables(tables: &RulesTables) -> Tables {
    let mut rows = [[Row::default(); CLUBS]; DIVISIONS];
    for (division, table) in tables.rows.iter().enumerate() {
        for (place, &[team, glob, won, goals_for, goals_against, points]) in
            table.iter().enumerate()
        {
            rows[division][place] = Row {
                team,
                glob,
                won,
                goals_for,
                goals_against,
                points,
            };
        }
    }
    Tables::new(tables.divisions, tables.counts, rows).expect("tables within the image")
}

/// The core's league of `setup`.
pub(crate) fn core_league(setup: &LeagueSetup) -> League {
    League {
        divisions: setup.divisions,
        promoted: setup.promoted.clone(),
        relegated: setup.relegated.clone(),
        second_teams: SecondTeams {
            teams: setup
                .seconds
                .iter()
                .map(|&[team, first]| SecondTeam {
                    team,
                    first: (first != NONE).then_some(first),
                })
                .collect(),
            top: (setup.second_top != NONE).then_some(setup.second_top),
            play_offs: setup.second_flags & PLAY_OFFS != 0,
            leave_lowest: setup.second_flags & LEAVE_LOWEST != 0,
            words: setup.second_words.clone(),
        },
        fixed_tiers: (setup.italy != 0).then_some(FixedTiers {
            upper: setup.serie_b,
            lower_first: setup.group_first,
            lower_groups: setup.groups,
        }),
        spec: setup.spec.clone(),
        promedio_seeds: setup
            .promedio_seeds
            .iter()
            .map(
                |&([glob, points_two, points_one], [matches_two, matches_one])| PromedioSeed {
                    glob,
                    points_two,
                    points_one,
                    matches_two,
                    matches_one,
                },
            )
            .collect(),
        team_file_high: setup.team_file_high != 0,
    }
}
