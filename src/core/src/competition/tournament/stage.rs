//! A tournament's current stage loaded into the competition under way (docs/PLAN.md M5 #4d2,
//! #7: upstream diy_competition.c `DIY_LoadCurrentCompetitionSlot`): a stage of groups started
//! as a league (`DIY_InitCompetitionDefaults`, `DIY_BuildTeamGridAndIndexMaps`,
//! `DIY_RecomputeCompetitionDerivedFields`), a knockout as a cup (`InitDIYCup`,
//! `DIY_ShuffleTeamsAndBuildCompTeamList`, `DIY_InitCupBracketState`), each with its draw: the
//! first stage's from the seeds, a later one's from the order the stage before left (its
//! groups' ranking, its winners), a world's own draw from its tables (`buildCompSpecificTeamList`,
//! `buildCompSpecificTeamShuffle`).

use openswos_rng::Rng;
use openswos_team_resource::TeamControls;
use thiserror::Error;

use super::own_draw::OwnDraw;
use super::start::ResetNames;
use super::{TEAMS, TournamentState};
use crate::competition::draw::{Pots, seed_by_value, shuffle};
use crate::competition::running::{RunningCompetition, TeamBytes};
use crate::competition::start::{LoadedTeam, copy_string};
use crate::competition::value::average_price;
use crate::competition::{cup, league};

/// The first stage: the one the seeds order (the state's `roundComplete`, which the stages'
/// count clears).
const FIRST: usize = 0;

/// The bytes of a standings row: a place's entry offset counts in them.
const ROW_BYTES: i16 = 18;

/// The fewest teams a knockout stage plays (upstream's months run on past its memory for
/// fewer).
const MIN_KNOCKOUT_TEAMS: u16 = 2;

/// What a stage's load takes from the world: the names the resets write first, the
/// tournament's own draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageRules<'a> {
    /// The names the resets write first.
    pub reset_names: ResetNames<'a>,
    /// The tournament's own draw (`rules/draws.json`): none for the engine's.
    pub own_draw: Option<&'a OwnDraw>,
}

/// Why a tournament's stage does not load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LoadError {
    /// A stage whose slot is past the tournament's state (upstream reads past it).
    #[error("stage {0} is past the tournament's state")]
    StagePastState(u16),
    /// A stage of no teams to play or more than the competition under way holds (upstream's
    /// months run on past its memory, or it reads past the tournament's state).
    #[error("stage {stage} of {teams} teams")]
    StageTeams {
        /// The stage.
        stage: usize,
        /// Its teams.
        teams: u16,
    },
    /// A stage of groups of no groups (upstream divides by 0).
    #[error("stage {0} of groups has none")]
    NoGroups(usize),
    /// A stage whose groups hold more teams than the state, or whose draw does not deal a team
    /// to each of its places (upstream reads past the state, or its draw's scratch).
    #[error("{groups} groups of {per_group} for a stage of {teams} teams")]
    GroupsNotTeams {
        /// Its groups.
        groups: u16,
        /// Its teams a group.
        per_group: u16,
        /// Its teams.
        teams: u16,
    },
    /// A stage of more teams than the world's own draw orders (upstream reads its draw's
    /// scratch).
    #[error("a stage of {teams} teams, its own draw ordering {places}")]
    OwnDrawPastPlaces {
        /// The places the own draw orders.
        places: usize,
        /// The stage's teams.
        teams: u16,
    },
    /// An odd knockout seeded by value (its home and away swap leaves a place to upstream's
    /// scratch).
    #[error("a seeded knockout of {0} teams")]
    OddSeededKnockout(u16),
    /// A stage's months past the tournament's state or the competition under way.
    #[error("the stage's {0} months run past the state")]
    MonthsPastState(u16),
    /// A best placed team of a group past upstream's shifts (below 0, or 31 and up).
    #[error("a best placed team of group {0}")]
    BestPlacedGroup(u8),
    /// No pattern of the own draw for the groups the best placed come from (upstream halts,
    /// 0x8034).
    #[error("no pattern for the best placed of groups {0:#010b} (upstream halts, 0x8034)")]
    NoBestPlacedPattern(u8),
    /// A stage of groups that does not start as a league.
    #[error(transparent)]
    League(#[from] league::StartError),
    /// A knockout stage that does not start as a cup.
    #[error(transparent)]
    Cup(#[from] cup::StartError),
    /// A team of the tournament that is not loaded.
    #[error("team {0:?} is not loaded")]
    UnknownTeam(TeamBytes),
}

/// Loads the current stage of `state` (its `roundComplete`) into `competition` (upstream
/// `DIY_LoadCurrentCompetitionSlot`), its draw from `rng`, by `rules`. A world's league phase
/// and state championships start their stages their own way (`LeaguePhase_StageStart`,
/// `LeaguePhase_KnockoutDraw`, `States_StageStart`, `States_KnockoutStart`): with World
/// 2026-27's components (M5 #9).
pub(super) fn load(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    rules: &StageRules<'_>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), LoadError> {
    let stage = slot(state.round_complete().cast_unsigned())?;
    let teams_count = state.slot_num_teams(stage);
    if usize::from(teams_count) > TEAMS {
        return Err(LoadError::StageTeams {
            stage,
            teams: teams_count,
        });
    }
    state.set_competition_finished(0);
    if state.slot_is_league_format(stage) != 0 {
        group_stage(state, competition, stage, rules, teams, rng)
    } else {
        knockout_stage(state, competition, stage, rules, teams, rng)
    }
}

/// The slot of stage `stage`, checked within the state.
pub(super) fn slot(stage: u16) -> Result<usize, LoadError> {
    let index = usize::from(stage);
    if TournamentState::has_slot(index) {
        Ok(index)
    } else {
        Err(LoadError::StagePastState(stage))
    }
}

/// A stage of groups started as a league.
fn group_stage(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    stage: usize,
    rules: &StageRules<'_>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), LoadError> {
    let groups = state.slot_groups(stage);
    let per_group = state.slot_teams_per_group(stage);
    let stage_teams = state.slot_num_teams(stage);
    if groups == 0 {
        return Err(LoadError::NoGroups(stage));
    }
    if usize::from(groups) * usize::from(per_group) > TEAMS {
        return Err(LoadError::GroupsNotTeams {
            groups,
            per_group,
            teams: stage_teams,
        });
    }
    let months = stage_months(state, stage)?;
    league::reset(competition, rules.reset_names.league);
    competition.set_has_groups(1);
    competition.set_match_started(state.match_started());
    competition.set_career_team_involved(state.career_team_involved());
    copy_string(competition, &state.name());
    competition.set_menu_state(state.round_complete());
    competition.set_round_iterator(state.total_matches_in_round());
    competition.set_comp_slot(state.comp_slot());
    competition.set_slot_id(state.slot_id());
    competition.set_num_teams(stage_teams.cast_signed());
    competition.set_match_pairs_per_round(groups.cast_signed());
    // Faithful to the original: the teams that go through, sign-extended, then divided as
    // unsigned.
    let through = i32::from(state.slot_division_factor(stage).cast_signed()).cast_unsigned();
    let quotient = through / u32::from(groups);
    competition.set_round_complete(low_word(quotient).cast_signed());
    if through % u32::from(groups) != 0 {
        competition.set_competition_finished(1);
    }
    for team in 0..stored_teams(state) {
        competition.set_league_team(team, state.team(team));
    }
    group_draw(state, competition, stage, rules.own_draw, teams, rng)?;
    competition.set_play_each_team(state.slot_play_each_team(stage).cast_signed());
    competition.set_start_month(state.slot_month(stage, 0).cast_signed());
    league::derive(competition, teams)?;
    copy_settings(state, competition);
    competition.set_points_for_win(state.slot_points_for_win(stage));
    competition.set_league_min_substitutes(state.round_iterator().cast_unsigned());
    competition.set_league_max_substitutes(state.num_teams_in_stage().cast_unsigned());
    for (day, month) in months.into_iter().enumerate() {
        competition.set_phase_start_month(day, month);
    }
    competition.set_comp_id(state.comp_id());
    competition.set_num_teams_in_stage(state.num_teams());
    Ok(())
}

/// A knockout stage started as a cup.
fn knockout_stage(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    stage: usize,
    rules: &StageRules<'_>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), LoadError> {
    let stage_teams = state.slot_num_teams(stage);
    if stage_teams < MIN_KNOCKOUT_TEAMS {
        return Err(LoadError::StageTeams {
            stage,
            teams: stage_teams,
        });
    }
    let months = stage_months(state, stage)?;
    cup::reset(competition, rules.reset_names.cup);
    competition.set_has_groups(1);
    competition.set_match_started(state.match_started());
    competition.set_career_team_involved(state.career_team_involved());
    copy_string(competition, &state.name());
    competition.set_num_rounds(1);
    competition.set_comp_slot(state.comp_slot());
    competition.set_slot_id(state.slot_id());
    competition.set_num_teams(stage_teams.cast_signed());
    competition.set_cup_round_legs(0, state.slot_legs(stage));
    competition.set_cup_round_extra_time(0, state.slot_extra_time(stage));
    competition.set_cup_round_penalties(0, state.slot_penalties(stage));
    for team in 0..stored_teams(state) {
        competition.set_cup_team(team, state.team(team));
    }
    knockout_draw(state, competition, stage, rules.own_draw, teams, rng)?;
    competition.set_start_month(state.slot_month(stage, 0).cast_signed());
    cup::bracket(competition, teams)?;
    competition.set_round_iterator(state.has_groups());
    copy_settings(state, competition);
    competition.set_play_each_team(state.menu_state());
    competition.set_min_substitutes(state.round_iterator().cast_unsigned());
    competition.set_points_for_win(state.num_teams_in_stage().cast_unsigned());
    competition.set_total_matches(state.slot_matches(stage));
    for (index, month) in months.into_iter().enumerate() {
        competition.set_prog_table(index, month);
    }
    competition.set_comp_id(state.comp_id());
    competition.set_num_teams_in_stage(state.num_teams());
    Ok(())
}

/// The teams the state holds, as upstream counts them to copy (`numTeams`; none below 0).
fn stored_teams(state: &TournamentState) -> usize {
    usize::try_from(state.num_teams()).unwrap_or_default()
}

/// A count as upstream walks it: a word that, as a signed one, below 1 walks none.
fn walked(count: u16) -> usize {
    usize::try_from(count.cast_signed()).unwrap_or_default()
}

/// `stage`'s months, from its table (none for a count below 0 as a word).
fn stage_months(state: &TournamentState, stage: usize) -> Result<Vec<u8>, LoadError> {
    let matches = state.slot_matches(stage);
    let count = walked(matches);
    if count > 0 && !TournamentState::has_month(stage, count - 1) {
        return Err(LoadError::MonthsPastState(matches));
    }
    Ok((0..count)
        .map(|index| state.slot_month(stage, index))
        .collect())
}

/// The settings the stage takes from the tournament: the strength, the pitch, the months,
/// the suspensions' track.
fn copy_settings(state: &TournamentState, competition: &mut RunningCompetition) {
    competition.set_avg_skill(state.avg_skill());
    competition.set_strength_tier(state.strength_tier());
    competition.set_pitch_type_or_season(state.pitch_type_or_season());
    competition.set_pitch_type(state.pitch_type());
    competition.set_start_month(state.start_month());
    competition.set_end_month(state.end_month());
    competition.set_substitutes_count(state.substitutes_count());
}

/// The low word of `value`.
fn low_word(value: u32) -> u16 {
    let [low, high, _, _] = value.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// The first `count` places of the state's team order.
fn order(state: &TournamentState, count: usize) -> Vec<u8> {
    (0..count).map(|place| state.team_index(place)).collect()
}

/// Writes `order` over the state's team order from its first place.
fn set_order(state: &mut TournamentState, order: &[u8]) {
    for (place, &index) in order.iter().enumerate() {
        state.set_team_index(place, index);
    }
}

/// Each team's value, by its index, for the teams `order` names (upstream
/// `GetAveragePlayerPrice` of the team `VerifyTeamLoaded` gives).
fn values(
    state: &TournamentState,
    order: &[u8],
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
) -> Result<Vec<u16>, LoadError> {
    let mut values = vec![0; TEAMS];
    for &index in order {
        let team = state.team(usize::from(index));
        let loaded = teams(team).ok_or(LoadError::UnknownTeam(team))?;
        values[usize::from(index)] = average_price(&loaded.prices);
    }
    Ok(values)
}

/// The stage's order from the world's own draw: its first places, as many as the stage's
/// teams, checked within what the draw orders.
fn own_drawn(
    state: &TournamentState,
    own_draw: &OwnDraw,
    stage: usize,
) -> Result<Vec<u8>, LoadError> {
    let teams = state.slot_num_teams(stage);
    let mut drawn = own_draw.stage(stage).drawn(state)?;
    if usize::from(teams) > drawn.len() {
        return Err(LoadError::OwnDrawPastPlaces {
            places: drawn.len(),
            teams,
        });
    }
    drawn.truncate(usize::from(teams));
    Ok(drawn)
}

/// Whether the stage keeps the seeds' order: a first stage they ordered.
fn seeds_kept(state: &TournamentState, stage: usize) -> bool {
    stage == FIRST && state.match_pairs_per_round() != 0
}

/// The group stage's draw and maps (upstream `DIY_BuildTeamGridAndIndexMaps`): the world's own
/// draw, or its order kept, shuffled, or dealt into the groups (seeded by value into pots
/// first, unless the stage before was of groups too: its ranking's order dealt each of its
/// groups' teams on a group); each place's index and row offset into the league; each team's
/// group.
fn group_draw(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    stage: usize,
    own_draw: Option<&OwnDraw>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), LoadError> {
    let count = state.slot_num_teams(stage);
    let places = usize::from(count);
    let groups = state.slot_groups(stage);
    let per_group = state.slot_teams_per_group(stage);
    let dealt_teams = usize::from(groups) * usize::from(per_group);
    if let Some(own_draw) = own_draw {
        let drawn = own_drawn(state, own_draw, stage)?;
        set_order(state, &drawn);
    } else if !seeds_kept(state, stage) {
        if state.slot_seeding_type(stage) == 0 {
            let mut drawn = order(state, places);
            shuffle(&mut drawn, rng);
            set_order(state, &drawn);
        } else {
            let not_dealt = LoadError::GroupsNotTeams {
                groups,
                per_group,
                teams: count,
            };
            let after_groups = stage != FIRST && state.slot_is_league_format(stage - 1) != 0;
            let shift = if after_groups {
                // Faithful to the original: the groups before as a signed word, none below 0.
                u16::try_from(state.slot_groups(stage - 1).cast_signed()).ok()
            } else {
                if dealt_teams != places {
                    return Err(not_dealt);
                }
                let mut drawn = order(state, places);
                let values = values(state, &drawn, teams)?;
                seed_by_value(&mut drawn, &values, Pots::Groups { groups, per_group }, rng)
                    .expect("the pots are the teams");
                set_order(state, &drawn);
                None
            };
            let dealt = deal(&order(state, dealt_teams), places, groups, per_group, shift)
                .ok_or(not_dealt)?;
            set_order(state, &dealt);
        }
    }
    for (place, index) in order(state, places).into_iter().enumerate() {
        competition.set_shuffle_and_pad(place, index);
        let offset = i16::from(index.cast_signed()) * ROW_BYTES;
        competition.set_team_entry_offset(place, offset.cast_unsigned());
    }
    let mut placed = order(state, dealt_teams).into_iter();
    for group in 0..groups {
        for index in placed.by_ref().take(usize::from(per_group)) {
            // Faithful to the original: the group's number is a byte.
            state.set_group(usize::from(index), group.to_le_bytes()[0]);
        }
    }
    Ok(())
}

/// `drawn`, pot by pot, dealt into `places` places of `groups` groups of `per_group`: the
/// pots' teams go to the groups in turn, a place of each group a pot. After a stage of groups,
/// `shift` is its groups: each of them a pot moves the next place on by a group. `None` when
/// a place is dealt no team (upstream leaves it to its scratch) or the places are none
/// (upstream's wrap never ends).
fn deal(
    drawn: &[u8],
    places: usize,
    groups: u16,
    per_group: u16,
    shift: Option<u16>,
) -> Option<Vec<u8>> {
    let (rounds, turns) = (walked(per_group), walked(groups));
    if rounds == 0 || turns == 0 {
        return (places == 0).then(Vec::new);
    }
    if places == 0 {
        return None;
    }
    let mut dealt = vec![None; places];
    let mut next = drawn.iter();
    let mut place = 0;
    let mut since: u16 = 0;
    for _ in 0..rounds {
        if shift.is_some() && since == 0 {
            place += usize::from(per_group);
        }
        for _ in 0..turns {
            place %= places;
            dealt[place] = Some(*next.next().expect("a team a place"));
            since = since.wrapping_add(1);
            if Some(since) == shift {
                since = 0;
            }
            place += usize::from(per_group);
        }
        place += 1;
    }
    dealt.into_iter().collect()
}

/// The knockout's draw and bracket list (upstream `DIY_ShuffleTeamsAndBuildCompTeamList`): the
/// world's own draw, or its order kept, shuffled, or each pair's home side drawn (seeded by
/// value into quarters first, unless the stage before was of groups or seeded: its order
/// kept); its places (doubled, a doubled bracket) into the pairings; the teams no computer
/// controls counted.
fn knockout_draw(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    stage: usize,
    own_draw: Option<&OwnDraw>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), LoadError> {
    let count = state.slot_num_teams(stage);
    let places = usize::from(count);
    if let Some(own_draw) = own_draw {
        let drawn = own_drawn(state, own_draw, stage)?;
        set_order(state, &drawn);
    } else if !seeds_kept(state, stage) {
        let mut drawn = order(state, places);
        if state.slot_seeding_type(stage) == 0 {
            shuffle(&mut drawn, rng);
        } else {
            if places % 2 != 0 {
                return Err(LoadError::OddSeededKnockout(count));
            }
            let ordered = stage != FIRST
                && (state.slot_is_league_format(stage - 1) != 0
                    || state.slot_seeding_type(stage - 1) != 0);
            if !ordered {
                let values = values(state, &drawn, teams)?;
                seed_by_value(&mut drawn, &values, Pots::Quarters, rng)
                    .expect("quarters within the teams");
            }
            drawn = home_and_away(&drawn, rng);
        }
        set_order(state, &drawn);
    }
    let listed = if state.slot_double_count(stage) != 0 {
        places * 2
    } else {
        places
    };
    if listed > TEAMS {
        return Err(LoadError::StageTeams {
            stage,
            teams: count,
        });
    }
    let listed_order = order(state, listed);
    for (place, &index) in listed_order.iter().enumerate() {
        competition.set_grid_pairing(place, index);
    }
    let mut not_computer: i16 = 0;
    for &index in &listed_order {
        let team = state.team(usize::from(index));
        let loaded = teams(team).ok_or(LoadError::UnknownTeam(team))?;
        if loaded.controls != TeamControls::Computer {
            not_computer += 1;
        }
    }
    state.set_has_groups(not_computer);
    Ok(())
}

/// Each pair of `drawn` (its first and its last, inwards), the side drawn at home: the last
/// first on an odd draw.
fn home_and_away(drawn: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut paired = Vec::with_capacity(drawn.len());
    let (mut low, mut high) = (0, drawn.len());
    for _ in 0..drawn.len() / 2 {
        high -= 1;
        if rng.rand2() & 1 != 0 {
            paired.extend([drawn[high], drawn[low]]);
        } else {
            paired.extend([drawn[low], drawn[high]]);
        }
        low += 1;
    }
    paired
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pots_are_dealt_a_place_of_each_group_a_pot() {
        // Two groups of three: pot 1 (0, 1), pot 2 (2, 3), pot 3 (4, 5).
        assert_eq!(
            deal(&[0, 1, 2, 3, 4, 5], 6, 2, 3, None),
            Some(vec![0, 2, 4, 1, 3, 5])
        );
        assert_eq!(deal(&[7, 8], 2, 1, 2, None), Some(vec![7, 8]));
    }

    #[test]
    fn after_groups_each_of_their_groups_a_pot_moves_the_places_on() {
        // Two groups of two after two groups: each pot starts a group on.
        assert_eq!(
            deal(&[0, 1, 2, 3], 4, 2, 2, Some(2)),
            Some(vec![1, 2, 0, 3])
        );
        // Two teams dealt into four places: places 0 and 3 never dealt.
        assert_eq!(deal(&[0, 1], 4, 1, 2, Some(2)), None);
    }

    #[test]
    fn a_deal_leaving_a_place_or_of_no_places_is_refused() {
        assert_eq!(deal(&[0, 1], 3, 1, 2, None), None);
        assert_eq!(deal(&[0, 1], 0, 1, 2, None), None);
        assert_eq!(deal(&[], 0, 0, 2, None), Some(Vec::new()));
        assert_eq!(deal(&[], 2, 0, 2, None), None);
    }

    #[test]
    fn a_pair_s_home_side_is_drawn() {
        let mut heads = Rng::default();
        heads.fixed = Some(1);
        assert_eq!(home_and_away(&[0, 1, 2, 3], &mut heads), [3, 0, 2, 1]);
        let mut tails = Rng::default();
        tails.fixed = Some(0);
        assert_eq!(home_and_away(&[0, 1, 2, 3], &mut tails), [0, 3, 1, 2]);
    }

    #[test]
    fn a_stage_past_the_state_is_refused() {
        assert_eq!(slot(6), Ok(6));
        assert_eq!(slot(7), Err(LoadError::StagePastState(7)));
    }
}
