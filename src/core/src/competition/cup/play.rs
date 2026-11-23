//! A cup played (docs/PLAN.md M5 #6): a tie decided from its match's results as they are
//! needed (normal time, extra time, penalties; the first leg's goals, the away goals), a
//! match's result kept in the bracket, the round's and the cup's end, then the next round
//! arranged: the winners into the next round's draw, a second leg, the drawn ties' replays
//! (upstream diy_competition.c `ProcessDIYKnockoutMatchResultAndCareerUpdates`, the
//! bookkeeping of `DIY_PlayCupMatchAndAdvance`, `DIY_RecountMissingTeamsInTeamGrid`,
//! `DIY_CountUnselectedTeams`, `DIY_UpdateCompetitionProgress`,
//! `DIY_ApplyHomeAwaySwapFlagsToTeamGridPairs`, `DIY_ReorderTeamGridByOrderList`,
//! `DIY_ReorderTeamsBySelectionFlags`). The match itself is played or simulated elsewhere (M5
//! #8, M6); the career records the bracket's progress into its calendar
//! (`DIY_RecordCupBracketProgress`) with M7.
//!
//! A world's play-offs, national cups, Serie D cup, Super Cups, UEFA qualifying and continental
//! ties decide level ties, follow decided ones and arrange their next rounds their own way
//! (`WorldPlayoff_*`, `WorldCups_*`, `CupD_*`, `SuperCup_TieDecided`, `UefaQual_TieDecided`,
//! `Conti_TieDecided`, `UefaPrize_CupWon`): they join with World 2026-27's components (M5 #9),
//! and a classic world's European cups (`WorldEurope_RoundPlayed`) with Europe 1984-85 (D10).

use openswos_rng::Rng;
use openswos_team_resource::TeamControls;
use thiserror::Error;

use crate::competition::draw::shuffled_order;
use crate::competition::league::keep_progress;
use crate::competition::running::{CUP_ROUND_PENALTIES, IMAGE_LEN, RunningCompetition, TeamBytes};
use crate::competition::start::LoadedTeam;
use crate::competition::table::Score;
use crate::competition::tournament::{STAGES, TournamentState};

/// The legs code of a round played home and away (`roundLegs`, `roundFormat`).
const TWO_LEGS: u16 = 2;

/// A rule's code for "on" (extra time, penalties): 1. A code of 2 is on only in a replay.
const ON: u16 = 1;

/// The away goals' rule (`playEachTeam`, a cup's): 1, away goals count after normal time; 0,
/// never.
const AWAY_GOALS_AFTER_NORMAL_TIME: u16 = 1;

/// The ties the grid holds (`teamGridPairings[64]`: two places a tie).
const MAX_TIES: usize = 32;

/// The round results the next round clears (upstream clears 32 bytes of `cupRoundResults`).
const CLEARED_RESULTS: usize = 32;

/// The places of the shuffle order upstream searches (`teamShuffleOrder[698]`).
const ORDER_PLACES: usize = 698;

/// The offset of a cup bracket's months (bracket scratch `progTable`).
const PROG_TABLE: usize = 832;

/// The identity of the career club in the bracket.
const CAREER_CLUB: i32 = -1;

/// The teams' identities the bracket holds (`teamOffsetTable[64]`).
const BRACKET_TEAMS: usize = 64;

/// What a match's result is made of, each asked for only when the tie needs it, in upstream's
/// order: normal time, then extra time, then penalties (upstream `getCachedOrCalcViewResult`,
/// `getCachedOrCalcExtraTimeResult`, `getCachedOrCalcPenaltyResult`). A simulated match draws
/// each from the generators (M5 #8), a played one is the match engine's (M6).
pub trait MatchResults {
    /// The goals of normal time.
    fn normal_time(&mut self, fixture: CupFixture, venue: Venue) -> Score;
    /// The goals of extra time alone.
    fn extra_time(&mut self, fixture: CupFixture) -> Score;
    /// The shoot-out's penalties.
    fn penalties(&mut self, fixture: CupFixture) -> Score;
}

/// Where a match is played (upstream's mode flag: 0 the home side's ground, 0xFF neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Venue {
    /// The home side's ground: its advantage counts.
    Home,
    /// Neutral: a single-leg final's.
    Neutral,
}

/// A match of the cup: its places in the grid and its sides' identities, the home side first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CupFixture {
    /// The home side's place in the bracket (`teamGridPairings`' entry).
    pub home_slot: usize,
    /// The away side's place.
    pub away_slot: usize,
    /// The home side's identity: -1 the career club, otherwise its index among the selected
    /// teams.
    pub home: i32,
    /// The away side's identity.
    pub away: i32,
}

/// A match's result's flags (upstream's `cupRoundResults` byte): bit 0 the away side won,
/// bit 1 decided, bit 2 extra time, bit 3 penalties, bit 4 away goals, bit 5 a return leg,
/// bit 6 a replay, bit 7 played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResultFlags(pub u8);

impl ResultFlags {
    /// The away side won.
    pub const AWAY_WON: u8 = 0x01;
    /// The tie is decided.
    pub const DECIDED: u8 = 0x02;
    /// It went to extra time.
    pub const EXTRA_TIME: u8 = 0x04;
    /// It went to penalties.
    pub const PENALTIES: u8 = 0x08;
    /// Away goals decided it.
    pub const AWAY_GOALS: u8 = 0x10;
    /// A return leg.
    pub const RETURN_LEG: u8 = 0x20;
    /// A replay.
    pub const REPLAY: u8 = 0x40;
    /// Played.
    pub const PLAYED: u8 = 0x80;
}

/// A match's result as the bracket keeps it (upstream's packed words): each pair of goals the
/// home side's in the high byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Decision {
    /// The match's goals, extra time's among them.
    pub goals: u16,
    /// The tie's aggregate (a return leg's; else the match's goals, or the first leg's).
    pub aggregate: u16,
    /// The shoot-out's penalties (else the first leg's goals).
    pub penalties: u16,
    /// Its flags.
    pub flags: ResultFlags,
}

/// Why a cup cannot be played on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CupPlayError {
    /// A round whose rules are past the competition.
    #[error("round {0}'s rules are past the competition")]
    RoundPastRules(i16),
    /// A round of no ties: a cup played past its final, its rounds more than its bracket's
    /// (upstream plays on into the grid's other places, and past it).
    #[error("a round of no ties")]
    NoTies,
    /// More ties than the grid holds (upstream reads past it).
    #[error("{0} ties, more than the grid's 32")]
    TiesPastGrid(i16),
    /// A grid's place naming no team of the bracket (upstream reads past it).
    #[error("place {0} names no team of the bracket")]
    NotABracketTeam(i16),
    /// A stage past the tournament's six.
    #[error("stage {0} is past the tournament's")]
    StagePastTournament(u16),
    /// A place of the next round's draw the shuffle order does not hold (upstream reads past
    /// it).
    #[error("place {0} is not in the draw's order")]
    NotInOrder(u16),
    /// A round's next month past the competition (upstream reads past it).
    #[error("month {0} is past the competition")]
    MonthPastImage(usize),
    /// A team of the bracket that is not loaded.
    #[error("team {0:?} is not loaded")]
    UnknownTeam(TeamBytes),
}

/// The round's index, whose rules the tie reads (`currentRoundIndex`).
fn round_index(competition: &RunningCompetition) -> Result<usize, CupPlayError> {
    let round = competition.current_round_index();
    usize::try_from(round)
        .ok()
        .filter(|&round| CUP_ROUND_PENALTIES + 2 * (round + 1) <= IMAGE_LEN)
        .ok_or(CupPlayError::RoundPastRules(round))
}

/// The ties a count of upstream's walks, checked within the grid.
fn ties(count: i16) -> Result<usize, CupPlayError> {
    let ties = usize::try_from(count).unwrap_or_default();
    if ties > MAX_TIES {
        return Err(CupPlayError::TiesPastGrid(count));
    }
    Ok(ties)
}

/// The team identity of grid place `slot` (a signed byte of the grid).
fn bracket_team(competition: &RunningCompetition, slot: u8) -> Result<(usize, i32), CupPlayError> {
    let slot = i16::from(slot.cast_signed());
    let index = usize::try_from(slot)
        .ok()
        .filter(|&index| index < BRACKET_TEAMS)
        .ok_or(CupPlayError::NotABracketTeam(slot))?;
    Ok((index, competition.team_offset(index)))
}

/// The cup's current match (upstream `getCurrentMatchTeamSlotIndices`): the current tie's two
/// places, each its team's identity.
///
/// # Errors
///
/// [`CupPlayError`] when the tie is past the grid or a place names no team of the bracket.
pub fn cup_fixture(competition: &RunningCompetition) -> Result<CupFixture, CupPlayError> {
    let tie = usize::from(competition.cup_current_match_index());
    if tie >= MAX_TIES {
        return Err(CupPlayError::TiesPastGrid(
            competition.cup_current_match_index().cast_signed(),
        ));
    }
    let (home_slot, home) = bracket_team(competition, competition.grid_pairing(2 * tie))?;
    let (away_slot, away) = bracket_team(competition, competition.grid_pairing(2 * tie + 1))?;
    Ok(CupFixture {
        home_slot,
        away_slot,
        home,
        away,
    })
}

/// How a tie's decision ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    /// The home side wins it.
    HomeWins,
    /// The away side wins it.
    AwayWins,
    /// It is level and stays so: a draw, to be replayed.
    Draw,
}

/// A tie's decision as upstream keeps it while deciding (its registers): the match's goals
/// (`d1`, `d2`), the aggregate (`d3`, `d4`), the first leg's goals or the penalties (`d5`,
/// `d6`), the flags (`d7`).
#[derive(Debug, Clone, Copy, Default)]
struct Deciding {
    /// The match's goals, home and away.
    goals: (u16, u16),
    /// The aggregate.
    aggregate: (u16, u16),
    /// The first leg's goals, then the penalties.
    other: (u16, u16),
    /// The flags.
    flags: u8,
}

/// The rules a tie is decided by.
#[derive(Debug, Clone, Copy)]
struct Rules {
    /// A replay's (the replays' round).
    replay: bool,
    /// The round's extra time code.
    extra_time: u16,
    /// The round's penalties code.
    penalties: u16,
    /// The away goals' rule.
    away_goals: u16,
}

impl Rules {
    /// Whether a code of the round's rules is on: 1, or (2 and up) in a replay.
    fn on(self, code: u16) -> bool {
        code == ON || (code != 0 && self.replay)
    }
}

/// The shoot-out when the round's rules play one (into the penalties), else a draw.
fn shoot_out(
    state: &mut Deciding,
    rules: Rules,
    fixture: CupFixture,
    results: &mut impl MatchResults,
) -> Next {
    if !rules.on(rules.penalties) {
        return Next::Draw;
    }
    state.flags |= ResultFlags::PENALTIES;
    let penalties = results.penalties(fixture);
    state.other = (penalties.home, penalties.away);
    by_goals(state.other)
}

/// The tie of `fixture` decided from its match's `results` (upstream
/// `ProcessDIYKnockoutMatchResultAndCareerUpdates`), the first leg's goals `first_leg` (home
/// side's, away side's: a return leg's): a first leg played; a return leg on aggregate, then
/// away goals, extra time, away goals again, penalties; a single leg or a replay on its goals,
/// then extra time and penalties where the round's rules say so (a code of 2 on in a replay
/// only); else a draw. Faithful to the original: a side wins on the low bytes of its goals.
///
/// # Errors
///
/// [`CupPlayError::RoundPastRules`] when the round's rules are past the competition.
pub fn decide(
    competition: &RunningCompetition,
    fixture: CupFixture,
    first_leg: (u16, u16),
    results: &mut impl MatchResults,
) -> Result<Decision, CupPlayError> {
    let round = round_index(competition)?;
    let rules = Rules {
        replay: competition.group_section_flag() != 0,
        extra_time: competition.cup_round_extra_time(round),
        penalties: competition.cup_round_penalties(round),
        away_goals: competition.play_each_team().cast_unsigned(),
    };
    let mut state = Deciding {
        other: first_leg,
        ..Deciding::default()
    };
    let two_legs = !rules.replay && competition.round_format() == TWO_LEGS;
    let next = if two_legs && competition.competition_format() == 0 {
        first_of_two(&mut state, fixture, results)
    } else if two_legs {
        return_leg(&mut state, rules, fixture, results)
    } else {
        single_leg(
            &mut state,
            rules,
            competition.round_format(),
            fixture,
            results,
        )
    };
    match next {
        Next::Draw => {
            state.flags |= ResultFlags::PLAYED;
            state.flags &= !ResultFlags::DECIDED;
        }
        Next::HomeWins => {
            state.flags &= !ResultFlags::AWAY_WON;
            state.flags |= ResultFlags::PLAYED | ResultFlags::DECIDED;
        }
        Next::AwayWins => {
            state.flags |= ResultFlags::AWAY_WON | ResultFlags::PLAYED | ResultFlags::DECIDED;
        }
    }
    let pack = |(high, low): (u16, u16)| (high << 8).wrapping_add(low);
    Ok(Decision {
        goals: pack(state.goals),
        aggregate: pack(state.aggregate),
        penalties: pack(state.other),
        flags: ResultFlags(state.flags),
    })
}

/// The first of two legs: decided only by its goals.
fn first_of_two(
    state: &mut Deciding,
    fixture: CupFixture,
    results: &mut impl MatchResults,
) -> Next {
    let score = results.normal_time(fixture, Venue::Home);
    state.goals = (score.home, score.away);
    if score.home == score.away {
        Next::Draw
    } else {
        by_goals(state.goals)
    }
}

/// A return leg: the aggregate, then away goals after normal time, extra time (the
/// aggregate, away goals), penalties.
fn return_leg(
    state: &mut Deciding,
    rules: Rules,
    fixture: CupFixture,
    results: &mut impl MatchResults,
) -> Next {
    state.flags |= ResultFlags::RETURN_LEG;
    let first_leg = state.other;
    let score = results.normal_time(fixture, Venue::Home);
    state.goals = (score.home, score.away);
    let aggregate = |goals: (u16, u16)| {
        (
            goals.0.wrapping_add(first_leg.0),
            goals.1.wrapping_add(first_leg.1),
        )
    };
    // Decided: the aggregate gives way to the first leg's goals.
    let decided = |state: &mut Deciding, next: Next| {
        state.aggregate = first_leg;
        next
    };
    state.aggregate = aggregate(state.goals);
    if state.aggregate.0 != state.aggregate.1 {
        return decided(state, by_goals(state.aggregate));
    }
    // Away goals: the away side's goals here against the home side's in the first leg.
    let away = |state: &mut Deciding| {
        state.flags |= ResultFlags::AWAY_GOALS;
        if state.goals.1 > first_leg.0 {
            Next::AwayWins
        } else {
            Next::HomeWins
        }
    };
    if rules.away_goals == AWAY_GOALS_AFTER_NORMAL_TIME && state.goals.1 != first_leg.0 {
        let next = away(state);
        return decided(state, next);
    }
    if rules.extra_time != ON {
        return shoot_out(state, rules, fixture, results);
    }
    state.flags |= ResultFlags::EXTRA_TIME;
    let extra = results.extra_time(fixture);
    state.goals = (
        state.goals.0.wrapping_add(extra.home),
        state.goals.1.wrapping_add(extra.away),
    );
    state.aggregate = aggregate(state.goals);
    if state.aggregate.0 != state.aggregate.1 {
        return decided(state, by_goals(state.aggregate));
    }
    if rules.away_goals != 0 && state.goals.1 != first_leg.0 {
        let next = away(state);
        return decided(state, next);
    }
    state.aggregate = first_leg;
    shoot_out(state, rules, fixture, results)
}

/// A single leg or a replay: its goals, then extra time and penalties where the rules play
/// them. A final of one leg (format 0) is played on neutral ground.
fn single_leg(
    state: &mut Deciding,
    rules: Rules,
    format: u16,
    fixture: CupFixture,
    results: &mut impl MatchResults,
) -> Next {
    if rules.replay {
        state.flags |= ResultFlags::REPLAY;
    }
    state.flags &= !ResultFlags::RETURN_LEG;
    let venue = if format == 0 {
        Venue::Neutral
    } else {
        Venue::Home
    };
    let score = results.normal_time(fixture, venue);
    state.goals = (score.home, score.away);
    if score.home != score.away {
        return by_goals(state.goals);
    }
    state.aggregate = state.goals;
    if rules.on(rules.extra_time) {
        state.flags |= ResultFlags::EXTRA_TIME;
        let extra = results.extra_time(fixture);
        state.goals = (
            state.goals.0.wrapping_add(extra.home),
            state.goals.1.wrapping_add(extra.away),
        );
        if state.goals.0 != state.goals.1 {
            return by_goals(state.goals);
        }
    }
    shoot_out(state, rules, fixture, results)
}

/// The side whose goals' low byte is the greater: the home side on a level one.
fn by_goals((home, away): (u16, u16)) -> Next {
    if away.to_le_bytes()[0] > home.to_le_bytes()[0] {
        Next::AwayWins
    } else {
        Next::HomeWins
    }
}

/// The cup's next match played (the bookkeeping of upstream `DIY_PlayCupMatchAndAdvance`
/// around its match): the match begun, its tie decided ([`decide`], from `results`), the
/// result kept in the bracket, the career club's part noted (both kept in the tournament's
/// state for a stage), the match counted; at the round's end its next month, the eliminated
/// teams no computer controls counted off (by `teams`), the undecided ties counted, the next
/// round's draw made by `rng`; at the cup's end (or a stage's, `tournament` telling) its
/// winner. The career's own records of the match are M7's.
///
/// # Errors
///
/// [`CupPlayError`] when the round has no tie, the match cannot be found or decided, or the
/// bracket cannot be counted.
pub fn play_cup_match(
    competition: &mut RunningCompetition,
    tournament: &mut TournamentState,
    results: &mut impl MatchResults,
    teams: impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<CupFixture, CupPlayError> {
    if competition.total_matches_in_round() <= 0 {
        return Err(CupPlayError::NoTies);
    }
    let fixture = cup_fixture(competition)?;
    round_index(competition)?;
    competition.set_match_started(1);
    keep_progress(competition, tournament);
    let tie = usize::from(competition.cup_current_match_index());
    let first_leg = if competition.competition_format() == 0 {
        (0, 0)
    } else {
        let [low, high] = competition.cup_aggregate_score(tie).to_le_bytes();
        (u16::from(high), u16::from(low))
    };
    let decision = decide(competition, fixture, first_leg, results)?;
    competition.set_cup_round_result(tie, decision.flags.0);
    competition.set_cup_round_score(tie, decision.goals);
    if competition.competition_format() == 0 {
        competition.set_cup_aggregate_score(tie, decision.aggregate);
    }
    competition.set_cup_penalty_score(tie, decision.penalties);
    competition.set_match_count(competition.match_count().wrapping_add(1));
    if fixture.home == CAREER_CLUB || fixture.away == CAREER_CLUB {
        competition.set_career_team_involved(1);
    }
    keep_progress(competition, tournament);
    let next_tie = competition.cup_current_match_index().wrapping_add(1);
    competition.set_cup_current_match_index(next_tie);
    competition.set_round_complete(0);
    if next_tie == competition.total_matches_in_round().cast_unsigned() {
        competition.set_round_complete(1);
        competition.set_cup_current_match_index(0);
        // Faithful to the original: the next month's index is an int, past a word.
        let month = u32::from(competition.cur_month_idx()) + 1;
        if month != u32::from(competition.total_matches()) {
            let month = usize::try_from(month).expect("a few months");
            if PROG_TABLE + month >= IMAGE_LEN {
                return Err(CupPlayError::MonthPastImage(month));
            }
            competition.set_view_random_phase(competition.prog_table(month));
        }
    }
    if competition.round_complete() == 0 {
        return Ok(fixture);
    }
    if competition.round_format() == TWO_LEGS && competition.competition_format() != 1 {
        return Ok(fixture);
    }
    let last_round = u32::from(competition.current_round_index().cast_unsigned()) + 1
        == u32::from(competition.num_rounds().cast_unsigned());
    if competition.has_groups() == 0 && !last_round {
        recount(competition, tournament, &teams)?;
        count_undecided(competition)?;
        if competition.group_section_flag() == 0 {
            let order = shuffled_order(ties(competition.match_pairs_per_round())?, rng);
            for (place, index) in order.into_iter().enumerate() {
                competition.set_shuffle_order(place, index);
            }
        }
        return Ok(fixture);
    }
    recount(competition, tournament, &teams)?;
    if count_undecided(competition)? != 0 {
        return Ok(fixture);
    }
    competition.set_competition_finished(1);
    let last_stage = u32::from(tournament.round_complete().cast_unsigned()) + 1
        == u32::from(tournament.total_matches_in_round().cast_unsigned());
    if competition.has_groups() != 0 && !last_stage {
        return Ok(fixture);
    }
    let winner = if competition.cup_round_result(0) & ResultFlags::AWAY_WON != 0 {
        competition.grid_pairing(1)
    } else {
        competition.grid_pairing(0)
    };
    let (slot, _) = bracket_team(competition, winner)?;
    competition.set_result_code(i16::from_be_bytes(competition.cup_team(slot)));
    Ok(fixture)
}

/// The eliminated teams no computer controls counted off the round iterator (upstream
/// `DIY_RecountMissingTeamsInTeamGrid`): each decided tie's loser; a tournament's doubled
/// knockout only its first tie's two teams, and a stage whose next is doubled none.
fn recount(
    competition: &mut RunningCompetition,
    tournament: &TournamentState,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
) -> Result<(), CupPlayError> {
    let off = |competition: &mut RunningCompetition, place: u8| {
        let team = competition.cup_team(usize::from(place));
        let loaded = teams(team).ok_or(CupPlayError::UnknownTeam(team))?;
        if loaded.controls != TeamControls::Computer {
            competition.set_round_iterator(competition.round_iterator().wrapping_sub(1));
        }
        Ok(())
    };
    if competition.has_groups() != 0 {
        let stage = tournament.round_complete().cast_unsigned();
        if usize::from(stage) >= STAGES {
            return Err(CupPlayError::StagePastTournament(stage));
        }
        let stage = usize::from(stage);
        if tournament.slot_double_count(stage) != 0 {
            if competition.cup_round_result(0) & ResultFlags::DECIDED == 0 {
                return Ok(());
            }
            off(competition, competition.grid_pairing(0))?;
            return off(competition, competition.grid_pairing(1));
        }
        if tournament.slot_division_factor(stage) != 1
            && tournament.slot_double_count(stage + 1) != 0
        {
            return Ok(());
        }
    }
    for tie in 0..ties(competition.total_matches_in_round())? {
        let flags = competition.cup_round_result(tie);
        if flags & ResultFlags::DECIDED != 0 {
            let loser = if flags & ResultFlags::AWAY_WON != 0 {
                competition.grid_pairing(2 * tie)
            } else {
                competition.grid_pairing(2 * tie + 1)
            };
            off(competition, loser)?;
        }
    }
    Ok(())
}

/// The round's undecided ties, counted into the second-leg counter (upstream
/// `DIY_CountUnselectedTeams`): the ties to replay.
fn count_undecided(competition: &mut RunningCompetition) -> Result<u16, CupPlayError> {
    let count = (0..ties(competition.total_matches_in_round())?)
        .filter(|&tie| competition.cup_round_result(tie) & ResultFlags::DECIDED == 0)
        .count();
    let count = u16::try_from(count).expect("32 ties at most");
    competition.set_second_leg_counter(count);
    Ok(count)
}

/// The cup's next round arranged after a round (upstream `DIY_UpdateCompetitionProgress`):
/// after a first leg, the second legs (the sides the other way round, the first legs' goals
/// kept for the aggregate); after drawn ties, their replays first (the other way round);
/// otherwise the winners into the next round in the draw's order, the round and its months
/// moved on, its pairs halved, its format its legs'. Nothing once the cup is finished.
///
/// # Errors
///
/// [`CupPlayError`] when the round's ties are past the grid, its rules past the competition,
/// or the draw's order lacks a place.
pub fn next_round(competition: &mut RunningCompetition) -> Result<(), CupPlayError> {
    if competition.competition_finished() != 0 {
        return Ok(());
    }
    if competition.competition_format() != 1 && competition.round_format() == TWO_LEGS {
        return second_legs(competition);
    }
    if competition.second_leg_counter() != 0 {
        return replays(competition);
    }
    advance_winners(competition)?;
    if competition.round_format() != TWO_LEGS && competition.group_section_flag() == 0 {
        let round = round_index(competition)?;
        if competition.cup_round_penalties(round) != ON {
            competition.set_cur_month_idx(competition.cur_month_idx().wrapping_add(1));
        }
    }
    competition.set_cur_month_idx(competition.cur_month_idx().wrapping_add(1));
    competition.set_current_round_index(competition.current_round_index().wrapping_add(1));
    let half = (competition.match_pairs_per_round().cast_unsigned() >> 1).cast_signed();
    competition.set_match_pairs_per_round(half);
    competition.set_total_matches_in_round(half);
    competition.set_cup_current_match_index(0);
    let round = round_index(competition)?;
    let mut format = competition.cup_round_legs(round);
    if half == 1 && format != TWO_LEGS {
        format = 0;
    }
    competition.set_round_format(format);
    competition.set_group_section_flag(0);
    competition.set_second_leg_counter(0);
    competition.set_round_complete(0);
    competition.set_competition_format(0);
    for result in 0..CLEARED_RESULTS {
        competition.set_cup_round_result(result, 0);
    }
    Ok(())
}

/// A tie's two places swapped (upstream rotates the tie's word by a byte).
fn swap_sides(competition: &mut RunningCompetition, tie: usize) {
    let (home, away) = (
        competition.grid_pairing(2 * tie),
        competition.grid_pairing(2 * tie + 1),
    );
    competition.set_grid_pairing(2 * tie, away);
    competition.set_grid_pairing(2 * tie + 1, home);
}

/// A word's two bytes swapped.
fn swapped(word: u16) -> u16 {
    word.rotate_left(u8::BITS)
}

/// The second legs: each tie the other way round, its first leg's goals kept the other way
/// round for the aggregate.
fn second_legs(competition: &mut RunningCompetition) -> Result<(), CupPlayError> {
    competition.set_cur_month_idx(competition.cur_month_idx().wrapping_add(1));
    competition.set_competition_format(competition.competition_format().wrapping_add(1));
    competition.set_cup_current_match_index(0);
    competition.set_round_complete(0);
    for tie in 0..ties(competition.total_matches_in_round())? {
        competition.set_cup_round_result(tie, ResultFlags::RETURN_LEG);
        swap_sides(competition, tie);
        let score = competition.cup_round_score(tie);
        competition.set_cup_aggregate_score(tie, swapped(score));
    }
    Ok(())
}

/// The drawn ties replayed (upstream `DIY_ReorderTeamsBySelectionFlags`, then the replays'
/// round): the undecided ties moved to the front, each with its scores and flags (and its
/// place in the draw's order, for a cup), then played the other way round.
fn replays(competition: &mut RunningCompetition) -> Result<(), CupPlayError> {
    let count = ties(competition.total_matches_in_round())?;
    let mut front = 0;
    for tie in 0..count {
        if competition.cup_round_result(tie) & ResultFlags::DECIDED != 0 {
            continue;
        }
        swap_ties(competition, tie, front);
        if competition.has_groups() == 0 {
            let (tie_byte, front_byte) = (byte(tie), byte(front));
            let at_tie = order_place(competition, tie_byte)?;
            let at_front = order_place(competition, front_byte)?;
            competition.set_shuffle_order(at_front, tie_byte);
            competition.set_shuffle_order(at_tie, front_byte);
        }
        front += 1;
    }
    if competition.group_section_flag() == 0 && competition.round_format() != TWO_LEGS {
        competition.set_cur_month_idx(competition.cur_month_idx().wrapping_add(1));
    }
    let replayed = competition.second_leg_counter();
    competition.set_total_matches_in_round(replayed.cast_signed());
    if competition.round_format() == TWO_LEGS {
        competition.set_round_format(0);
    }
    competition.set_competition_format(0);
    competition.set_group_section_flag(competition.group_section_flag().wrapping_add(1));
    competition.set_cup_current_match_index(0);
    competition.set_round_complete(0);
    let flags = if competition.group_section_flag() != 0 {
        ResultFlags::REPLAY
    } else {
        0
    };
    for tie in 0..ties(replayed.cast_signed())? {
        competition.set_cup_round_result(tie, flags);
        swap_sides(competition, tie);
    }
    Ok(())
}

/// Ties `a` and `b` swapped: their places, scores, aggregates, shoot-outs and flags.
fn swap_ties(competition: &mut RunningCompetition, a: usize, b: usize) {
    macro_rules! swap {
        ($get:ident, $set:ident, $a:expr, $b:expr) => {{
            let (first, second) = (competition.$get($a), competition.$get($b));
            competition.$set($a, second);
            competition.$set($b, first);
        }};
    }
    swap!(grid_pairing, set_grid_pairing, 2 * a, 2 * b);
    swap!(grid_pairing, set_grid_pairing, 2 * a + 1, 2 * b + 1);
    swap!(cup_round_score, set_cup_round_score, a, b);
    swap!(cup_aggregate_score, set_cup_aggregate_score, a, b);
    swap!(cup_penalty_score, set_cup_penalty_score, a, b);
    swap!(cup_round_result, set_cup_round_result, a, b);
}

/// A tie's index as the draw's order holds it: a byte.
fn byte(tie: usize) -> u8 {
    u8::try_from(tie).expect("32 ties at most")
}

/// Where `value` is first in the draw's order.
fn order_place(competition: &RunningCompetition, value: u8) -> Result<usize, CupPlayError> {
    (0..ORDER_PLACES)
        .find(|&place| competition.shuffle_order(place) == value)
        .ok_or(CupPlayError::NotInOrder(u16::from(value)))
}

/// Each tie's winner and loser, in the ties' order (upstream
/// `DIY_ApplyHomeAwaySwapFlagsToTeamGridPairs`).
fn winners_and_losers(
    competition: &RunningCompetition,
) -> Result<(Vec<u8>, Vec<u8>), CupPlayError> {
    let pairs = competition.match_pairs_per_round();
    let count = ties(pairs)?;
    // Faithful to the original: the places written back are twice the pairs, a word; past
    // the ties' (a count below 0 that doubles to one above) upstream copies its scratch.
    let written = usize::try_from(pairs.cast_unsigned().wrapping_mul(2).cast_signed())
        .unwrap_or_default();
    if written > 2 * count {
        return Err(CupPlayError::TiesPastGrid(pairs));
    }
    let mut winners = Vec::with_capacity(count);
    let mut losers = Vec::with_capacity(count);
    for tie in 0..count {
        let (home, away) = (
            competition.grid_pairing(2 * tie),
            competition.grid_pairing(2 * tie + 1),
        );
        if competition.cup_round_result(tie) & ResultFlags::AWAY_WON != 0 {
            winners.push(away);
            losers.push(home);
        } else {
            winners.push(home);
            losers.push(away);
        }
    }
    Ok((winners, losers))
}

/// Each tie's winner to the front, its loser behind them all (upstream
/// `DIY_ApplyHomeAwaySwapFlagsToTeamGridPairs`): a knockout stage's end.
///
/// # Errors
///
/// [`CupPlayError::TiesPastGrid`] when the round's ties are past the grid.
pub(in crate::competition) fn winners_first(
    competition: &mut RunningCompetition,
) -> Result<(), CupPlayError> {
    let (winners, losers) = winners_and_losers(competition)?;
    for (place, team) in winners.into_iter().chain(losers).enumerate() {
        competition.set_grid_pairing(place, team);
    }
    Ok(())
}

/// The winners into the next round, in the draw's order (upstream
/// `DIY_ReorderTeamGridByOrderList`): each tie's winner to the front, its loser behind them
/// all ([`winners_and_losers`]); then the winners' places as the order says.
fn advance_winners(competition: &mut RunningCompetition) -> Result<(), CupPlayError> {
    let (winners, losers) = winners_and_losers(competition)?;
    let count = winners.len();
    let order = (0..count)
        .map(|place| {
            let index = competition.shuffle_order(place);
            winners
                .get(usize::from(index))
                .copied()
                .ok_or(CupPlayError::NotInOrder(u16::from(index)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (place, team) in order.into_iter().chain(losers).enumerate() {
        competition.set_grid_pairing(place, team);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::cup::{CupRecord, CupStart, start_cup};
    use crate::competition::start::Pitch;

    /// Two legs, extra time and penalties.
    const TWO_LEGGED: u8 = 0x94;

    /// One leg, extra time and penalties.
    const ONE_LEG: u8 = 0x54;

    /// One leg, no extra time, no penalties: a replay.
    const REPLAYED: u8 = 0x40;

    /// A match's normal time, extra time and penalties, each the home side's and the away
    /// side's.
    type Parts = ((u16, u16), (u16, u16), (u16, u16));

    /// The results a script gives, each part in turn, and what was asked.
    #[derive(Default)]
    struct Script {
        /// Each match's normal time, extra time and penalties.
        matches: Vec<(Score, Score, Score)>,
        /// The match the next part is of.
        next: usize,
        /// What was asked: `n` normal time (and its venue), `e` extra time, `p` penalties.
        asked: String,
    }

    impl Script {
        fn of(matches: &[Parts]) -> Self {
            let score = |(home, away)| Score { home, away };
            Self {
                matches: matches
                    .iter()
                    .map(|&(normal, extra, penalties)| {
                        (score(normal), score(extra), score(penalties))
                    })
                    .collect(),
                ..Self::default()
            }
        }
    }

    impl MatchResults for Script {
        fn normal_time(&mut self, _fixture: CupFixture, venue: Venue) -> Score {
            self.asked
                .push(if venue == Venue::Home { 'n' } else { 'N' });
            let (normal, _, _) = self.matches[self.next];
            self.next = (self.next + 1) % self.matches.len();
            normal
        }

        fn extra_time(&mut self, _fixture: CupFixture) -> Score {
            self.asked.push('e');
            let previous = (self.next + self.matches.len() - 1) % self.matches.len();
            self.matches[previous].1
        }

        fn penalties(&mut self, _fixture: CupFixture) -> Score {
            self.asked.push('p');
            let previous = (self.next + self.matches.len() - 1) % self.matches.len();
            self.matches[previous].2
        }
    }

    /// Team `[0, n]` the selected team `n`, the computer's but team 1.
    fn teams(team: TeamBytes) -> Option<LoadedTeam> {
        let [_, ordinal] = team;
        Some(LoadedTeam {
            identity: i32::from(ordinal),
            controls: if ordinal == 1 {
                TeamControls::PlayerCoach
            } else {
                TeamControls::Computer
            },
            prices: [ordinal; 16],
        })
    }

    /// A cup of `teams` started, its draw kept in order, its rounds' rules `rules`.
    fn started(count: i8, rules: [u8; 6]) -> RunningCompetition {
        let record = CupRecord {
            comp_id: 109,
            begin_month: 0x40,
            end_month: 0x60,
            seed_reorder: 1,
            num_rounds_override: 0,
            teams: count,
            away_goals_mode: 1,
            substitutes: 0x35,
            round_string_base: 1,
            round_rules: rules.to_vec(),
        };
        let mut competition = RunningCompetition::default();
        for ordinal in 0..8 {
            competition.set_cup_team(usize::from(ordinal), [0, ordinal]);
        }
        let start = CupStart {
            record: &record,
            name: b"FA CUP",
            prefix: None,
            default_name: b"DIY CUP",
            continental: false,
            pitch: Pitch::Seasonal,
        };
        start_cup(&mut competition, &start, teams, &mut Rng::default()).expect("started");
        competition
    }

    /// Plays the cup to its end as the career does: its matches until a round is complete,
    /// then the next round unless it is finished.
    fn played(competition: &mut RunningCompetition, script: &mut Script) -> usize {
        let mut tournament = TournamentState::default();
        let mut rng = Rng::default();
        for played in 1..100 {
            play_cup_match(competition, &mut tournament, script, teams, &mut rng).expect("played");
            if competition.round_complete() == 0 {
                continue;
            }
            if competition.competition_finished() != 0 {
                return played;
            }
            next_round(competition).expect("the next round");
        }
        panic!("a cup that never ends");
    }

    fn fixture() -> CupFixture {
        CupFixture {
            home_slot: 0,
            away_slot: 1,
            home: 0,
            away: 1,
        }
    }

    #[test]
    fn a_single_leg_goes_to_extra_time_then_penalties() {
        let competition = started(2, [ONE_LEG, 0, 0, 0, 0, 0]);
        let mut script = Script::of(&[((1, 1), (0, 0), (4, 5))]);
        let decision = decide(&competition, fixture(), (0, 0), &mut script).expect("decided");
        assert_eq!(script.asked, "Nep", "a final of one leg, on neutral ground");
        assert_eq!(decision.goals, 0x0101);
        assert_eq!(decision.aggregate, 0x0101, "the level score kept");
        assert_eq!(decision.penalties, 0x0405);
        let flags = ResultFlags::PLAYED
            | ResultFlags::DECIDED
            | ResultFlags::EXTRA_TIME
            | ResultFlags::PENALTIES
            | ResultFlags::AWAY_WON;
        assert_eq!(decision.flags, ResultFlags(flags));
    }

    #[test]
    fn a_single_leg_without_extra_time_or_penalties_is_a_draw_to_replay() {
        let competition = started(4, [REPLAYED, REPLAYED, 0, 0, 0, 0]);
        let mut script = Script::of(&[((2, 2), (0, 0), (0, 0))]);
        let decision = decide(&competition, fixture(), (0, 0), &mut script).expect("decided");
        assert_eq!(script.asked, "n");
        assert_eq!(
            decision.flags,
            ResultFlags(ResultFlags::PLAYED),
            "not decided"
        );
    }

    #[test]
    fn a_return_leg_goes_by_aggregate_then_away_goals() {
        let mut competition = started(4, [TWO_LEGGED, ONE_LEG, 0, 0, 0, 0]);
        competition.set_competition_format(1);
        // 2-1 here after 0-1 there: level at 2-2, the away side's one goal here against the
        // home side's none there.
        let mut script = Script::of(&[((2, 1), (0, 0), (0, 0))]);
        let decision = decide(&competition, fixture(), (0, 1), &mut script).expect("decided");
        assert_eq!(script.asked, "n");
        assert_eq!(
            decision.flags,
            ResultFlags(
                ResultFlags::PLAYED
                    | ResultFlags::DECIDED
                    | ResultFlags::RETURN_LEG
                    | ResultFlags::AWAY_GOALS
                    | ResultFlags::AWAY_WON
            ),
            "the away side through on away goals"
        );
        assert_eq!(decision.aggregate, 0x0001, "the first leg's goals");
        // 1-0 here after 0-1 there: level, neither side scored away: extra time (1-1, an
        // away goal each), then penalties.
        let mut script = Script::of(&[((1, 0), (1, 1), (3, 2))]);
        let mut no_away_goals = competition.clone();
        no_away_goals.set_play_each_team(0);
        let decision = decide(&no_away_goals, fixture(), (0, 1), &mut script).expect("decided");
        assert_eq!(script.asked, "nep");
        assert_eq!(decision.goals, 0x0201, "extra time's goals counted");
        assert_eq!(decision.penalties, 0x0302);
    }

    #[test]
    fn a_cup_plays_through_to_its_winner() {
        // Four teams: one-leg semi-finals, then a one-leg final; the home sides win.
        let mut competition = started(4, [ONE_LEG, ONE_LEG, 0, 0, 0, 0]);
        let mut script = Script::of(&[((2, 0), (0, 0), (0, 0))]);
        let matches = played(&mut competition, &mut script);
        assert_eq!(matches, 3);
        assert_eq!(competition.competition_finished(), 1);
        assert_eq!(
            competition.result_code(),
            0,
            "team [0, 0]: first in the draw, winning twice"
        );
        assert_eq!(competition.match_count(), 3);
        assert_eq!(
            competition.round_iterator(),
            0,
            "the human team knocked out"
        );
    }

    #[test]
    fn two_legged_ties_play_their_second_legs_the_other_way_round() {
        let mut competition = started(4, [TWO_LEGGED, ONE_LEG, 0, 0, 0, 0]);
        let mut script = Script::of(&[((1, 0), (0, 0), (0, 0))]);
        let mut tournament = TournamentState::default();
        let mut rng = Rng::default();
        for _ in 0..2 {
            play_cup_match(
                &mut competition,
                &mut tournament,
                &mut script,
                teams,
                &mut rng,
            )
            .expect("a first leg");
        }
        assert_eq!(competition.round_complete(), 1);
        assert_eq!(competition.competition_finished(), 0);
        let before = (competition.grid_pairing(0), competition.grid_pairing(1));
        next_round(&mut competition).expect("the second legs");
        assert_eq!(competition.competition_format(), 1);
        assert_eq!(
            (competition.grid_pairing(1), competition.grid_pairing(0)),
            before,
            "the other way round"
        );
        assert_eq!(competition.cup_round_result(0), ResultFlags::RETURN_LEG);
        assert_eq!(
            competition.cup_aggregate_score(0),
            0x0001,
            "the first leg's 1-0, turned"
        );
    }

    #[test]
    fn drawn_ties_are_replayed_first_the_other_way_round() {
        let mut competition = started(4, [REPLAYED, REPLAYED, 0, 0, 0, 0]);
        // The first tie won, the second drawn.
        let mut script = Script::of(&[((1, 0), (0, 0), (0, 0)), ((1, 1), (0, 0), (0, 0))]);
        let mut tournament = TournamentState::default();
        let mut rng = Rng::default();
        for _ in 0..2 {
            play_cup_match(
                &mut competition,
                &mut tournament,
                &mut script,
                teams,
                &mut rng,
            )
            .expect("played");
        }
        assert_eq!(competition.second_leg_counter(), 1, "one tie to replay");
        let drawn = (competition.grid_pairing(2), competition.grid_pairing(3));
        next_round(&mut competition).expect("the replays");
        assert_eq!(competition.total_matches_in_round(), 1);
        assert_eq!(competition.group_section_flag(), 1);
        assert_eq!(
            (competition.grid_pairing(1), competition.grid_pairing(0)),
            drawn,
            "the drawn tie first, the other way round"
        );
        assert_eq!(competition.cup_round_result(0), ResultFlags::REPLAY);
    }

    #[test]
    fn what_cannot_be_played_is_refused() {
        let mut competition = started(4, [ONE_LEG, ONE_LEG, 0, 0, 0, 0]);
        competition.set_grid_pairing(0, 0x80);
        assert_eq!(
            cup_fixture(&competition),
            Err(CupPlayError::NotABracketTeam(-128))
        );
        let mut competition = started(4, [ONE_LEG, ONE_LEG, 0, 0, 0, 0]);
        competition.set_current_round_index(-1);
        let mut script = Script::of(&[((1, 0), (0, 0), (0, 0))]);
        assert_eq!(
            decide(&competition, fixture(), (0, 0), &mut script),
            Err(CupPlayError::RoundPastRules(-1))
        );
        let mut competition = started(4, [ONE_LEG, ONE_LEG, 0, 0, 0, 0]);
        competition.set_match_pairs_per_round(33);
        assert_eq!(
            next_round(&mut competition),
            Err(CupPlayError::TiesPastGrid(33))
        );
    }
}
