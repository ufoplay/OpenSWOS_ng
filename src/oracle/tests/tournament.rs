//! `openswos_core::competition::tournament` against upstream's `DIYTournamentState`
//! (include/game_types.h) and src/diy_competition.c: any image, every field the port reads is
//! the field the C reads, so every offset is the compiler's; from any state the reset
//! (`InitDIYTournament`) writes the same bytes; from any state of valid stages, each stage's
//! matches, their months and the strength (`DIYTournament_RecalculateMatchesPerTeam`) are
//! counted alike; from any images, any tournament's record started alike, its first stage
//! loaded (`InitDIYLeagueFromRecord`), both images byte for byte (the C's name pointers
//! aside), odd groups refused as upstream halts; and every tournament of World 1996-97,
//! resolved from its package (`WorldTournament`), started as upstream starts its compiled
//! records. Without upstream's checkout the oracle is not built and these tests are not either
//! (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

#[macro_use]
mod header;
mod tournaments;

use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition};
use openswos_core::competition::start::{LoadedTeam, Pitch};
use openswos_core::competition::tournament::{
    ResetNames, STAGES, STATE_LEN, StartError, TEAMS, TournamentStart, TournamentState,
    recalculate, reset, start_tournament,
};
use openswos_oracle::Oracle;
use openswos_oracle::draw::Prices;
use openswos_oracle::rng::RandState;
use openswos_oracle::tables::country;
use openswos_oracle::tournament::{RecalculateSetup, TournamentSetup};
use openswos_rng::{Generator, Rng};
use openswos_team_resource::TeamControls;
use proptest::collection::vec;
use proptest::prelude::*;
use tournaments::{
    CAREER_COUNTRY, DIY_CUP, DIY_LEAGUE, DIY_TOURNAMENT, LEAGUE, MIN_TOURNAMENTS, SEASONAL,
    SELECTED_TEAMS, WORLD, assert_alike, core_record, own_draw, record, selected, selected_loader,
    start_9697, tournaments, world_teams,
};

/// A stage's months (`progressionTable[90]`).
const MONTHS: usize = 90;

/// A group's teams here: 2 to 8, so every stage's months stay within the state.
const PER_GROUP: std::ops::RangeInclusive<u16> = 2..=8;

/// The times a group's teams play each other here: 0 (a stage of no match) to 4.
const PLAYS: std::ops::RangeInclusive<u16> = 0..=4;

/// The port's team bytes as the native word the C reads.
fn word(team: [u8; 2]) -> u16 {
    u16::from_ne_bytes(team)
}

/// A stage of valid counts: a knockout or a stage of groups of two teams or more, any legs and
/// penalties.
fn stage() -> impl Strategy<Value = (u16, u16, u16, u16, u16)> {
    (
        prop_oneof![Just(0u16), 1..=u16::MAX],
        PER_GROUP,
        PLAYS,
        0..=3u16,
        0..=3u16,
    )
}

/// The loader the stub is: the career club for country 0xFF, else the selected team of the
/// ordinal.
fn loader<'a>(
    teams: &'a [Prices],
    career_team: &'a Prices,
) -> impl Fn([u8; 2]) -> Option<LoadedTeam> + 'a {
    move |[country, ordinal]| {
        Some(if country == CAREER_COUNTRY {
            LoadedTeam {
                identity: -1,
                controls: TeamControls::Computer,
                prices: *career_team,
            }
        } else {
            let index = usize::from(ordinal) % SELECTED_TEAMS;
            LoadedTeam {
                identity: i32::try_from(index).expect("160"),
                controls: TeamControls::Computer,
                prices: teams[index],
            }
        })
    }
}

proptest! {
    #[test]
    fn every_field_is_where_upstream_s_compiler_puts_it(bytes in vec(any::<u8>(), STATE_LEN)) {
        let image: [u8; STATE_LEN] = bytes.try_into().expect("the image's bytes");
        let c = Oracle::lock().tournament_fields(&image);
        let ours = TournamentState::from_image(&image);
        prop_assert_eq!(ours.image(), &image);
        assert_header!(ours, c.header);
        prop_assert_eq!(ours.league_warnings_enabled(), c.league_warnings_enabled);
        for team in 0..TEAMS {
            prop_assert_eq!(word(ours.team(team)), c.team_ids[team]);
            prop_assert_eq!(ours.group(team), c.group_map[team]);
            prop_assert_eq!(ours.team_index(team), c.team_index_order[team]);
        }
        for (stage, slot) in c.slots.iter().enumerate().take(STAGES) {
            prop_assert_eq!(ours.slot_pad(stage).to_ne_bytes(), slot.pad);
            prop_assert_eq!(
                [
                    ours.slot_double_count(stage),
                    ours.slot_is_league_format(stage),
                    ours.slot_seeding_type(stage),
                    ours.slot_num_teams(stage),
                    ours.slot_division_factor(stage),
                    ours.slot_play_each_team(stage),
                    ours.slot_points_for_win(stage),
                    ours.slot_groups(stage),
                    ours.slot_teams_per_group(stage),
                    ours.slot_legs(stage),
                    ours.slot_extra_time(stage),
                    ours.slot_penalties(stage),
                    ours.slot_matches(stage),
                ],
                slot.words
            );
            for month in 0..MONTHS {
                prop_assert_eq!(ours.slot_month(stage, month), slot.months[month]);
            }
        }
    }

    #[test]
    fn the_reset_writes_what_upstream_s_writes(bytes in vec(any::<u8>(), STATE_LEN)) {
        let before: [u8; STATE_LEN] = bytes.try_into().expect("the image's bytes");
        let upstream = Oracle::lock().reset_tournament(&before);
        let mut ours = TournamentState::from_image(&before);
        reset(&mut ours, DIY_TOURNAMENT);
        for (offset, (&mine, &theirs)) in ours.image().iter().zip(upstream.iter()).enumerate() {
            prop_assert_eq!(mine, theirs, "byte {}", offset);
        }
    }

    #[test]
    fn the_stages_are_counted_as_upstream_counts_them(
        state in vec(any::<u8>(), STATE_LEN),
        competition in vec(any::<u8>(), IMAGE_LEN),
        stages in vec(stage(), STAGES),
        stage_count in 1..=6i16,
        num_teams in 1..=64i16,
        months in any::<(i8, i8)>(),
        teams in vec(any::<Prices>(), SELECTED_TEAMS),
        career_team in any::<Prices>(),
    ) {
        let state: [u8; STATE_LEN] = state.try_into().expect("the state's bytes");
        let competition: [u8; IMAGE_LEN] = competition.try_into().expect("the image's bytes");
        let mut ours = TournamentState::from_image(&state);
        ours.set_total_matches_in_round(stage_count);
        ours.set_num_teams(num_teams);
        ours.set_start_month(months.0);
        ours.set_end_month(months.1);
        for (index, &(groups, per_group, plays, legs, penalties)) in stages.iter().enumerate() {
            ours.set_slot_is_league_format(index, groups);
            ours.set_slot_teams_per_group(index, per_group);
            ours.set_slot_play_each_team(index, plays);
            ours.set_slot_legs(index, legs);
            ours.set_slot_penalties(index, penalties);
        }
        let before = *ours.image();
        let mut ours_competition = RunningCompetition::from_image(&competition);
        let counted = recalculate(&mut ours, &mut ours_competition, loader(&teams, &career_team));
        // Stages of no match in all: upstream's months run on past its memory.
        prop_assume!(counted.is_ok(), "{:?}", counted);
        let (after, competition_after) = Oracle::lock().recalculate_tournament(&RecalculateSetup {
            state: Box::new(before),
            competition: Box::new(competition),
            teams: teams.clone(),
            career_team,
        });
        for (offset, (&mine, &theirs)) in ours.image().iter().zip(after.iter()).enumerate() {
            prop_assert_eq!(mine, theirs, "state byte {}", offset);
        }
        prop_assert_eq!(ours_competition.image(), &*competition_after);
    }
}

proptest! {
    #[test]
    fn a_tournament_starts_as_upstream_starts_it(
        state in vec(any::<u8>(), STATE_LEN),
        competition in vec(any::<u8>(), IMAGE_LEN),
        (mut record, seeds) in record(),
        seed_bytes in vec(any::<[u8; 2]>(), 32),
        (national, continental, odd_groups) in any::<(bool, bool, bool)>(),
        pitch_type in prop_oneof![Just(SEASONAL), any::<u16>()],
        teams in vec(selected(), SELECTED_TEAMS),
        career_team in selected(),
        rand in any::<(u8, u8, u8)>(),
    ) {
        let state: [u8; STATE_LEN] = state.try_into().expect("the state's bytes");
        let competition: [u8; IMAGE_LEN] = competition.try_into().expect("the image's bytes");
        let before = TournamentState::from_image(&state);
        record.seeds = seeds
            .iter()
            .zip(&seed_bytes)
            .map(|(&pick, &bytes)| pick.map_or(bytes, |index| before.team(index)))
            .collect();
        let core_record = core_record(&record);
        let adjective = country(record.header[2]).expect("built").map(|entry| entry.adjective);
        let prefix = adjective.filter(|_| !national);
        let name = record.names[0].clone().unwrap_or_else(|| LEAGUE.to_vec());
        let start = TournamentStart {
            record: &core_record,
            name: &name,
            prefix: prefix.as_deref(),
            reset_names: ResetNames { tournament: DIY_TOURNAMENT, league: DIY_LEAGUE, cup: DIY_CUP },
            continental,
            own_draw: own_draw(record.header[0]),
            odd_groups,
            pitch: if pitch_type == SEASONAL { Pitch::Seasonal } else { Pitch::Type(pitch_type.cast_signed()) },
        };
        let (seed, xor_index, xor_key) = rand;
        let mut rng = Rng::default();
        rng.secondary = Generator { seed, xor_index, xor_key };
        let mut ours = before.clone();
        let mut ours_competition = RunningCompetition::from_image(&competition);
        let started = start_tournament(
            &mut ours,
            &mut ours_competition,
            &start,
            selected_loader(&teams, &career_team),
            &mut rng,
        );
        // Refused where upstream runs past its memory or divides by zero: not run.
        prop_assume!(
            matches!(started, Ok(()) | Err(StartError::OddGroups { .. })),
            "{:?}",
            started
        );
        let mut oracle = Oracle::lock();
        oracle.set_rand2_state(RandState { seed, xor_index, xor_key });
        let upstream = oracle.start_tournament(&TournamentSetup {
            state: Box::new(state),
            competition: Box::new(competition),
            record: record.clone(),
            national,
            continental,
            odd_groups,
            pitch_type,
            teams: teams.clone(),
            career_team,
        });
        match upstream {
            Err(halt) => {
                prop_assert_eq!(halt, 0x1313);
                prop_assert!(matches!(started, Err(StartError::OddGroups { .. })), "{:?}", started);
            }
            Ok(after) => {
                prop_assert!(started.is_ok(), "{:?}", started);
                assert_alike((&ours, &ours_competition), &after, "random")?;
                let state = oracle.rand2_state();
                prop_assert_eq!(
                    (rng.secondary.seed, rng.secondary.xor_index, rng.secondary.xor_key),
                    (state.seed, state.xor_index, state.xor_key)
                );
            }
        }
    }
}

#[test]
fn world_1996_97s_tournaments_start_as_upstreams_compiled_records() {
    let world = &*WORLD;
    let started = tournaments(world);
    assert!(
        started.len() >= MIN_TOURNAMENTS,
        "{} tournaments",
        started.len()
    );
    let (teams, career_team) = world_teams();
    let mut oracle = Oracle::lock();
    let mut started_alike = 0;
    for tournament in &started {
        let both = start_9697(
            &mut oracle,
            world,
            tournament,
            (&teams, career_team),
            selected_loader(&teams, &career_team),
        );
        let id = world.competition_id(tournament.competition).as_str();
        match both.upstream {
            Err(halt) => {
                assert_eq!(halt, 0x1313, "{id}");
                assert!(
                    matches!(both.result, Err(StartError::OddGroups { .. })),
                    "{id}: {:?}",
                    both.result
                );
            }
            Ok(after) => {
                assert!(both.result.is_ok(), "{id}: {:?}", both.result);
                assert_alike((&both.state, &both.competition), &after, id)
                    .unwrap_or_else(|error| panic!("{id} ({:?}): {error}", tournament.source));
                started_alike += 1;
            }
        }
    }
    assert!(started_alike >= MIN_TOURNAMENTS, "{started_alike} started");
}
