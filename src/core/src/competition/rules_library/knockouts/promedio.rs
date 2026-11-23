//! Relegation by the promedio (upstream src/rules/sr_gplay.c `pr_find`, `pr_seed`, `promedio`;
//! the Mod's gplay.c promedio; LPF reglamento 2026 art. 26, AFA statute art. 93): the last of the
//! promedios (points per match over this season's annual table and the two before, top-flight
//! seasons only) and the last of the annual table go down; a tie in a relegation place is a
//! play-off match on neutral ground. The history per club is the league's seed at a new career,
//! shifted after every season end.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::play::{HOST_NEUTRAL, LEVEL_EXTRA_TIME_PENALTIES, ONE_LEG, Play};
use crate::competition::rules_library::season::{PROMEDIO_CLUBS, Promedio, PromedioClub};

/// The bank's annual table.
pub(super) const ANNUAL: usize = 2;

/// The annual table's bit in the bank's fresh tables.
pub(super) const ANNUAL_FRESH: u8 = 1 << ANNUAL;

/// The clubs a promedio needs at least.
const FEWEST: usize = 3;

/// The division the promedio relegates from: the top.
const TOP: i32 = 0;

/// The history of club `glob` (upstream `pr_find`).
fn find(promedio: &Promedio, glob: u16) -> Option<PromedioClub> {
    promedio.clubs[..usize::from(promedio.count).min(PROMEDIO_CLUBS)]
        .iter()
        .find(|club| club.glob == glob)
        .copied()
}

impl Play<'_, '_> {
    /// The promedio's history at a new career (upstream `pr_seed`): the league's seeds.
    fn seed_promedio(&mut self) {
        let seeds = &self.league.promedio_seeds;
        let Some(promedio) = self.season.promedio.as_deref_mut() else {
            return;
        };
        promedio.count = 0;
        for seed in seeds.iter().take(PROMEDIO_CLUBS) {
            promedio.clubs[usize::from(promedio.count)] = PromedioClub {
                glob: seed.glob,
                points: [seed.points_two, seed.points_one],
                matches: [seed.matches_two, seed.matches_one],
            };
            promedio.count += 1;
        }
        promedio.init = true;
    }

    /// The promedio's relegation (upstream `promedio`): `matches` this season's matches
    /// counted, the relegated down to division `loser`.
    pub(super) fn promedio(&mut self, matches: u8, loser: u8) {
        if self.season.promedio.is_none() {
            return;
        }
        if !self
            .season
            .promedio
            .as_deref()
            .is_some_and(|promedio| promedio.init)
        {
            self.seed_promedio();
        }
        let bank = self.bank();
        let mut annual = Some(bank.tables[ANNUAL]);
        let mut clubs = usize::from(bank.counts[ANNUAL]);
        if bank.fresh & ANNUAL_FRESH == 0 || i32::try_from(clubs).expect("32") != self.count(TOP) {
            clubs = usize::try_from(self.count(TOP)).expect("32 clubs");
            annual = None;
        }
        self.stages_used = true;
        if clubs < FEWEST {
            return;
        }
        let glob_of = |play: &Self, at: usize| {
            annual.map_or_else(
                || play.row_at(TOP, i32::try_from(at).expect("32")).glob,
                |rows| rows[at].glob,
            )
        };
        let points_of = |play: &Self, at: usize| {
            annual.map_or_else(
                || play.row_at(TOP, i32::try_from(at).expect("32")).points,
                |rows| rows[at].points,
            )
        };
        let history = self.season.promedio.as_deref().expect("a promedio").clone();
        let (mut points, mut played) = (Vec::with_capacity(clubs), Vec::with_capacity(clubs));
        for at in 0..clubs {
            let club = find(&history, glob_of(self, at));
            points.push(
                u32::from(points_of(self, at))
                    + club.map_or(0, |club| {
                        u32::from(club.points[0]) + u32::from(club.points[1])
                    }),
            );
            played.push(
                u32::from(matches)
                    + club.map_or(0, |club| {
                        u32::from(club.matches[0]) + u32::from(club.matches[1])
                    }),
            );
        }
        // Faithful to the original: the products wrap past 32 bits.
        let below = |i: usize, j: usize| {
            points[i].wrapping_mul(played[j]) < points[j].wrapping_mul(played[i])
        };
        let level = |i: usize, j: usize| {
            points[i].wrapping_mul(played[j]) == points[j].wrapping_mul(played[i])
        };
        let mut worst = 0;
        for at in 1..clubs {
            if below(at, worst) || (level(at, worst) && at > worst) {
                worst = at;
            }
        }
        let mut next_worst: Option<usize> = None;
        for at in 0..clubs {
            if at != worst && next_worst.is_none_or(|next| below(at, next)) {
                next_worst = Some(at);
            }
        }
        let mut down_first = worst;
        if let Some(next) = next_worst
            && level(next, worst)
        {
            let a = self.glob_club(TOP, glob_of(self, worst));
            let b = self.glob_club(TOP, glob_of(self, next));
            if a >= 0 && b >= 0 {
                let winner = self.tie(a, b, ONE_LEG, HOST_NEUTRAL, LEVEL_EXTRA_TIME_PENALTIES);
                down_first = if winner == a { next } else { worst };
            }
        }
        let mut last = clubs - 1;
        if last == down_first {
            last -= 1;
        }
        let before_last = last.checked_sub(1).and_then(|before| {
            if before == down_first {
                before.checked_sub(1)
            } else {
                Some(before)
            }
        });
        let mut down_second = last;
        if let Some(before) = before_last
            && points_of(self, before) == points_of(self, last)
        {
            let a = self.glob_club(TOP, glob_of(self, last));
            let b = self.glob_club(TOP, glob_of(self, before));
            if a >= 0 && b >= 0 {
                let winner = self.tie(a, b, ONE_LEG, HOST_NEUTRAL, LEVEL_EXTRA_TIME_PENALTIES);
                down_second = if winner == a { before } else { last };
            }
        }
        let first = self.glob_club(TOP, glob_of(self, down_first));
        let second = self.glob_club(TOP, glob_of(self, down_second));
        let delta = i32::from(loser) - TOP;
        if first >= 0 {
            self.move_club(first, delta);
        }
        if second >= 0 {
            self.move_club(second, delta);
        }
        let mut next = Vec::with_capacity(PROMEDIO_CLUBS);
        for at in 0..clubs {
            if next.len() >= PROMEDIO_CLUBS {
                break;
            }
            if at == down_first || at == down_second {
                continue;
            }
            let glob = glob_of(self, at);
            let club = find(&history, glob);
            next.push(PromedioClub {
                glob,
                points: [club.map_or(0, |club| club.points[1]), points_of(self, at)],
                matches: [club.map_or(0, |club| club.matches[1]), matches],
            });
        }
        let promedio = self.season.promedio.as_deref_mut().expect("a promedio");
        promedio.clubs[..next.len()].copy_from_slice(&next);
        promedio.count = u8::try_from(next.len()).expect("48 clubs");
        promedio.init = true;
    }
}

#[cfg(test)]
mod tests {
    use super::super::Knockouts;
    use super::super::fixture::{glob, league, rows, season, tables, tables_of, with_play};
    use super::super::play::{ONE_LEG, club_at};
    use super::*;
    use crate::competition::rules_library::moves::{Move, MoveRule};
    use crate::competition::rules_library::season::{PromedioSeed, StageBank};

    /// This season's matches.
    const MATCHES: u8 = 30;

    /// The relegated's division.
    const DOWN: u8 = 1;

    /// A club's move down to [`DOWN`].
    fn down(glob: u16) -> Move {
        Move {
            glob,
            delta: 1,
            rule: MoveRule::Knockouts,
        }
    }

    #[test]
    fn the_last_promedio_and_the_last_of_the_rest_go_down_the_history_shifted() {
        let tables = tables(&[6, 4]);
        let league = league(2, None);
        let mut history = Promedio {
            init: true,
            ..Promedio::default()
        };
        // The top's 1st: a poor history, the last promedio.
        history.clubs[0] = PromedioClub {
            glob: glob(0, 0),
            points: [1, 2],
            matches: [38, 38],
        };
        history.count = 1;
        let mut played = season();
        played.promedio = Some(&mut history);
        let moves = with_play(
            &tables,
            &league,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                play.promedio(MATCHES, DOWN);
                assert!(play.stages_used);
                play.moves.clone()
            },
        );
        assert_eq!(moves, [down(glob(0, 0)), down(glob(0, 5))]);
        assert_eq!(history.count, 4);
        assert_eq!(
            history.clubs[0],
            PromedioClub {
                glob: glob(0, 1),
                points: [0, 57],
                matches: [0, MATCHES],
            }
        );
        assert_eq!(find(&history, glob(0, 0)), None);
        assert_eq!(
            find(&history, glob(0, 4)).map(|club| club.points),
            Some([0, 48])
        );
    }

    #[test]
    fn without_a_history_the_last_two_go_down() {
        let tables = tables(&[6, 4]);
        let league = league(2, None);
        let mut history = Promedio {
            init: true,
            ..Promedio::default()
        };
        let mut played = season();
        played.promedio = Some(&mut history);
        let moves = with_play(
            &tables,
            &league,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                play.promedio(MATCHES, DOWN);
                play.moves.clone()
            },
        );
        assert_eq!(moves, [down(glob(0, 5)), down(glob(0, 4))]);
    }

    #[test]
    fn a_level_promedio_is_a_play_off_on_neutral_ground() {
        let counts = [6, 4];
        let mut level = rows(&counts);
        level[0][5].points = level[0][4].points;
        let tables = tables_of(&counts, level);
        let league = league(2, None);
        for seed in 0..8 {
            let mut history = Promedio {
                init: true,
                ..Promedio::default()
            };
            let mut played = season();
            played.promedio = Some(&mut history);
            let moves = with_play(
                &tables,
                &league,
                &mut played,
                &mut Knockouts::default(),
                |play| {
                    play.rng = seed;
                    let (worst, next) = (club_at(0, 5), club_at(0, 4));
                    let winner = play.tie(
                        worst,
                        next,
                        ONE_LEG,
                        HOST_NEUTRAL,
                        LEVEL_EXTRA_TIME_PENALTIES,
                    );
                    let (first, rest) = if winner == worst {
                        (next, worst)
                    } else {
                        (worst, next)
                    };
                    play.rng = seed;
                    play.moves.clear();
                    play.promedio(MATCHES, DOWN);
                    (play.moves.clone(), first, rest)
                },
            );
            let (moves, first, _) = moves;
            assert_eq!(moves[0].glob, u16::try_from(first).expect("a club"));
            assert_eq!(moves.len(), 2);
        }
    }

    #[test]
    fn the_annual_table_of_the_season_counts_when_fresh() {
        let tables = tables(&[4, 4]);
        let league = league(2, None);
        let mut bank = StageBank {
            fresh: ANNUAL_FRESH,
            ..StageBank::default()
        };
        // The annual table: the final table upside down.
        bank.counts[ANNUAL] = 4;
        for place in 0..4 {
            let mut row = tables.row(0, 3 - place);
            row.points = 50 - 3 * u16::try_from(place).expect("4");
            bank.tables[ANNUAL][place] = row;
        }
        let mut history = Promedio {
            init: true,
            ..Promedio::default()
        };
        let mut played = season();
        played.bank = Some(&mut bank);
        played.promedio = Some(&mut history);
        let moves = with_play(
            &tables,
            &league,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                play.promedio(MATCHES, DOWN);
                play.moves.clone()
            },
        );
        assert_eq!(moves, [down(glob(0, 0)), down(glob(0, 1))]);
    }

    #[test]
    fn a_new_career_s_history_is_the_league_s_seed() {
        let tables = tables(&[2, 4]);
        let mut seeded = league(2, None);
        seeded.promedio_seeds = vec![PromedioSeed {
            glob: glob(0, 1),
            points_two: 40,
            points_one: 41,
            matches_two: 30,
            matches_one: 31,
        }];
        let mut history = Promedio::default();
        let mut played = season();
        played.promedio = Some(&mut history);
        with_play(
            &tables,
            &seeded,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                play.promedio(MATCHES, DOWN);
                // Two clubs: too few, nothing more.
                assert!(play.moves.is_empty() && play.stages_used);
            },
        );
        assert!(history.init);
        assert_eq!(
            find(&history, glob(0, 1)),
            Some(PromedioClub {
                glob: glob(0, 1),
                points: [40, 41],
                matches: [30, 31],
            })
        );
        with_play(
            &tables,
            &seeded,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                play.promedio(MATCHES, DOWN);
                assert!(!play.stages_used, "no history: nothing");
            },
        );
    }
}
