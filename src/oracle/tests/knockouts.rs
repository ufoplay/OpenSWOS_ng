//! `openswos_core::competition::rules_library::Knockouts` against upstream's knockouts
//! (`sr_knockouts`, src/rules/sr_gplay.c, MIT: Davide Lorigliola's, src/rules/LICENSE-MIT; M5
//! #9a3): one league's season ends one after another over one library state. Random leagues of
//! a few divisions, their specs every play-off kind (the promedio and the supercup too) and
//! entrant type (by place, a stage table's, the previous loser, the stage titles' finalists,
//! the title deciders' seeds, a club by team word, absent or not), group rounds (a ranking
//! round of no meetings too) and knockout rounds (as listed, drawn, best v worst, too far
//! apart, legs, a best of three), the drawn pot, the final group's winners, placings past the
//! second teams; the stage tables, the promedio history and the SEASON WINNERS page kept
//! between seasons; the career club's ties, titles and stage titles answered by the host, its
//! played tie, the preview. The port's state after each season is upstream's. A season the
//! port refuses because upstream reads past its memory, or where it reads a round not played
//! (upstream reads its locals, never written), ends the run before it.
#![cfg(not(oracle_unavailable))]

mod rules;

use openswos_core::competition::rules_library::{
    GroupFigures, KnockoutHost, KnockoutResult, KnockoutSeason, Knockouts, Levels, Move, MoveRule,
    PlayedTie, Promedio, PromedioClub, Row, RulesError, StageBank, Winners, WinnersEntry,
};
use openswos_oracle::Oracle;
use openswos_oracle::knockouts::{
    BankSetup, FiguresSetup, HostAnswers, KnockoutsAfter, KnockoutsRun, KnockoutsSeason,
    PROMEDIO_CLUBS, PromedioSetup, ResultSetup, RowSetup, STAGE_TABLES, WINNERS, WinnersSetup,
};
use openswos_oracle::rules_library::{CLUBS, DIVISIONS, LeagueSetup, RulesTables};
use openswos_test_utils::cache::{RULES_SPECS, fetch_into, rules_specs_path};
use openswos_test_utils::skip::Skip;
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::{TestCaseError, TestRunner};
use rules::{NONE, core_league, core_tables};

/// A league's divisions here at most (its second teams' global numbers within a byte).
const MAX_DIVISIONS: usize = 5;

/// A division's team file (`FIRST_FILE` + the division), a club's ordinal its number in it.
const FIRST_FILE: u16 = 10;

/// An ordinal past every division's clubs: a club by team word absent from the tables.
const ABSENT_ORDINAL: u8 = 40;

/// The play-off kinds: promotion, two the season end does not play, play-out, title,
/// continental, promedio, stage title.
const PROMOTION: u8 = 1;
/// See [`PROMOTION`].
const PLAY_OUT: u8 = 4;
/// See [`PROMOTION`].
const TITLE: u8 = 5;
/// See [`PROMOTION`].
const CONTINENTAL: u8 = 6;
/// See [`PROMOTION`].
const PROMEDIO: u8 = 7;
/// See [`PROMOTION`].
const STAGE: u8 = 8;

/// The entrant types: by place, a stage table's place, the previous loser, a stage titles'
/// finalist, a kept stage's title decider's seed, a title play-off's seed, a club by team word.
const BY_PLACE: u8 = 0;
/// See [`BY_PLACE`].
const STAGE_PLACE: u8 = 3;
/// See [`BY_PLACE`].
const PREVIOUS_LOSER: u8 = 4;
/// See [`BY_PLACE`].
const FINALIST: u8 = 5;
/// See [`BY_PLACE`].
const SPLIT_SEED: u8 = 6;
/// See [`BY_PLACE`].
const TITLE_SEED: u8 = 7;
/// See [`BY_PLACE`].
const BY_TEAM: u8 = 8;

/// A play-off's flags: the better record hosts, the supercup, the drawn pot, its placings past
/// the second teams; how far its final group's winners are shifted.
const BY_RECORD: u8 = 2;
/// See [`BY_RECORD`].
const SUPERCUP: u8 = 4;
/// See [`BY_RECORD`].
const DRAWN_POT: u8 = 8;
/// See [`BY_RECORD`].
const SKIP_SECONDS: u8 = 0x80;
/// See [`BY_RECORD`].
const WINNERS_SHIFT: u8 = 4;

/// A round's flags: a group (its legs' byte), a club drawn against itself through (its host's),
/// a draw (its pairing's).
const GROUP: u8 = 0x80;
/// See [`GROUP`].
const SKIP_SAME: u8 = 0x80;
/// See [`GROUP`].
const DRAW: u8 = 0x80;

/// The member codes: the best loser, a placing, a tie's winner, a tie's loser.
const BEST_LOSER: u8 = 0x3F;
/// See [`BEST_LOSER`].
const PLACING: u8 = 0x40;
/// See [`BEST_LOSER`].
const WINNER_OF: u8 = 0x80;
/// See [`BEST_LOSER`].
const LOSER_OF: u8 = 0xC0;

/// The losers' division of the reserve.
const RESERVE: u8 = 0xFE;

/// A move's rule of the fixed tiers (the moves before the knockouts).
const FIXED_TIERS_RULE: u8 = 1;

/// A league's clubs: each division's, the team words' layout.
#[derive(Debug, Clone)]
struct Layout {
    /// Each division's clubs.
    counts: Vec<u8>,
    /// The team word's file in its high byte (`team_file_high`).
    high: bool,
}

impl Layout {
    /// The divisions.
    fn divisions(&self) -> u8 {
        u8::try_from(self.counts.len()).expect("a few divisions")
    }

    /// The team word of `ordinal` of file `file`.
    fn word(&self, file: u16, ordinal: u16) -> u16 {
        if self.high {
            file << 8 | ordinal
        } else {
            ordinal << 8 | file
        }
    }

    /// Club `club` of division `division`'s team word.
    fn team(&self, division: usize, club: usize) -> u16 {
        let division = u16::try_from(division).expect("16");
        self.word(FIRST_FILE + division, u16::try_from(club).expect("32"))
    }

    /// Club `club` of division `division`'s global number.
    fn glob(division: usize, club: usize) -> u16 {
        u16::try_from(division * CLUBS + club).expect("512")
    }

    /// Every club: division and number.
    fn clubs(&self) -> Vec<(usize, usize)> {
        let clubs: Vec<(usize, usize)> = self
            .counts
            .iter()
            .enumerate()
            .flat_map(|(division, &count)| {
                (0..usize::from(count)).map(move |club| (division, club))
            })
            .collect();
        if clubs.is_empty() {
            vec![(0, 0)]
        } else {
            clubs
        }
    }

    /// The clubs a second team can be (its global number a byte).
    fn second_clubs(&self) -> Vec<(usize, usize)> {
        self.clubs()
            .into_iter()
            .filter(|&(division, club)| u8::try_from(Self::glob(division, club)).is_ok())
            .collect()
    }

    /// Every club's team word.
    fn teams(&self) -> Vec<u16> {
        self.clubs()
            .into_iter()
            .map(|(division, club)| self.team(division, club))
            .collect()
    }
}

/// A league's clubs: 1 to 5 divisions of a few clubs (none or many now and then).
fn layout() -> impl Strategy<Value = Layout> {
    (
        vec(
            prop_oneof![6 => 4..=12u8, 1 => 0..=3u8, 1 => 13..=20u8],
            1..=MAX_DIVISIONS,
        ),
        any::<bool>(),
    )
        .prop_map(|(counts, high)| Layout { counts, high })
}

/// A season's tables: each division's clubs in a rotated order, points falling by small steps
/// (ties now and then), any record; the last division's clubs past the divisions in use now and
/// then.
fn tables(layout: &Layout) -> impl Strategy<Value = RulesTables> + use<> {
    let layout = layout.clone();
    (
        vec(
            prop_oneof![2 => Just(0u16), 3 => 1..=3u16, 1 => 4..=12u16],
            DIVISIONS * CLUBS,
        ),
        vec(any::<[u8; 3]>(), DIVISIONS * CLUBS),
        vec(0..CLUBS, DIVISIONS),
        30..=90u16,
        prop::bool::weighted(0.1),
    )
        .prop_map(move |(drops, records, shifts, top, fewer)| {
            let mut rows = Box::new([[[0u16; 6]; CLUBS]; DIVISIONS]);
            let mut counts = [0; DIVISIONS];
            for (division, &count) in layout.counts.iter().enumerate() {
                counts[division] = count;
                let mut points = top;
                for place in 0..usize::from(count) {
                    let club = (place + shifts[division]) % usize::from(count);
                    let at = division * CLUBS + place;
                    points = points.saturating_sub(drops[at]);
                    let [won, gf, ga] = records[at].map(u16::from);
                    rows[division][place] = [
                        layout.team(division, club),
                        Layout::glob(division, club),
                        won % 20,
                        gf % 60,
                        ga % 60,
                        points,
                    ];
                }
            }
            RulesTables {
                divisions: layout.divisions() - u8::from(fewer && layout.divisions() > 1),
                counts,
                rows,
            }
        })
}

/// A stage table of division `division`'s clubs: rotated, points falling.
fn stage_rows(layout: &Layout, division: usize, shift: usize, drops: &[u16]) -> [RowSetup; CLUBS] {
    let mut rows = [[0; 6]; CLUBS];
    let count = layout.counts.get(division).copied().map_or(0, usize::from);
    let mut points = 60u16;
    for (place, row) in rows.iter_mut().enumerate().take(count) {
        let club = (place + shift) % count;
        points = points.saturating_sub(drops[place]);
        *row = [
            layout.team(division, club),
            Layout::glob(division, club),
            0,
            0,
            0,
            points,
        ];
    }
    rows
}

/// The stage tables a season wrote: of the top division mostly, some fresh, a kept table now
/// and then (of a division past the league's too).
fn bank(layout: &Layout) -> impl Strategy<Value = BankSetup> + use<> {
    let layout = layout.clone();
    (
        0..16u8,
        vec(
            (
                prop_oneof![4 => Just(0usize), 1 => 0..MAX_DIVISIONS],
                0..CLUBS,
                prop::bool::weighted(0.8),
            ),
            STAGE_TABLES,
        ),
        vec(prop_oneof![2 => Just(0u16), 1 => 1..=4u16], CLUBS),
        (
            0..=MAX_DIVISIONS + 1,
            prop_oneof![2 => Just(0u8), 1 => 1..=12u8],
            0..CLUBS,
        ),
    )
        .prop_map(
            move |(fresh, tables, drops, (kept_division, kept, shift))| {
                let mut counts = [0; STAGE_TABLES];
                let mut rows = Box::new([[[0; 6]; CLUBS]; STAGE_TABLES]);
                for (k, &(division, shift, whole)) in tables.iter().enumerate() {
                    let count = layout.counts.get(division).copied().unwrap_or(0);
                    counts[k] = if whole { count } else { count / 2 };
                    rows[k] = stage_rows(&layout, division, shift, &drops);
                }
                let kept_count = layout
                    .counts
                    .get(kept_division)
                    .map_or(kept, |&count| kept.min(count));
                BankSetup {
                    used: 1,
                    key: 0,
                    fresh,
                    counts,
                    tables: rows,
                    kept_division: u8::try_from(kept_division).expect("a few"),
                    kept_count,
                    kept: Box::new(stage_rows(&layout, kept_division, shift, &drops)),
                }
            },
        )
}

/// The promedio history at first: the top division's clubs, seeded or not.
fn promedio(layout: &Layout) -> impl Strategy<Value = PromedioSetup> + use<> {
    let count = usize::from(layout.counts[0]);
    (
        0..=1u8,
        vec(
            (0..CLUBS, [20..=90u16, 20..=90u16], [25..=38u8, 25..=38u8]),
            0..=count + 2,
        ),
    )
        .prop_map(|(init, clubs)| {
            let mut history = Box::new([(0, [0; 2], [0; 2]); PROMEDIO_CLUBS]);
            for (to, &(club, points, matches)) in history.iter_mut().zip(&clubs) {
                *to = (Layout::glob(0, club), points, matches);
            }
            PromedioSetup {
                init,
                count: u8::try_from(clubs.len()).expect("a few"),
                clubs: history,
            }
        })
}

/// The SEASON WINNERS page at first: empty mostly, some entries, past its 40 now and then.
fn winners() -> impl Strategy<Value = WinnersSetup> {
    (
        any::<u8>(),
        prop_oneof![6 => Just(0u8), 3 => 1..=40u8, 1 => 41..=42u8],
        0..=1u8,
        vec(any::<(u8, u8, u16)>(), WINNERS),
    )
        .prop_map(|(season, count, fresh, entries)| WinnersSetup {
            season,
            count,
            fresh,
            entries: entries.try_into().expect("40"),
        })
}

/// What the host answers: some ties played, a title, stage titles.
fn host(layout: &Layout) -> impl Strategy<Value = HostAnswers> + use<> {
    let teams = prop::sample::select(layout.teams());
    let clubs = prop::sample::select(layout.clubs());
    (
        vec((0..4i32, teams.clone(), teams.clone(), 0..3u8), 0..=3),
        vec(
            (
                0..4i32,
                prop_oneof![4 => teams.clone(), 1 => any::<u16>()],
                teams.clone(),
            ),
            0..=1,
        ),
        vec((0..3i32, clubs.clone(), clubs), 0..=2),
    )
        .prop_map({
            let layout = layout.clone();
            move |(ties, titles, stages)| HostAnswers {
                ties: ties
                    .into_iter()
                    .map(|(play_off, a, b, which)| {
                        let winner = match which {
                            0 => a,
                            1 => b,
                            _ => a ^ b,
                        };
                        (play_off, a, b, winner)
                    })
                    .collect(),
                titles,
                stage_titles: stages
                    .into_iter()
                    .map(|(stage, (d1, c1), (d2, c2))| {
                        (
                            stage,
                            [layout.team(d1, c1), layout.team(d2, c2)],
                            [Layout::glob(d1, c1), Layout::glob(d2, c2)],
                        )
                    })
                    .collect(),
            }
        })
}

/// A play-off's entrant found in the tables: by place, a club by team word (absent now and
/// then).
fn clean_entrant(layout: &Layout) -> impl Strategy<Value = [u8; 4]> + use<> {
    let byte = |value: usize| u8::try_from(value).expect("a byte's");
    let clubs = prop::sample::select(layout.clubs());
    prop_oneof![
        4 => clubs.clone().prop_map(move |(division, club)| [BY_PLACE, byte(division), byte(club + 1), 0]),
        1 => (clubs, prop::bool::weighted(0.1)).prop_map(move |((division, club), absent)| {
            let ordinal = if absent { ABSENT_ORDINAL } else { byte(club) };
            [BY_TEAM, 0, byte(usize::from(FIRST_FILE) + division), ordinal]
        }),
    ]
}

/// A play-off's entrant of any type, mostly by place.
fn entrant(layout: &Layout) -> impl Strategy<Value = [u8; 4]> + use<> {
    let divisions = layout.divisions();
    let clubs = prop::sample::select(layout.clubs());
    let places = clubs.clone().prop_map(|(division, club)| {
        let byte = |value: usize| u8::try_from(value).expect("a byte's");
        [BY_PLACE, byte(division), byte(club + 1), 0]
    });
    prop_oneof![
        8 => places,
        1 => (0..divisions, 1..=24u8).prop_map(|(division, place)| [BY_PLACE, division, place, 0]),
        2 => (0..divisions, 1..=12u8, prop::sample::select(vec![0u8, 1, 4, 5, 6, 7]))
            .prop_map(|(division, place, table)| [STAGE_PLACE, division, place, table]),
        1 => Just([PREVIOUS_LOSER, 0, 0, 0]),
        1 => (0..divisions, 1..=4u8).prop_map(|(division, at)| [FINALIST, division, at, 0]),
        1 => (0..divisions, 1..=4u8, prop::sample::select(vec![0u8, 4, 5, 6, 7]))
            .prop_map(|(division, at, table)| [SPLIT_SEED, division, at, table]),
        1 => (0..divisions, 1..=4u8, 0..=1u8).prop_map(|(division, at, rule)| [TITLE_SEED, division, at, rule]),
        2 => (clubs, prop::bool::weighted(0.2)).prop_map(|((division, club), absent)| {
            let file = u8::try_from(FIRST_FILE).expect("a file") + u8::try_from(division).expect("a few");
            let ordinal = u8::try_from(club).expect("32");
            [BY_TEAM, 0, file, if absent { ABSENT_ORDINAL } else { ordinal }]
        }),
    ]
}

/// A play-off's flags.
fn flags() -> impl Strategy<Value = u8> {
    (
        prop::bool::weighted(0.3),
        prop::bool::weighted(0.04),
        prop::bool::weighted(0.2),
        0..=3u8,
        prop::bool::weighted(0.15),
    )
        .prop_map(|(record, supercup, pot, winners, skip)| {
            (u8::from(record) * BY_RECORD)
                | (u8::from(supercup) * SUPERCUP)
                | (u8::from(pot) * DRAWN_POT)
                | (winners << WINNERS_SHIFT)
                | (u8::from(skip) * SKIP_SECONDS)
        })
}

/// Random choices from a list of numbers.
struct Dice {
    /// The numbers.
    values: Vec<u32>,
    /// The next.
    at: usize,
}

impl Dice {
    /// A choice below `bound` (0 for none).
    fn roll(&mut self, bound: usize) -> usize {
        let value = self.values[self.at % self.values.len()];
        self.at += 1;
        if bound == 0 {
            0
        } else {
            usize::try_from(value).expect("32 bits") % bound
        }
    }

    /// A byte below `bound`.
    fn byte(&mut self, bound: usize) -> u8 {
        u8::try_from(self.roll(bound)).expect("a byte")
    }
}

/// A play-off's rounds over `entrants` entrants: each round's members the last round's
/// winners or placings (the entrants first), a best loser, a loser or an entrant added now and
/// then; a group (of up to 8) or a knockout round of any legs, host, level, pairing and gap.
fn rounds(entrants: usize, count: usize, dice: &mut Dice) -> Vec<u8> {
    let mut bytes = Vec::new();
    let (mut ties_so_far, mut placings) = (0usize, 0usize);
    let mut pool: Vec<u8> = (0..entrants)
        .map(|at| u8::try_from(at).expect("a few"))
        .collect();
    for round in 0..count {
        let mut members = pool.clone();
        if round > 0 && ties_so_far > 0 && dice.roll(5) == 0 {
            members.push(BEST_LOSER);
        }
        if ties_so_far > 0 && dice.roll(8) == 0 {
            members.push(LOSER_OF | dice.byte(ties_so_far));
        }
        if entrants > 0 && dice.roll(8) == 0 {
            members.push(dice.byte(entrants));
        }
        if dice.roll(10) < 3 && placings + members.len() <= 64 && !members.is_empty() {
            members.truncate(8);
            let ties = members.len();
            let meetings = [1u8, 1, 2, 0][dice.roll(4)];
            bytes.extend([
                u8::try_from(ties).expect("8"),
                GROUP | meetings,
                dice.byte(4),
                0,
                0,
                0,
            ]);
            bytes.extend(&members);
            bytes.extend((0..ties).map(|_| dice.byte(4)));
            pool = (placings..placings + ties)
                .map(|at| PLACING | u8::try_from(at).expect("64"))
                .collect();
            placings += ties;
            continue;
        }
        if members.len() % 2 == 1 && dice.roll(10) < 8 {
            members.pop();
        }
        if members.is_empty() {
            members.push(0);
        }
        if members.len() == 1 {
            members.push(members[0]);
        }
        let ties = (members.len() / 2).min(64 - ties_so_far);
        if ties == 0 {
            break;
        }
        let pairing = match dice.roll(8) {
            0..=3 => 0,
            4 | 5 => 2 * dice.byte(ties + 1),
            6 => DRAW | dice.byte(2 * ties + 1),
            _ => dice.byte(2 * ties + 1),
        };
        let host = dice.byte(4) | if dice.roll(4) == 0 { SKIP_SAME } else { 0 };
        let gap = if dice.roll(5) == 0 {
            1 + dice.byte(12)
        } else {
            0
        };
        bytes.extend([
            u8::try_from(ties).expect("64"),
            1 + dice.byte(3),
            host,
            dice.byte(6),
            pairing,
            gap,
        ]);
        bytes.extend(&members[..2 * ties]);
        pool = (ties_so_far..ties_so_far + ties)
            .map(|at| WINNER_OF | u8::try_from(at).expect("64"))
            .collect();
        ties_so_far += ties;
    }
    bytes
}

/// A spec's play-off: a promedio's, or a header, its entrants and its rounds.
fn play_off(layout: &Layout) -> impl Strategy<Value = Vec<u8>> + use<> {
    let divisions = layout.divisions();
    (
        prop_oneof![
            4 => Just(PROMOTION),
            2 => Just(PLAY_OUT),
            3 => Just(TITLE),
            2 => Just(CONTINENTAL),
            1 => Just(PROMEDIO),
            2 => Just(STAGE),
            1 => 2..=3u8,
        ],
        flags(),
        prop_oneof![7 => vec(clean_entrant(layout), 1..=8), 3 => vec(entrant(layout), 1..=8)],
        1..=3usize,
        vec(any::<u32>(), 16),
        (any::<u8>(), any::<u8>()),
    )
        .prop_map(
            move |(kind, flags, entrants, count, values, (first, second))| {
                let spread = |value: u8| value % divisions;
                if kind == PROMEDIO {
                    let loser = if divisions > 1 {
                        1 + second % (divisions - 1)
                    } else {
                        1
                    };
                    return vec![PROMEDIO, 0, 0, 10 + first % 31, loser, 0];
                }
                let (winner_division, loser_division) = match kind {
                    PROMOTION | PLAY_OUT => (
                        spread(first),
                        if second % 10 == 0 {
                            RESERVE
                        } else {
                            spread(second)
                        },
                    ),
                    STAGE => ((first % 4) | (((first >> 2) % (divisions + 1)) << 2), 0),
                    _ => (first, second),
                };
                let mut dice = Dice { values, at: 0 };
                let rounds = rounds(entrants.len(), count, &mut dice);
                let mut bytes = vec![
                    kind,
                    u8::try_from(entrants.len()).expect("8"),
                    u8::try_from(count).expect("3"),
                    winner_division,
                    loser_division,
                    flags,
                ];
                bytes.extend(entrants.iter().flatten());
                bytes.extend(rounds);
                bytes
            },
        )
}

/// A spec of a few play-offs of `layout`'s league (none now and then).
fn spec(layout: &Layout) -> impl Strategy<Value = Option<Vec<u8>>> + use<> {
    prop_oneof![1 => Just(None), 12 => vec(play_off(layout), 1..=4).prop_map(|play_offs| {
        let mut bytes: Vec<u8> = play_offs.into_iter().flatten().collect();
        bytes.push(0);
        Some(bytes)
    })]
}

/// A league of `layout` and `spec`: counts by place, second teams (found by word now and
/// then), the promedio's seeds.
fn league<S>(layout: &Layout, spec: S) -> impl Strategy<Value = LeagueSetup> + use<S>
where
    S: Strategy<Value = Option<Vec<u8>>>,
{
    let layout = layout.clone();
    let divisions = layout.divisions();
    let clubs = prop::sample::select(layout.second_clubs());
    (
        vec(0..=3u8, DIVISIONS),
        vec(0..=3u8, DIVISIONS),
        vec(
            (clubs, prop_oneof![1 => Just(NONE), 2 => 0..divisions]),
            0..=3,
        ),
        (prop_oneof![1 => Just(NONE), 2 => 0..=4u8], 0..=3u8, 0..3u8),
        vec(
            (0..CLUBS, [20..=90u16, 20..=90u16], [25..=38u8, 25..=38u8]),
            0..=6,
        ),
        spec,
    )
        .prop_map(
            move |(promoted, relegated, seconds, (top, flags, words), seeds, spec)| {
                let second_words = match words {
                    0 => None,
                    1 => Some(
                        seconds
                            .iter()
                            .map(|&((division, club), _)| layout.team(division, club))
                            .collect(),
                    ),
                    _ => Some(
                        seconds
                            .iter()
                            .map(|&((division, club), _)| layout.team(division, club) ^ 1)
                            .collect(),
                    ),
                };
                LeagueSetup {
                    divisions,
                    promoted,
                    relegated,
                    seconds: seconds
                        .iter()
                        .map(|&((division, club), first)| {
                            [
                                u8::try_from(Layout::glob(division, club)).expect("a byte's"),
                                first,
                            ]
                        })
                        .collect(),
                    second_top: top,
                    second_flags: flags,
                    second_words,
                    promedio_seeds: seeds
                        .into_iter()
                        .map(|(club, [two, one], matches)| {
                            ([Layout::glob(0, club), two, one], matches)
                        })
                        .collect(),
                    team_file_high: u8::from(layout.high),
                    spec,
                    ..LeagueSetup::default()
                }
            },
        )
}

/// A season of `layout`: its tables, the moves before (the state's 48 nearly full now and
/// then), the career club (none, absent), its played tie, the preview, the second teams in the
/// knockouts, the levels, the host, the stage tables it wrote.
fn season(layout: &Layout) -> impl Strategy<Value = KnockoutsSeason> + use<> {
    let teams = prop::sample::select(layout.teams());
    let globs: Vec<u16> = layout
        .clubs()
        .into_iter()
        .map(|(division, club)| Layout::glob(division, club))
        .collect();
    let moved = (
        prop::sample::select(globs),
        prop::sample::select(vec![-1i8, 1]),
    )
        .prop_map(|(glob, delta)| (glob, delta, FIXED_TIERS_RULE));
    (
        tables(layout),
        prop_oneof![8 => vec(moved.clone(), 0..=3), 1 => vec(moved, 44..=48)],
        prop_oneof![1 => Just(0u16), 3 => teams.clone(), 1 => any::<u16>()],
        prop_oneof![2 => Just([0u16; 3]), 1 => (teams.clone(), teams, any::<bool>()).prop_map(|(a, b, first)| [a, b, if first { a } else { b }])],
        (prop_oneof![5 => Just(0u8), 1 => Just(1u8)], [0..=4u8, 0..=4u8], 0..=1u8),
        prop_oneof![9 => Just(0u16), 1 => 0..=40u16],
        prop_oneof![4 => Just(None), 1 => vec(0..=4u8, 1..=3).prop_map(Some)],
        prop_oneof![1 => Just(None), 2 => host(layout).prop_map(Some)],
        prop_oneof![1 => Just(None), 2 => bank(layout).prop_map(Some)],
    )
        .prop_map(
            |(tables, moves, manager, tie, (preview, own, seconds_in), base, levels, host, bank)| {
                KnockoutsSeason {
                    tables,
                    moves,
                    manager,
                    tie,
                    preview,
                    own,
                    seconds_in,
                    base,
                    levels,
                    host,
                    bank,
                }
            },
        )
}

/// A league's season ends over `layout` and `spec`: its stage tables kept or not, the
/// promedio history and the SEASON WINNERS page at first, one to three seasons.
fn run_of<S>(layout: &Layout, spec: S) -> impl Strategy<Value = KnockoutsRun> + use<S>
where
    S: Strategy<Value = Option<Vec<u8>>>,
{
    (
        league(layout, spec),
        prop::bool::weighted(0.7),
        prop_oneof![1 => Just(None), 2 => promedio(layout).prop_map(Some)],
        prop_oneof![1 => Just(None), 2 => winners().prop_map(Some)],
        vec(season(layout), 1..=3),
    )
        .prop_map(|(league, bank, promedio, winners, seasons)| KnockoutsRun {
            league,
            bank,
            promedio,
            winners,
            seasons,
        })
}

/// A random league's season ends.
fn run() -> impl Strategy<Value = KnockoutsRun> {
    layout().prop_flat_map(|layout| run_of(&layout, spec(&layout)))
}

/// A pack: its divisions' clubs and its spec.
type Pack = (Vec<u8>, Vec<u8>);

/// The rules library's packs (World 2026-27's leagues), downloaded once.
fn packs() -> Result<Vec<Pack>, Skip> {
    let dir = rules_specs_path();
    let mut paths = Vec::with_capacity(RULES_SPECS.len());
    for download in &RULES_SPECS {
        paths.push(fetch_into(&dir, download)?);
    }
    let shapes = std::fs::read_to_string(&paths[0]).expect("the shapes read");
    Ok(shapes
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(code, counts)| {
            let counts = counts
                .split(',')
                .map(|count| count.trim().parse().expect("a division's clubs"))
                .collect();
            let spec = std::fs::read(dir.join(format!("{code}.spec"))).expect("the spec read");
            (counts, spec)
        })
        .collect())
}

/// A pack's season ends: tables of its shape, its spec.
fn pack_run(packs: Vec<Pack>) -> impl Strategy<Value = KnockoutsRun> {
    (prop::sample::select(packs), any::<bool>()).prop_flat_map(|((counts, spec), high)| {
        let layout = Layout { counts, high };
        run_of(&layout, Just(Some(spec)))
    })
}

/// The host answering as upstream's host does.
struct Host(HostAnswers);

impl KnockoutHost for Host {
    fn played_tie(&mut self, play_off: i32, a: u16, b: u16) -> Option<u16> {
        self.0.tie(play_off, a, b)
    }

    fn played_title(&mut self, play_off: i32) -> Option<(u16, u16)> {
        self.0.title(play_off)
    }

    fn stage_title(&mut self, stage: i32) -> Option<([u16; 2], [u16; 2])> {
        self.0.stage_title(stage)
    }
}

/// The core's row of `row`.
fn core_row(&[team, glob, won, goals_for, goals_against, points]: &RowSetup) -> Row {
    Row {
        team,
        glob,
        won,
        goals_for,
        goals_against,
        points,
    }
}

/// The row's setup.
fn row_setup(row: &Row) -> RowSetup {
    [
        row.team,
        row.glob,
        row.won,
        row.goals_for,
        row.goals_against,
        row.points,
    ]
}

/// The core's stage tables of `setup`.
fn core_bank(setup: &BankSetup) -> StageBank {
    StageBank {
        used: setup.used,
        key: setup.key,
        fresh: setup.fresh,
        counts: setup.counts,
        tables: setup.tables.map(|table| table.map(|row| core_row(&row))),
        kept_division: setup.kept_division,
        kept_count: setup.kept_count,
        kept: setup.kept.map(|row| core_row(&row)),
    }
}

/// The stage tables' setup.
fn bank_setup(bank: &StageBank) -> BankSetup {
    BankSetup {
        used: bank.used,
        key: bank.key,
        fresh: bank.fresh,
        counts: bank.counts,
        tables: Box::new(bank.tables.map(|table| table.map(|row| row_setup(&row)))),
        kept_division: bank.kept_division,
        kept_count: bank.kept_count,
        kept: Box::new(bank.kept.map(|row| row_setup(&row))),
    }
}

/// The core's promedio history of `setup`.
fn core_promedio(setup: &PromedioSetup) -> Promedio {
    Promedio {
        init: setup.init != 0,
        count: setup.count,
        clubs: setup.clubs.map(|(glob, points, matches)| PromedioClub {
            glob,
            points,
            matches,
        }),
    }
}

/// The promedio history's setup.
fn promedio_setup(promedio: &Promedio) -> PromedioSetup {
    PromedioSetup {
        init: u8::from(promedio.init),
        count: promedio.count,
        clubs: Box::new(
            promedio
                .clubs
                .map(|club| (club.glob, club.points, club.matches)),
        ),
    }
}

/// The core's SEASON WINNERS page of `setup`.
fn core_winners(setup: &WinnersSetup) -> Winners {
    Winners {
        season: setup.season,
        count: setup.count,
        fresh: setup.fresh,
        entries: setup
            .entries
            .map(|(kind, index, team)| WinnersEntry { kind, index, team }),
    }
}

/// The page's setup.
fn winners_setup(winners: &Winners) -> WinnersSetup {
    WinnersSetup {
        season: winners.season,
        count: winners.count,
        fresh: winners.fresh,
        entries: winners
            .entries
            .map(|entry| (entry.kind, entry.index, entry.team)),
    }
}

/// A result's setup.
fn result_setup(result: &KnockoutResult) -> ResultSetup {
    (
        result.kind.map_or(0, u8::from),
        result.division,
        result.team,
        result.glob,
    )
}

/// A group table's row's setup.
fn figures_setup(figures: &GroupFigures) -> FiguresSetup {
    [
        figures.club,
        figures.points,
        figures.goal_difference,
        figures.goals,
    ]
}

/// The core's moves of `moves`.
fn core_moves(moves: &[(u16, i8, u8)]) -> Vec<Move> {
    moves
        .iter()
        .map(|&(glob, delta, rule)| Move {
            glob,
            delta,
            rule: MoveRule::try_from(rule).expect("a rule"),
        })
        .collect()
}

/// The port's season ends of `run`, one after another on one state, until one it refuses
/// (upstream reads past its memory: the error) or one reading a round not played.
fn ours(run: &KnockoutsRun) -> (Vec<KnockoutsAfter>, Option<RulesError>) {
    let league = core_league(&run.league);
    let mut state = Knockouts::default();
    let mut bank = run.bank.then(StageBank::default);
    let mut promedio = run.promedio.as_ref().map(core_promedio);
    let mut winners = run.winners.as_ref().map(core_winners);
    let mut after = Vec::new();
    for season in &run.seasons {
        if let (Some(bank), Some(setup)) = (bank.as_mut(), &season.bank) {
            *bank = core_bank(setup);
        }
        let tables = core_tables(&season.tables);
        let mut moves = core_moves(&season.moves);
        let levels = season.levels.clone().map(|first| Levels { first });
        let mut host = season.host.clone().map(Host);
        let [a, b, winner] = season.tie;
        let mut knockouts = KnockoutSeason {
            manager: season.manager,
            played: PlayedTie { a, b, winner },
            bank: bank.as_mut(),
            promedio: promedio.as_mut(),
            winners: winners.as_mut(),
            host: host.as_mut().map(|host| host as &mut dyn KnockoutHost),
            preview: season.preview != 0,
            own_play_offs: season.own,
            second_teams_in_knockouts: season.seconds_in != 0,
            base: season.base,
            levels: levels.as_ref(),
        };
        let code = match state.season(&tables, &league, &mut knockouts, &mut moves) {
            Ok(outcome) => {
                if outcome.read_unplayed {
                    break;
                }
                i32::try_from(outcome.moves).expect("48 moves")
            }
            Err(RulesError::Check(_)) => -1,
            Err(error) => return (after, Some(error)),
        };
        let (members, placings) = state.figures();
        after.push(KnockoutsAfter {
            code,
            moves: moves
                .iter()
                .map(|made| (made.glob, made.delta, u8::from(made.rule)))
                .collect(),
            results: state.results().iter().map(result_setup).collect(),
            stage_titles: state.stage_titles().each_ref().map(result_setup),
            member_figures: Box::new(members.each_ref().map(figures_setup)),
            placing_figures: Box::new(placings.each_ref().map(figures_setup)),
            bank: bank.as_ref().map(bank_setup),
            promedio: promedio.as_ref().map(promedio_setup),
            winners: winners.as_ref().map(winners_setup),
        });
    }
    (after, None)
}

/// The port's season ends of `run` are upstream's.
fn check(run: &KnockoutsRun) -> Result<(), TestCaseError> {
    let (ours, _refused) = ours(run);
    let theirs = Oracle::lock().knockouts(run, ours.len());
    prop_assert_eq!(ours.len(), theirs.len());
    for (season, (ours, theirs)) in ours.iter().zip(&theirs).enumerate() {
        prop_assert_eq!(ours, theirs, "season {}", season);
    }
    Ok(())
}

proptest! {
    #[test]
    fn the_knockouts_are_upstream_s(run in run()) {
        check(&run)?;
    }
}

#[test]
fn the_packs_knockouts_are_upstream_s() {
    let packs = openswos_test_utils::require!(packs());
    // (No regressions file: the packs are not the source's.)
    let mut runner = TestRunner::new(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    });
    if let Err(failure) = runner.run(&pack_run(packs), |run| check(&run)) {
        panic!("{failure}");
    }
}
