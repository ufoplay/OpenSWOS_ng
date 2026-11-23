//! A tournament's stage ended and the next loaded (docs/PLAN.md M5 #7: upstream
//! src/career_teams.c `DIY_AdvanceToNextCompetitionSlot`): a stage of groups' teams ranked
//! into the order the next stage draws from (diy_competition.c
//! `DIY_BuildSeededBracketAndUpdateScores`, or a world's ranking:
//! `DIY_AdjustWorldCupBracketScores`), a knockout's winners first
//! (`DIY_CopyTeamGridFromCompToDIY`), then the next stage loaded ([`super::stage`]), or the
//! tournament ended after its last. A world's league phase, other confederations' main rounds
//! and state championships end their stages their own way (`LeaguePhase_GroupsEnd`,
//! `Conti_GroupsEnd`, `States_GroupsEnd`, `States_KnockoutEnd`): with World 2026-27's
//! components (M5 #9). The career then records the new stage into its calendar
//! (`DIY_PostLoadCompetitionDispatch`, M7).

use openswos_rng::Rng;
use thiserror::Error;

use super::TournamentState;
use super::own_draw::Ranking;
use super::stage::{LoadError, StageRules, load, slot};
use crate::competition::cup::{CupPlayError, winners_first};
use crate::competition::league::{PlayError, place_row, sort_table};
use crate::competition::running::{RunningCompetition, TeamBytes};
use crate::competition::start::LoadedTeam;

/// The state's `isActive` once its last stage is played.
const ENDED: u16 = 2;

/// The bonus of a group's winner, ranked by place (upstream's `0x7D00`): a group's next
/// places 1000 less each, a group's 100 less than the one before.
const PLACE_FIRST: i16 = 0x7D00;

/// What each place below a group's winner takes less, ranked by place.
const PLACE_STEP: i16 = 1000;

/// What each group takes less than the one before, ranked by place.
const GROUP_STEP: i16 = 100;

/// The bonus of the place only some go through on, ranked by place (upstream's `0x7D0`).
const PLACE_THROUGH: u16 = 2000;

/// The group a best placed team's bonus counts down from, ranked by place (upstream's
/// `(8 - group) * 25`).
const PLACE_GROUPS: i16 = 8;

/// What each group before a best placed team's adds, ranked by place.
const PLACE_GROUP_BONUS: i16 = 25;

/// The bonus of the first qualifier, ranked by group (upstream's `0x2328`): each next one
/// 100 less.
const GROUP_FIRST: i16 = 9000;

/// What each next qualifier takes less, ranked by group.
const GROUP_QUALIFIER_STEP: i16 = 100;

/// The bonus of the place only some go through on, ranked by group (upstream's `0x3E8`).
const GROUP_THROUGH: u16 = 1000;

/// The first place of the best placed, ranked by group (upstream reads the sorted entries
/// from the image's 0x86: the table's place 12).
const GROUP_BEST_FIRST: usize = 12;

/// The best placed ranked by their group (upstream's four).
const GROUP_BEST: usize = 4;

/// The group a best placed team's bonus counts down from, ranked by group (upstream's
/// `(6 - group) * 100`).
const GROUP_GROUPS: i16 = 6;

/// What each group before a best placed team's adds, ranked by group.
const GROUP_GROUP_BONUS: i16 = 100;

/// The bytes of a standings row: a place's entry offset counts in them.
const ROW_BYTES: u16 = 18;

/// The teams that go through on a stage of one: a final's (upstream compares the teams
/// through with 1).
const ONE: u16 = 1;

/// Where a tournament is after [`next_stage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Advance {
    /// It had ended before: nothing changes.
    Over,
    /// Its last stage was played: it ends (its state's `isActive` 2).
    Ended,
    /// Its next stage is loaded into the competition under way.
    Loaded,
}

/// Why a tournament's next stage cannot be reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NextStageError {
    /// A stage whose slot, or the next one's, is past the tournament's state (upstream reads
    /// past it).
    #[error("stage {0} is past the tournament's state")]
    StagePastState(u16),
    /// A stage ranked by its groups, of no groups (upstream divides by 0).
    #[error("stage {0} is ranked by groups and has none")]
    NoGroups(usize),
    /// More teams than the state's order holds (upstream writes past it).
    #[error("{0} teams, more than the tournament's order holds")]
    TeamsPastState(usize),
    /// A stage of groups whose table cannot be ranked.
    #[error(transparent)]
    Table(#[from] PlayError),
    /// A knockout whose winners cannot be put first.
    #[error(transparent)]
    Ties(#[from] CupPlayError),
    /// The next stage does not load.
    #[error(transparent)]
    Load(LoadError),
}

impl From<LoadError> for NextStageError {
    fn from(error: LoadError) -> Self {
        match error {
            LoadError::StagePastState(stage) => Self::StagePastState(stage),
            other => Self::Load(other),
        }
    }
}

/// The tournament of `state` moved on after its current stage (upstream
/// `DIY_AdvanceToNextCompetitionSlot`): nothing once it has ended; ended after its last stage;
/// otherwise the stage's teams ranked (a stage of groups, by the own draw's ranking of `rules`,
/// else by place; the tables sorted by `name`, by the team's identity) or its winners put
/// first (a knockout), into the order the next stage draws from, then that stage loaded into
/// `competition` (its draw from `rng`, its teams found by `teams`). The career copies the
/// stage's progress into `state` before (`Career_UpdateHybridCompetitionProgressAndMaybeAdvanceSlot`,
/// M7).
///
/// # Errors
///
/// [`NextStageError`] when a stage is past the state, a table cannot be ranked, a knockout's
/// winners cannot be put first, or the next stage does not load.
pub fn next_stage(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    rules: &StageRules<'_>,
    teams: impl Fn(TeamBytes) -> Option<LoadedTeam>,
    name: impl Fn(i32) -> Option<Vec<u8>>,
    rng: &mut Rng,
) -> Result<Advance, NextStageError> {
    if state.current_round_index() != 0 {
        return Ok(Advance::Over);
    }
    let current = state.round_complete().cast_unsigned();
    if current.wrapping_add(1) == state.total_matches_in_round().cast_unsigned() {
        state.set_is_active(ENDED);
        state.set_current_round_index(1);
        return Ok(Advance::Ended);
    }
    let stage = slot(current)?;
    let ranking = rules.own_draw.map_or(Ranking::ByPlace, |draw| draw.ranking());
    if state.slot_is_league_format(stage) != 0 {
        groups_end(state, competition, stage, ranking, &name)?;
    } else {
        knockout_end(state, competition, stage, ranking, &name)?;
    }
    state.set_round_complete(current.wrapping_add(1).cast_signed());
    load(state, competition, rules, &teams, rng)?;
    Ok(Advance::Loaded)
}

/// A count as upstream walks it: a word that, as a signed one, below 1 walks none.
fn walked(count: i16) -> usize {
    usize::try_from(count).unwrap_or_default()
}

/// The next stage's slot's doubled bracket (`s[1].doubleCount`).
fn next_doubled(state: &TournamentState, stage: usize) -> Result<u16, NextStageError> {
    let next = stage + 1;
    if !TournamentState::has_slot(next) {
        return Err(NextStageError::StagePastState(
            u16::try_from(next).expect("a stage within the state's"),
        ));
    }
    Ok(state.slot_double_count(next))
}

/// `bonus` added to standings row `row`'s points (a word, wrapping).
fn add_points(competition: &mut RunningCompetition, row: usize, bonus: u16) {
    let mut standing = competition.row(row);
    standing.line.points = standing.line.points.wrapping_add(bonus);
    competition.set_row(row, standing);
}

/// The teams that go through a stage of groups on their place, a group (the stage's teams
/// through over its groups), and the rest of the division.
fn through(state: &TournamentState, stage: usize) -> Result<(u16, u16), NextStageError> {
    let groups = state.slot_groups(stage);
    if groups == 0 {
        return Err(NextStageError::NoGroups(stage));
    }
    let teams = state.slot_division_factor(stage);
    Ok((teams / groups, teams % groups))
}

/// The table made one group of every team, then sorted.
fn sort_as_one(
    competition: &mut RunningCompetition,
    name: &impl Fn(i32) -> Option<Vec<u8>>,
) -> Result<(), NextStageError> {
    competition.set_match_pairs_per_round(1);
    competition.set_total_matches_in_round(competition.num_teams());
    sort_table(competition, name)?;
    Ok(())
}

/// A bonus by the group of the team at each of `count` places from `first`: the group's
/// distance from `groups`, times `step` (a team's group, by its row, from the state's map).
fn group_bonuses(
    state: &TournamentState,
    competition: &mut RunningCompetition,
    (first, count): (usize, usize),
    groups: i16,
    step: i16,
) -> Result<(), NextStageError> {
    for place in first..first + count {
        let row = place_row(competition, place)?;
        let group = i16::from(state.group(row).cast_signed());
        add_points(competition, row, ((groups - group) * step).cast_unsigned());
    }
    Ok(())
}

/// A stage of groups' end (upstream `DIY_BuildSeededBracketAndUpdateScores`): its teams ranked
/// by `ranking`, the ranked places' teams (their rows) the order the next stage draws from,
/// the first two swapped when the next stage's bracket is doubled.
fn groups_end(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    stage: usize,
    ranking: Ranking,
    name: &impl Fn(i32) -> Option<Vec<u8>>,
) -> Result<(), NextStageError> {
    match ranking {
        Ranking::ByPlace => by_place(state, competition, stage, name)?,
        Ranking::ByGroup => by_group(state, competition, stage, name)?,
    }
    let count = walked(competition.num_teams());
    if count > 0 && !TournamentState::has_team_index(count - 1) {
        return Err(NextStageError::TeamsPastState(count));
    }
    for place in 0..count {
        // Faithful to the original: the entry's row as a byte, the places' bytes written as
        // the entries are read (more than 64 places read the bytes just written).
        let [row, _] = (competition.team_entry_offset(place) / ROW_BYTES).to_le_bytes();
        competition.set_shuffle_and_pad(place, row);
    }
    for place in 0..count {
        state.set_team_index(place, competition.shuffle_and_pad(place));
    }
    if state.slot_division_factor(stage) != ONE && next_doubled(state, stage)? != 0 {
        let (first, second) = (state.team_index(0), state.team_index(1));
        state.set_team_index(0, second);
        state.set_team_index(1, first);
    }
    Ok(())
}

/// The engine's ranking: each group's places a bonus down from its winner's (the groups'
/// winners first, in the groups' order, then their runners-up), the place only some go
/// through on a smaller one, every team then one table sorted; those of that place who go
/// through a bonus by their group, sorted again.
fn by_place(
    state: &TournamentState,
    competition: &mut RunningCompetition,
    stage: usize,
    name: &impl Fn(i32) -> Option<Vec<u8>>,
) -> Result<(), NextStageError> {
    let (quotient, remainder) = through(state, stage)?;
    let groups = walked(competition.match_pairs_per_round());
    let per_group = walked(competition.total_matches_in_round());
    let mut place = 0;
    let mut first = PLACE_FIRST;
    for _ in 0..groups {
        let mut bonus = first;
        for position in 0..per_group {
            let row = place_row(competition, place)?;
            place += 1;
            if position == usize::from(quotient) {
                add_points(competition, row, PLACE_THROUGH);
            } else if position < usize::from(quotient) {
                add_points(competition, row, bonus.cast_unsigned());
                bonus = bonus.wrapping_sub(PLACE_STEP);
            }
        }
        first = first.wrapping_sub(GROUP_STEP);
    }
    sort_as_one(competition, name)?;
    if remainder != 0 {
        let skipped = usize::from(quotient.wrapping_mul(state.slot_groups(stage)));
        group_bonuses(
            state,
            competition,
            (skipped, walked(remainder.cast_signed())),
            PLACE_GROUPS,
            PLACE_GROUP_BONUS,
        )?;
        sort_table(competition, name)?;
    }
    Ok(())
}

/// A world's ranking, of the first stage only: its qualifiers group by group, a bonus down
/// from the first's, the place only some go through on a smaller one, every team then one
/// table sorted; its best four a bonus by their group, sorted again. A later stage's tables
/// are kept as they are.
fn by_group(
    state: &TournamentState,
    competition: &mut RunningCompetition,
    stage: usize,
    name: &impl Fn(i32) -> Option<Vec<u8>>,
) -> Result<(), NextStageError> {
    if state.round_complete() != 0 {
        return Ok(());
    }
    let (quotient, _) = through(state, stage)?;
    let groups = walked(competition.match_pairs_per_round());
    let per_group = walked(competition.total_matches_in_round());
    let mut place = 0;
    let mut bonus = GROUP_FIRST;
    for _ in 0..groups {
        for position in 0..per_group {
            let row = place_row(competition, place)?;
            place += 1;
            if position == usize::from(quotient) {
                add_points(competition, row, GROUP_THROUGH);
            } else if position < usize::from(quotient) {
                add_points(competition, row, bonus.cast_unsigned());
                bonus = bonus.wrapping_sub(GROUP_QUALIFIER_STEP);
            }
        }
    }
    sort_as_one(competition, name)?;
    group_bonuses(
        state,
        competition,
        (GROUP_BEST_FIRST, GROUP_BEST),
        GROUP_GROUPS,
        GROUP_GROUP_BONUS,
    )?;
    sort_table(competition, name)?;
    Ok(())
}

/// A knockout's end (upstream `DIY_CopyTeamGridFromCompToDIY`): its winners put first, the
/// first two ties swapped when the bracket or the next stage's is doubled, a world's ranking
/// applied (upstream ranks the competition's table whatever its stage); then the bracket's
/// places (doubled, a doubled bracket) the order the next stage draws from.
fn knockout_end(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    stage: usize,
    ranking: Ranking,
    name: &impl Fn(i32) -> Option<Vec<u8>>,
) -> Result<(), NextStageError> {
    let through = state.slot_division_factor(stage);
    if !(through == ONE && state.current_round_index() != 0) {
        winners_first(competition)?;
        let doubled = state.slot_double_count(stage) != 0
            || (through != ONE && next_doubled(state, stage)? != 0);
        if doubled {
            for place in 0..2 {
                let (first, second) = (
                    competition.grid_pairing(place),
                    competition.grid_pairing(place + 2),
                );
                competition.set_grid_pairing(place, second);
                competition.set_grid_pairing(place + 2, first);
            }
        }
        if ranking == Ranking::ByGroup {
            by_group(state, competition, stage, name)?;
        }
    }
    let mut count = competition.num_teams().cast_unsigned();
    if state.slot_double_count(stage) != 0 {
        count <<= 1;
    }
    let count = walked(count.cast_signed());
    if count > 0 && !TournamentState::has_team_index(count - 1) {
        return Err(NextStageError::TeamsPastState(count));
    }
    for place in 0..count {
        state.set_team_index(place, competition.grid_pairing(place));
    }
    Ok(())
}
