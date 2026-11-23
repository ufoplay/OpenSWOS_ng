//! A season end's knockouts at play (upstream src/rules/sr_gplay.c's file statics, the caller's
//! state): the tables, the league, the season, the moves so far, the generator; a club's row,
//! its strength, a tie, a series, the career club's played ties, the results.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::Knockouts;
use crate::competition::rules_library::generator::below;
use crate::competition::rules_library::league::League;
use crate::competition::rules_library::moves::{MOVES, Move, MoveRule};
use crate::competition::rules_library::season::{
    KnockoutResult, KnockoutSeason, NO_CLUB, ResultKind, StageBank,
};
use crate::competition::rules_library::table::{Row, Tables};

/// A club of the tables: its division times 32 plus its place; below 0, none.
pub(super) type Club = i32;

/// No club.
pub(super) const NONE: Club = -1;

/// A club by team word not in the tables (`ABSENT`): a group round goes on without it.
pub(super) const ABSENT: Club = -2;

/// The clubs of a division in a club's number (`GT`).
const ROW_CLUBS: i32 = 32;

/// A tie's host: as listed (the first club), the better placed, drawn, neutral.
pub(super) const HOST_LISTED: u8 = 0;
/// See [`HOST_LISTED`].
pub(super) const HOST_BETTER: u8 = 1;
/// See [`HOST_LISTED`].
pub(super) const HOST_DRAWN: u8 = 2;
/// See [`HOST_LISTED`].
pub(super) const HOST_NEUTRAL: u8 = 3;

/// A tie level after its regulation time: the better placed through; extra time and penalties;
/// extra time, then the better placed; penalties; away goals then penalties; away goals then the
/// better placed.
pub(super) const LEVEL_BETTER: u8 = 0;
/// See [`LEVEL_BETTER`].
pub(super) const LEVEL_EXTRA_TIME_PENALTIES: u8 = 1;
/// See [`LEVEL_BETTER`].
pub(super) const LEVEL_EXTRA_TIME_BETTER: u8 = 2;
/// See [`LEVEL_BETTER`].
pub(super) const LEVEL_AWAY_BETTER: u8 = 5;

/// A club's strength's base, its place's share and its tier's toll.
const STRENGTH_BASE: i32 = 30;
/// See [`STRENGTH_BASE`].
const STRENGTH_PLACE: i32 = 40;
/// See [`STRENGTH_BASE`].
const STRENGTH_TIER: i32 = 25;
/// The weakest strength.
const STRENGTH_FLOOR: i32 = 5;

/// The home side's strength, by tenths (11/10).
const HOME_TENTHS: i32 = 11;
/// A tenth's whole.
const TENTHS: i32 = 10;

/// A tie's legs: a single match, two.
pub(super) const ONE_LEG: u8 = 1;
/// See [`ONE_LEG`].
const TWO_LEGS: u8 = 2;

/// A coin toss's sides (a drawn host).
const COIN: u32 = 2;

/// Extra time and penalties: each side's strength counts twice in its own odds.
const OWN_WEIGHT: u32 = 2;

/// The odds in 100 of a single match level, and of two legs.
const LEVEL_SINGLE: u32 = 22;
/// See [`LEVEL_SINGLE`].
const LEVEL_LEGS: u32 = 12;

/// A percentage's whole.
const PERCENT: u32 = 100;

/// A series' games: best of three.
const SERIES_GAMES: usize = 3;
/// The wins that take a series.
const SERIES_WINS: u8 = 2;

/// The results a season end keeps (`res[4]`).
pub(super) const RESULTS: usize = 4;

/// A season end's knockouts at play.
pub(super) struct Play<'p, 'a> {
    /// The league's final tables.
    pub(super) tables: &'p Tables,
    /// The league's rules.
    pub(super) league: &'p League,
    /// The season.
    pub(super) season: &'p mut KnockoutSeason<'a>,
    /// The state the season ends keep.
    pub(super) state: &'p mut Knockouts,
    /// The moves so far.
    pub(super) moves: &'p mut Vec<Move>,
    /// The bank when the season has none (`no_bank`).
    pub(super) no_bank: StageBank,
    /// A 'keep' stage's frozen table (`st[0]`).
    pub(super) kept: Vec<Row>,
    /// The generator.
    pub(super) rng: u32,
    /// The better record hosts within a division (spec flag 2).
    pub(super) by_record: bool,
    /// The play-off at play.
    pub(super) play_off: i32,
    /// The stage tables used this season end.
    pub(super) stages_used: bool,
    /// The SEASON WINNERS page written.
    pub(super) winners_on: bool,
    /// A round not played was read.
    pub(super) read_unplayed: bool,
}

impl Play<'_, '_> {
    /// The stage tables: the season's, else none.
    pub(super) fn bank(&self) -> &StageBank {
        self.season.bank.as_deref().unwrap_or(&self.no_bank)
    }

    /// A draw below `bound`.
    pub(super) fn below(&mut self, bound: u32) -> u32 {
        below(&mut self.rng, bound)
    }

    /// The tables' divisions in use (`nd`).
    pub(super) fn divisions(&self) -> i32 {
        i32::try_from(self.tables.divisions()).expect("16 divisions")
    }

    /// Division `division`'s clubs (any of the 16).
    pub(super) fn count(&self, division: i32) -> i32 {
        i32::try_from(self.tables.count(index(division))).expect("32 clubs")
    }

    /// The row of division `division`, place `place` (from 0).
    pub(super) fn row_at(&self, division: i32, place: i32) -> Row {
        self.tables.row(index(division), index(place))
    }

    /// Club `club`'s row.
    pub(super) fn row(&self, club: Club) -> Row {
        self.row_at(tier(club), place(club))
    }

    /// Whether club `a` is better placed than club `b` (upstream `gp_better`): the higher tier,
    /// then, by record within a division when the spec says (points, wins, goal difference,
    /// goals), the better place.
    pub(super) fn better(&self, a: Club, b: Club) -> bool {
        if tier(a) != tier(b) {
            return tier(a) < tier(b);
        }
        if self.by_record {
            let (x, y) = (self.row(a), self.row(b));
            let difference = |row: &Row| i32::from(row.goals_for) - i32::from(row.goals_against);
            if x.points != y.points {
                return x.points > y.points;
            }
            if x.won != y.won {
                return x.won > y.won;
            }
            if difference(&x) != difference(&y) {
                return difference(&x) > difference(&y);
            }
            if x.goals_for != y.goals_for {
                return x.goals_for > y.goals_for;
            }
        }
        place(a) < place(b)
    }

    /// Club `club`'s strength (upstream `gp_str2`): by its place in its division and its tier,
    /// 10% up at home, squared.
    pub(super) fn strength(&self, club: Club, home: bool) -> u32 {
        let clubs = self.count(tier(club));
        let mut strength = STRENGTH_BASE + STRENGTH_PLACE * (clubs - place(club)) / clubs.max(1)
            - STRENGTH_TIER * tier(club);
        strength = strength.max(STRENGTH_FLOOR);
        if home {
            strength = strength * HOME_TENTHS / TENTHS;
        }
        (strength * strength).cast_unsigned()
    }

    /// A tie between `a` (the single match's host, the first leg's home club, as listed) and
    /// `b` (upstream `gp_tie`): the better placed hosting the single match or the return leg
    /// (`host` 1, or 3 neutral), or drawn (2); level 22 times in 100 (one leg) or 12, then the
    /// better placed through, or extra time and penalties, close to a coin toss; else by the
    /// strengths.
    pub(super) fn tie(&mut self, a: Club, b: Club, legs: u8, host: u8, level: u8) -> Club {
        let (mut home, mut other) = (a, b);
        if host == HOST_BETTER || host == HOST_NEUTRAL {
            if self.better(b, a) {
                (home, other) = (b, a);
            }
            if legs == TWO_LEGS || host == HOST_NEUTRAL {
                (home, other) = (other, home);
            }
        } else if host == HOST_DRAWN && self.below(COIN) != 0 {
            (home, other) = (b, a);
        }
        let advantage = if legs == ONE_LEG {
            (host != HOST_NEUTRAL).then_some(home)
        } else {
            Some(other)
        };
        let mut home_strength = self.strength(home, advantage == Some(home));
        let mut other_strength = self.strength(other, advantage == Some(other));
        let odds = if legs == ONE_LEG {
            LEVEL_SINGLE
        } else {
            LEVEL_LEGS
        };
        if self.below(PERCENT) < odds {
            let better_placed = if self.better(home, other) {
                home
            } else {
                other
            };
            if [LEVEL_BETTER, LEVEL_EXTRA_TIME_BETTER, LEVEL_AWAY_BETTER].contains(&level) {
                return better_placed;
            }
            let (x, y) = (self.strength(home, false), self.strength(other, false));
            home_strength = x.wrapping_mul(OWN_WEIGHT).wrapping_add(y);
            other_strength = x.wrapping_add(y.wrapping_mul(OWN_WEIGHT));
        }
        if self.below(home_strength.wrapping_add(other_strength)) < home_strength {
            home
        } else {
            other
        }
    }

    /// A best of three (upstream `gp_series`): games 1 and 3 at the better placed's.
    pub(super) fn series(&mut self, a: Club, b: Club, level: u8) -> Club {
        let home = if self.better(a, b) { a } else { b };
        let other = if home == a { b } else { a };
        let (mut home_wins, mut other_wins) = (0, 0);
        for game in 0..SERIES_GAMES {
            if home_wins >= SERIES_WINS || other_wins >= SERIES_WINS {
                break;
            }
            let (host, guest) = if game == 1 {
                (other, home)
            } else {
                (home, other)
            };
            if self.tie(host, guest, ONE_LEG, HOST_LISTED, level) == home {
                home_wins += 1;
            } else {
                other_wins += 1;
            }
        }
        if home_wins == SERIES_WINS {
            home
        } else {
            other
        }
    }

    /// A tie the career club played (upstream `gp_played`), applied after the tie is simulated
    /// so the draws are the same either way: the host's play-off, played, else the season's
    /// played tie when it is this one; else `winner`.
    pub(super) fn played(&mut self, a: Club, b: Club, winner: Club) -> Club {
        let (team_a, team_b) = (self.row(a).team, self.row(b).team);
        let play_off = self.play_off;
        if let Some(host) = self.season.host.as_deref_mut()
            && let Some(played) = host.played_tie(play_off, team_a, team_b)
        {
            return if played == team_a {
                a
            } else if played == team_b {
                b
            } else {
                winner
            };
        }
        let tie = self.season.played;
        if tie.winner == 0
            || !((team_a == tie.a && team_b == tie.b) || (team_a == tie.b && team_b == tie.a))
        {
            return winner;
        }
        if tie.winner == team_a {
            a
        } else if tie.winner == team_b {
            b
        } else {
            winner
        }
    }

    /// The career club's division in the tables (upstream `gp_mgr_div`), none for none.
    pub(super) fn manager_division(&self) -> i32 {
        let manager = self.season.manager;
        (0..self.divisions().min(DIVISIONS_I32))
            .find(|&division| {
                (0..self.count(division))
                    .any(|at| manager != 0 && self.row_at(division, at).team == manager)
            })
            .unwrap_or(NONE)
    }

    /// A result decided beside the moves (upstream `gp_result`): of kind `kind`, the winner
    /// `winner`'s, the loser `loser`'s (none for none); at most four.
    pub(super) fn result(&mut self, kind: ResultKind, winner: Club, loser: Club) {
        if self.state.result_count >= RESULTS {
            return;
        }
        let won = self.row(winner);
        let (team, glob) = if loser >= 0 {
            let lost = self.row(loser);
            (lost.team, lost.glob)
        } else {
            (NO_CLUB, NO_CLUB)
        };
        self.state.results[self.state.result_count] = KnockoutResult {
            kind: Some(kind),
            division: u8::try_from(tier(winner)).expect("16 divisions"),
            team: [won.team, team],
            glob: [won.glob, glob],
        };
        self.state.result_count += 1;
    }

    /// Club `club`'s move of `delta` divisions (upstream `gp_move`, `add_move`): none for 0, at
    /// most 48 in all. Faithful to the original: the delta wraps to a signed byte.
    pub(super) fn move_club(&mut self, club: Club, delta: i32) {
        let glob = self.row(club).glob;
        if delta != 0 && self.moves.len() < MOVES {
            let [low, ..] = delta.to_le_bytes();
            self.moves.push(Move {
                glob,
                delta: low.cast_signed(),
                rule: MoveRule::Knockouts,
            });
        }
    }
}

/// The divisions the tables hold, as a count.
pub(super) const DIVISIONS_I32: i32 = 16;

/// A club's division (upstream `DV`).
pub(super) fn tier(club: Club) -> i32 {
    club / ROW_CLUBS
}

/// A club's place (upstream `PL`).
pub(super) fn place(club: Club) -> i32 {
    club % ROW_CLUBS
}

/// The club of division `division`, place `place`.
pub(super) fn club_at(division: i32, place: i32) -> Club {
    division * ROW_CLUBS + place
}

/// An index of the tables (never below 0 where the C reads).
fn index(value: i32) -> usize {
    usize::try_from(value).expect("an index of the tables")
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{Host, league, rows, season, tables, tables_of, team, with_play};
    use super::*;
    use crate::competition::rules_library::generator::below as draw;
    use crate::competition::rules_library::season::PlayedTie;

    /// A seed whose first draw below `bound` meets `wanted`.
    fn seed_where(bound: u32, wanted: impl Fn(u32) -> bool) -> u32 {
        (0..)
            .find(|&seed| {
                let mut state = seed;
                wanted(draw(&mut state, bound))
            })
            .expect("a seed")
    }

    #[test]
    fn a_club_is_its_division_and_place() {
        let club = club_at(2, 5);
        assert_eq!((tier(club), place(club)), (2, 5));
        // Faithful to the original: an absent club's division is 0 (C's division truncates).
        assert_eq!((tier(ABSENT), place(ABSENT)), (0, -2));
        assert_eq!(index(3), 3);
    }

    #[test]
    fn a_play_reads_the_tables_and_the_bank() {
        let tables = tables(&[4, 3]);
        let league = league(2, None);
        let mut bank = StageBank {
            fresh: 1,
            ..StageBank::default()
        };
        let mut with_bank = season();
        with_bank.bank = Some(&mut bank);
        with_play(
            &tables,
            &league,
            &mut with_bank,
            &mut Knockouts::default(),
            |play| {
                assert_eq!((play.divisions(), play.count(1), play.count(2)), (2, 3, 0));
                assert_eq!(play.row(club_at(1, 2)).team, team(1, 2));
                assert_eq!(play.row_at(0, 1), play.row(1));
                assert_eq!(play.bank().fresh, 1);
                let mut state = play.rng;
                assert_eq!(play.below(10), draw(&mut state, 10));
                assert_eq!(play.rng, state);
            },
        );
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.bank().fresh, 0);
            },
        );
    }

    #[test]
    fn the_better_placed_is_the_higher_tier_then_the_place_or_the_record() {
        let counts = [4, 4];
        let mut better_record = rows(&counts);
        // The 3rd of division 1 level on points with the 2nd, more wins.
        better_record[1][2].points = better_record[1][1].points;
        better_record[1][2].won += 1;
        let tables = tables_of(&counts, better_record);
        let league = league(2, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                assert!(play.better(club_at(0, 3), club_at(1, 0)));
                assert!(!play.better(club_at(1, 0), club_at(0, 3)));
                assert!(play.better(club_at(1, 1), club_at(1, 2)));
                play.by_record = true;
                assert!(play.better(club_at(1, 2), club_at(1, 1)));
                assert!(play.better(club_at(1, 0), club_at(1, 1)), "more points");
            },
        );
    }

    #[test]
    fn a_club_s_strength_is_its_place_and_tier_squared() {
        let tables = tables(&[4, 4, 4, 4]);
        let league = league(4, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                // 30 + 40 * (4 - 0) / 4 = 70, 77 at home.
                assert_eq!(play.strength(club_at(0, 0), false), 70 * 70);
                assert_eq!(play.strength(club_at(0, 0), true), 77 * 77);
                // 30 + 40 * 1 / 4 - 25 = 15.
                assert_eq!(play.strength(club_at(1, 3), false), 15 * 15);
                // Below the floor: 5.
                assert_eq!(play.strength(club_at(3, 3), false), 5 * 5);
            },
        );
    }

    #[test]
    fn a_tie_level_after_its_time_goes_to_the_better_placed_by_its_level() {
        let tables = tables(&[4, 4]);
        let league = league(2, None);
        let (better, worse) = (club_at(0, 0), club_at(1, 3));
        let level = seed_where(PERCENT, |percent| percent < LEVEL_SINGLE);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                play.rng = level;
                assert_eq!(
                    play.tie(worse, better, ONE_LEG, HOST_LISTED, LEVEL_BETTER),
                    better
                );
                // Extra time and penalties: close to a coin toss, the strengths' draw.
                play.rng = level;
                let mut state = level;
                draw(&mut state, PERCENT);
                let (x, y) = (play.strength(worse, false), play.strength(better, false));
                let (home, other) = (2 * x + y, x + 2 * y);
                let expected = if draw(&mut state, home + other) < home {
                    worse
                } else {
                    better
                };
                assert_eq!(
                    play.tie(
                        worse,
                        better,
                        ONE_LEG,
                        HOST_LISTED,
                        LEVEL_EXTRA_TIME_PENALTIES
                    ),
                    expected
                );
                assert_eq!(play.rng, state);
            },
        );
    }

    #[test]
    fn a_tie_decided_in_its_time_goes_by_the_strengths_the_host_up() {
        let tables = tables(&[4, 4]);
        let league = league(2, None);
        let (better, worse) = (club_at(0, 1), club_at(1, 0));
        let decided = seed_where(PERCENT, |percent| percent >= LEVEL_LEGS.max(LEVEL_SINGLE));
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                for (legs, host) in [
                    (ONE_LEG, HOST_LISTED),
                    (TWO_LEGS, HOST_BETTER),
                    (ONE_LEG, HOST_NEUTRAL),
                ] {
                    play.rng = decided;
                    let mut state = decided;
                    draw(&mut state, PERCENT);
                    // The better placed hosts the single match, the return leg; none at neutral.
                    let (home, other) = if legs == TWO_LEGS || host == HOST_NEUTRAL {
                        (worse, better)
                    } else if host == HOST_BETTER {
                        (better, worse)
                    } else {
                        (worse, better)
                    };
                    let advantage = if legs == ONE_LEG {
                        (host != HOST_NEUTRAL).then_some(home)
                    } else {
                        Some(other)
                    };
                    let x = play.strength(home, advantage == Some(home));
                    let y = play.strength(other, advantage == Some(other));
                    let expected = if draw(&mut state, x + y) < x {
                        home
                    } else {
                        other
                    };
                    assert_eq!(
                        play.tie(worse, better, legs, host, LEVEL_BETTER),
                        expected,
                        "{legs} {host}"
                    );
                    assert_eq!(play.rng, state);
                }
            },
        );
    }

    #[test]
    fn a_drawn_host_tosses_a_coin_first() {
        let tables = tables(&[4]);
        let league = league(1, None);
        let (a, b) = (club_at(0, 0), club_at(0, 1));
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                for (coin, (home, away)) in [(0, (a, b)), (1, (b, a))] {
                    let seed = seed_where(COIN, |side| side == coin);
                    play.rng = seed;
                    let drawn = play.tie(a, b, ONE_LEG, HOST_DRAWN, LEVEL_BETTER);
                    let after = play.rng;
                    let mut state = seed;
                    draw(&mut state, COIN);
                    play.rng = state;
                    assert_eq!(
                        play.tie(home, away, ONE_LEG, HOST_LISTED, LEVEL_BETTER),
                        drawn
                    );
                    assert_eq!(play.rng, after);
                }
            },
        );
    }

    #[test]
    fn a_series_is_best_of_three_its_games_1_and_3_at_the_better_placed_s() {
        let tables = tables(&[6]);
        let league = league(1, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                for seed in 0..50 {
                    let (better, worse) = (club_at(0, 1), club_at(0, 4));
                    play.rng = seed;
                    let mut wins = [0u8; 2];
                    for game in 0..SERIES_GAMES {
                        if wins.contains(&SERIES_WINS) {
                            break;
                        }
                        let (host, guest) = if game == 1 {
                            (worse, better)
                        } else {
                            (better, worse)
                        };
                        let won = play.tie(host, guest, ONE_LEG, HOST_LISTED, LEVEL_BETTER);
                        wins[usize::from(won != better)] += 1;
                    }
                    let (expected, after) = (
                        if wins[0] == SERIES_WINS {
                            better
                        } else {
                            worse
                        },
                        play.rng,
                    );
                    play.rng = seed;
                    assert_eq!(play.series(worse, better, LEVEL_BETTER), expected);
                    assert_eq!(play.rng, after);
                }
            },
        );
    }

    #[test]
    fn the_career_club_s_tie_is_as_played() {
        let tables = tables(&[4]);
        let league = league(1, None);
        let (a, b) = (club_at(0, 0), club_at(0, 1));
        let mut played = season();
        played.played = PlayedTie {
            a: team(0, 1),
            b: team(0, 0),
            winner: team(0, 1),
        };
        with_play(
            &tables,
            &league,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.played(a, b, a), b, "either order");
                assert_eq!(play.played(a, club_at(0, 2), a), a, "another tie");
            },
        );
        let mut host = Host {
            tie: Some((0, team(0, 0), team(0, 1), team(0, 0))),
            ..Host::default()
        };
        let mut hosted = season();
        hosted.host = Some(&mut host);
        hosted.played.winner = team(0, 1);
        with_play(
            &tables,
            &league,
            &mut hosted,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.played(a, b, b), a, "the host's first");
                play.play_off = 1;
                assert_eq!(play.played(a, b, a), a, "not the host's play-off");
            },
        );
        let mut none = season();
        none.played = PlayedTie {
            a: team(0, 0),
            b: team(0, 1),
            winner: 0,
        };
        with_play(
            &tables,
            &league,
            &mut none,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.played(a, b, b), b, "no winner: simulated");
            },
        );
    }

    #[test]
    fn the_career_club_s_division_is_where_its_team_is() {
        let tables = tables(&[4, 4]);
        let league = league(2, None);
        for (manager, division) in [(team(1, 2), 1), (0, NONE), (team(5, 0), NONE)] {
            let mut played = season();
            played.manager = manager;
            with_play(
                &tables,
                &league,
                &mut played,
                &mut Knockouts::default(),
                |play| {
                    assert_eq!(play.manager_division(), division);
                },
            );
        }
    }

    #[test]
    fn four_results_are_kept_a_loser_none_or_its_club() {
        let tables = tables(&[4, 4]);
        let league = league(2, None);
        let mut state = Knockouts::default();
        with_play(&tables, &league, &mut season(), &mut state, |play| {
            play.result(ResultKind::Title, club_at(1, 0), NONE);
            for _ in 0..RESULTS {
                play.result(ResultKind::Continental, club_at(0, 0), club_at(0, 1));
            }
        });
        let results = state.results();
        assert_eq!(results.len(), RESULTS);
        assert_eq!(
            results[0],
            KnockoutResult {
                kind: Some(ResultKind::Title),
                division: 1,
                team: [team(1, 0), NO_CLUB],
                glob: [32, NO_CLUB],
            }
        );
        assert_eq!(results[1].team, [team(0, 0), team(0, 1)]);
    }

    #[test]
    fn a_move_wraps_to_a_byte_none_for_0_at_most_48() {
        let tables = tables(&[4]);
        let league = league(1, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                play.move_club(club_at(0, 1), 0);
                assert!(play.moves.is_empty());
                // Faithful to the original: 300 wraps to 44.
                play.move_club(club_at(0, 1), 300);
                assert_eq!(
                    play.moves[0],
                    Move {
                        glob: 1,
                        delta: 44,
                        rule: MoveRule::Knockouts
                    }
                );
                for _ in 0..MOVES {
                    play.move_club(club_at(0, 2), -1);
                }
                assert_eq!(play.moves.len(), MOVES);
            },
        );
    }
}
