//! A simulated result (docs/PLAN.md M5 #8c; upstream src/match_sim.c `CalculateViewResult`,
//! `ExtraTimeSecondPhaseScoring`, `PenaltyShootoutScorerPair`, `AssignSimulatedGoalsToScorers`,
//! `ScaleViewIndexByTeamPriceRank`). Before the goals, each side's stand-ins are picked, its
//! players' status buckets, cards and injuries drawn; then each side's strength (its rank by
//! price moving it, unless the computer plays both), its form of the day added, and the
//! difference, the home side 2 up, draws the first side's goals and the other side's; each goal
//! a scorer. Extra time draws from the second half's forms; a shoot-out from its own table. A
//! world's computer clubs field available players for a result, and its keepers may be injured
//! after one (`WorldRules_CpuLineupsBegin`, `WorldRules_CpuLineupsEnd`, `WorldRules_GkAfterSim`,
//! the SWOS 96/97 Mod's rules in another world): with their components (M5 #9). The career
//! club's players' form after a match (`Career_ApplyPostMatchPlayerStatModsIfCareerTeam`) is the
//! career's (M7).

use openswos_rng::Rng;
use openswos_tac_resource::TeamTactics;

use super::stand_ins::{StandIns, stand_ins};
use super::standing::{Day, Form, form, price_rank};
use super::status::{SimRules, after_match, status_buckets};
use super::tables::{
    ELEVEN, EXTRA_GOAL_ODDS, EXTRA_TIME_MORE_ODDS, EXTRA_TIME_ODDS, GOAL_ODDS, GOALS_BY_FORM,
    GOALS_BY_STRENGTH, HIGH_GOAL_DRAWS, HIGH_GOALS, LINE_UP_ORDERS, SCORER_ODDS, SHOOT_OUT_RESULTS,
};
use super::team::{CAREER_LINE_UP, MatchTeam};
use super::value::{SimError, strength};
use crate::competition::results::{MatchSetup, Venue};
use crate::competition::running::{RunningCompetition, TeamBytes};
use crate::competition::start::LoadedTeam;
use crate::competition::table::Score;

/// The controls of a team the computer plays (`TEAM_COMPUTER`).
const COMPUTER: u8 = 1;

/// A percentage's whole.
const PERCENT: u32 = 100;

/// The highest strength a side's goals are drawn by (a form's too).
const MAX_STRENGTH: i16 = 31;

/// What the home side's strength is raised by on its own ground.
const HOME_ADVANTAGE: i16 = 2;

/// The largest difference of strengths the goals are drawn by.
const MAX_DIFFERENCE: i16 = 23;

/// The bits of a draw a goals' table's draw is picked by.
const GOAL_DRAW: u16 = 0x03;

/// The draw a goals' table's draw is picked by the more goals' odds.
const ODDS_COUNTED: usize = 4;

/// The goals more the still more goals' odds start from.
const STILL_MORE_FROM: i16 = 3;

/// The lowest draw that scores an own goal.
const OWN_GOAL_DRAW: u16 = 0xF8;

/// The bits of a draw a scorer is picked by (of 128).
const SCORER_DRAW: u16 = 0x7F;

/// The eleven's last place, from which the scorers' odds count down.
const LAST_OF_ELEVEN: i16 = 10;

/// The places the scorers' first draw picks among.
const FIRST_DRAW_PLACES: usize = 11;

/// The place of the scorers' second draw's first odds.
const SECOND_DRAW_PLACE: i16 = 1;

/// The bits of a draw a shoot-out's result is picked by.
const SHOOT_OUT_DRAW: u16 = 0x3F;

/// The bit of a draw that turns a shoot-out's result round.
const SHOOT_OUT_TURN: u16 = 0x80;

/// How far a shoot-out's home side's penalties are shifted in their byte.
const NIBBLE_SHIFT: u8 = 4;

/// A nibble.
const NIBBLE: u8 = 0x0F;

/// What a simulation takes besides its match: the session's tactics and its formations'
/// orders, its rules, the career's answers, the day.
#[derive(Debug, Clone, Copy)]
pub struct Session<'a> {
    /// The session's tactics (upstream `_teamTacticsTable`), by a team's tactics byte.
    pub tactics: &'a [TeamTactics],
    /// The formations' defensive orders (upstream `_playersPositionsByTactics`:
    /// [`formation_orders`](super::formation_orders) of the tactics).
    pub orders: &'a [[u8; ELEVEN]],
    /// The rules of the players' statuses (their mode replaced by the competition's track).
    pub rules: SimRules,
    /// Whether a match's statuses apply (upstream `ShouldApplyRandomPlayerStatusAfterSim`: the
    /// career's competitions, M7).
    pub statuses_apply: bool,
    /// Whether the career's manager selects a national team (`_isNationalSelector`).
    pub national_selector: bool,
    /// The day the match is played on.
    pub day: Day,
}

/// What a simulated result leaves for its extra time (upstream's module state): each side's
/// stand-ins, strength, form, goals so far, the goals' column, each record's goals.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Simulated {
    /// Each side's stand-ins.
    pub stand_ins: [StandIns; 2],
    /// Each side's strength (its view's base).
    pub strengths: [i16; 2],
    /// Each side's form.
    pub forms: [Form; 2],
    /// Each side's goals so far.
    pub goals: [u16; 2],
    /// The goals' column.
    pub column: u16,
    /// Each side's goals by record (`_team1GoalCounterByPlayer`, `_team2GoalCounterByPlayer`).
    pub scorers: [[u8; CAREER_LINE_UP]; 2],
}

/// `strength` moved by `rank` percent (upstream `ScaleViewIndexByTeamPriceRank`), 0 to 31.
/// Faithful to the original: below 0 as a signed word, 0; above 31 as a long, 31.
fn scaled(strength: u16, rank: i16) -> u16 {
    let percent = i16::try_from(PERCENT).expect("100");
    let factor = u32::from(rank.wrapping_add(percent).cast_unsigned());
    let full = u32::from(strength) * factor / PERCENT;
    let [low, high, _, _] = full.to_le_bytes();
    if i16::from_le_bytes([low, high]) < 0 {
        0
    } else if full > u32::from(MAX_STRENGTH.cast_unsigned()) {
        MAX_STRENGTH.cast_unsigned()
    } else {
        u16::from_le_bytes([low, high])
    }
}

/// The goal more `odds` count for a draw (upstream's walk of its thresholds): how many of the
/// first `counted` the draw passes, each taken off it.
fn passed(odds: &[u8], counted: usize, draw: &mut u16) -> i16 {
    let mut more = 0;
    for &odds in &odds[..counted] {
        let odds = u16::from(odds);
        if *draw < odds {
            break;
        }
        *draw -= odds;
        more += 1;
    }
    more
}

/// A side's goals and the other's from the strengths' difference (upstream
/// `CalculateViewResult`'s draws): the first side's from the difference's row (a high draw from
/// its own table), the other's by the odds of that count.
fn goals(difference: i16, rng: &mut Rng) -> (i16, i16) {
    let row = usize::from(difference.unsigned_abs());
    let mut first =
        i16::from(GOALS_BY_STRENGTH[row][usize::from(rng.rand2() & GOAL_DRAW)].cast_signed());
    if first < 0 {
        let draw = rng.rand2();
        let pick = usize::from((draw * u16::try_from(HIGH_GOAL_DRAWS).expect("6")) >> u8::BITS);
        first = i16::from(HIGH_GOALS[pick]);
    }
    let count = usize::try_from(first).expect("a count of goals");
    let mut draw = rng.rand2();
    let mut second = passed(&GOAL_ODDS[count], ODDS_COUNTED, &mut draw);
    if second == i16::try_from(ODDS_COUNTED).expect("4") {
        let mut draw = rng.rand2();
        second = STILL_MORE_FROM + passed(&EXTRA_GOAL_ODDS[count], 2, &mut draw);
    }
    (first + second, second)
}

/// Each of `count` goals of `team` given a scorer (upstream `AssignSimulatedGoalsToScorers`):
/// a high draw an own goal; else a place of the eleven by the ball-out-of-play tactics' odds,
/// its player or, a draw in two, his stand-in, his goals counted in `scorers` and in his
/// record's `column`.
fn assign_scorers(
    team: &mut MatchTeam,
    stand_ins: &[u8; CAREER_LINE_UP],
    scorers: &mut [u8; CAREER_LINE_UP],
    (count, column): (u16, u16),
    tactics: &[TeamTactics],
    rng: &mut Rng,
) -> Result<(), SimError> {
    let goals = usize::try_from(count.cast_signed()).unwrap_or_default();
    if goals == 0 {
        return Ok(());
    }
    let index = team.tactics().cast_signed();
    let team_tactics = usize::try_from(index)
        .ok()
        .and_then(|index| tactics.get(index))
        .ok_or(SimError::TacticsPastTable(index))?;
    let ball_out = usize::from(u8::from(team_tactics.ball_out_of_play));
    let (order, odds) = (&LINE_UP_ORDERS[ball_out], &SCORER_ODDS[ball_out]);
    for _ in 0..goals {
        let mut draw = rng.rand2();
        if draw >= OWN_GOAL_DRAW {
            team.set_own_goals(team.own_goals().wrapping_add(1));
            continue;
        }
        draw &= SCORER_DRAW;
        let mut place = LAST_OF_ELEVEN;
        let found = odds[..FIRST_DRAW_PLACES].iter().any(|&odds| {
            let odds = u16::from(odds);
            if draw < odds {
                return true;
            }
            draw -= odds;
            place -= 1;
            false
        });
        if !found {
            let draw = rng.rand2() & SCORER_DRAW;
            place = if draw < u16::from(odds[FIRST_DRAW_PLACES]) {
                SECOND_DRAW_PLACE
            } else {
                SECOND_DRAW_PLACE - 1
            };
        }
        let at = order[usize::try_from(place).expect("a place of the eleven")].wrapping_sub(1);
        let mut slot = team
            .line_up(usize::from(at))
            .ok_or(SimError::LineUpPastOrder(usize::from(at)))?
            .cast_unsigned();
        let stand_in = *stand_ins
            .get(usize::from(slot))
            .ok_or(SimError::StandInPastTable(slot.cast_signed()))?;
        if stand_in.cast_signed() >= 0 && rng.rand2() & 1 == 0 {
            slot = stand_in;
        }
        let record = usize::from(slot);
        if record >= team.records() {
            return Err(SimError::PlayerPastTeam(slot.cast_signed()));
        }
        let goals_of = scorers
            .get_mut(record)
            .ok_or(SimError::StandInPastTable(slot.cast_signed()))?;
        *goals_of = goals_of.wrapping_add(1);
        if !team.has_goals(record, usize::from(column)) {
            return Err(SimError::ColumnPastTeam(column));
        }
        let goals = team.goals(record, usize::from(column));
        team.set_goals(record, usize::from(column), goals.wrapping_add(1));
    }
    Ok(())
}

/// A strength and a form, as a view's index: 0 to 31.
fn clamped(value: i16) -> i16 {
    value.clamp(0, MAX_STRENGTH)
}

/// The simulated result of `setup` between `home` and `away` in `competition` (upstream
/// `CalculateViewResult`), drawn by `rng`: each side's stand-ins ([`stand_ins`]), its players'
/// status buckets ([`status_buckets`]), cards and injuries ([`after_match`]); each side's
/// strength ([`strength`], the bench its most substitutes) moved by its rank by price
/// ([`price_rank`], its teams found by `teams`) unless the computer plays both, its form
/// ([`form`]) added; the difference, the home side 2 up on its own ground, at most 23 either
/// way, draws the goals; each goal a scorer. The players' statuses are checked in the
/// competition's track (its `substitutesCount`).
///
/// # Errors
///
/// [`SimError`] when a side's stand-ins, statuses, strength, rank or scorers cannot be drawn
/// (upstream reads past its records, tables or memory, or divides by 0).
pub fn simulate(
    competition: &RunningCompetition,
    setup: &MatchSetup,
    [home, away]: [&mut MatchTeam; 2],
    teams: impl Fn(TeamBytes) -> Option<LoadedTeam>,
    session: &Session<'_>,
    rng: &mut Rng,
) -> Result<(Score, Simulated), SimError> {
    let mut rules = session.rules;
    rules.status.mode = competition.substitutes_count();
    let mode = rules.status.mode;
    let home_stand_ins = stand_ins(home, setup.substitutes, session.orders, mode, rng)?;
    let away_stand_ins = stand_ins(away, setup.substitutes, session.orders, mode, rng)?;
    status_buckets(home, Some(&home_stand_ins.table), &rules, rng)?;
    status_buckets(away, Some(&away_stand_ins.table), &rules, rng)?;
    after_match(
        home,
        &home_stand_ins.table,
        &rules,
        session.statuses_apply,
        rng,
    )?;
    after_match(
        away,
        &away_stand_ins.table,
        &rules,
        session.statuses_apply,
        rng,
    )?;
    let (_, bench) = setup.substitutes;
    let mut home_strength = strength(home, bench, session.tactics, &rules.status)?;
    let mut away_strength = strength(away, bench, session.tactics, &rules.status)?;
    if !(home.controls() == COMPUTER && away.controls() == COMPUTER) {
        let home_rank = price_rank(competition, home, &teams, session.national_selector)?;
        home_strength = scaled(home_strength, home_rank);
        let away_rank = price_rank(competition, away, &teams, session.national_selector)?;
        away_strength = scaled(away_strength, away_rank);
    }
    let forms = [form(home, &session.day), form(away, &session.day)];
    let strengths = [home_strength.cast_signed(), away_strength.cast_signed()];
    // Faithful to the original: signed words, wrapping.
    let home_view = strengths[0].wrapping_add(forms[0].first);
    let away_view = strengths[1].wrapping_add(forms[1].first);
    let mut difference = home_view.wrapping_sub(away_view);
    if setup.venue == Venue::Home {
        difference = difference.wrapping_add(HOME_ADVANTAGE);
    }
    let difference = difference.clamp(-MAX_DIFFERENCE, MAX_DIFFERENCE);
    let (mut first, mut second) = goals(difference, rng);
    if difference < 0 {
        std::mem::swap(&mut first, &mut second);
    }
    let goals = [first.cast_unsigned(), second.cast_unsigned()];
    let mut scorers = [[0; CAREER_LINE_UP]; 2];
    let [home_scorers, away_scorers] = &mut scorers;
    let column = setup.column;
    assign_scorers(
        home,
        &home_stand_ins.table,
        home_scorers,
        (goals[0], column),
        session.tactics,
        rng,
    )?;
    assign_scorers(
        away,
        &away_stand_ins.table,
        away_scorers,
        (goals[1], column),
        session.tactics,
        rng,
    )?;
    Ok((
        Score {
            home: goals[0],
            away: goals[1],
        },
        Simulated {
            stand_ins: [home_stand_ins, away_stand_ins],
            strengths,
            forms,
            goals,
            column,
            scorers,
        },
    ))
}

/// Extra time's goals after `simulated` between `home` and `away` (upstream
/// `ExtraTimeSecondPhaseScoring`), drawn by `rng`: the second half's forms on the strengths,
/// their difference's row draws the stronger side's goals, more by their odds; each goal a
/// scorer, the goals so far raised.
///
/// # Errors
///
/// [`SimError`] when a scorer cannot be drawn.
pub fn extra_time(
    simulated: &mut Simulated,
    [home, away]: [&mut MatchTeam; 2],
    tactics: &[TeamTactics],
    rng: &mut Rng,
) -> Result<Score, SimError> {
    let home_view = clamped(simulated.strengths[0].wrapping_add(simulated.forms[0].second));
    let away_view = clamped(simulated.strengths[1].wrapping_add(simulated.forms[1].second));
    let row = usize::from((home_view - away_view).unsigned_abs());
    let mut stronger =
        i16::from(GOALS_BY_FORM[row][usize::from(rng.rand2() & GOAL_DRAW)].cast_signed());
    let mut draw = rng.rand2();
    let mut weaker = passed(&EXTRA_TIME_ODDS, 2, &mut draw);
    if weaker == 2 {
        let mut draw = rng.rand2();
        weaker = 1 + passed(&EXTRA_TIME_MORE_ODDS, 1, &mut draw);
    }
    stronger += weaker;
    if home_view <= away_view {
        std::mem::swap(&mut stronger, &mut weaker);
    }
    let goals = [stronger.cast_unsigned(), weaker.cast_unsigned()];
    for (side, &more) in goals.iter().enumerate() {
        simulated.goals[side] = simulated.goals[side].wrapping_add(more);
    }
    let column = simulated.column;
    let [home_scorers, away_scorers] = &mut simulated.scorers;
    let [home_stand_ins, away_stand_ins] = &simulated.stand_ins;
    assign_scorers(
        home,
        &home_stand_ins.table,
        home_scorers,
        (goals[0], column),
        tactics,
        rng,
    )?;
    assign_scorers(
        away,
        &away_stand_ins.table,
        away_scorers,
        (goals[1], column),
        tactics,
        rng,
    )?;
    Ok(Score {
        home: goals[0],
        away: goals[1],
    })
}

/// A shoot-out's penalties (upstream `PenaltyShootoutScorerPair`), drawn by `rng`: a result of
/// its table, turned round on a draw's high bit, a nibble each side.
#[must_use]
pub fn penalties(rng: &mut Rng) -> Score {
    let draw = rng.rand2();
    let mut result = SHOOT_OUT_RESULTS[usize::from(draw & SHOOT_OUT_DRAW)];
    if draw & SHOOT_OUT_TURN != 0 {
        result = result.rotate_left(u32::from(NIBBLE_SHIFT));
    }
    Score {
        home: u16::from(result >> NIBBLE_SHIFT),
        away: u16::from(result & NIBBLE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rank_moves_a_strength_within_31() {
        assert_eq!(scaled(20, 25), 25);
        assert_eq!(scaled(20, -25), 15);
        assert_eq!(scaled(30, 25), 31);
        // Past a word, its low half small: still past 31.
        assert_eq!(scaled(52_429, 25), 31);
        // A negative word: 0.
        assert_eq!(scaled(40_000, 0), 0);
    }

    #[test]
    fn a_shoot_out_is_its_table_s_turned_round_on_the_high_bit() {
        let mut rng = Rng::default();
        rng.fixed = Some(0);
        let [first, ..] = SHOOT_OUT_RESULTS;
        assert_eq!(
            penalties(&mut rng),
            Score {
                home: u16::from(first >> 4),
                away: u16::from(first & 0x0F)
            }
        );
        rng.fixed = Some(0x80);
        assert_eq!(
            penalties(&mut rng),
            Score {
                home: u16::from(first & 0x0F),
                away: u16::from(first >> 4)
            }
        );
    }

    #[test]
    fn the_more_goals_odds_count_what_a_draw_passes() {
        let mut draw = 10;
        assert_eq!(passed(&[4, 4, 4, 4, 0], 4, &mut draw), 2);
        assert_eq!(draw, 2);
    }
}
