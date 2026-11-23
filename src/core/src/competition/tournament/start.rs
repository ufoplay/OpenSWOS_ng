//! A tournament started (docs/PLAN.md M5 #4d2): its record made a tournament's state, the
//! seeds ordering its first stage, its stages into the slots, then counted (upstream
//! diy_competition.c `InitDIYLeagueFromRecord`, the type-2 branch of
//! `InitCompetitionFromRecord`), and its first stage loaded into the competition under way
//! ([`super::stage`]).

use openswos_rng::Rng;
use thiserror::Error;

use super::own_draw::OwnDraw;
use super::record::{SEEDS, TournamentRecord};
use super::stage::{LoadError, StageRules, load};
use super::{STAGES, StagesError, TEAMS, TournamentState, recalculate, reset};
use crate::competition::running::{RunningCompetition, TeamBytes};
use crate::competition::start::{LoadedTeam, NIBBLE_MASK, NIBBLE_SHIFT, Pitch, named};

/// How far a round rule byte's legs are shifted (its bits 7-6).
const LEGS_SHIFT: u8 = 6;

/// How far its extra time is shifted (bits 5-4).
const EXTRA_TIME_SHIFT: u8 = 4;

/// How far its penalties are shifted (bits 3-2).
const PENALTIES_SHIFT: u8 = 2;

/// A rule's two bits.
const RULE_MASK: u8 = 0b11;

/// The names the resets write first: the tournament's, and each stage's, a league's or a
/// cup's (upstream `_aDiyTournament`, `_aDiyLeague`, `_aDiyCup`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetNames<'a> {
    /// The tournament's reset's.
    pub tournament: &'a [u8],
    /// A league's reset's: a stage of groups.
    pub league: &'a [u8],
    /// A cup's reset's: a knockout stage.
    pub cup: &'a [u8],
}

/// What a tournament is started from: its record, and what the world's data resolves for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TournamentStart<'a> {
    /// The tournament's record.
    pub record: &'a TournamentRecord,
    /// Its name (the record's first, else the default tournament name).
    pub name: &'a [u8],
    /// The country's adjective before the name: none for a national teams' competition or a
    /// record of no country.
    pub prefix: Option<&'a [u8]>,
    /// The names the resets write first.
    pub reset_names: ResetNames<'a>,
    /// Whether its suspensions are the continental track's (upstream `continental_record`).
    pub continental: bool,
    /// Its own draw, when the world draws its stages by its own tables (`rules/draws.json`;
    /// upstream `buildCompSpecificTeamList`, `buildCompSpecificTeamShuffle`, picked by
    /// competition id).
    pub own_draw: Option<&'a OwnDraw>,
    /// Whether its groups may hold an odd number of teams (`rules/fixtures.json`; upstream
    /// halts in the original's world).
    pub odd_groups: bool,
    /// The session's pitch.
    pub pitch: Pitch,
}

impl<'a> TournamentStart<'a> {
    /// What each of its stages' loads takes ([`super::next_stage`]'s too).
    #[must_use]
    pub fn stage_rules(&self) -> StageRules<'a> {
        StageRules {
            reset_names: self.reset_names,
            own_draw: self.own_draw,
        }
    }
}

/// Why a tournament does not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StartError {
    /// Stages other than 1 to 6 (upstream writes slots past its six, or counts months for no
    /// match and runs past its memory).
    #[error("a tournament of {0} stages")]
    Stages(i8),
    /// A stage of groups of an odd number of teams where the world refuses them (upstream
    /// halts, 0x1313).
    #[error("stage {stage}'s groups have {teams} teams (upstream halts, 0x1313)")]
    OddGroups {
        /// The stage.
        stage: usize,
        /// Its teams a group.
        teams: u8,
    },
    /// A first stage ordered by its seeds with more teams than the seeds' list (upstream reads
    /// past the record).
    #[error("a first stage of {0} teams ordered by 32 seeds")]
    SeedsPastList(i8),
    /// The stages cannot be counted.
    #[error(transparent)]
    Counted(#[from] StagesError),
    /// The first stage does not load.
    #[error(transparent)]
    Load(#[from] LoadError),
}

/// Starts `start`'s tournament (upstream `InitDIYLeagueFromRecord`): the reset of `state`,
/// the record's fields, its seeds ordering the first stage's teams (`state`'s team slots, set
/// before, found by `teams`) when it says so, its stages into the slots, the stages counted
/// ([`recalculate`]), then its first stage loaded into `competition`, drawn by `rng`.
///
/// # Errors
///
/// [`StartError`] when the record's stages are not 1 to 6, a stage's groups are odd where the
/// world refuses them, the first stage cannot be ordered, counted, drawn or started, or a team
/// is not loaded.
pub fn start_tournament(
    state: &mut TournamentState,
    competition: &mut RunningCompetition,
    start: &TournamentStart<'_>,
    teams: impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), StartError> {
    let record = start.record;
    let stages = usize::try_from(record.stage_count)
        .ok()
        .filter(|stages| (1..=STAGES).contains(stages))
        .ok_or(StartError::Stages(record.stage_count))?;
    if !start.odd_groups {
        for stage in 0..stages {
            let [_, groups, per_group, _] = record.stage(stage);
            if groups.cast_signed() > 0 && per_group & 1 != 0 {
                return Err(StartError::OddGroups {
                    stage,
                    teams: per_group,
                });
            }
        }
    }
    let [first, ..] = record.stage(0);
    let first_teams = first.cast_signed();
    let seeds = i8::try_from(SEEDS).expect("32 seeds");
    if record.seed_reorder != 0 && first_teams > seeds {
        return Err(StartError::SeedsPastList(first_teams));
    }
    reset(state, start.reset_names.tournament);
    state.set_substitutes_count(i16::from(!start.continental));
    state.set_comp_id(record.comp_id);
    state.set_match_pairs_per_round(0);
    if record.seed_reorder != 0 {
        state.set_match_pairs_per_round(1);
        order_by_seeds(state, record, first_teams);
    }
    state.set_start_month(record.begin_month);
    state.set_end_month(record.end_month);
    match start.pitch {
        Pitch::Seasonal => state.set_pitch_type_or_season(1),
        Pitch::Type(pitch_type) => {
            state.set_pitch_type_or_season(0);
            state.set_pitch_type(pitch_type);
        }
    }
    state.set_num_teams(i16::from(first_teams));
    state.set_name(named(state.name(), start.prefix, start.name));
    state.set_menu_state(i16::from(record.menu_state));
    state.set_round_iterator(i16::from(record.substitutes >> NIBBLE_SHIFT));
    state.set_num_teams_in_stage(i16::from(record.substitutes & NIBBLE_MASK));
    state.set_slot_pad(0, 0);
    state.set_total_matches_in_round(i16::from(record.stage_count));
    for stage in 0..stages {
        write_stage(state, record, stage);
    }
    // The slot after the last carries the teams left at the end: the last stage's that go
    // through.
    if stages != STAGES {
        state.set_slot_num_teams(stages, state.slot_division_factor(stages - 1));
    }
    recalculate(state, competition, &teams)?;
    load(state, competition, &start.stage_rules(), &teams, rng)?;
    Ok(())
}

/// The first stage's teams ordered by the record's seeds: each seed found among them is moved
/// to its seed's place, the team in that place to the seed's.
fn order_by_seeds(state: &mut TournamentState, record: &TournamentRecord, first_teams: i8) {
    let count = usize::try_from(first_teams).unwrap_or_default();
    for (place, &seed) in record.seeds.iter().enumerate().take(count) {
        let Some(index) = (0..count).find(|&index| state.team(index) == seed) else {
            continue;
        };
        let index = u8::try_from(index).expect("32 teams at most");
        if index != state.team_index(place) {
            let at = (0..TEAMS)
                .find(|&at| state.team_index(at) == index)
                .expect("the order holds every team once");
            let moved = state.team_index(place);
            state.set_team_index(at, moved);
            state.set_team_index(place, index);
        }
    }
}

/// Stage `stage` of `record` into its slot: its seeding, a group's points and plays, its
/// teams, its groups (a byte below 0 a doubled knockout), its teams a group, the teams that go
/// through, its rules.
fn write_stage(state: &mut TournamentState, record: &TournamentRecord, stage: usize) {
    let [teams, groups, per_group, through] = record.stage(stage);
    let widened = |byte: i8| i16::from(byte).cast_unsigned();
    state.set_slot_seeding_type(stage, widened(record.seeding_type));
    state.set_slot_points_for_win(stage, widened(record.away_goals_mode));
    state.set_slot_play_each_team(stage, widened(record.play_each_team));
    state.set_slot_num_teams(stage, u16::from(teams));
    state.set_slot_double_count(stage, 0);
    let mut format = groups;
    if groups.cast_signed() < 0 {
        state.set_slot_pad(0, 1);
        state.set_slot_double_count(stage, 1);
        format = 0;
    }
    state.set_slot_is_league_format(stage, u16::from(format));
    state.set_slot_groups(stage, u16::from(groups));
    state.set_slot_teams_per_group(stage, u16::from(per_group));
    state.set_slot_division_factor(stage, u16::from(through));
    let rules = record.round_rules[stage];
    let rule = |shift: u8| u16::from(rules >> shift & RULE_MASK);
    state.set_slot_legs(stage, rule(LEGS_SHIFT));
    state.set_slot_extra_time(stage, rule(EXTRA_TIME_SHIFT));
    state.set_slot_penalties(stage, rule(PENALTIES_SHIFT));
}

#[cfg(test)]
mod tests {
    use openswos_team_resource::TeamControls;

    use super::*;
    use crate::competition::tournament::own_draw::{Ranking, StageDraw};
    use crate::competition::tournament::record::STAGES_BYTES;

    /// One leg settled on penalties.
    const ONE_LEG: u8 = 0x54;

    /// A record of 8 teams in 2 groups of 4, then a knockout of 4 and one of 2, its seeds
    /// `[1, 7]` then `[1, 6]`, each group playing once.
    fn record() -> TournamentRecord {
        let mut stages = [0; STAGES_BYTES];
        stages[..9].copy_from_slice(&[8, 2, 4, 4, 0, 4, 2, 0, 2]);
        stages[9] = 1;
        let mut seeds = [[0; 2]; SEEDS];
        seeds[0] = [1, 7];
        seeds[1] = [1, 6];
        TournamentRecord {
            comp_id: 40,
            begin_month: 0x40,
            end_month: 0x60,
            seed_reorder: 1,
            seeding_type: 1,
            menu_state: 1,
            play_each_team: 1,
            away_goals_mode: 3,
            substitutes: 0x35,
            stage_count: 3,
            stages,
            round_rules: [0, ONE_LEG, ONE_LEG, 0, 0, 0],
            seeds,
        }
    }

    fn start(record: &TournamentRecord) -> TournamentStart<'_> {
        TournamentStart {
            record,
            name: b"WORLD CUP",
            prefix: None,
            reset_names: ResetNames {
                tournament: b"DIY TOURNAMENT",
                league: b"DIY LEAGUE",
                cup: b"DIY CUP",
            },
            continental: false,
            own_draw: None,
            odd_groups: false,
            pitch: Pitch::Seasonal,
        }
    }

    /// Each team `[1, n]` the selected team `n`, priced `n`, the computer's.
    fn teams(team: TeamBytes) -> Option<LoadedTeam> {
        let [_, ordinal] = team;
        Some(LoadedTeam {
            identity: i32::from(ordinal),
            controls: TeamControls::Computer,
            prices: [ordinal; 16],
        })
    }

    /// A state whose teams are `[1, 0]` to `[1, 63]`.
    fn state() -> TournamentState {
        let mut state = TournamentState::default();
        for ordinal in 0..64 {
            state.set_team(usize::from(ordinal), [1, ordinal]);
        }
        state
    }

    fn started(
        start: &TournamentStart<'_>,
    ) -> Result<(TournamentState, RunningCompetition), StartError> {
        let mut state = state();
        let mut competition = RunningCompetition::default();
        start_tournament(
            &mut state,
            &mut competition,
            start,
            teams,
            &mut Rng::default(),
        )?;
        Ok((state, competition))
    }

    #[test]
    fn the_record_s_stages_go_into_the_slots_and_the_seeds_order_the_teams() {
        let record = record();
        let (state, competition) = started(&start(&record)).expect("started");
        assert_eq!(state.comp_id(), 40);
        assert_eq!((state.total_matches_in_round(), state.num_teams()), (3, 8));
        assert_eq!(&state.name()[..10], b"WORLD CUP\0");
        assert_eq!((state.round_iterator(), state.num_teams_in_stage()), (3, 5));
        let stage = |stage| {
            [
                state.slot_num_teams(stage),
                state.slot_groups(stage),
                state.slot_teams_per_group(stage),
                state.slot_division_factor(stage),
            ]
        };
        assert_eq!(stage(0), [8, 2, 4, 4]);
        assert_eq!(stage(2), [2, 0, 2, 1]);
        assert_eq!(state.slot_num_teams(3), 1, "the teams left at the end");
        assert_eq!(state.slot_points_for_win(0), 3);
        assert_eq!(state.slot_penalties(1), 1);
        // The seeds first, in their order: their teams swapped with the first places'.
        let order: Vec<u8> = (0..8).map(|place| state.team_index(place)).collect();
        assert_eq!(order, [7, 6, 2, 3, 4, 5, 1, 0]);
        // Two groups, by the order.
        assert_eq!((state.group(7), state.group(3), state.group(4)), (0, 0, 1));
        // The first stage under way, a league of two groups.
        assert_eq!(
            (competition.competition_type(), competition.has_groups()),
            (0, 1)
        );
        assert_eq!(competition.match_pairs_per_round(), 2);
        assert_eq!(
            competition.round_complete(),
            2,
            "four go through, two a group"
        );
        assert_eq!(competition.points_for_win(), 3);
        assert_eq!(&competition.name()[..10], b"WORLD CUP\0");
        assert_eq!(competition.shuffle_and_pad(0), 7);
        assert_eq!(competition.team_entry_offset(0), 7 * 18);
    }

    #[test]
    fn a_knockout_first_stage_starts_as_a_cup() {
        let mut record = record();
        record.stages[..3].copy_from_slice(&[4, 0, 4]);
        record.stage_count = 1;
        record.seed_reorder = 0;
        record.seeding_type = 0;
        let (state, competition) = started(&start(&record)).expect("started");
        assert_eq!(competition.competition_type(), 1);
        assert_eq!((competition.num_teams(), competition.num_rounds()), (4, 2));
        assert_eq!(competition.play_each_team(), 1, "the record's menu state");
        // A shuffle: the four teams, drawn.
        let mut pairings: Vec<u8> = (0..4)
            .map(|place| competition.grid_pairing(place))
            .collect();
        pairings.sort_unstable();
        assert_eq!(pairings, [0, 1, 2, 3]);
        assert_eq!(competition.total_matches(), state.slot_matches(0));
        assert_eq!(state.has_groups(), 0, "the computer's teams all");
    }

    #[test]
    fn what_cannot_start_is_refused() {
        let keep_four = OwnDraw {
            ranking: Ranking::ByPlace,
            stages: vec![StageDraw::Keep(4)],
        };
        let refused = |change: &dyn Fn(&mut TournamentRecord), own_draw: Option<&OwnDraw>| {
            let mut record = record();
            change(&mut record);
            let mut start = start(&record);
            start.own_draw = own_draw;
            started(&start).map(|_| ()).expect_err("refused")
        };
        assert_eq!(
            refused(&|record| record.stage_count = 0, None),
            StartError::Stages(0)
        );
        assert_eq!(
            refused(&|record| record.stage_count = 7, None),
            StartError::Stages(7)
        );
        assert_eq!(
            refused(&|record| record.stages[2] = 3, None),
            StartError::OddGroups { stage: 0, teams: 3 }
        );
        assert_eq!(
            refused(&|record| record.stages[0] = 40, None),
            StartError::SeedsPastList(40)
        );
        assert_eq!(
            refused(&|_| {}, Some(&keep_four)),
            StartError::Load(LoadError::OwnDrawPastPlaces {
                places: 4,
                teams: 8
            })
        );
        assert_eq!(
            refused(
                &|record| {
                    record.seed_reorder = 0;
                    record.stages[0] = 7;
                },
                None
            ),
            StartError::Load(LoadError::GroupsNotTeams {
                groups: 2,
                per_group: 4,
                teams: 7
            })
        );
        assert_eq!(
            refused(
                &|record| {
                    record.seed_reorder = 0;
                    record.stages[..3].copy_from_slice(&[5, 0, 5]);
                },
                None
            ),
            StartError::Load(LoadError::OddSeededKnockout(5))
        );
        assert_eq!(
            refused(
                &|record| record.stages[..3].copy_from_slice(&[1, 0, 1]),
                None
            ),
            StartError::Load(LoadError::StageTeams { stage: 0, teams: 1 })
        );
    }

    #[test]
    fn odd_groups_start_where_the_world_allows_them() {
        let mut record = record();
        record.stages[..3].copy_from_slice(&[6, 2, 3]);
        let mut start = start(&record);
        start.odd_groups = true;
        let (state, _) = started(&start).expect("started");
        assert_eq!(state.slot_teams_per_group(0), 3);
    }
}
