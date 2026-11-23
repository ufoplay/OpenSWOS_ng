//! A play-off's group rounds (upstream src/rules/sr_gplay.c `gmatch`, `gs_before`, `group`,
//! `pot_perm`): each match's goals by the clubs' strengths, a group's table by points, goal
//! difference, goals and then the place; a ranking round of the members' own groups' figures;
//! the drawn pot's entrants after its heads.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::play::{Club, Play};
use super::{DRAWN_POT, ENTRANT, GROUP, GroupFigures, HEADER, PAIR};

/// The odds in 100 of a group match drawn.
const DRAW_ODDS: u32 = 26;

/// A percentage's whole.
const PERCENT: u32 = 100;

/// The goals of a drawn match, and the winner's above one: below 3.
const GOALS: u32 = 3;

/// The points of a win.
const WIN: i32 = 3;

impl Play<'_, '_> {
    /// A group match of `home` v `away` (upstream `gmatch`): drawn 26 times in 100 (0 to 2
    /// goals each), else by the strengths, the winner 1 to 3 goals, the loser fewer; home
    /// advantage unless `neutral`.
    fn group_match(&mut self, home: Club, away: Club, neutral: bool) -> (i32, i32) {
        let home_strength = self.strength(home, !neutral);
        let away_strength = self.strength(away, false);
        if self.below(PERCENT) < DRAW_ODDS {
            let goals = i32::try_from(self.below(GOALS)).expect("3 goals");
            return (goals, goals);
        }
        let home_won = self.below(home_strength.wrapping_add(away_strength)) < home_strength;
        let winner = 1 + i32::try_from(self.below(GOALS)).expect("3 goals");
        let loser = i32::try_from(self.below(winner.cast_unsigned())).expect("3 goals");
        if home_won {
            (winner, loser)
        } else {
            (loser, winner)
        }
    }

    /// Whether figures `x` come before `y` (upstream `gs_before`): points, goal difference,
    /// goals, then the better placed.
    fn before(&self, x: &GroupFigures, y: &GroupFigures) -> bool {
        if x.points != y.points {
            return x.points > y.points;
        }
        if x.goal_difference != y.goal_difference {
            return x.goal_difference > y.goal_difference;
        }
        if x.goals != y.goals {
            return x.goals > y.goals;
        }
        self.better(x.club, y.club)
    }

    /// A group round (upstream `group`): `members` play a round robin `meetings` times (home
    /// and away alternating), from their `bonus` points; with no meetings, a ranking round of the
    /// members' own groups' figures (`self.state`'s members' figures). Their order and figures
    /// into `order` and the placings' figures from `first`.
    pub(super) fn group(
        &mut self,
        members: &[Club],
        meetings: u8,
        neutral: bool,
        bonus: &[u8],
        order: &mut [Club],
        first: usize,
    ) {
        let mut table: Vec<GroupFigures> = members
            .iter()
            .zip(bonus)
            .enumerate()
            .map(|(index, (&club, &bonus))| {
                if meetings == 0 {
                    let carried = self.state.member_figures[index];
                    GroupFigures { club, ..carried }
                } else {
                    GroupFigures {
                        club,
                        points: i32::from(bonus),
                        goal_difference: 0,
                        goals: 0,
                    }
                }
            })
            .collect();
        for meeting in 0..meetings {
            for i in 0..table.len() {
                for j in i + 1..table.len() {
                    let (home, away) = if meeting & 1 == 0 { (i, j) } else { (j, i) };
                    let (home_goals, away_goals) =
                        self.group_match(table[home].club, table[away].club, neutral);
                    table[home].goals += home_goals;
                    table[away].goals += away_goals;
                    table[home].goal_difference += home_goals - away_goals;
                    table[away].goal_difference += away_goals - home_goals;
                    match home_goals.cmp(&away_goals) {
                        std::cmp::Ordering::Greater => table[home].points += WIN,
                        std::cmp::Ordering::Less => table[away].points += WIN,
                        std::cmp::Ordering::Equal => {
                            table[home].points += 1;
                            table[away].points += 1;
                        }
                    }
                }
            }
        }
        for index in 1..table.len() {
            let figures = table[index];
            let mut at = index;
            while at > 0 && self.before(&figures, &table[at - 1]) {
                table[at] = table[at - 1];
                at -= 1;
            }
            table[at] = figures;
        }
        for (index, figures) in table.iter().enumerate() {
            order[index] = figures.club;
            self.state.placing_figures[first + index] = *figures;
        }
    }

    /// The drawn pot (upstream `pot_perm`): the entrants' order, those after the heads (one per
    /// leading group round) drawn.
    pub(super) fn pot_permutation(&mut self, spec: &[u8]) -> Vec<usize> {
        let entrants = usize::from(spec[1]);
        let mut order: Vec<usize> = (0..entrants).collect();
        if spec[5] & DRAWN_POT == 0 {
            return order;
        }
        let mut heads = 0;
        let mut round = HEADER + ENTRANT * entrants;
        for _ in 0..spec[2] {
            if spec[round + 1] & GROUP == 0 {
                break;
            }
            heads += 1;
            round += HEADER + PAIR * usize::from(spec[round]);
        }
        let mut index = entrants;
        while index > heads + 1 {
            index -= 1;
            let bound = u32::try_from(index - heads + 1).expect("64 entrants");
            let other = heads + usize::try_from(self.below(bound)).expect("an entrant");
            order.swap(index, other);
        }
        order
    }
}

#[cfg(test)]
mod tests {
    use super::super::Knockouts;
    use super::super::fixture::{league, season, tables, with_play};
    use super::super::play::{NONE, club_at};
    use super::*;
    use crate::competition::rules_library::generator::below as draw;

    /// Figures of `club`: points, goal difference, goals.
    fn figures(club: Club, points: i32, goal_difference: i32, goals: i32) -> GroupFigures {
        GroupFigures {
            club,
            points,
            goal_difference,
            goals,
        }
    }

    #[test]
    fn a_group_table_is_by_points_goal_difference_goals_then_the_place() {
        let tables = tables(&[4]);
        let league = league(1, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                let (first, second) = (club_at(0, 0), club_at(0, 1));
                assert!(play.before(&figures(second, 4, 0, 0), &figures(first, 3, 9, 9)));
                assert!(play.before(&figures(second, 3, 1, 0), &figures(first, 3, 0, 9)));
                assert!(play.before(&figures(second, 3, 1, 2), &figures(first, 3, 1, 1)));
                assert!(play.before(&figures(first, 3, 1, 1), &figures(second, 3, 1, 1)));
                assert!(!play.before(&figures(second, 3, 1, 1), &figures(first, 3, 1, 1)));
            },
        );
    }

    #[test]
    fn a_group_match_is_drawn_26_times_in_100_else_won_by_the_strengths() {
        let tables = tables(&[4]);
        let league = league(1, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                let (home, away) = (club_at(0, 0), club_at(0, 3));
                for seed in 0..200 {
                    play.rng = seed;
                    let (home_goals, away_goals) = play.group_match(home, away, false);
                    let mut state = seed;
                    let drawn = draw(&mut state, PERCENT) < DRAW_ODDS;
                    assert_eq!(home_goals == away_goals, drawn, "{seed}");
                    let goals = i32::try_from(GOALS).expect("3");
                    assert!(home_goals.max(away_goals) <= goals && home_goals.min(away_goals) >= 0);
                    if !drawn {
                        let (x, y) = (play.strength(home, true), play.strength(away, false));
                        let home_won = draw(&mut state, x + y) < x;
                        assert_eq!(home_goals > away_goals, home_won, "{seed}");
                    }
                }
            },
        );
    }

    #[test]
    fn a_group_plays_each_pair_its_meetings_its_placings_kept_from_the_first() {
        let tables = tables(&[6]);
        let league = league(1, None);
        let mut state = Knockouts::default();
        let members = [club_at(0, 4), club_at(0, 0), club_at(0, 2)];
        let mut order = [NONE; 3];
        with_play(&tables, &league, &mut season(), &mut state, |play| {
            play.group(&members, 2, true, &[1, 0, 0], &mut order, 5);
        });
        let (_, placings) = state.figures();
        let table = &placings[5..8];
        assert_eq!(
            table.iter().map(|row| row.club).collect::<Vec<_>>(),
            order.to_vec()
        );
        let mut clubs = order.to_vec();
        clubs.sort_unstable();
        assert_eq!(clubs, [0, 2, 4]);
        assert_eq!(table.iter().map(|row| row.goal_difference).sum::<i32>(), 0);
        // Six matches, 2 or 3 points each, and the bonus point.
        let points: i32 = table.iter().map(|row| row.points).sum();
        assert!((13..=19).contains(&points), "{points}");
        assert!(
            table
                .windows(2)
                .all(|pair| (pair[0].points, pair[0].goal_difference)
                    >= (pair[1].points, pair[1].goal_difference))
        );
    }

    #[test]
    fn a_ranking_round_orders_the_members_own_figures() {
        let tables = tables(&[6]);
        let league = league(1, None);
        let mut state = Knockouts::default();
        state.member_figures[0] = figures(NONE, 4, 1, 3);
        state.member_figures[1] = figures(NONE, 6, 0, 2);
        let members = [club_at(0, 0), club_at(0, 1)];
        let mut order = [NONE; 2];
        with_play(&tables, &league, &mut season(), &mut state, |play| {
            play.group(&members, 0, false, &[9, 9], &mut order, 0);
            assert_eq!(play.rng, 0, "no match");
        });
        assert_eq!(order, [club_at(0, 1), club_at(0, 0)]);
        assert_eq!(state.figures().1[0], figures(club_at(0, 1), 6, 0, 2));
    }

    #[test]
    fn the_drawn_pot_shuffles_the_entrants_after_the_heads() {
        let tables = tables(&[6]);
        let league = league(1, None);
        // Five entrants; a group round of two (one head), then a knockout round.
        let mut spec = vec![1, 5, 2, 0, 0, DRAWN_POT];
        spec.extend([0; 5 * ENTRANT]);
        spec.extend([2, GROUP | 1, 0, 0, 0, 0, 0, 1, 0, 0]);
        spec.extend([1, 1, 0, 0, 0, 0, 2, 3]);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                let mut moved = [false; 5];
                for seed in 0..20 {
                    play.rng = seed;
                    let order = play.pot_permutation(&spec);
                    assert_eq!(order[0], 0, "the head kept");
                    let mut sorted = order.clone();
                    sorted.sort_unstable();
                    assert_eq!(sorted, [0, 1, 2, 3, 4]);
                    for (at, &entrant) in order.iter().enumerate() {
                        moved[at] |= entrant != at;
                    }
                }
                assert_eq!(moved, [false, true, true, true, true]);
                let mut plain = spec.clone();
                plain[5] = 0;
                assert_eq!(play.pot_permutation(&plain), [0, 1, 2, 3, 4]);
            },
        );
    }
}
