//! A tournament's first stage loaded into the competition under way (docs/PLAN.md M5 #4d2:
//! upstream diy_competition.c `DIY_LoadCurrentCompetitionSlot` for the stage the start leaves
//! current): a stage of groups started as a league (`DIY_InitCompetitionDefaults`,
//! `DIY_BuildTeamGridAndIndexMaps`, `DIY_RecomputeCompetitionDerivedFields`), a knockout as a
//! cup (`InitDIYCup`, `DIY_ShuffleTeamsAndBuildCompTeamList`, `DIY_InitCupBracketState`), each
//! with its draw. The later stages' loads come with M5 #7.

use openswos_rng::Rng;
use openswos_team_resource::TeamControls;

use super::start::{StartError, TournamentStart};
use super::{TEAMS, TournamentState};
use crate::competition::draw::{Pots, seed_by_value, shuffle};
use crate::competition::running::{RunningCompetition, TeamBytes};
use crate::competition::start::{LoadedTeam, copy_string};
use crate::competition::value::average_price;
use crate::competition::{cup, league};

/// The stage the start leaves current (the state's `roundComplete`, which the stages' count
/// clears).
const FIRST: usize = 0;

/// The bytes of a standings row: a place's entry offset counts in them.
const ROW_BYTES: i16 = 18;

/// The fewest teams a knockout stage plays (upstream's months run on past its memory for
/// fewer).
const MIN_KNOCKOUT_TEAMS: u16 = 2;

/// Loads the first stage of `state` into `competition` (upstream
/// `DIY_LoadCurrentCompetitionSlot`), its draw from `rng`. A world's league phase and state
/// championships start their stages their own way (`LeaguePhase_StageStart`,
/// `LeaguePhase_KnockoutDraw`, `States_StageStart`, `States_KnockoutStart`): with World
/// 2026-27's components (M5 #9).
pub(super) fn load_first(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    start: &TournamentStart<'_>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), StartError> {
    let teams_count = state.slot_num_teams(FIRST);
    if usize::from(teams_count) > TEAMS {
        return Err(StartError::FirstStageTeams(teams_count));
    }
    state.set_competition_finished(0);
    if state.slot_is_league_format(FIRST) != 0 {
        group_stage(state, competition, start, teams, rng)
    } else {
        knockout_stage(state, competition, start, teams, rng)
    }
}

/// The first stage of groups started as a league.
fn group_stage(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    start: &TournamentStart<'_>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), StartError> {
    let groups = state.slot_groups(FIRST);
    let per_group = state.slot_teams_per_group(FIRST);
    let stage_teams = state.slot_num_teams(FIRST);
    if usize::from(groups) * usize::from(per_group) > TEAMS {
        return Err(StartError::GroupsNotTeams {
            groups,
            per_group,
            teams: stage_teams,
        });
    }
    let months = stage_months(state)?;
    league::reset(competition, start.reset_names.league);
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
    let through = i32::from(state.slot_division_factor(FIRST).cast_signed()).cast_unsigned();
    let quotient = through / u32::from(groups);
    competition.set_round_complete(low_word(quotient).cast_signed());
    if through % u32::from(groups) != 0 {
        competition.set_competition_finished(1);
    }
    for team in 0..stored_teams(state) {
        competition.set_league_team(team, state.team(team));
    }
    group_draw(state, competition, start.own_draw, teams, rng)?;
    competition.set_play_each_team(state.slot_play_each_team(FIRST).cast_signed());
    competition.set_start_month(state.slot_month(FIRST, 0).cast_signed());
    league::derive(competition, teams)?;
    copy_settings(state, competition);
    competition.set_points_for_win(state.slot_points_for_win(FIRST));
    competition.set_league_min_substitutes(state.round_iterator().cast_unsigned());
    competition.set_league_max_substitutes(state.num_teams_in_stage().cast_unsigned());
    for (day, month) in months.into_iter().enumerate() {
        competition.set_phase_start_month(day, month);
    }
    competition.set_comp_id(state.comp_id());
    competition.set_num_teams_in_stage(state.num_teams());
    Ok(())
}

/// The knockout first stage started as a cup.
fn knockout_stage(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    start: &TournamentStart<'_>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), StartError> {
    let stage_teams = state.slot_num_teams(FIRST);
    if stage_teams < MIN_KNOCKOUT_TEAMS {
        return Err(StartError::FirstStageTeams(stage_teams));
    }
    let months = stage_months(state)?;
    cup::reset(competition, start.reset_names.cup);
    competition.set_has_groups(1);
    competition.set_match_started(state.match_started());
    competition.set_career_team_involved(state.career_team_involved());
    copy_string(competition, &state.name());
    competition.set_num_rounds(1);
    competition.set_comp_slot(state.comp_slot());
    competition.set_slot_id(state.slot_id());
    competition.set_num_teams(stage_teams.cast_signed());
    competition.set_cup_round_legs(0, state.slot_legs(FIRST));
    competition.set_cup_round_extra_time(0, state.slot_extra_time(FIRST));
    competition.set_cup_round_penalties(0, state.slot_penalties(FIRST));
    for team in 0..stored_teams(state) {
        competition.set_cup_team(team, state.team(team));
    }
    knockout_draw(state, competition, start.own_draw, teams, rng)?;
    competition.set_start_month(state.slot_month(FIRST, 0).cast_signed());
    cup::bracket(competition, teams)?;
    competition.set_round_iterator(state.has_groups());
    copy_settings(state, competition);
    competition.set_play_each_team(state.menu_state());
    competition.set_min_substitutes(state.round_iterator().cast_unsigned());
    competition.set_points_for_win(state.num_teams_in_stage().cast_unsigned());
    competition.set_total_matches(state.slot_matches(FIRST));
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

/// The first stage's months, from its table (none for a count below 0 as a word).
fn stage_months(state: &TournamentState) -> Result<Vec<u8>, StartError> {
    let matches = state.slot_matches(FIRST);
    let count = usize::try_from(matches.cast_signed()).unwrap_or_default();
    if count > 0 && !TournamentState::has_month(FIRST, count - 1) {
        return Err(StartError::MonthsPastState(matches));
    }
    Ok((0..count)
        .map(|index| state.slot_month(FIRST, index))
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
) -> Result<Vec<u16>, StartError> {
    let mut values = vec![0; TEAMS];
    for &index in order {
        let team = state.team(usize::from(index));
        let loaded = teams(team).ok_or(StartError::UnknownTeam(team))?;
        values[usize::from(index)] = average_price(&loaded.prices);
    }
    Ok(values)
}

/// Whether the stage keeps its order: its own draw's first stage (checked to cover its teams),
/// or a first stage the seeds ordered.
fn keeps_order(state: &TournamentState, own_draw: Option<u8>) -> Result<bool, StartError> {
    let teams = state.slot_num_teams(FIRST);
    if let Some(places) = own_draw {
        if teams > u16::from(places) {
            return Err(StartError::OwnDrawPastPlaces { places, teams });
        }
        return Ok(true);
    }
    Ok(state.round_complete() == 0 && state.match_pairs_per_round() != 0)
}

/// The group stage's draw and maps (upstream `DIY_BuildTeamGridAndIndexMaps`): its order kept,
/// shuffled, or seeded by value into pots then dealt into the groups; each place's index and
/// row offset into the league; each team's group.
fn group_draw(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    own_draw: Option<u8>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), StartError> {
    let count = state.slot_num_teams(FIRST);
    let places = usize::from(count);
    let groups = state.slot_groups(FIRST);
    let per_group = state.slot_teams_per_group(FIRST);
    if !keeps_order(state, own_draw)? {
        let mut drawn = order(state, places);
        if state.slot_seeding_type(FIRST) == 0 {
            shuffle(&mut drawn, rng);
        } else {
            if usize::from(groups) * usize::from(per_group) != places {
                return Err(StartError::GroupsNotTeams {
                    groups,
                    per_group,
                    teams: count,
                });
            }
            let values = values(state, &drawn, teams)?;
            seed_by_value(&mut drawn, &values, Pots::Groups { groups, per_group }, rng)
                .expect("the pots are the teams");
            drawn = deal(&drawn, groups, per_group);
        }
        set_order(state, &drawn);
    }
    for (place, index) in order(state, places).into_iter().enumerate() {
        competition.set_shuffle_and_pad(place, index);
        let offset = i16::from(index.cast_signed()) * ROW_BYTES;
        competition.set_team_entry_offset(place, offset.cast_unsigned());
    }
    let dealt = order(state, usize::from(groups) * usize::from(per_group));
    let mut placed = dealt.into_iter();
    for group in 0..groups {
        for index in placed.by_ref().take(usize::from(per_group)) {
            // Faithful to the original: the group's number is a byte.
            state.set_group(usize::from(index), group.to_le_bytes()[0]);
        }
    }
    Ok(())
}

/// `drawn`, pot by pot, dealt into `groups` groups of `per_group`: the pots' teams go to the
/// groups in turn, a place of each group a pot.
fn deal(drawn: &[u8], groups: u16, per_group: u16) -> Vec<u8> {
    let places = drawn.len();
    let mut dealt = vec![0; places];
    let mut next = drawn.iter();
    let mut place = 0;
    for _ in 0..per_group {
        for _ in 0..groups {
            place %= places;
            dealt[place] = *next.next().expect("a team a place");
            place += usize::from(per_group);
        }
        place += 1;
    }
    dealt
}

/// The knockout's draw and bracket list (upstream `DIY_ShuffleTeamsAndBuildCompTeamList`): its
/// order kept, shuffled, or seeded by value into quarters and each pair's home side drawn;
/// its places (doubled, a doubled bracket) into the pairings; the teams no computer controls
/// counted.
fn knockout_draw(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    own_draw: Option<u8>,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), StartError> {
    let count = state.slot_num_teams(FIRST);
    let places = usize::from(count);
    if !keeps_order(state, own_draw)? {
        let mut drawn = order(state, places);
        if state.slot_seeding_type(FIRST) == 0 {
            shuffle(&mut drawn, rng);
        } else {
            if places % 2 != 0 {
                return Err(StartError::OddSeededKnockout(count));
            }
            let values = values(state, &drawn, teams)?;
            seed_by_value(&mut drawn, &values, Pots::Quarters, rng)
                .expect("quarters within the teams");
            drawn = home_and_away(&drawn, rng);
        }
        set_order(state, &drawn);
    }
    let listed = if state.slot_double_count(FIRST) != 0 {
        places * 2
    } else {
        places
    };
    if listed > TEAMS {
        return Err(StartError::FirstStageTeams(count));
    }
    let listed_order = order(state, listed);
    for (place, &index) in listed_order.iter().enumerate() {
        competition.set_grid_pairing(place, index);
    }
    let mut not_computer: i16 = 0;
    for &index in &listed_order {
        let team = state.team(usize::from(index));
        let loaded = teams(team).ok_or(StartError::UnknownTeam(team))?;
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
        assert_eq!(deal(&[0, 1, 2, 3, 4, 5], 2, 3), [0, 2, 4, 1, 3, 5]);
        assert_eq!(deal(&[7, 8], 1, 2), [7, 8]);
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
}
