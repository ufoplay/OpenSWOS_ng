//! `openswos_core::competition::rules_library` against upstream's rules library (src/rules/, MIT:
//! Davide Lorigliola's, src/rules/LICENSE-MIT; M5 #9a1), each function from random inputs:
//! the season's moves (`sr_final_moves`, `sr_b_teams`) over any tables (the counts past the
//! divisions in use read too), play-offs' moves, counts by place, second teams (their first
//! teams in the league or not, a highest level, the flags) and levels, the state's wants before;
//! a cup's clubs (`sr_cup_entrants`) by any quotas, slots, divisions, places, levels and career
//! club; parallel groups (`sr_pg_season`) over chains of single divisions and groups, clubs of
//! any areas moving a level or staying, some entering from below, past the limits now and then;
//! the grouped cup's season end and later rounds (`sr_cupd_*`) from any season end or none. The
//! fixed tiers' brackets (`sr_italy`) and held rules, the checks of a spec and of a league
//! (M5 #9a2); the season's check (`sr_season_check`) of any counts near their limits (M5 #9a3;
//! the knockouts are `knockouts.rs`'). Inputs the port refuses because upstream reads past its
//! memory are not run. Without
//! upstream's checkout the oracle is not built and these tests are not either
//! (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

mod rules;

use openswos_core::competition::rules_library::{
    CheckCode, CheckError, CupClubs, FixedTiersSeason, GroupClub, GroupLevel, GroupMoves,
    GroupedCup, GroupedCupEnd, KnockoutSeason, Levels, Move, MoveRule, PlayedTie, Promedio, Quota,
    RulesError, StageBank, Wants, Winners, cup_entrants, final_moves, fixed_tier_brackets,
    grouped_cup_rounds, league_check, parallel_groups, play_off_held, play_out_held, season_check,
    second_team_moves, spec_check,
};
use openswos_oracle::Oracle;
use openswos_oracle::rules_library::{
    CLUB_SLOTS, CLUBS, CUP_SIDES, CheckOutcome, CupEnd, CupEndStep, CupEntrantsSetup,
    CupRoundsSetup, DIVISIONS, FixedTiersSetup, GroupsSetup, LeagueSetup, MovesSetup, RulesTables,
    SeasonCheckSetup, UNWRITTEN,
};
use proptest::collection::vec;
use proptest::prelude::*;
use rules::{NONE, core_league, core_tables};

/// The levels a league's chain has here at most.
const MAX_LEVELS: usize = 8;

/// A table's clubs: up to the table's 32, few most of the time.
fn count() -> impl Strategy<Value = u8> {
    prop_oneof![3 => 0..=8u8, 1 => 0..=32u8]
}

/// Tables: their divisions in use, every division's count, its rows numbered from the base by
/// division and place (a few numbers repeated now and then).
fn tables() -> impl Strategy<Value = RulesTables> {
    (
        0..=16u8,
        vec(count(), DIVISIONS),
        vec(any::<[u16; 4]>(), DIVISIONS * CLUBS),
        vec(
            prop_oneof![9 => Just(None), 1 => (0u16..64).prop_map(Some)],
            DIVISIONS * CLUBS,
        ),
    )
        .prop_map(|(divisions, counts, records, repeats)| {
            let mut rows = Box::new([[[0u16; 6]; CLUBS]; DIVISIONS]);
            for (index, (record, repeat)) in records.iter().zip(&repeats).enumerate() {
                let glob = repeat.unwrap_or(u16::try_from(index).expect("512"));
                let [team, won, gf, ga] = *record;
                rows[index / CLUBS][index % CLUBS] = [team, glob, won, gf, ga, gf % 7];
            }
            RulesTables {
                divisions,
                counts: counts.try_into().expect("16"),
                rows,
            }
        })
}

/// A league's rules and the rest the season's moves read (the tables' globals numbered from 0,
/// a base of 0 mostly so a second team's number finds its club).
fn moves_setup() -> impl Strategy<Value = MovesSetup> {
    (
        tables(),
        vec(
            (
                0u16..300,
                prop_oneof![5 => -1i8..=1, 1 => any::<i8>()],
                1..=2u8,
            ),
            0..=48,
        ),
        0..=16u8,
        (vec(0..=4u8, 16..=17), vec(0..=4u8, 16..=17)),
        vec(
            (
                any::<usize>(),
                prop_oneof![1 => Just(None), 3 => any::<usize>().prop_map(Some)],
                any::<bool>(),
            ),
            0..=4,
        ),
        (prop_oneof![1 => Just(NONE), 2 => 0..=4u8], 0..=3u8),
        proptest::option::of(vec(0..=3u8, 1..=MAX_LEVELS)),
        prop_oneof![4 => Just(0u16), 1 => any::<u16>()],
        vec(
            prop_oneof![6 => -1i8..=1, 1 => any::<i8>()],
            DIVISIONS * CLUBS,
        ),
    )
        .prop_map(
            |(
                tables,
                moves,
                divisions,
                (promoted, relegated),
                seconds,
                (top, flags),
                steps,
                base,
                wants,
            )| {
                let levels = steps.map(|steps| {
                    steps
                        .iter()
                        .scan(0u8, |first, &step| {
                            let level = *first;
                            *first = first.saturating_add(step.max(1));
                            Some(level)
                        })
                        .collect()
                });
                let mut want = [[0i8; CLUBS]; DIVISIONS];
                for (index, &value) in wants.iter().enumerate() {
                    want[index / CLUBS][index % CLUBS] = value;
                }
                // A second team and its first team: clubs of the tables in use mostly (their
                // numbers less the base, when they fit a byte), else any byte.
                let in_use: Vec<u8> = (0..usize::from(tables.divisions))
                    .flat_map(|division| {
                        let rows = &tables.rows[division];
                        (0..usize::from(tables.counts[division])).map(move |place| rows[place][1])
                    })
                    .filter_map(|glob| u8::try_from(glob.wrapping_sub(base)).ok())
                    .collect();
                let pick = |index: usize, any: bool| {
                    if in_use.is_empty() || any {
                        u8::try_from(index % 256).expect("a byte")
                    } else {
                        in_use[index % in_use.len()]
                    }
                };
                let seconds = seconds
                    .into_iter()
                    .map(|(team, first, any)| {
                        [
                            pick(team, any),
                            first.map_or(NONE, |first| pick(first, false)),
                        ]
                    })
                    .collect();
                MovesSetup {
                    tables,
                    moves,
                    league: LeagueSetup {
                        divisions,
                        promoted,
                        relegated,
                        seconds,
                        second_top: top,
                        second_flags: flags,
                        ..LeagueSetup::default()
                    },
                    levels,
                    base,
                    wants: want,
                }
            },
        )
}

/// The core's moves of `setup`.
fn core_moves(setup: &MovesSetup) -> Vec<Move> {
    setup
        .moves
        .iter()
        .map(|&(glob, delta, rule)| Move {
            glob,
            delta,
            rule: MoveRule::try_from(rule).expect("a rule"),
        })
        .collect()
}

/// A level's quotas: division, clubs, byes.
fn quotas() -> impl Strategy<Value = Vec<[u8; 3]>> {
    vec(
        (prop_oneof![9 => 0..=4u8, 1 => 5..=20u8], 0..=12u8, 0..=14u8)
            .prop_map(|(division, clubs, byes)| [division, clubs, byes]),
        0..=4,
    )
}

/// A place byte: none, or a place and a level.
fn place() -> impl Strategy<Value = u8> {
    prop_oneof![1 => Just(0u8), 3 => (1..=20u8, 0..=4u8).prop_map(|(place, level)| place | level << 5), 1 => any::<u8>()]
}

/// A spec's member: an entrant, a tie's winner, a group's placing, the best loser, any byte.
fn member() -> impl Strategy<Value = u8> {
    prop_oneof![6 => 0..=7u8, 2 => 0x80..=0x83u8, 1 => 0x40..=0x43u8, 1 => Just(0x3Fu8), 1 => any::<u8>()]
}

/// A spec's round: its header (ties, legs or a group's meetings, host, level, pairing) and its
/// member bytes, twice its ties.
fn spec_round() -> impl Strategy<Value = Vec<u8>> {
    (
        prop_oneof![8 => 1..=3u8, 1 => 0..=70u8, 1 => 31..=33u8, 1 => 63..=65u8],
        prop_oneof![6 => 1..=3u8, 2 => (0..=9u8).prop_map(|meetings| 0x80 | meetings), 1 => prop::sample::select(vec![0u8, 4, 0x88, 0x89]), 1 => any::<u8>()],
        prop_oneof![9 => 0..=3u8, 1 => any::<u8>()],
        prop_oneof![9 => 0..=5u8, 1 => any::<u8>()],
        0..=7u8,
        vec(member(), 140),
    )
        .prop_map(|(ties, legs, host, level, pairing, members)| {
            let mut bytes = vec![ties, legs, host, level, pairing, 0];
            bytes.extend(&members[..2 * usize::from(ties)]);
            bytes
        })
}

/// A spec's entrant: its type, division, place, stage table.
fn spec_entrant() -> impl Strategy<Value = [u8; 4]> {
    (
        prop_oneof![5 => Just(0u8), 3 => 1..=8u8, 1 => any::<u8>()],
        prop_oneof![9 => 0..=4u8, 1 => any::<u8>()],
        prop_oneof![9 => 1..=10u8, 1 => Just(0u8)],
        prop_oneof![3 => 0..=1u8, 3 => 4..=7u8, 1 => 2..=3u8, 1 => 8..=9u8, 1 => any::<u8>()],
        // A stage table entrant at the tables' edges now and then.
        prop_oneof![3 => Just(None), 1 => prop::sample::select(vec![1u8, 2, 3, 4, 7, 8]).prop_map(Some)],
    )
        .prop_map(|(kind, division, place, table, edge)| match edge {
            Some(table) => [3, division % 3, place, table],
            None => [kind, division, place, table],
        })
}

/// A spec's play-off: a promedio's header, or a header, entrants and rounds.
fn spec_play_off() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        1 => (0..=4u8, 0..=6u8).prop_map(|(matches, division)| vec![7, 0, 0, matches * 10, division, 0]),
        8 => (
            prop_oneof![9 => 1..=8u8, 1 => any::<u8>()],
            vec(spec_entrant(), 0..=6),
            vec(spec_round(), 1..=3),
            // A header's counts at their limits now and then (its bytes fewer).
            prop_oneof![8 => Just(None), 1 => (63..=65u8).prop_map(Some)],
            prop_oneof![8 => Just(None), 1 => prop::sample::select(vec![0u8, 16, 17]).prop_map(Some)],
        )
            .prop_map(|(kind, entrants, rounds, stated_entrants, stated_rounds)| {
                let kind = if kind == 7 { 1 } else { kind };
                let count = |len: usize| u8::try_from(len).expect("a few");
                let mut bytes = vec![
                    kind,
                    stated_entrants.unwrap_or(count(entrants.len())),
                    stated_rounds.unwrap_or(count(rounds.len())),
                    0,
                    0,
                    0,
                ];
                bytes.extend(entrants.iter().flatten());
                bytes.extend(rounds.iter().flatten());
                bytes
            }),
    ]
}

/// A spec: some play-offs and the 0 that ends them, a byte overwritten or the bytes cut now
/// and then.
fn spec() -> impl Strategy<Value = Vec<u8>> {
    (
        vec(spec_play_off(), 0..=3),
        proptest::option::of((any::<prop::sample::Index>(), any::<u8>())),
        proptest::option::of(any::<prop::sample::Index>()),
    )
        .prop_map(|(play_offs, overwrite, cut)| {
            let mut bytes: Vec<u8> = play_offs.into_iter().flatten().collect();
            bytes.push(0);
            if let Some((at, value)) = overwrite {
                let at = at.index(bytes.len());
                bytes[at] = value;
            }
            if let Some(at) = cut {
                bytes.truncate(at.index(bytes.len() + 1));
            }
            bytes
        })
}

/// A check's outcome as the C reports it: the code, the play-off (-1 none), the byte, the
/// field.
fn reported(checked: Result<(), CheckError>) -> (i32, i32, u32, Option<&'static str>) {
    match checked {
        Ok(()) => (0, -1, 0, None),
        Err(error) => (
            match error.code {
                CheckCode::Truncated => 1,
                CheckCode::Count => 2,
                CheckCode::Index => 3,
                CheckCode::Value => 4,
            },
            error
                .play_off
                .map_or(-1, |play_off| i32::try_from(play_off).expect("a few")),
            error.offset,
            Some(error.field),
        ),
    }
}

/// The C's outcome in the same terms.
fn theirs_reported(outcome: &CheckOutcome) -> (i32, i32, u32, Option<&str>) {
    (
        outcome.code,
        outcome.play_off,
        outcome.offset,
        outcome.field.as_deref(),
    )
}

/// A league's rules as the checks read them: its divisions, its fixed tiers (on mostly, sound
/// mostly), a spec now and then.
fn league_setup() -> impl Strategy<Value = LeagueSetup> {
    (
        prop_oneof![9 => 0..=16u8, 1 => 17..=20u8],
        prop_oneof![1 => Just(0u8), 3 => Just(1u8), 1 => any::<u8>()],
        (0..=4u8, 0..=6u8, prop_oneof![4 => Just(3u8), 1 => 0..=6u8]),
        proptest::option::of(prop_oneof![4 => spec(), 1 => Just(Vec::new())]),
    )
        .prop_map(
            |(divisions, italy, (serie_b, group_first, groups), spec)| LeagueSetup {
                divisions,
                promoted: vec![0; 20],
                relegated: vec![0; 20],
                italy,
                serie_b,
                group_first,
                groups,
                spec,
                ..LeagueSetup::default()
            },
        )
}

/// The fixed tiers' league and tables: the upper tier and the groups below it, their clubs (17
/// or more mostly), points falling by small steps from a top (the held rules' edges), team
/// words and global numbers by division and place, the career club's results now and then.
fn fixed_tiers_setup() -> impl Strategy<Value = FixedTiersSetup> {
    (
        (
            0..=2u8,
            0..=1u8,
            prop_oneof![1 => Just(2u8), 4 => Just(3u8), 1 => Just(4u8), 1 => 0..=6u8],
            prop_oneof![6 => 0..=2u8, 1 => Just(0u8)],
        ),
        vec(prop_oneof![4 => 17..=24u8, 1 => 0..=24u8], DIVISIONS),
        vec(
            (
                40..=90u16,
                vec(prop_oneof![8 => 0..=6u16, 1 => 7..=20u16], CLUBS),
                vec(any::<(u8, u8)>(), CLUBS),
            ),
            DIVISIONS,
        ),
        // The career club's results: the upper tier's play-off (3rd to 8th) and play-out (16th,
        // 17th), the lower tier's play-off and its cup's winner (a group's club, the 11th and the
        // play-out places often).
        (
            prop_oneof![1 => Just(None), 1 => (2..=7usize).prop_map(Some)],
            prop_oneof![1 => Just(None), 1 => (15..=16usize).prop_map(Some)],
            prop_oneof![2 => Just(None), 1 => (0..4usize, 0..24usize).prop_map(Some)],
            prop_oneof![2 => Just(None), 1 => (0..4usize, 0..24usize).prop_map(Some), 2 => (0..4usize, prop::sample::select(vec![10usize, 100, 101, 102, 103])).prop_map(Some)],
        ),
        prop_oneof![5 => Just(0u8), 1 => Just(1u8)],
        prop_oneof![9 => Just(1u8), 1 => Just(0u8)],
        // The groups alike now and then: one profile of points, no wins or goals (the lots
        // decide); their play-out places 7 to 9 points apart now and then.
        (
            prop_oneof![3 => Just(false), 1 => Just(true)],
            prop_oneof![2 => Just(None), 1 => (7..=9u16).prop_map(Some)],
        ),
    )
        .prop_map(
            |((upper, gap, groups, extra), counts, points, winners, test_held, italy, (alike, edge))| {
                let group_first = upper + 1 + gap;
                let divisions = (group_first + groups + extra).min(16);
                let counts: [u8; DIVISIONS] = counts.try_into().expect("16");
                let mut rows = Box::new([[[0u16; 6]; CLUBS]; DIVISIONS]);
                for (division, profile) in points.iter().enumerate() {
                    let in_groups = division >= usize::from(group_first);
                    let (top, steps, records) = if alike && in_groups {
                        &points[usize::from(group_first)]
                    } else {
                        profile
                    };
                    let mut points = *top;
                    for place in 0..CLUBS {
                        let (won, goals) = if alike && in_groups { (0, 0) } else { records[place] };
                        let division_word = u16::try_from(division + 1).expect("16");
                        let place_word = u16::try_from(place).expect("32");
                        rows[division][place] = [
                            division_word << 8 | place_word,
                            division_word * 32 + place_word,
                            u16::from(won % 30),
                            u16::from(goals % 90),
                            0,
                            points,
                        ];
                        points = points.saturating_sub(steps[place]);
                    }
                }
                if let Some(edge) = edge {
                    for division in usize::from(group_first)..DIVISIONS {
                        let clubs = usize::from(counts[division]);
                        if clubs >= 6 {
                            let top = rows[division][clubs - 5][5];
                            rows[division][clubs - 4][5] = top;
                            rows[division][clubs - 3][5] = top.saturating_sub(edge);
                            rows[division][clubs - 2][5] = top.saturating_sub(edge);
                        }
                    }
                }
                let (upper_play_off, upper_play_out, lower_play_off, cup_winner) = winners;
                let upper_word = |place: Option<usize>| {
                    place.map_or(0, |place| rows[usize::from(upper)][place][0])
                };
                // A group's club: a place, or (past 99) the 11th and the play-out places, n - 5
                // on, of its count.
                let group_word = |pick: Option<(usize, usize)>| {
                    pick.map_or(0, |(group, place)| {
                        let division = (usize::from(group_first) + group).min(DIVISIONS - 1);
                        let clubs = usize::from(counts[division]);
                        let place = match place {
                            100.. => (clubs + place).saturating_sub(105).min(CLUBS - 1),
                            _ => place,
                        };
                        rows[division][place][0]
                    })
                };
                let winners = [
                    upper_word(upper_play_off),
                    upper_word(upper_play_out),
                    group_word(lower_play_off),
                    group_word(cup_winner),
                ];
                FixedTiersSetup {
                    tables: RulesTables {
                        divisions,
                        counts,
                        rows,
                    },
                    league: LeagueSetup {
                        divisions,
                        promoted: vec![0; 16],
                        relegated: vec![0; 16],
                        italy,
                        serie_b: upper,
                        group_first,
                        groups,
                        ..LeagueSetup::default()
                    },
                    winners,
                    test_held,
                }
            },
        )
}

proptest! {
    #[test]
    fn the_season_s_moves_are_upstream_s(setup in moves_setup()) {
        let tables = core_tables(&setup.tables);
        let league = core_league(&setup.league);
        let levels = Levels { first: setup.levels.clone().unwrap_or_default() };
        let mut wants = Wants(setup.wants);
        let ours = final_moves(&tables, &core_moves(&setup), &league, &levels, setup.base, &mut wants);
        prop_assert!(ours.is_ok(), "{:?}", ours);
        let mut oracle = Oracle::lock();
        prop_assert_eq!(wants.0, oracle.final_moves(&setup), "the moves");
        let mut alone = Wants(setup.wants);
        second_team_moves(&tables, &mut alone, &league, &levels, setup.base);
        prop_assert_eq!(alone.0, oracle.second_team_moves(&setup), "the second teams' rules alone");
    }

    #[test]
    fn a_cup_s_clubs_are_upstream_s(
        quotas in quotas(),
        entrants in prop_oneof![2 => Just(None), 1 => (0..=40i32).prop_map(Some)],
        count in prop_oneof![20 => 0..=80usize, 1 => 500..=520usize],
        shuffled in any::<bool>(),
        seed in any::<u64>(),
        divisions in vec(prop_oneof![6 => 0..=4u8, 1 => 0..=16u8], 520),
        places in proptest::option::of(vec(place(), CLUB_SLOTS)),
        levels in vec(0..=4u8, 21),
        career in prop_oneof![1 => Just(-1i32), 1 => 0..80i32],
    ) {
        // The slots in order, or scattered over a country's.
        let slots: Vec<u16> = (0..count)
            .map(|index| {
                let index = u64::try_from(index).expect("520");
                let slot = if shuffled { (index.wrapping_mul(2_654_435_761) ^ seed) % 512 } else { index % 512 };
                u16::try_from(slot).expect("512")
            })
            .collect();
        let entrants = entrants.unwrap_or_else(|| {
            quotas.iter().map(|&[_, clubs, _]| i32::from(clubs)).sum()
        });
        let quotas_core: Vec<Quota> = quotas
            .iter()
            .map(|&[division, clubs, byes]| Quota { division, clubs, byes })
            .collect();
        let clubs = CupClubs {
            slots: &slots,
            divisions: &divisions[..count],
            places: places.as_deref(),
            levels: &levels,
        };
        let ours = cup_entrants(&quotas_core, entrants, &clubs, career);
        prop_assert!(ours.is_ok(), "{:?}", ours);
        let theirs = Oracle::lock().cup_entrants(&CupEntrantsSetup {
            quotas,
            entrants,
            slots,
            divisions: divisions[..count].to_vec(),
            places,
            levels,
            career_slot: career,
        });
        prop_assert_eq!(ours.expect("ok").unwrap_or_default(), theirs);
    }

    #[test]
    fn a_season_end_s_parallel_groups_are_upstream_s(
        shape in vec((0..=2u8, prop_oneof![2 => Just(1u8), 3 => 2..=5u8, 1 => 6..=10u8], vec(0..=5u8, 9)), 0..=9),
        clubs in vec((0..=2u8, any::<[u8; 4]>(), 0..=30i8, prop_oneof![2 => Just(0i8), 1 => Just(-1i8), 1 => Just(1i8)], prop_oneof![9 => Just(0u8), 1 => Just(1u8)]), 0..=60),
        with_entering in any::<bool>(),
    ) {
        let mut first = 0u8;
        let levels: Vec<GroupLevel> = shape
            .iter()
            .map(|&(gap, groups, ref size)| {
                first = first.saturating_add(gap);
                let level = GroupLevel { first, groups, size: size.clone().try_into().expect("9") };
                first = first.saturating_add(groups);
                level
            })
            .collect();
        let old: Vec<i8> = clubs.iter().map(|&(_, _, old, ..)| old).collect();
        let new: Vec<i8> = clubs.iter().map(|&(_, _, old, step, _)| old.saturating_add(step)).collect();
        let entering: Option<Vec<u8>> = with_entering.then(|| clubs.iter().map(|&(.., entering)| entering).collect());
        let group_clubs: Vec<GroupClub> = clubs.iter().map(|&(base, area, ..)| GroupClub { base, area }).collect();
        let mut ours_new = new.clone();
        let ours = parallel_groups(
            &levels,
            &group_clubs,
            GroupMoves { old: &old, new: &mut ours_new, entering: entering.as_deref() },
        );
        // A club staying above the chain's first group: refused, upstream counts before it.
        prop_assume!(!matches!(ours, Err(RulesError::ClubAboveChain(_))), "{:?}", ours);
        let (code, theirs) = Oracle::lock().parallel_groups(&GroupsSetup {
            levels: levels.iter().map(|level| (level.first, level.groups, level.size)).collect(),
            clubs: clubs.iter().map(|&(base, area, ..)| (base, area)).collect(),
            old,
            new: new.clone(),
            entering,
        });
        match ours {
            Ok(()) => {
                prop_assert_eq!(code, 0);
                prop_assert_eq!(ours_new, theirs);
            }
            Err(error) => {
                prop_assert_eq!(error, RulesError::GroupsPastLimits);
                prop_assert_eq!(code, -1);
                prop_assert_eq!(&theirs, &new, "nothing changed");
                prop_assert_eq!(&ours_new, &new, "nothing changed");
            }
        }
    }

    #[test]
    fn the_grouped_cup_s_season_end_and_rounds_are_upstream_s(
        steps in vec(prop_oneof![
            3 => (0..=12u8, 0..=4u8, 0..=20u16, prop_oneof![9 => 0..=200u16, 1 => 500..=520u16], 0..=20i32, 0..=20i32)
                .prop_map(|(first, groups, division, slot, place, clubs)| CupEndStep::Keep { first, groups, division, slot, place, clubs }),
            1 => prop_oneof![9 => 0..=200u16, 1 => 500..=520u16].prop_map(CupEndStep::Arrival),
        ], 0..=200),
        with_end in any::<bool>(),
        season in any::<u32>(),
        cup in (prop_oneof![9 => 2..=8u8, 1 => 0..=10u8], 0..=12u8, 1..=5u8),
        slots in prop_oneof![9 => 0..=200usize, 1 => 500..=520usize],
        divisions in vec(prop_oneof![3 => 0..=16u8, 1 => Just(0xFFu8)], 520),
        words in vec(any::<u16>(), 520),
    ) {
        let (rounds, first, groups) = cup;
        let shape = GroupedCup { rounds, first, groups };
        // Most places kept in the cup's own level, the last of a group now and then.
        let steps: Vec<CupEndStep> = steps
            .into_iter()
            .enumerate()
            .map(|(index, step)| match step {
                CupEndStep::Keep { division, slot, place, clubs, .. } if index % 4 != 0 => CupEndStep::Keep {
                    first,
                    groups,
                    division: u16::from(first) + division % u16::from(groups),
                    slot,
                    place: if index % 3 == 0 { clubs - 1 } else { place },
                    clubs,
                },
                other @ (CupEndStep::Keep { .. } | CupEndStep::Arrival(_)) => other,
            })
            .collect();
        let mut end = GroupedCupEnd::default();
        for &step in &steps {
            match step {
                CupEndStep::Keep { first, groups, division, slot, place, clubs } => {
                    end.keep_place(GroupedCup { rounds, first, groups }, division, slot, place, clubs);
                }
                CupEndStep::Arrival(slot) => end.note_arrival(slot),
            }
        }
        let mut oracle = Oracle::lock();
        let theirs_end = oracle.cup_end(&steps);
        prop_assert_eq!(&end.last_division[..], &theirs_end.last_division[..], "the divisions");
        prop_assert_eq!(&end.place[..], &theirs_end.place[..], "the places");
        prop_assert_eq!(end.arrival, theirs_end.arrival, "the arrivals");
        let ours = grouped_cup_rounds(with_end.then_some(&end), season, shape, &divisions[..slots], &words[..slots]);
        // A preliminary round of more sides than clubs: refused, upstream reads entries it never
        // wrote.
        prop_assume!(!matches!(ours, Err(RulesError::PreliminaryPastClubs(_))), "{:?}", ours);
        let theirs = oracle.cup_rounds(&CupRoundsSetup {
            end: with_end.then(|| CupEnd {
                last_division: Box::new(end.last_division),
                place: Box::new(end.place),
                arrival: end.arrival,
            }),
            season,
            rounds,
            first,
            groups,
            divisions: divisions[..slots].to_vec(),
            words: words[..slots].to_vec(),
        });
        match ours {
            Ok(made) => {
                prop_assert_eq!(theirs.clubs, i32::try_from(made.clubs).expect("512"));
                prop_assert_eq!(theirs.preliminary, made.preliminary);
                prop_assert_eq!(&theirs.sides[..made.sides.len()], &made.sides[..], "the sides");
                prop_assert!(theirs.sides[made.sides.len()..].iter().all(|&side| side == UNWRITTEN), "nothing more written");
                prop_assert!(made.sides.len() <= CUP_SIDES);
            }
            Err(error) => {
                prop_assert!(matches!(error, RulesError::CupPastLimits { .. }), "{:?}", error);
                prop_assert_eq!(theirs.clubs, -1);
            }
        }
    }

    #[test]
    fn the_fixed_tiers_brackets_are_upstream_s(setup in fixed_tiers_setup()) {
        let tables = core_tables(&setup.tables);
        let league = core_league(&setup.league);
        let [upper_play_off, upper_play_out, lower_play_off, cup_winner] = setup.winners;
        let season = FixedTiersSeason {
            upper_play_off,
            upper_play_out,
            lower_play_off,
            cup_winner,
            test_held: setup.test_held != 0,
        };
        let ours = fixed_tier_brackets(&tables, &league, &season);
        // A lower tier of four groups reaching its national rounds: refused, upstream writes
        // past its tables.
        prop_assume!(!matches!(ours, Err(RulesError::GroupsPastBracket(_))), "{:?}", ours);
        let (code, theirs) = Oracle::lock().fixed_tiers(&setup);
        match ours {
            Ok(moves) => {
                prop_assert_eq!(code, i32::try_from(moves.len()).expect("a few"));
                let ours: Vec<(u16, i8, u8)> = moves.iter().map(|played| (played.glob, played.delta, u8::from(played.rule))).collect();
                prop_assert_eq!(ours, theirs);
            }
            Err(error) => {
                prop_assert!(matches!(error, RulesError::Check(_)), "{:?}", error);
                prop_assert_eq!(code, -1);
                prop_assert!(theirs.is_empty());
            }
        }
    }

    #[test]
    fn the_held_rules_are_upstream_s(points in any::<[u16; 2]>()) {
        let [first, second] = points;
        let (out, play_off) = Oracle::lock().held(points);
        prop_assert_eq!(i32::from(play_out_held(first, second)), out);
        prop_assert_eq!(i32::from(play_off_held(first, second)), play_off);
    }

    #[test]
    fn a_spec_s_check_is_upstream_s(spec in spec(), divisions in prop_oneof![9 => 0..=6i32, 1 => -3..=20i32]) {
        let theirs = Oracle::lock().spec_check(&spec, divisions);
        prop_assert_eq!(reported(spec_check(&spec, divisions)), theirs_reported(&theirs));
    }

    #[test]
    fn a_league_s_check_is_upstream_s(setup in league_setup()) {
        let theirs = Oracle::lock().league_check(&setup);
        let ours = reported(league_check(&core_league(&setup)));
        let theirs = theirs_reported(&theirs);
        // The fixed tiers' field is named by what it does here (the C names the Italian
        // rules' divisions).
        prop_assert_eq!((ours.0, ours.1, ours.2), (theirs.0, theirs.1, theirs.2));
        if ours.1 >= 0 || ours.0 != 3 {
            prop_assert_eq!(ours.3, theirs.3);
        }
    }

    #[test]
    fn a_season_s_check_is_upstream_s(setup in season_check_setup()) {
        let mut bank = setup.bank.map(|(counts, kept_count, kept_division)| StageBank {
            counts,
            kept_count,
            kept_division,
            ..StageBank::default()
        });
        let mut promedio = setup.promedio.map(|count| Promedio { count, ..Promedio::default() });
        let mut winners = setup.winners.map(|count| Winners { count, ..Winners::default() });
        let levels = setup.levels.map(|count| Levels { first: vec![0; usize::from(count)] });
        let season = KnockoutSeason {
            manager: 0,
            played: PlayedTie::default(),
            bank: bank.as_mut(),
            promedio: promedio.as_mut(),
            winners: winners.as_mut(),
            host: None,
            preview: false,
            own_play_offs: [0; 2],
            second_teams_in_knockouts: false,
            base: 0,
            levels: levels.as_ref(),
        };
        let ours = reported(season_check(&season));
        let theirs = Oracle::lock().season_check(&setup);
        prop_assert_eq!(ours, theirs_reported(&theirs));
    }
}

/// What the season's check reads: the stage tables' counts and the kept table's (its
/// division), the promedio's and the page's counts, the levels; each none now and then, at
/// their limits mostly.
fn season_check_setup() -> impl Strategy<Value = SeasonCheckSetup> {
    let count =
        |limit: u8| prop_oneof![3 => 0..=limit, 1 => limit + 1..=limit + 2, 1 => any::<u8>()];
    let limits = |clubs: u8| {
        (
            [count(clubs), count(clubs), count(clubs), count(clubs)],
            count(clubs),
            prop_oneof![3 => 0..=15u8, 1 => 16..=17u8, 1 => any::<u8>()],
        )
    };
    let clubs = u8::try_from(CLUBS).expect("32");
    (
        proptest::option::weighted(0.8, limits(clubs)),
        proptest::option::weighted(0.8, count(48)),
        proptest::option::weighted(0.8, count(40)),
        proptest::option::weighted(0.5, count(16)),
    )
        .prop_map(|(bank, promedio, winners, levels)| SeasonCheckSetup {
            bank,
            promedio,
            winners,
            levels,
        })
}
