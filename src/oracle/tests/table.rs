//! `openswos_core::competition::table` against upstream's src/diy_competition.c
//! (`DIY_SortLeagueTableByStandings`): any table, in groups, ordered alike. Small numbers and
//! two-letter names make lines tie on every criterion, down to the name's bytes; large goal
//! counts make the goal difference wrap. Without upstream's checkout the oracle is not built
//! and these tests are not either (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

use openswos_core::competition::table::{Line, sort_groups};
use openswos_oracle::Oracle;
use openswos_oracle::table::{STANDINGS_ROWS, StandingsRow};
use proptest::collection::vec;
use proptest::prelude::*;

/// The most groups a table is cut into here.
const MAX_GROUPS: u16 = 4;

/// The most places of a group here (a round robin's: upstream's tables go to 24).
const MAX_PER_GROUP: u16 = 24;

/// The bytes of a name upstream's field holds, its end aside.
const MAX_NAME: usize = 16;

/// A count, small (many ties) or anywhere in a word (wrapping differences).
fn count() -> impl Strategy<Value = u16> {
    prop_oneof![4 => 0u16..4, 1 => any::<u16>()]
}

/// A name of a few letters out of two (many ties and prefixes), or of any bytes but 0.
fn name() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        4 => vec(prop_oneof![Just(b'A'), Just(b'B')], 0..4),
        1 => vec(1u8..=u8::MAX, 0..=MAX_NAME),
    ]
}

/// A line.
fn line() -> impl Strategy<Value = Line> {
    (count(), count(), count(), count()).prop_map(|(points, goals_for, goals_against, won)| Line {
        played: won,
        won,
        drawn: 0,
        lost: 0,
        goals_for,
        goals_against,
        points,
    })
}

/// A table: its groups, its places a group, each team's line and name.
fn table() -> impl Strategy<Value = (u16, u16, Vec<(Line, Vec<u8>)>)> {
    let rows = u16::try_from(STANDINGS_ROWS).expect("64 rows");
    (1..=MAX_GROUPS)
        .prop_flat_map(move |groups| (Just(groups), 0..=MAX_PER_GROUP.min(rows / groups)))
        .prop_flat_map(|(groups, per_group)| {
            let teams = usize::from(groups * per_group);
            (Just(groups), Just(per_group), vec((line(), name()), teams))
        })
}

proptest! {
    #[test]
    fn a_table_is_ordered_as_upstream_orders_it((groups, per_group, teams) in table()) {
        let lines: Vec<Line> = teams.iter().map(|(line, _)| *line).collect();
        let names: Vec<Vec<u8>> = teams.iter().map(|(_, name)| name.clone()).collect();
        let rows: Vec<StandingsRow> = (0..)
            .zip(&lines)
            .map(|(team, line)| StandingsRow {
                team,
                played: line.played,
                won: line.won,
                drawn: line.drawn,
                lost: line.lost,
                goals_for: line.goals_for,
                goals_against: line.goals_against,
                points: line.points,
            })
            .collect();
        let upstream = Oracle::lock().sort_league_table(&rows, &names, groups, per_group);
        let mut places: Vec<usize> = (0..upstream.len()).collect();
        let named: Vec<&[u8]> = names.iter().map(Vec::as_slice).collect();
        sort_groups(&mut places, usize::from(per_group), &lines, &named);
        prop_assert_eq!(places, upstream);
    }
}
