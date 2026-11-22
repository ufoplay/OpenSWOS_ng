//! `openswos_core::competition::league`'s play against upstream's src/diy_competition.c: from
//! any league under way, a match's result recorded alike (`DIY_PlayOrSimulateLeagueFixture`,
//! its fixture, its progress, its table, the qualifiers at the season's end), both images byte
//! for byte; any table's qualifiers (`DIY_UpdateLeagueQualifiers`) and season's moves
//! (`DIY_UpdatePromotionRelegationScratchCounts`) alike; and every division of World 1996-97's
//! leagues, started as upstream starts its compiled records, played through its season alike,
//! match by match. Without upstream's checkout the oracle is not built and these tests are not
//! either (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

use std::sync::LazyLock;

use openswos_competitions_json_resource::SymbolId;
use openswos_core::competition::league::{
    Moves, PlayError, WorldDivision, play_result, season_moves, start_division, update_qualifiers,
};
use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition, STANDINGS_ROWS};
use openswos_core::competition::schedule::Schedule;
use openswos_core::competition::start::{LoadedTeam, Pitch};
use openswos_core::competition::table::{Line, Score};
use openswos_core::competition::tournament::{STATE_LEN, TournamentState};
use openswos_core::world::{CompetitionRef, Source, TableContent, World};
use openswos_oracle::Oracle;
use openswos_oracle::draw::Prices;
use openswos_oracle::league::{CompiledRecord, CompiledSetup};
use openswos_oracle::play::{MovesCounted, ResultSetup, Teams};
use openswos_oracle::rng::RandState;
use openswos_rng::{Generator, Rng};
use openswos_team_resource::TeamControls;
use openswos_test_utils::world_1996_97_path;
use openswos_world_package_resource::{PackageFiles, WorldPackage};
use proptest::collection::vec;
use proptest::prelude::*;

/// The selected teams the stubbed loader gives, by ordinal.
const SELECTED_TEAMS: usize = 160;

/// The career club's identity in a standings row.
const CAREER_CLUB: i32 = -1;

/// The career club's global team number here (past the selected teams').
const CAREER_NUMBER: u16 = 1000;

/// The bytes of a standings row: a place's entry counts in them.
const ROW_BYTES: u16 = 18;

/// The European Championships' ids, whose round robin is their own (upstream picks it by them).
const EURO_CHAMPIONSHIPS: [u8; 2] = [21, 22];

/// A league's groups' teams here: 2 to 8.
const PER_GROUP: std::ops::RangeInclusive<u16> = 2..=8;

/// The session's pitch type for the season's.
const SEASONAL: u16 = 0xFFFE;

/// A `WORLD.ini` for the committed package, which holds none.
const WORLD_INI: &str = "tag = 9697\nname = World 1996-97\nfingerprint = 0000000000000000\n";

/// World 1996-97, from its committed package: its round robins are the ones upstream compiles.
static WORLD: LazyLock<World> = LazyLock::new(|| {
    let mut files = PackageFiles::read_dir(&world_1996_97_path()).expect("the committed package");
    files
        .insert("WORLD.ini", WORLD_INI.as_bytes().to_vec())
        .expect("a safe path");
    let opened = WorldPackage::from_files(&files).expect("the package opens");
    World::from_package(&opened.package).expect("a valid world")
});

/// The round robin a league of `comp_id` plays with `teams` a group, as upstream picks it: the
/// European Championships' own, else its size's.
fn schedule(comp_id: u8, teams: u8) -> Schedule {
    let world = &*WORLD;
    if EURO_CHAMPIONSHIPS.contains(&comp_id) {
        let euro = world
            .competition(&SymbolId("_europeanChampionships".to_owned()))
            .expect("the European Championships");
        world.schedules().of(euro, teams)
    } else {
        world.schedules().of_size(teams)
    }
    .expect("a round robin")
    .clone()
}

/// A name: 1 to 16 letters.
fn name() -> impl Strategy<Value = Vec<u8>> {
    vec(b'A'..=b'Z', 1..=16)
}

/// The teams: each selected team a name and its ordinal as its number, the career club too.
fn teams() -> impl Strategy<Value = Teams> {
    (vec(name(), SELECTED_TEAMS), name()).prop_map(|(names, career)| Teams {
        selected: names.into_iter().zip(0..).collect(),
        career: (career, CAREER_NUMBER),
    })
}

/// The core's name of identity `identity` among `teams`.
fn name_of(teams: &Teams) -> impl Fn(i32) -> Option<Vec<u8>> + '_ {
    |identity| match identity {
        CAREER_CLUB => Some(teams.career.0.clone()),
        _ => usize::try_from(identity)
            .ok()
            .and_then(|index| teams.selected.get(index))
            .map(|(name, _)| name.clone()),
    }
}

/// A league under way: any image, its groups and places valid (a size the world has, every
/// place a row, every entry a row's), its rows any lines of selected teams or the career club,
/// its progress and its qualifiers' counts small.
fn league() -> impl Strategy<Value = ([u8; IMAGE_LEN], Schedule)> {
    (
        vec(any::<u8>(), IMAGE_LEN),
        prop_oneof![0..=161u8, Just(21u8), Just(22u8)],
        PER_GROUP,
        1..=8u16,
        vec(-1..160i32, STANDINGS_ROWS),
        vec(any::<[u16; 7]>(), STANDINGS_ROWS),
        (
            0..=3u16,
            0..=1u16,
            0..=8u16,
            1..=8u16,
            0..=100u16,
            0..=6u16,
            1..=30u16,
        ),
        (0..=1i16, 0..=2i16, 0..=2i16, 0..=2i16, 0..=2i16),
        (any::<u64>(), any::<u64>(), any::<(u16, u16)>()),
    )
        .prop_map(
            |(
                bytes,
                comp_id,
                per_group,
                groups,
                identities,
                lines,
                progress,
                counts,
                (shuffle, order, picks),
            )| {
                let per_group = if EURO_CHAMPIONSHIPS.contains(&comp_id) {
                    4
                } else {
                    per_group
                };
                let groups = groups.min(64 / per_group);
                let image: [u8; IMAGE_LEN] = bytes.try_into().expect("an image");
                let mut competition = RunningCompetition::from_image(&image);
                competition.set_competition_type(0);
                competition.set_comp_id(comp_id);
                competition.set_total_matches_in_round(per_group.cast_signed());
                competition.set_match_pairs_per_round(groups.cast_signed());
                let teams = per_group * groups;
                competition.set_num_teams(teams.cast_signed());
                let places = usize::from(teams);
                for (place, row) in permutation(places, shuffle).into_iter().enumerate() {
                    competition.set_shuffle_and_pad(place, row);
                }
                for (place, row) in permutation(places, order).into_iter().enumerate() {
                    competition.set_team_entry_offset(place, u16::from(row) * ROW_BYTES);
                }
                for (row, (&team, words)) in identities.iter().zip(&lines).enumerate() {
                    let [played, won, drawn, lost, goals_for, goals_against, points] = *words;
                    competition.set_row(
                        row,
                        openswos_core::competition::running::StandingsRow {
                            team,
                            line: Line {
                                played,
                                won,
                                drawn,
                                lost,
                                goals_for,
                                goals_against,
                                points,
                            },
                        },
                    );
                }
                let schedule = schedule(comp_id, u8::try_from(per_group).expect("8"));
                let (cycles, slot, half, per_half, phase, live, section) = progress;
                let (pick_group, pick_day) = picks;
                competition.set_min_substitutes(cycles);
                competition.set_league_current_slot(slot);
                competition.set_half_groups_counter(half);
                competition.set_matches_per_half(per_half);
                competition.set_league_match_progress(phase);
                competition.set_total_matches_per_section_live(live);
                competition.set_total_matches_per_section(section);
                competition.set_sub_round_counter((pick_group % groups).cast_signed());
                let days = u16::try_from(schedule.pairs().len()).expect("a few pairs");
                competition.set_section_matches_progress(pick_day % days);
                let (has_groups, promoted, promotions, relegated, relegations) = counts;
                competition.set_has_groups(has_groups);
                competition.set_round_complete(promoted);
                competition.set_competition_finished(promotions);
                competition.set_current_round_index(relegated);
                competition.set_num_rounds(relegations);
                (*competition.image(), schedule)
            },
        )
}

/// The rows `0..count` in an order `seed` draws.
fn permutation(count: usize, seed: u64) -> Vec<u8> {
    let mut rows: Vec<u8> = (0..count)
        .map(|row| u8::try_from(row).expect("64"))
        .collect();
    let mut state = seed;
    for place in (1..rows.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let other = usize::try_from(state >> 33).expect("a place") % (place + 1);
        rows.swap(place, other);
    }
    rows
}

/// Whether a refusal is the port's where upstream reads past the competition's places or rows.
fn refused_as_garbage(error: &PlayError) -> bool {
    matches!(
        error,
        PlayError::PlacePastOrder(_)
            | PlayError::EntryNotRow(_)
            | PlayError::NotARow { .. }
            | PlayError::UnknownTeam(_)
    )
}

proptest! {
    #[test]
    fn a_result_is_recorded_as_upstream_records_it(
        (image, schedule) in league(),
        state in vec(any::<u8>(), STATE_LEN),
        teams in teams(),
        goals in (0..20u16, 0..20u16),
    ) {
        let state: [u8; STATE_LEN] = state.try_into().expect("a state");
        let mut ours = RunningCompetition::from_image(&image);
        let mut our_state = TournamentState::from_image(&state);
        let played = play_result(
            &mut ours,
            &mut our_state,
            &schedule,
            Score { home: goals.0, away: goals.1 },
            name_of(&teams),
        );
        prop_assume!(!matches!(&played, Err(error) if refused_as_garbage(error)), "{:?}", played);
        prop_assert!(played.is_ok(), "{:?}", played);
        let (theirs, their_state) = Oracle::lock().play_result(&ResultSetup {
            competition: Box::new(image),
            state: Box::new(state),
            teams: teams.clone(),
            goals,
        });
        for (offset, (&mine, &upstream)) in ours.image().iter().zip(theirs.iter()).enumerate() {
            prop_assert_eq!(mine, upstream, "competition byte {}", offset);
        }
        for (offset, (&mine, &upstream)) in our_state.image().iter().zip(their_state.iter()).enumerate() {
            prop_assert_eq!(mine, upstream, "state byte {}", offset);
        }
    }

    #[test]
    fn the_qualifiers_are_upstream_s(
        (image, _) in league(),
        state in vec(any::<u8>(), STATE_LEN),
        teams in teams(),
    ) {
        let state: [u8; STATE_LEN] = state.try_into().expect("a state");
        let mut ours = RunningCompetition::from_image(&image);
        let updated = update_qualifiers(&mut ours, &TournamentState::from_image(&state), name_of(&teams));
        prop_assume!(!matches!(&updated, Err(error) if refused_as_garbage(error)), "{:?}", updated);
        prop_assert!(updated.is_ok(), "{:?}", updated);
        let theirs = Oracle::lock().update_qualifiers(&image, &state, &teams);
        prop_assert_eq!(ours.image(), &*theirs);
    }

    #[test]
    fn the_season_moves_are_upstream_s(
        (image, _) in league(),
        teams in teams(),
        kind in prop_oneof![Just(0i16), any::<i16>()],
    ) {
        let mut competition = RunningCompetition::from_image(&image);
        competition.set_competition_type(kind);
        let image = *competition.image();
        let moves = season_moves(&competition);
        prop_assume!(!matches!(&moves, Err(error) if refused_as_garbage(error)), "{:?}", moves);
        let moves = moves.expect("moves");
        let counted = Oracle::lock().season_moves(&image, &teams);
        prop_assert_eq!(counted_as_upstream(&moves, teams.selected.len()), counted);
    }
}

/// `moves` counted as upstream counts them: each promoted team one division down in the
/// scratch, each relegated one up, the career club's flagged.
fn counted_as_upstream(moves: &Moves, selected: usize) -> MovesCounted {
    let mut counted = MovesCounted {
        selected: vec![0; selected],
        career: 0,
        promoted: false,
        relegated: false,
    };
    let count = |team: i32, change: i8, counted: &mut MovesCounted| match team {
        CAREER_CLUB => counted.career = counted.career.wrapping_add(change),
        _ => {
            let index = usize::try_from(team).expect("a selected team");
            counted.selected[index] = counted.selected[index].wrapping_add(change);
        }
    };
    for &team in &moves.promoted {
        count(team, -1, &mut counted);
        counted.promoted |= team == CAREER_CLUB;
    }
    for &team in &moves.relegated {
        count(team, 1, &mut counted);
        counted.relegated |= team == CAREER_CLUB;
    }
    counted
}

/// The divisions of World 1996-97 the season check plays: every division of each country
/// table's league.
const MIN_SEASONS: usize = 88;

/// Each division of World 1996-97's country tables' leagues: the oracle's record, the world's
/// league and the division.
fn divisions(world: &World) -> Vec<(CompiledRecord, CompetitionRef, usize)> {
    let mut divisions = Vec::new();
    for table in world.country_tables() {
        let TableContent::Own { leagues, .. } = &table.content else {
            continue;
        };
        if let Some(&league) = leagues.first() {
            for division in 0..world.league(league).record.divisions.len() {
                divisions.push((
                    CompiledRecord::CountryLeague(table.number.0),
                    CompetitionRef::League(league),
                    division,
                ));
            }
        }
    }
    divisions
}

/// The name pointer's bytes (an address: the C's own, which its start writes).
const NAME_POINTER: std::ops::Range<usize> = 40..48;

/// `ours` and `theirs` alike, the name pointer aside.
fn assert_alike(ours: &[u8; IMAGE_LEN], theirs: &[u8; IMAGE_LEN], what: &str) {
    let mut theirs = *theirs;
    theirs[NAME_POINTER].copy_from_slice(&ours[NAME_POINTER]);
    for (offset, (&mine, &upstream)) in ours.iter().zip(theirs.iter()).enumerate() {
        assert_eq!(mine, upstream, "{what}: byte {offset}");
    }
}

/// A match's result, from its number in the season: varied, draws among them.
fn result(number: usize) -> (u16, u16) {
    let home = u16::try_from(number * 7 % 5).expect("below 5");
    let away = u16::try_from(number * 3 % 4).expect("below 4");
    (home, away)
}

#[test]
fn world_1996_97s_leagues_play_their_seasons_as_upstream_s() {
    let world = &*WORLD;
    let prices: Vec<Prices> = (0..SELECTED_TEAMS)
        .map(|ordinal| [u8::try_from(ordinal).expect("160"); 16])
        .collect();
    let teams = Teams {
        selected: (0..SELECTED_TEAMS)
            .map(|ordinal| {
                (
                    format!("TEAM {ordinal:03}").into_bytes(),
                    u16::try_from(ordinal).expect("160"),
                )
            })
            .collect(),
        career: (b"CAREER".to_vec(), CAREER_NUMBER),
    };
    // The image's teams the selected teams in order, so the clubs differ.
    let mut before = RunningCompetition::default();
    for ordinal in 0..=u8::MAX >> 2 {
        before.set_league_team(usize::from(ordinal), [0, ordinal]);
    }
    let before = *before.image();
    let state = [0; STATE_LEN];
    let mut oracle = Oracle::lock();
    let mut seasons = 0;
    for (record, competition, division) in divisions(world) {
        oracle.set_rand2_state(RandState {
            seed: 0,
            xor_index: 0,
            xor_key: 0,
        });
        let started = oracle
            .start_compiled_division(&CompiledSetup {
                before: Box::new(before),
                record,
                division: u16::try_from(division).expect("a few"),
                pitch_type: SEASONAL,
                teams: prices.clone(),
                career_team: [u8::MAX; 16],
            })
            .expect("upstream's record");
        let Ok(mut theirs) = started else {
            continue;
        };
        let resolved = WorldDivision::resolve(world, competition, Source::Record, division)
            .expect("a division");
        let mut rng = Rng::default();
        rng.secondary = Generator::default();
        let mut ours = RunningCompetition::from_image(&before);
        start_division(
            &mut ours,
            &resolved.start(Pitch::Seasonal),
            world.schedules(),
            |[_, ordinal]| {
                let index = usize::from(ordinal) % SELECTED_TEAMS;
                Some(LoadedTeam {
                    identity: i32::try_from(index).expect("160"),
                    controls: TeamControls::Computer,
                    prices: prices[index],
                })
            },
            &mut rng,
        )
        .expect("started");
        let id = world.competition_id(competition).as_str();
        assert_alike(ours.image(), &theirs, &format!("{id} {division}: started"));
        let groups = u8::try_from(ours.total_matches_in_round()).expect("a size");
        let schedule = world
            .schedules()
            .of(competition, groups)
            .expect("its round robin")
            .clone();
        let mut our_state = TournamentState::from_image(&state);
        let mut number = 0;
        while ours.min_substitutes() != 0 {
            let goals = result(number);
            play_result(
                &mut ours,
                &mut our_state,
                &schedule,
                Score {
                    home: goals.0,
                    away: goals.1,
                },
                name_of(&teams),
            )
            .unwrap_or_else(|error| panic!("{id} {division}, match {number}: {error}"));
            let competition_before: [u8; IMAGE_LEN] = *theirs;
            let (after, _) = oracle.play_result(&ResultSetup {
                competition: Box::new(competition_before),
                state: Box::new(state),
                teams: teams.clone(),
                goals,
            });
            theirs = after;
            assert_alike(
                ours.image(),
                &theirs,
                &format!("{id} {division}, match {number}"),
            );
            number += 1;
        }
        let moves = season_moves(&ours).expect("moves");
        let counted = oracle.season_moves(&theirs, &teams);
        assert_eq!(
            counted_as_upstream(&moves, SELECTED_TEAMS),
            counted,
            "{id} {division}: the moves"
        );
        seasons += 1;
    }
    assert!(seasons >= MIN_SEASONS, "{seasons} seasons");
}
