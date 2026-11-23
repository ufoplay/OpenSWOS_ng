//! The fixed tiers' brackets: the season end's first step (upstream src/rules/sr_dplay.c
//! `sr_italy`, `sr_out_held`, `sr_playoff_held`; the SWOS 96/97 Mod's tools/dplay.c
//! `dp_season`), ported as the Mod wrote it: its generator, seeds, tie model and order. The
//! rules the Mod follows (2025-26: FIGC C.U. 39/A, NOIF art. 49, C.U. 114/A, C.U. 209/A):
//!
//! - The upper tier: the 3rd up directly if it leads the 4th by more than 14 points, else the
//!   play-off's winner (3rd to 8th: preliminaries 5th v 8th and 6th v 7th, single, a draw to
//!   the better placed; semi-finals 3rd v the winner of 6th v 7th and 4th v the winner of 5th
//!   v 8th, two legs, level to the better placed; a final of two legs, level to the better
//!   placed, or extra time and penalties when level on points). The 17th down directly if the
//!   16th leads it by more than 4 points, else the play-out's loser (two legs, level to the
//!   16th, or extra time and penalties when level on points).
//! - The lower tier's groups: the play-off's winner up (in each group 5th v 10th, 6th v 9th,
//!   7th v 8th, then the three winners and the 4th; national rounds with the thirds, the cup
//!   winner's place and the best group qualifier, then the seconds and the best winner; a Final
//!   Four).
//!
//! A tie is decided by the clubs' strengths, (points + 20) squared, the home side's points
//! 10% up. A play-off the career club played keeps its real winner.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::check::league_check;
use super::generator::below;
use super::league::League;
use super::moves::{Move, MoveRule};
use super::table::{Row, RulesError, Tables};

/// A tier's clubs the brackets hold (dplay.c `NT`).
const TIER_CLUBS: usize = 24;

/// The tiers the brackets hold: the upper one, then up to four groups (dplay.c `NDIV`).
const TIERS: usize = 5;

/// The upper tier's clubs at least for its play-off and play-out.
const UPPER_CLUBS: usize = 17;

/// A group's clubs at least for the lower tier's play-off.
const GROUP_CLUBS: usize = 11;

/// The upper tier's places (from 0): the 3rd to the 8th, the 16th, the 17th.
const THIRD: usize = 2;
/// See [`THIRD`].
const FOURTH: usize = 3;
/// See [`THIRD`].
const FIFTH: usize = 4;
/// See [`THIRD`].
const SIXTH: usize = 5;
/// See [`THIRD`].
const SEVENTH: usize = 6;
/// See [`THIRD`].
const EIGHTH: usize = 7;
/// See [`THIRD`].
const SIXTEENTH: usize = 15;
/// See [`THIRD`].
const SEVENTEENTH: usize = 16;

/// The lead over the 4th by which the 3rd goes up directly (more than it).
const DIRECT_LEAD: u16 = 14;

/// The lead over the 17th by which the 16th is safe (more than it).
const SAFE_LEAD: u16 = 4;

/// A group's place from which the cup winner keeps its own place in the play-off (the 11th).
const CUP_PLACE: usize = 10;

/// The smallest group with play-out places.
const PLAY_OUT_GROUP: usize = 6;

/// A play-out place's first, its second and its third from the bottom (n - 5, n - 4).
const PLAY_OUT_FIRST: usize = 5;
/// See [`PLAY_OUT_FIRST`].
const PLAY_OUT_SECOND: usize = 4;
/// The play-out places' opponents from the bottom (n - 2, n - 3).
const PLAY_OUT_OPPONENT_FIRST: usize = 2;
/// See [`PLAY_OUT_OPPONENT_FIRST`].
const PLAY_OUT_OPPONENT_SECOND: usize = 3;

/// The lead by which a play-out is not played (8 points or more).
const PLAY_OUT_GAP: i32 = 8;

/// What a tie's strength adds to the points.
const STRENGTH_BASE: u32 = 20;

/// The home side's strength, by tenths (11/10).
const HOME_TENTHS: u32 = 11;

/// A tenth's whole.
const TENTHS: u32 = 10;

/// The odds in 100 of a single match level, and of two legs.
const LEVEL_SINGLE: u32 = 22;
/// See [`LEVEL_SINGLE`].
const LEVEL_LEGS: u32 = 12;

/// A percentage's whole.
const PERCENT: u32 = 100;

/// The seed's multiplier over the tables.
const SEED_MULTIPLIER: u32 = 31;

/// How far the goals are shifted in the seed.
const GOALS_SHIFT: u32 = 8;

/// The lots' draws (a byte each).
const LOT_DRAW: u32 = 256;

/// The legs of a tie of two.
const TWO_LEGS: u8 = 2;

/// The groups the lower tier's national rounds hold at most (the C's tables of 5 seeds).
const NATIONAL_SEEDS: usize = 5;

/// The Final Four's clubs.
const FINAL_FOUR: usize = 4;

/// What the career club played itself (`sr_season`'s): each play-off's real winner (its team
/// word, 0 simulated), the cup's winner (0 unknown), whether the upper tier's play-off and
/// play-out are held whatever the points (a test).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FixedTiersSeason {
    /// The upper tier's play-off's winner (`po_win`).
    pub upper_play_off: u16,
    /// The upper tier's play-out's winner (`out_win`).
    pub upper_play_out: u16,
    /// The lower tier's play-off's winner (`cpo_win`).
    pub lower_play_off: u16,
    /// The lower tier's cup's winner (`cup_winner`).
    pub cup_winner: u16,
    /// Both of the upper tier's held whatever the points (`test_held`).
    pub test_held: bool,
}

/// Whether the upper tier's play-out is held (upstream `sr_out_held`): unless the 16th leads
/// the 17th by more than 4 points.
#[must_use]
pub fn play_out_held(sixteenth: u16, seventeenth: u16) -> bool {
    u32::from(sixteenth) <= u32::from(seventeenth) + u32::from(SAFE_LEAD)
}

/// Whether the upper tier's play-off is held (upstream `sr_playoff_held`): unless the 3rd leads
/// the 4th by more than 14 points.
#[must_use]
pub fn play_off_held(third: u16, fourth: u16) -> bool {
    u32::from(third) <= u32::from(fourth) + u32::from(DIRECT_LEAD)
}

/// A club of the brackets: its tier times 24 plus its place.
type Club = usize;

/// The brackets' working state (upstream `st->dp`).
struct Brackets<'a> {
    /// Each tier's clubs.
    counts: [usize; TIERS],
    /// Each tier's rows.
    rows: [[Row; TIER_CLUBS]; TIERS],
    /// The generator.
    state: u32,
    /// Each club's lot, the last tie-breaker.
    lots: [u8; TIERS * TIER_CLUBS],
    /// The lower tier's groups.
    groups: usize,
    /// What the career club played.
    season: &'a FixedTiersSeason,
    /// The moves so far.
    moves: Vec<Move>,
}

/// A club's tier.
fn tier(club: Club) -> usize {
    club / TIER_CLUBS
}

/// A club's place.
fn place(club: Club) -> usize {
    club % TIER_CLUBS
}

impl Brackets<'_> {
    /// A club's row.
    fn row(&self, club: Club) -> &Row {
        &self.rows[tier(club)][place(club)]
    }

    /// A draw below `bound`.
    fn below(&mut self, bound: u32) -> u32 {
        below(&mut self.state, bound)
    }

    /// Whether club `a` is better placed than club `b` (upstream `dp_better`): by place, in one
    /// tier or at different places; else by points, wins, goals, then the lot.
    fn better(&self, a: Club, b: Club) -> bool {
        if place(a) != place(b) || tier(a) == tier(b) {
            return place(a) < place(b);
        }
        let (x, y) = (self.row(a), self.row(b));
        if x.points != y.points {
            return x.points > y.points;
        }
        if x.won != y.won {
            return x.won > y.won;
        }
        if x.goals_for != y.goals_for {
            return x.goals_for > y.goals_for;
        }
        self.lots[a] < self.lots[b]
    }

    /// A club's strength (upstream `dp_str2`): its points plus 20, 10% up at home, squared.
    /// Faithful to the original: 32 bits, wrapping.
    fn strength(&self, club: Club, home: bool) -> u32 {
        let mut strength = u32::from(self.row(club).points) + STRENGTH_BASE;
        if home {
            strength = strength * HOME_TENTHS / TENTHS;
        }
        strength.wrapping_mul(strength)
    }

    /// A tie between `host` (the single match's or the return leg's home side) and `other`
    /// over `legs` (upstream `dp_tie`): level 22 times in 100 (one leg) or 12 (two), then
    /// `level` through, or with none extra time and penalties, close to a coin toss; else by
    /// the strengths.
    fn tie(&mut self, host: Club, other: Club, legs: u8, level: Option<Club>) -> Club {
        let (mut home, mut away) = (self.strength(host, true), self.strength(other, false));
        let odds = if legs == 1 { LEVEL_SINGLE } else { LEVEL_LEGS };
        if self.below(PERCENT) < odds {
            if let Some(level) = level {
                return level;
            }
            let even = self.strength(host, false);
            home = even.wrapping_mul(2).wrapping_add(away);
            away = even.wrapping_add(away.wrapping_mul(2));
        }
        if self.below(home.wrapping_add(away)) < home {
            host
        } else {
            other
        }
    }

    /// A move of one level for `club` (upstream `dp_move`).
    fn record(&mut self, club: Club, delta: i8) {
        let glob = self.row(club).glob;
        self.moves.push(Move {
            glob,
            delta,
            rule: MoveRule::FixedTiers,
        });
    }

    /// The best placed of `clubs` (upstream `best_of`), the first at equal.
    fn best_of(&self, clubs: &[Club]) -> Club {
        let mut best = clubs[0];
        for &club in &clubs[1..] {
            if self.better(club, best) {
                best = club;
            }
        }
        best
    }

    /// `clubs` sorted best placed first (upstream `sort_better`: an insertion sort).
    fn sort_better(&self, clubs: &mut [Club]) {
        for index in 1..clubs.len() {
            let club = clubs[index];
            let mut at = index;
            while at > 0 && self.better(club, clubs[at - 1]) {
                clubs[at] = clubs[at - 1];
                at -= 1;
            }
            clubs[at] = club;
        }
    }

    /// `clubs` shuffled (upstream `shuffle`).
    fn shuffle(&mut self, clubs: &mut [Club]) {
        for index in (1..clubs.len()).rev() {
            let bound = u32::try_from(index + 1).expect("a few clubs");
            let other = usize::try_from(self.below(bound)).expect("a club");
            clubs.swap(index, other);
        }
    }

    /// A seeded round (upstream `seeded`): `others` drawn against `seeds`, two legs, the return
    /// at the seed's, level to the seed. Each tie's winner.
    fn seeded(&mut self, seeds: &[Club], others: &mut [Club]) -> Vec<Club> {
        self.shuffle(others);
        seeds
            .iter()
            .zip(others.iter())
            .map(|(&seed, &other)| self.tie(seed, other, TWO_LEGS, Some(seed)))
            .collect()
    }

    /// The upper tier's play-off and play-out (upstream `serie_b`).
    fn upper(&mut self) {
        if self.counts[0] < UPPER_CLUBS {
            return;
        }
        let points = |brackets: &Self, place: usize| brackets.rows[0][place].points;
        let held = self.season.test_held;
        if !(play_off_held(points(self, THIRD), points(self, FOURTH)) || held) {
            self.record(THIRD, -1);
        } else {
            let fifth_eighth = self.tie(FIFTH, EIGHTH, 1, Some(FIFTH));
            let sixth_seventh = self.tie(SIXTH, SEVENTH, 1, Some(SIXTH));
            let first = self.tie(THIRD, sixth_seventh, TWO_LEGS, Some(THIRD));
            let second = self.tie(FOURTH, fifth_eighth, TWO_LEGS, Some(FOURTH));
            let host = if self.better(first, second) {
                first
            } else {
                second
            };
            let other = if host == first { second } else { first };
            let level = (points(self, place(host)) != points(self, place(other))).then_some(host);
            let mut winner = self.tie(host, other, TWO_LEGS, level);
            if self.season.upper_play_off != 0 {
                for club in THIRD..=EIGHTH {
                    if self.rows[0][club].team == self.season.upper_play_off {
                        winner = club;
                    }
                }
            }
            self.record(winner, -1);
        }
        if !(play_out_held(points(self, SIXTEENTH), points(self, SEVENTEENTH)) || held) {
            self.record(SEVENTEENTH, 1);
        } else {
            let level = (points(self, SIXTEENTH) != points(self, SEVENTEENTH)).then_some(SIXTEENTH);
            let mut winner = self.tie(SIXTEENTH, SEVENTEENTH, TWO_LEGS, level);
            // Faithful to the original: no winner given (0) matches a team word of 0.
            if self.season.upper_play_out == self.rows[0][SIXTEENTH].team {
                winner = SIXTEENTH;
            }
            if self.season.upper_play_out == self.rows[0][SEVENTEENTH].team {
                winner = SEVENTEENTH;
            }
            let loser = if winner == SIXTEENTH {
                SEVENTEENTH
            } else {
                SIXTEENTH
            };
            self.record(loser, 1);
        }
    }

    /// Whether place `at` of group `group` is relegated directly or a play-out place (upstream
    /// `cup_out`: the Mod 4.1): the play-out places n-5 v n-2 and n-4 v n-3, each not played
    /// when 8 points or more apart.
    fn cup_out(&self, group: usize, at: usize) -> bool {
        let clubs = self.counts[group];
        if clubs < PLAY_OUT_GROUP || at < clubs - PLAY_OUT_FIRST {
            return false;
        }
        let points = |at: usize| i32::from(self.rows[group][at].points);
        if at == clubs - PLAY_OUT_FIRST {
            return points(clubs - PLAY_OUT_FIRST) - points(clubs - PLAY_OUT_OPPONENT_FIRST)
                < PLAY_OUT_GAP;
        }
        if at == clubs - PLAY_OUT_SECOND {
            return points(clubs - PLAY_OUT_SECOND) - points(clubs - PLAY_OUT_OPPONENT_SECOND)
                < PLAY_OUT_GAP;
        }
        true
    }

    /// The cup winner's place in the lower tier's play-off (upstream `cup_place`): the winner
    /// itself when it finished 11th or lower and is neither relegated nor in a play-out, else
    /// the 11th of its group; unknown, the best 11th of the groups.
    fn cup_place(&self) -> Club {
        let winner = self.season.cup_winner;
        if winner != 0 {
            for group in 1..=self.groups {
                for at in 0..self.counts[group] {
                    if self.rows[group][at].team == winner {
                        let kept = at >= CUP_PLACE && !self.cup_out(group, at);
                        return group * TIER_CLUBS + if kept { at } else { CUP_PLACE };
                    }
                }
            }
        }
        let elevenths: Vec<Club> = (0..self.groups)
            .map(|group| (group + 1) * TIER_CLUBS + CUP_PLACE)
            .collect();
        self.best_of(&elevenths)
    }

    /// The lower tier's play-off (upstream `serie_c`).
    fn lower(&mut self) -> Result<(), RulesError> {
        let groups = self.groups;
        if !(2..=4).contains(&groups) {
            return Ok(());
        }
        if (1..=groups).any(|group| self.counts[group] < GROUP_CLUBS) {
            return Ok(());
        }
        let (mut qualified, mut thirds, mut seconds) = (Vec::new(), Vec::new(), Vec::new());
        for group in 1..=groups {
            let base = group * TIER_CLUBS;
            let mut round = [base + FOURTH, 0, 0, 0];
            for k in 0..3 {
                round[1 + k] = self.tie(base + FIFTH + k, base + 9 - k, 1, Some(base + FIFTH + k));
            }
            self.sort_better(&mut round);
            qualified.push(self.tie(round[0], round[3], 1, Some(round[0])));
            qualified.push(self.tie(round[1], round[2], 1, Some(round[1])));
            thirds.push(base + THIRD);
            seconds.push(base + 1);
        }
        let cup = self.cup_place();
        let best = self.best_of(&qualified);
        let mut seeds = thirds;
        seeds.extend([cup, best]);
        let mut others: Vec<Club> = qualified.into_iter().filter(|&club| club != best).collect();
        if seeds.len() > NATIONAL_SEEDS || others.len() > NATIONAL_SEEDS {
            // Upstream writes past its tables of 5 (four groups), then stops.
            return Err(RulesError::GroupsPastBracket(
                u8::try_from(groups).expect("4 groups"),
            ));
        }
        if seeds.len() != others.len() {
            return Ok(());
        }
        let winners = self.seeded(&seeds, &mut others);
        let best = self.best_of(&winners);
        let mut seeds = seconds;
        seeds.push(best);
        let mut others: Vec<Club> = winners.into_iter().filter(|&club| club != best).collect();
        if seeds.len() != others.len() {
            return Ok(());
        }
        let mut four = self.seeded(&seeds, &mut others);
        if four.len() != FINAL_FOUR {
            return Ok(());
        }
        self.shuffle(&mut four);
        let first = self.tie(four[1], four[0], TWO_LEGS, None);
        let second = self.tie(four[3], four[2], TWO_LEGS, None);
        let mut winner = self.tie(second, first, TWO_LEGS, None);
        let played = self.season.lower_play_off;
        if played != 0 {
            for group in 1..=groups {
                for at in 0..self.counts[group] {
                    if self.rows[group][at].team == played {
                        winner = group * TIER_CLUBS + at;
                    }
                }
            }
        }
        self.record(winner, -1);
        Ok(())
    }
}

/// The fixed tiers' brackets of `league` over `tables` (upstream `sr_italy`, dplay.c
/// `dp_season`): none without them; the upper tier's play-off and play-out, then the lower
/// tier's play-off, each club's move (rule [`MoveRule::FixedTiers`]). The generator is seeded
/// from the tables, so a save gives the same results; each club's lot comes first.
///
/// # Errors
///
/// [`RulesError::Check`] when the league's rules are not sound (upstream's refusal, nothing
/// played); [`RulesError::GroupsPastBracket`] when a lower tier of four groups reaches its
/// national rounds (upstream writes past its tables).
pub fn fixed_tier_brackets(
    tables: &Tables,
    league: &League,
    season: &FixedTiersSeason,
) -> Result<Vec<Move>, RulesError> {
    let Some(tiers) = league.fixed_tiers else {
        return Ok(Vec::new());
    };
    league_check(league).map_err(RulesError::Check)?;
    let mut brackets = Brackets {
        counts: [0; TIERS],
        rows: [[Row::default(); TIER_CLUBS]; TIERS],
        state: 0,
        lots: [0; TIERS * TIER_CLUBS],
        groups: usize::from(tiers.lower_groups),
        season,
        moves: Vec::new(),
    };
    for tier_index in 0..TIERS {
        let source = if tier_index == 0 {
            usize::from(tiers.upper)
        } else {
            usize::from(tiers.lower_first) + tier_index - 1
        };
        if tier_index > usize::from(tiers.lower_groups) || source >= usize::from(league.divisions) {
            continue;
        }
        let clubs = tables.count(source).min(TIER_CLUBS);
        brackets.counts[tier_index] = clubs;
        brackets.rows[tier_index][..clubs].copy_from_slice(&tables.table(source)[..clubs]);
    }
    let mut seed: u32 = 0;
    for tier_index in 0..TIERS {
        for row in &brackets.rows[tier_index][..brackets.counts[tier_index]] {
            seed = seed
                .wrapping_mul(SEED_MULTIPLIER)
                .wrapping_add(u32::from(row.glob));
            seed = seed
                .wrapping_mul(SEED_MULTIPLIER)
                .wrapping_add(u32::from(row.points) + (u32::from(row.goals_for) << GOALS_SHIFT));
        }
    }
    brackets.state = seed;
    for index in 0..brackets.lots.len() {
        brackets.lots[index] = u8::try_from(brackets.below(LOT_DRAW)).expect("a byte");
    }
    brackets.upper();
    brackets.lower()?;
    Ok(brackets.moves)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::rules_library::league::FixedTiers;

    /// A division of `count` clubs: global numbers and team words from `first`, points falling
    /// by `step` from `top`.
    fn division(first: u16, count: u16, top: u16, step: u16) -> Vec<Row> {
        (0..count)
            .map(|place| Row {
                team: first + place,
                glob: first + place,
                won: 0,
                goals_for: 0,
                goals_against: 0,
                points: top.saturating_sub(place * step),
            })
            .collect()
    }

    /// A league of an upper tier of 20 and three groups of 20, at divisions 1 to 4.
    fn league() -> League {
        League {
            divisions: 5,
            promoted: vec![0; 5],
            relegated: vec![0; 5],
            fixed_tiers: Some(FixedTiers {
                upper: 1,
                lower_first: 2,
                lower_groups: 3,
            }),
            ..League::default()
        }
    }

    /// The tables: the upper tier's points `upper_step` apart, the groups' 2 apart.
    fn tables(upper_step: u16) -> Tables {
        Tables::of(&[
            division(100, 20, 90, 3),
            division(200, 20, 300, upper_step),
            division(300, 20, 80, 2),
            division(400, 20, 80, 2),
            division(500, 20, 80, 2),
        ])
        .expect("tables")
    }

    #[test]
    fn a_runaway_third_goes_up_and_a_lost_seventeenth_down() {
        // 15 points between places: the 3rd leads the 4th by 15, the 16th the 17th by 15.
        let moves = fixed_tier_brackets(&tables(15), &league(), &FixedTiersSeason::default())
            .expect("played");
        assert_eq!(moves[0].glob, 202);
        assert_eq!((moves[0].delta, moves[0].rule), (-1, MoveRule::FixedTiers));
        assert_eq!((moves[1].glob, moves[1].delta), (216, 1));
        // The lower tier's play-off sends one club of the groups up.
        assert_eq!(moves.len(), 3);
        assert!((300..520).contains(&moves[2].glob) && moves[2].delta == -1);
    }

    #[test]
    fn a_held_play_off_s_real_winner_is_kept() {
        let season = FixedTiersSeason {
            upper_play_off: 207,
            upper_play_out: 216,
            ..FixedTiersSeason::default()
        };
        // 1 point between places: both held.
        let moves = fixed_tier_brackets(&tables(1), &league(), &season).expect("played");
        assert_eq!((moves[0].glob, moves[0].delta), (207, -1));
        // The 17th won its play-out: the 16th goes down.
        assert_eq!((moves[1].glob, moves[1].delta), (215, 1));
    }

    #[test]
    fn the_same_tables_give_the_same_results() {
        let season = FixedTiersSeason::default();
        assert_eq!(
            fixed_tier_brackets(&tables(1), &league(), &season),
            fixed_tier_brackets(&tables(1), &league(), &season)
        );
        assert_eq!(
            fixed_tier_brackets(&tables(1), &League::default(), &season),
            Ok(Vec::new())
        );
    }

    #[test]
    fn a_play_out_place_is_played_under_8_points_apart() {
        let season = FixedTiersSeason::default();
        let mut brackets = Brackets {
            counts: [0, 18, 0, 0, 0],
            rows: [[Row::default(); TIER_CLUBS]; TIERS],
            state: 0,
            lots: [0; TIERS * TIER_CLUBS],
            groups: 1,
            season: &season,
            moves: Vec::new(),
        };
        // The play-out places of 18: the 14th v the 17th, the 15th v the 16th (n - 5 v n - 2,
        // n - 4 v n - 3).
        let points = |gap: u16| {
            let mut points = [40u16; 18];
            points[16] = 40 - gap;
            points[15] = 40 - gap;
            points
        };
        for (gap, out) in [(7, true), (8, false), (9, false)] {
            for (place, &points) in points(gap).iter().enumerate() {
                brackets.rows[1][place].points = points;
            }
            assert_eq!(brackets.cup_out(1, 13), out, "the 14th, {gap} apart");
            assert_eq!(brackets.cup_out(1, 14), out, "the 15th, {gap} apart");
        }
        assert!(brackets.cup_out(1, 15), "the 16th: in the play-out or down");
        assert!(!brackets.cup_out(1, 12), "the 13th: safe");
    }

    #[test]
    fn what_cannot_be_played_is_refused() {
        let mut four = league();
        four.divisions = 6;
        four.fixed_tiers = Some(FixedTiers {
            upper: 1,
            lower_first: 2,
            lower_groups: 4,
        });
        let mut rows = vec![division(100, 20, 90, 3), division(200, 20, 90, 1)];
        rows.extend((3..7).map(|tier| division(tier * 100, 20, 80, 2)));
        let tables = Tables::of(&rows).expect("tables");
        assert_eq!(
            fixed_tier_brackets(&tables, &four, &FixedTiersSeason::default()),
            Err(RulesError::GroupsPastBracket(4))
        );
        let mut narrow = league();
        narrow.divisions = 4;
        assert!(matches!(
            fixed_tier_brackets(&tables, &narrow, &FixedTiersSeason::default()),
            Err(RulesError::Check(_))
        ));
        assert!(play_out_held(20, 16) && !play_out_held(21, 16));
        assert!(play_off_held(30, 16) && !play_off_held(31, 16));
    }
}
