//! A tournament's state and its stages (docs/PLAN.md M5 #4d1): upstream's `DIYTournamentState`
//! ([`TournamentState`]), the reset that gives it a DIY tournament's defaults
//! (diy_competition.c `InitDIYTournament`), and each stage's matches with their months and the
//! teams' strength (`DIYTournament_RecalculateMatchesPerTeam`,
//! `DIYTournament_BuildMatchesPerTeamProgressionTable`); a tournament started from its record,
//! its first stage loaded into the competition under way ([`start_tournament`], M5 #4d2), and
//! [`WorldTournament`], which resolves its texts, classes and draws from a world's data; a
//! stage ended, its teams ranked or its winners first into the next stage's order, and the
//! next stage loaded ([`next_stage`], M5 #7), drawn by the world's own draw
//! ([`OwnDraw`]) or the engine's.

mod advance;
mod own_draw;
mod record;
mod stage;
mod start;
mod state;
mod world_tournament;

pub use advance::{Advance, NextStageError, next_stage};
pub use own_draw::{BestPlaced, OwnDraw, Ranking, StageDraw};
pub(crate) use own_draw::{ORDER_PLACES, PATTERN_GROUPS};
pub use record::{RecordError, TournamentRecord};
pub use stage::{LoadError, StageRules};
pub use start::{ResetNames, StartError, TournamentStart, start_tournament};
pub use state::{STAGES, STATE_LEN, TEAMS, TournamentState};
pub use world_tournament::WorldTournament;

use thiserror::Error;

use super::running::{RunningCompetition, TeamBytes};
use super::start::{LoadedTeam, copied, defaults as common, low_word, months};
use super::value::strength;

/// The stages the reset lays out (upstream `diyTournamentInitTable`): the World Cup's, 24 teams
/// in six groups of four, then knockouts of 16, 8, 4 and 2, and a stage of one. Each is its
/// teams, its groups (0 a knockout; nonzero a stage of groups), its teams a group, and the
/// teams that go through.
const DEFAULT_STAGES: [[u16; 4]; STAGES] = [
    [24, 6, 4, 16],
    [16, 0, 16, 8],
    [8, 0, 8, 4],
    [4, 0, 4, 2],
    [2, 0, 2, 1],
    [1, 1, 1, 1],
];

/// The stages the reset plays (upstream `diyTournamentInitTable[0]`).
const DEFAULT_STAGE_COUNT: i16 = 5;

/// The legs code of a round played home and away.
const TWO_LEGS: u16 = 2;

/// The penalties code of a single-leg round a draw settles on penalties.
const PENALTIES: u16 = 1;

/// The matches of a knockout stage: two legs, or a match and a replay's date.
const KNOCKOUT_MATCHES: u16 = 2;

/// The bytes of a team id in the state.
const TEAM_ID_LEN: usize = 2;

/// The tournament reset's defaults (upstream `InitDIYTournament`; the ones every reset shares
/// are [`common`]'s).
mod defaults {
    /// A tournament (`competitionType`).
    pub const COMPETITION_TYPE: i16 = 2;
    /// The slot's cache key (`compSlot`; upstream writes it and `slotId` as the word 2).
    pub const COMP_SLOT: u8 = 2;
    /// The fewest substitutes (`roundIterator`, which a stage hands on as them).
    pub const MIN_SUBSTITUTES: i16 = 3;
    /// The most substitutes (`numTeamsInStage`, likewise).
    pub const MAX_SUBSTITUTES: i16 = 5;
    /// Each stage's legs, extra time and penalties.
    pub const ROUND_RULE: u16 = 1;
    /// Each stage's points for a win.
    pub const POINTS_FOR_WIN: u16 = 2;
}

/// Why a tournament's stages cannot be counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StagesError {
    /// Stages other than 1 to 6 (upstream reads slots past its six, or counts months for no
    /// match and runs past its memory).
    #[error("a tournament of {0} stages")]
    Stages(i16),
    /// A stage of groups of fewer than two teams (upstream divides by zero).
    #[error("stage {stage}'s groups have {teams} teams")]
    GroupOfOne {
        /// The stage.
        stage: usize,
        /// Its teams a group.
        teams: u16,
    },
    /// Stages of no match in all (upstream's months run on past its memory).
    #[error("the stages have no match")]
    NoMatches,
    /// A stage's months past the state (upstream writes them past it).
    #[error("stage {0}'s months run past the state")]
    MonthsPastState(usize),
    /// Teams other than 1 to those whose ids the state holds (upstream divides by zero, or
    /// reads past the state).
    #[error("a tournament of {0} teams")]
    Teams(i16),
    /// A team of the tournament that is not loaded.
    #[error("team {0:?} is not loaded")]
    UnknownTeam(TeamBytes),
}

/// The reset (upstream `InitDIYTournament`): a DIY tournament's defaults, the name
/// `default_name`, the default stages, the teams in order.
pub fn reset(state: &mut TournamentState, default_name: &[u8]) {
    state.set_competition_type(defaults::COMPETITION_TYPE);
    state.set_comp_slot(defaults::COMP_SLOT);
    state.set_slot_id(0);
    state.set_is_active(0);
    state.set_match_started(0);
    state.set_career_team_involved(0);
    state.set_result_code(common::RESULT_CODE);
    state.set_substitutes_count(1);
    state.set_slot_state_a(common::SLOT_STATE_A);
    state.set_slot_state_b(common::SLOT_STATE_B);
    state.set_slot_state_c(common::SLOT_STATE_C);
    state.set_comp_id(common::COMP_ID);
    state.set_match_pairs_per_round(0);
    state.set_name(copied(state.name(), default_name));
    state.set_start_month(common::START_MONTH);
    state.set_end_month(common::END_MONTH);
    state.set_pitch_type_or_season(1);
    state.set_pitch_type(common::PITCH_TYPE);
    state.set_round_iterator(defaults::MIN_SUBSTITUTES);
    state.set_num_teams_in_stage(defaults::MAX_SUBSTITUTES);
    state.set_menu_state(0);
    state.set_slot_pad(0, 0);
    state.set_total_matches_in_round(DEFAULT_STAGE_COUNT);
    state.set_round_complete(0);
    let [[first_teams, ..], ..] = DEFAULT_STAGES;
    state.set_num_teams(first_teams.cast_signed());
    for (stage, &[teams, groups, per_group, through]) in DEFAULT_STAGES.iter().enumerate() {
        state.set_slot_num_teams(stage, teams);
        state.set_slot_is_league_format(stage, groups);
        state.set_slot_seeding_type(stage, 0);
        state.set_slot_double_count(stage, 0);
        state.set_slot_groups(stage, groups);
        state.set_slot_teams_per_group(stage, per_group);
        state.set_slot_division_factor(stage, through);
        state.set_slot_legs(stage, defaults::ROUND_RULE);
        state.set_slot_extra_time(stage, defaults::ROUND_RULE);
        state.set_slot_penalties(stage, defaults::ROUND_RULE);
        state.set_slot_points_for_win(stage, defaults::POINTS_FOR_WIN);
        state.set_slot_play_each_team(stage, 1);
    }
    for (index, team) in (0..TEAMS).zip(0..) {
        state.set_team_index(index, team);
    }
}

/// Each stage's matches, their months and the teams' strength (upstream
/// `DIYTournament_RecalculateMatchesPerTeam`): the tournament under way from its first stage,
/// and `competition`, the stage under way, not yet. A world's league phase and state
/// championships count their own matches (`LeaguePhase_StageDays`, `States_StageDays`): they
/// join with World 2026-27's components (M5 #9).
///
/// # Errors
///
/// [`StagesError`] when the stages are not 1 to 6, a stage's groups are of fewer than two
/// teams, the stages have no match or their months run past the state, the teams are not 1 to
/// those the state holds, or a team is not loaded. Nothing is written then.
pub fn recalculate(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    teams: impl Fn(TeamBytes) -> Option<LoadedTeam>,
) -> Result<(), StagesError> {
    let stage_count = state.total_matches_in_round();
    let stages = usize::try_from(stage_count)
        .ok()
        .filter(|stages| (1..=STAGES).contains(stages))
        .ok_or(StagesError::Stages(stage_count))?;
    let matches = (0..stages)
        .map(|stage| stage_matches(state, stage))
        .collect::<Result<Vec<_>, _>>()?;
    let months = stage_months(state, &matches)?;
    let num_teams = state.num_teams();
    let count = usize::try_from(num_teams)
        .ok()
        .filter(|&count| count > 0 && state::TEAM_IDS + count * TEAM_ID_LEN <= STATE_LEN)
        .ok_or(StagesError::Teams(num_teams))?;
    let prices = (0..count)
        .map(|index| {
            let team = state.team(index);
            teams(team)
                .map(|loaded| loaded.prices)
                .ok_or(StagesError::UnknownTeam(team))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let strength = strength(&prices).expect("one team at least");
    state.set_is_active(1);
    state.set_current_round_index(0);
    state.set_round_complete(0);
    competition.set_is_active(0);
    for (stage, &matches) in matches.iter().enumerate() {
        state.set_slot_matches(stage, matches);
    }
    for (stage, index, month) in months {
        state.set_slot_month(stage, index, month);
    }
    state.set_avg_skill(strength.average.to_le_bytes()[0]);
    state.set_strength_tier(strength.tier);
    Ok(())
}

/// The matches a team of `stage` plays: a group stage's round robin, each pair of a group as
/// many times as it plays the others, its days of half the group a day; a knockout's two legs,
/// or one match settled on penalties, or one and a replay's date.
fn stage_matches(state: &TournamentState, stage: usize) -> Result<u16, StagesError> {
    if state.slot_is_league_format(stage) == 0 {
        let legs = state.slot_legs(stage);
        return Ok(
            if legs != TWO_LEGS && state.slot_penalties(stage) == PENALTIES {
                1
            } else {
                KNOCKOUT_MATCHES
            },
        );
    }
    let per_group = state.slot_teams_per_group(stage);
    let half = per_group >> 1;
    if half == 0 {
        return Err(StagesError::GroupOfOne {
            stage,
            teams: per_group,
        });
    }
    let pairs = ((u32::from(per_group) - 1) * u32::from(per_group)) >> 1;
    let days = (pairs / u32::from(half)).wrapping_mul(u32::from(state.slot_play_each_team(stage)));
    Ok(low_word(days))
}

/// The months of every stage's matches (upstream
/// `DIYTournament_BuildMatchesPerTeamProgressionTable`), each its stage, its place and its
/// month: the matches of all stages from the first month to the last, each stage's in its own
/// table. Faithful to the original: a stage of no match takes the next match, and every match
/// after it, its months running on past its table.
fn stage_months(
    state: &TournamentState,
    matches: &[u16],
) -> Result<Vec<(usize, usize, u8)>, StagesError> {
    let total = matches
        .iter()
        .fold(0u16, |total, &matches| total.wrapping_add(matches));
    if total == 0 {
        return Err(StagesError::NoMatches);
    }
    let months = months(
        state.start_month().cast_unsigned(),
        state.end_month().cast_unsigned(),
        total,
    );
    let mut placed = Vec::with_capacity(months.len());
    let (mut stage, mut place, mut remaining) = (None, 0, 0u16);
    for month in months {
        if remaining == 0 {
            let next = stage.map_or(0, |stage| stage + 1);
            remaining = *matches
                .get(next)
                .expect("the months are as many as the stages' matches");
            stage = Some(next);
            place = 0;
        }
        let current = stage.expect("a stage once a month is placed");
        if !TournamentState::has_month(current, place) {
            return Err(StagesError::MonthsPastState(current));
        }
        placed.push((current, place, month));
        place += 1;
        remaining = remaining.wrapping_sub(1);
    }
    Ok(placed)
}

#[cfg(test)]
mod tests {
    use openswos_team_resource::TeamControls;

    use super::*;

    /// Each team `[0, n]` priced `n` throughout.
    fn teams(team: TeamBytes) -> Option<LoadedTeam> {
        let [_, ordinal] = team;
        Some(LoadedTeam {
            identity: i32::from(ordinal),
            controls: TeamControls::Computer,
            prices: [ordinal; 16],
        })
    }

    /// The reset's state, its teams `[0, 0]` to `[0, 23]`.
    fn state() -> TournamentState {
        let mut state = TournamentState::default();
        reset(&mut state, b"DIY TOURNAMENT");
        for ordinal in 0..24 {
            state.set_team(usize::from(ordinal), [0, ordinal]);
        }
        state
    }

    #[test]
    fn the_reset_lays_out_the_world_cup_s_stages() {
        let state = state();
        assert_eq!(state.competition_type(), 2);
        assert_eq!((state.comp_slot(), state.slot_id()), (2, 0));
        assert_eq!(state.result_code(), -1);
        assert_eq!(&state.name()[..15], b"DIY TOURNAMENT\0");
        assert_eq!((state.round_iterator(), state.num_teams_in_stage()), (3, 5));
        assert_eq!((state.total_matches_in_round(), state.num_teams()), (5, 24));
        let stages: Vec<[u16; 4]> = (0..STAGES)
            .map(|stage| {
                [
                    state.slot_num_teams(stage),
                    state.slot_groups(stage),
                    state.slot_teams_per_group(stage),
                    state.slot_division_factor(stage),
                ]
            })
            .collect();
        assert_eq!(stages, DEFAULT_STAGES);
        assert_eq!(state.slot_is_league_format(0), 6, "the groups' count");
        assert_eq!(
            (
                state.slot_legs(3),
                state.slot_points_for_win(3),
                state.slot_play_each_team(3)
            ),
            (1, 2, 1)
        );
        let order: Vec<u8> = (0..TEAMS).map(|index| state.team_index(index)).collect();
        assert_eq!(order, (0..64).collect::<Vec<u8>>());
    }

    #[test]
    fn the_stages_matches_and_months_are_counted() {
        let mut state = state();
        // Groups of four once: three days; then knockouts of two legs, one leg settled on
        // penalties, one leg replayed.
        state.set_slot_legs(1, 2);
        state.set_slot_penalties(2, 1);
        state.set_slot_penalties(3, 0);
        state.set_total_matches_in_round(4);
        state.set_start_month(0x40);
        state.set_end_month(0x58);
        let mut competition = RunningCompetition::default();
        competition.set_is_active(1);
        recalculate(&mut state, &mut competition, teams).expect("counted");
        let matches: Vec<u16> = (0..4).map(|stage| state.slot_matches(stage)).collect();
        assert_eq!(matches, [3, 2, 1, 2]);
        assert_eq!(competition.is_active(), 0, "the stage not yet under way");
        assert_eq!(state.is_active(), 1);
        // Eight matches from 0x40 to 0x58: each 24/7 of a step on, rounded down.
        let months: Vec<u8> = [
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 1),
            (2, 0),
            (3, 0),
            (3, 1),
        ]
        .iter()
        .map(|&(stage, index)| state.slot_month(stage, index))
        .collect();
        assert_eq!(months, [0x40, 0x43, 0x46, 0x4A, 0x4D, 0x51, 0x54, 0x58]);
        // Prices 0 to 23: the average 11, tier 2.
        assert_eq!((state.avg_skill(), state.strength_tier()), (11, 2));
    }

    #[test]
    fn a_group_stage_plays_its_round_robin_as_often_as_it_says() {
        let mut state = state();
        state.set_slot_teams_per_group(0, 6);
        state.set_slot_play_each_team(0, 2);
        state.set_total_matches_in_round(1);
        recalculate(&mut state, &mut RunningCompetition::default(), teams).expect("counted");
        // Fifteen pairs, three a day, twice.
        assert_eq!(state.slot_matches(0), 10);
    }

    #[test]
    fn a_stage_of_no_match_takes_the_months_after_it() {
        let mut state = state();
        state.set_slot_play_each_team(0, 0);
        state.set_slot_legs(1, 2);
        state.set_total_matches_in_round(2);
        state.set_start_month(0x40);
        state.set_end_month(0x48);
        recalculate(&mut state, &mut RunningCompetition::default(), teams).expect("counted");
        assert_eq!(state.slot_matches(0), 0);
        // The knockout's two months, in the first stage's table.
        assert_eq!(
            (state.slot_month(0, 0), state.slot_month(0, 1)),
            (0x40, 0x48)
        );
    }

    #[test]
    fn what_cannot_be_counted_is_refused_and_nothing_written() {
        let refused = |change: &dyn Fn(&mut TournamentState)| {
            let mut state = state();
            change(&mut state);
            let before = state.clone();
            let error = recalculate(&mut state, &mut RunningCompetition::default(), teams)
                .expect_err("refused");
            assert_eq!(state, before, "nothing written");
            error
        };
        assert_eq!(
            refused(&|state| state.set_total_matches_in_round(0)),
            StagesError::Stages(0)
        );
        assert_eq!(
            refused(&|state| state.set_total_matches_in_round(7)),
            StagesError::Stages(7)
        );
        assert_eq!(
            refused(&|state| state.set_slot_teams_per_group(0, 1)),
            StagesError::GroupOfOne { stage: 0, teams: 1 }
        );
        assert_eq!(
            refused(&|state| {
                state.set_total_matches_in_round(1);
                state.set_slot_play_each_team(0, 0);
            }),
            StagesError::NoMatches
        );
        assert_eq!(
            refused(&|state| {
                state.set_total_matches_in_round(6);
                state.set_slot_teams_per_group(5, 4);
                state.set_slot_play_each_team(5, 200);
            }),
            StagesError::MonthsPastState(5)
        );
        assert_eq!(
            refused(&|state| state.set_num_teams(0)),
            StagesError::Teams(0)
        );
        assert_eq!(
            refused(&|state| state.set_num_teams(-1)),
            StagesError::Teams(-1)
        );
        let mut state = state();
        let error = recalculate(&mut state, &mut RunningCompetition::default(), |_| None)
            .expect_err("no teams");
        assert_eq!(error, StagesError::UnknownTeam([0, 0]));
    }
}
