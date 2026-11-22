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

use std::sync::LazyLock;

use openswos_common::Nibble;
use openswos_common::json::JsonByte;
use openswos_competitions_json_resource::{
    CompetitionHeader, CompetitionType, Round, RoundRules, SymbolId, Tournament, TournamentKind,
};
use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition};
use openswos_core::competition::start::{LoadedTeam, Pitch};
use openswos_core::competition::tournament::{
    ResetNames, STAGES, STATE_LEN, StartError, TEAMS, TournamentRecord, TournamentStart,
    TournamentState, WorldTournament, recalculate, reset, start_tournament,
};
use openswos_core::world::{CompetitionRef, Source, World};
use openswos_oracle::Oracle;
use openswos_oracle::cup::SelectedTeam;
use openswos_oracle::draw::Prices;
use openswos_oracle::league::CompiledRecord;
use openswos_oracle::rng::RandState;
use openswos_oracle::tables::country;
use openswos_oracle::tournament::{
    CompiledTournamentSetup, RecalculateSetup, Started, TournamentBytes, TournamentSetup,
};
use openswos_rng::{Generator, Rng};
use openswos_team_resource::TeamControls;
use openswos_test_utils::world_1996_97_path;
use openswos_world_package_resource::{PackageFiles, WorldPackage};
use proptest::collection::vec;
use proptest::prelude::*;

/// The name upstream's tournament reset writes (src/swos_amiga_strings.c `_aDiyTournament`).
const DIY_TOURNAMENT: &[u8] = b"DIY TOURNAMENT";

/// A stage's months (`progressionTable[90]`).
const MONTHS: usize = 90;

/// The selected teams the stubbed loader gives, by ordinal.
const SELECTED_TEAMS: usize = 160;

/// The country of the career club's team numbers in the stubbed loader.
const CAREER_COUNTRY: u8 = 0xFF;

/// The name pointers' bytes in both images (addresses: the C's own, not compared).
const NAME_POINTER: std::ops::Range<usize> = 40..48;

/// The names upstream's league and cup resets write (`_aDiyLeague`, `_aDiyCup`).
const DIY_LEAGUE: &[u8] = b"DIY LEAGUE";

/// See [`DIY_LEAGUE`].
const DIY_CUP: &[u8] = b"DIY CUP";

/// A tournament's name when its record names none (`_aLeague_1`).
const LEAGUE: &[u8] = b"LEAGUE";

/// The session's pitch type for the season's (upstream `_pitchType` -2).
const SEASONAL: u16 = 0xFFFE;

/// A tournament's type (`CompetitionType::Tournament`): upstream's `comp_name_table` finds its
/// names by it.
const TOURNAMENT: u8 = 2;

/// The competition ids a record takes here: below World 2026-27's continental contests, which
/// `continental_record` names by id (the Copa, 162, and up).
const IDS: std::ops::RangeInclusive<u8> = 0..=161;

/// The places a first stage keeps when upstream draws a competition of these ids by its own
/// tables (diy_competition.c: the World Cup's `buildCompSpecificTeamList`, the European
/// Championships' `buildCompSpecificTeamShuffle`): World 1996-97's `rules/draws.json`.
fn own_draw(comp_id: u8) -> Option<u8> {
    match comp_id {
        4 => Some(24),
        21 | 22 => Some(16),
        _ => None,
    }
}

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

/// A name: ASCII letters and spaces, up to past the 35 the buffer takes.
fn name() -> impl Strategy<Value = Vec<u8>> {
    vec(prop_oneof![b'A'..=b'Z', Just(b' ')], 0..=40)
}

/// A first stage's three bytes: groups of 2 to 8 (their teams mostly theirs), or a knockout
/// (doubled sometimes) of 2 to 64 teams.
fn first_stage() -> impl Strategy<Value = [i8; 3]> {
    prop_oneof![
        (1..=8i8, 2..=8i8, any::<bool>()).prop_map(|(groups, per_group, exact)| {
            let teams = if exact {
                groups * per_group
            } else {
                (groups * per_group).min(60) + 1
            };
            [teams, groups, per_group]
        }),
        (2..=64i8, prop_oneof![Just(0i8), Just(-1i8)], any::<i8>())
            .prop_map(|(teams, groups, per_group)| [teams, groups, per_group]),
    ]
}

/// A later stage's three bytes: groups of 2 to 8, or a knockout.
fn later_stage() -> impl Strategy<Value = [i8; 3]> {
    (
        0..=64i8,
        prop_oneof![Just(0i8), Just(-1i8), 1..=8i8],
        2..=8i8,
    )
        .prop_map(|(teams, groups, per_group)| [teams, groups, per_group])
}

/// A tournament's record: the id within [`IDS`], the own draws' as often as the rest, a
/// tournament's type, the seeds' order and the seeded draw each on half the time, 1 to 6
/// stages, the group stages' plays 0 to 4, its seeds each a team of the state (by index) or
/// any.
fn record() -> impl Strategy<Value = (TournamentBytes, Vec<Option<usize>>)> {
    // Each path as often as the others: the own draws' ids, the seeds' order on or off, the
    // shuffle or the seeded draw.
    let id = prop_oneof![Just(4u8), Just(21u8), Just(22u8), IDS, IDS, IDS];
    let either = |zero| prop_oneof![Just(zero), any::<i8>()];
    (
        (id, any::<[u8; 6]>(), either(0)),
        (
            either(0),
            any::<i8>(),
            0..=4i8,
            any::<i8>(),
            any::<u8>(),
            1..=6i8,
        ),
        first_stage(),
        vec(later_stage(), STAGES - 1),
        (any::<i8>(), any::<[i8; 6]>()),
        [
            proptest::option::weighted(0.8, name()),
            proptest::option::of(name()),
        ],
        vec(proptest::option::weighted(0.7, 0..TEAMS), 0..=32),
    )
        .prop_map(
            |(
                (id, [team, begin, end, a, b, c], reorder),
                (seeding, menu, plays, away, subs, stages),
                first,
                later,
                (reserved, rules),
                names,
                seeds,
            )| {
                let mut rounds = [[0; 3]; STAGES];
                rounds[0] = first;
                rounds[1..].copy_from_slice(&later);
                (
                    TournamentBytes {
                        header: [
                            id,
                            TOURNAMENT,
                            team,
                            begin,
                            end,
                            a,
                            b,
                            c,
                            reorder.cast_unsigned(),
                        ],
                        seeding_type: seeding,
                        menu_state: menu,
                        play_each_team: plays,
                        away_goals_mode: away,
                        substitutes: subs,
                        round_string_base: stages,
                        rounds,
                        reserved,
                        round_rules: rules,
                        names,
                        seeds: Vec::new(),
                    },
                    seeds,
                )
            },
        )
}

/// The selected teams: any prices, one of upstream's controls.
fn selected() -> impl Strategy<Value = SelectedTeam> {
    (any::<Prices>(), 0..=3u8).prop_map(|(prices, controls)| SelectedTeam { prices, controls })
}

/// The core's record of `bytes`, through the competition files' model.
fn core_record(bytes: &TournamentBytes) -> TournamentRecord {
    let [id, kind, team, begin, end, a, b, c, d] = bytes.header;
    let byte = |value: i8| JsonByte::from(value.cast_unsigned());
    let nibble = |value: u8| Nibble::try_from(value).expect("a nibble");
    let tournament = Tournament {
        id: SymbolId("_tournament".to_owned()),
        kind: TournamentKind::Tournament,
        comp: CompetitionHeader {
            comp_id: JsonByte::from(id),
            kind: CompetitionType::try_from(kind).expect("a type"),
            team_number: JsonByte::from(team),
            begin_month: JsonByte::from(begin),
            end_month: JsonByte::from(end),
            name_table_offset: JsonByte::from(a),
            reserved: JsonByte::from(b),
            team_list_offset: JsonByte::from(c),
            seed_reorder_flag: JsonByte::from(d),
        },
        seeding_type: byte(bytes.seeding_type),
        menu_state: byte(bytes.menu_state),
        play_each_team: byte(bytes.play_each_team),
        away_goals_mode: byte(bytes.away_goals_mode),
        substitutes: nibble(bytes.substitutes >> 4),
        points_for_win: nibble(bytes.substitutes & 0x0F),
        round_string_base: byte(bytes.round_string_base),
        rounds: bytes
            .rounds
            .iter()
            .map(|&[teams, groups, per_group]| Round {
                team_count: byte(teams),
                group_count: byte(groups),
                team_group_count: byte(per_group),
            })
            .collect(),
        header_reserved: byte(bytes.reserved),
        round_rule_flags: bytes
            .round_rules
            .iter()
            .map(|&rule| RoundRules::from(rule.cast_unsigned()))
            .collect(),
        name: Vec::new(),
        team_seed_list: Vec::new(),
    };
    TournamentRecord::from_tournament(&tournament, &bytes.seeds).expect("a record")
}

/// The core's view of a selected team.
fn loaded(identity: i32, team: &SelectedTeam) -> LoadedTeam {
    LoadedTeam {
        identity,
        controls: TeamControls::try_from(team.controls).expect("a control"),
        prices: team.prices,
    }
}

/// The loader the stub is, over selected teams with their controls.
fn selected_loader<'a>(
    teams: &'a [SelectedTeam],
    career_team: &'a SelectedTeam,
) -> impl Fn([u8; 2]) -> Option<LoadedTeam> + 'a {
    move |[country, ordinal]| {
        Some(if country == CAREER_COUNTRY {
            loaded(-1, career_team)
        } else {
            let index = usize::from(ordinal) % SELECTED_TEAMS;
            loaded(i32::try_from(index).expect("160"), &teams[index])
        })
    }
}

/// Both images of `ours` and `theirs` alike, the name pointers aside.
fn assert_alike(
    (state, competition): (&TournamentState, &RunningCompetition),
    (their_state, their_competition): &Started,
    what: &str,
) -> Result<(), TestCaseError> {
    let mut theirs = **their_state;
    theirs[NAME_POINTER].copy_from_slice(&state.image()[NAME_POINTER]);
    for (offset, (&mine, &upstream)) in state.image().iter().zip(theirs.iter()).enumerate() {
        prop_assert_eq!(mine, upstream, "{}: state byte {}", what, offset);
    }
    let mut theirs = **their_competition;
    theirs[NAME_POINTER].copy_from_slice(&competition.image()[NAME_POINTER]);
    for (offset, (&mine, &upstream)) in competition.image().iter().zip(theirs.iter()).enumerate() {
        prop_assert_eq!(mine, upstream, "{}: competition byte {}", what, offset);
    }
    Ok(())
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

/// A `WORLD.ini` for the committed package, which holds none (its fingerprint, which covers
/// the rosters, is not read here).
const WORLD_INI: &str = "tag = 9697\nname = World 1996-97\nfingerprint = 0000000000000000\n";

/// World 1996-97, from its committed package.
static WORLD: LazyLock<World> = LazyLock::new(|| {
    let mut files = PackageFiles::read_dir(&world_1996_97_path()).expect("the committed package");
    files
        .insert("WORLD.ini", WORLD_INI.as_bytes().to_vec())
        .expect("a safe path");
    let opened = WorldPackage::from_files(&files).expect("the package opens");
    World::from_package(&opened.package).expect("a valid world")
});

/// The fewest tournaments of World 1996-97 the check starts: the career's eleven from their
/// records and copies, the World Cup and the European Championships, the Copa.
const MIN_TOURNAMENTS: usize = 25;

/// How upstream reaches a tournament of the world: a compiled record, or (the Copa, which
/// upstream's SDL build loads from the JSON) the record built from the world's JSON.
#[derive(Debug, Clone, Copy)]
enum Reached {
    /// A compiled record.
    Compiled(CompiledRecord),
    /// The JSON's record.
    Json,
}

/// A tournament of World 1996-97 to start: how upstream reaches it, the world's competition
/// and source for it.
struct Started9697 {
    /// How upstream reaches it.
    reached: Reached,
    /// The world's competition.
    competition: CompetitionRef,
    /// Whether the career's copy is started.
    source: Source,
}

/// Every tournament of World 1996-97: the career's from their records and copies, the two the
/// oracle binds, the others (the Copa) from the JSON.
fn tournaments(world: &World) -> Vec<Started9697> {
    let mut started = Vec::new();
    let mut reached = std::collections::BTreeSet::new();
    for contest in world.career() {
        let Some(competition @ CompetitionRef::Tournament(_)) = contest.competition else {
            continue;
        };
        let comp_id = world.header(competition).comp_id.byte();
        for (source, copy) in [(Source::Record, false), (Source::CareerCopy, true)] {
            started.push(Started9697 {
                reached: Reached::Compiled(CompiledRecord::CareerContest { comp_id, copy }),
                competition,
                source,
            });
        }
        reached.insert(competition);
    }
    for (index, tournament) in world.tournaments().iter().enumerate() {
        let competition = world
            .competition(&tournament.record.id)
            .expect("the world's");
        if reached.contains(&competition) {
            continue;
        }
        let comp_id = tournament.record.comp.comp_id.byte();
        let compiled = matches!(comp_id, 4 | 22);
        started.push(Started9697 {
            reached: if compiled {
                Reached::Compiled(CompiledRecord::Bound(comp_id))
            } else {
                Reached::Json
            },
            competition,
            source: Source::Record,
        });
        assert!(index < SELECTED_TEAMS, "a few tournaments");
    }
    started
}

/// The C record of a world's tournament, as upstream's JSON loader builds it.
fn json_record(world: &World, competition: CompetitionRef) -> TournamentBytes {
    let CompetitionRef::Tournament(tournament) = competition else {
        panic!("a tournament");
    };
    let tournament = world.tournament(tournament);
    let record = &tournament.record;
    let comp = &record.comp;
    let signed = |byte: JsonByte| byte.byte().cast_signed();
    let mut rounds = [[0; 3]; STAGES];
    for (slot, round) in rounds.iter_mut().zip(&record.rounds) {
        *slot = [
            signed(round.team_count),
            signed(round.group_count),
            signed(round.team_group_count),
        ];
    }
    let mut round_rules = [0; 6];
    for (slot, &rules) in round_rules.iter_mut().zip(&record.round_rule_flags) {
        *slot = u8::from(rules).cast_signed();
    }
    let names = [0, 1].map(|slot| tournament.names.get(slot).cloned().flatten());
    TournamentBytes {
        header: [
            comp.comp_id.byte(),
            u8::from(comp.kind),
            comp.team_number.byte(),
            comp.begin_month.byte(),
            comp.end_month.byte(),
            comp.name_table_offset.byte(),
            comp.reserved.byte(),
            comp.team_list_offset.byte(),
            comp.seed_reorder_flag.byte(),
        ],
        seeding_type: signed(record.seeding_type),
        menu_state: signed(record.menu_state),
        play_each_team: signed(record.play_each_team),
        away_goals_mode: signed(record.away_goals_mode),
        substitutes: u8::from(record.substitutes) << 4 | u8::from(record.points_for_win),
        round_string_base: signed(record.round_string_base),
        rounds,
        reserved: signed(record.header_reserved),
        round_rules,
        names,
        seeds: tournament
            .seeds
            .iter()
            .map(|seed| seed.to_bytes())
            .collect(),
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
    let controls = [0u8, 1, 2, 3];
    let teams: Vec<SelectedTeam> = (0..SELECTED_TEAMS)
        .map(|ordinal| SelectedTeam {
            prices: [u8::try_from(ordinal).expect("160"); 16],
            controls: controls[ordinal % controls.len()],
        })
        .collect();
    let career_team = SelectedTeam {
        prices: [u8::MAX; 16],
        controls: u8::from(TeamControls::PlayerCoach),
    };
    let mut oracle = Oracle::lock();
    let mut started_alike = 0;
    for tournament in &started {
        let resolved = WorldTournament::resolve(world, tournament.competition, tournament.source)
            .expect("a tournament of the world");
        let start = resolved.start(Pitch::Seasonal);
        // The state's teams: the seeds backwards, so they order them, then the selected teams.
        let mut state = TournamentState::default();
        let seeds: Vec<[u8; 2]> = start
            .record
            .seeds
            .iter()
            .copied()
            .filter(|&seed| seed != [0, 0])
            .collect();
        for index in 0..TEAMS {
            let team = seeds
                .len()
                .checked_sub(index + 1)
                .map_or([0, u8::try_from(index).expect("64")], |seed| seeds[seed]);
            state.set_team(index, team);
        }
        let before = *state.image();
        let competition = [0; IMAGE_LEN];
        oracle.set_rand2_state(RandState {
            seed: 0,
            xor_index: 0,
            xor_key: 0,
        });
        let upstream = match tournament.reached {
            Reached::Compiled(record) => oracle
                .start_compiled_tournament(&CompiledTournamentSetup {
                    state: Box::new(before),
                    competition: Box::new(competition),
                    record,
                    pitch_type: SEASONAL,
                    teams: teams.clone(),
                    career_team,
                })
                .unwrap_or_else(|| panic!("upstream has no {record:?}")),
            Reached::Json => oracle.start_tournament(&TournamentSetup {
                state: Box::new(before),
                competition: Box::new(competition),
                record: json_record(world, tournament.competition),
                national: world.is_national_teams(tournament.competition, tournament.source),
                continental: false,
                odd_groups: world.odd_tournament_groups(),
                pitch_type: SEASONAL,
                teams: teams.clone(),
                career_team,
            }),
        };
        let mut rng = Rng::default();
        rng.secondary = Generator::default();
        let mut ours = TournamentState::from_image(&before);
        let mut ours_competition = RunningCompetition::from_image(&competition);
        let result = start_tournament(
            &mut ours,
            &mut ours_competition,
            &start,
            selected_loader(&teams, &career_team),
            &mut rng,
        );
        let id = world.competition_id(tournament.competition).as_str();
        match upstream {
            Err(halt) => {
                assert_eq!(halt, 0x1313, "{id}");
                assert!(
                    matches!(result, Err(StartError::OddGroups { .. })),
                    "{id}: {result:?}"
                );
            }
            Ok(after) => {
                assert!(result.is_ok(), "{id}: {result:?}");
                assert_alike((&ours, &ours_competition), &after, id)
                    .unwrap_or_else(|error| panic!("{id} ({:?}): {error}", tournament.source));
                started_alike += 1;
            }
        }
    }
    assert!(started_alike >= MIN_TOURNAMENTS, "{started_alike} started");
}
