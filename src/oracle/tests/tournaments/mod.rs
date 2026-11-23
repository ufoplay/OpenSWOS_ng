//! What the tournament checks share (`tournament.rs`, `stages.rs`): the C's constants and the
//! stubbed loader's, random records and teams, the core's view of them, World 1996-97 and its
//! tournaments, each started on both sides. Each check uses a part of it.
#![allow(dead_code, reason = "each check uses a part of it")]

use std::sync::LazyLock;

use openswos_common::Nibble;
use openswos_common::json::JsonByte;
use openswos_competitions_json_resource::{
    CompetitionHeader, CompetitionType, Round, RoundRules, SymbolId, Tournament, TournamentKind,
};
use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition};
use openswos_core::competition::start::{LoadedTeam, Pitch};
use openswos_core::competition::tournament::{
    OwnDraw, STAGES, StartError, TEAMS, TournamentRecord, TournamentState, WorldTournament,
    start_tournament,
};
use openswos_core::world::{CompetitionRef, Source, World};
use openswos_oracle::Oracle;
use openswos_oracle::cup::SelectedTeam;
use openswos_oracle::draw::Prices;
use openswos_oracle::league::CompiledRecord;
use openswos_oracle::rng::RandState;
use openswos_oracle::tournament::{
    CompiledTournamentSetup, Started, TournamentBytes, TournamentSetup,
};
use openswos_rng::{Generator, Rng};
use openswos_team_resource::TeamControls;
use openswos_test_utils::world_1996_97_path;
use openswos_world_package_resource::{PackageFiles, WorldPackage};
use proptest::collection::vec;
use proptest::prelude::*;

/// The name upstream's tournament reset writes (src/swos_amiga_strings.c `_aDiyTournament`).
pub(crate) const DIY_TOURNAMENT: &[u8] = b"DIY TOURNAMENT";

/// The selected teams the stubbed loader gives, by ordinal.
pub(crate) const SELECTED_TEAMS: usize = 160;

/// The country of the career club's team numbers in the stubbed loader.
pub(crate) const CAREER_COUNTRY: u8 = 0xFF;

/// The name pointers' bytes in both images (addresses: the C's own, not compared).
pub(crate) const NAME_POINTER: std::ops::Range<usize> = 40..48;

/// The names upstream's league and cup resets write (`_aDiyLeague`, `_aDiyCup`).
pub(crate) const DIY_LEAGUE: &[u8] = b"DIY LEAGUE";

/// See [`DIY_LEAGUE`].
pub(crate) const DIY_CUP: &[u8] = b"DIY CUP";

/// A tournament's name when its record names none (`_aLeague_1`).
pub(crate) const LEAGUE: &[u8] = b"LEAGUE";

/// The session's pitch type for the season's (upstream `_pitchType` -2).
pub(crate) const SEASONAL: u16 = 0xFFFE;

/// A tournament's type (`CompetitionType::Tournament`): upstream's `comp_name_table` finds its
/// names by it.
pub(crate) const TOURNAMENT: u8 = 2;

/// The competition ids a record takes here: below World 2026-27's continental contests, which
/// `continental_record` names by id (the Copa, 162, and up).
pub(crate) const IDS: std::ops::RangeInclusive<u8> = 0..=161;

/// The own draw upstream draws a competition of id `comp_id` by (diy_competition.c: the World
/// Cup's `buildCompSpecificTeamList`, the European Championships'
/// `buildCompSpecificTeamShuffle`): World 1996-97's `rules/draws.json`'s, none for the
/// engine's draw.
pub(crate) fn own_draw(comp_id: u8) -> Option<&'static OwnDraw> {
    let id = match comp_id {
        4 => "_worldCup",
        21 | 22 => "_europeanChampionships",
        _ => return None,
    };
    let world = &*WORLD;
    let competition = world
        .competition(&SymbolId(id.to_owned()))
        .expect("a tournament of the world");
    Some(world.own_draw(competition).expect("its own draw"))
}

/// A name: ASCII letters and spaces, up to past the 35 the buffer takes.
pub(crate) fn name() -> impl Strategy<Value = Vec<u8>> {
    vec(prop_oneof![b'A'..=b'Z', Just(b' ')], 0..=40)
}

/// A first stage's three bytes: groups of 2 to 8 (their teams mostly theirs), or a knockout
/// (doubled sometimes) of 2 to 64 teams.
pub(crate) fn first_stage() -> impl Strategy<Value = [i8; 3]> {
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
pub(crate) fn later_stage() -> impl Strategy<Value = [i8; 3]> {
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
pub(crate) fn record() -> impl Strategy<Value = (TournamentBytes, Vec<Option<usize>>)> {
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
pub(crate) fn selected() -> impl Strategy<Value = SelectedTeam> {
    (any::<Prices>(), 0..=3u8).prop_map(|(prices, controls)| SelectedTeam { prices, controls })
}

/// The core's record of `bytes`, through the competition files' model.
pub(crate) fn core_record(bytes: &TournamentBytes) -> TournamentRecord {
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
pub(crate) fn loaded(identity: i32, team: &SelectedTeam) -> LoadedTeam {
    LoadedTeam {
        identity,
        controls: TeamControls::try_from(team.controls).expect("a control"),
        prices: team.prices,
    }
}

/// The loader the stub is, over selected teams with their controls.
pub(crate) fn selected_loader<'a>(
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
pub(crate) fn assert_alike(
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

/// A `WORLD.ini` for the committed package, which holds none (its fingerprint, which covers
/// the rosters, is not read here).
pub(crate) const WORLD_INI: &str =
    "tag = 9697\nname = World 1996-97\nfingerprint = 0000000000000000\n";

/// World 1996-97, from its committed package.
pub(crate) static WORLD: LazyLock<World> = LazyLock::new(|| {
    let mut files = PackageFiles::read_dir(&world_1996_97_path()).expect("the committed package");
    files
        .insert("WORLD.ini", WORLD_INI.as_bytes().to_vec())
        .expect("a safe path");
    let opened = WorldPackage::from_files(&files).expect("the package opens");
    World::from_package(&opened.package).expect("a valid world")
});

/// The fewest tournaments of World 1996-97 the check starts: the career's eleven from their
/// records and copies, the World Cup and the European Championships, the Copa.
pub(crate) const MIN_TOURNAMENTS: usize = 25;

/// How upstream reaches a tournament of the world: a compiled record, or (the Copa, which
/// upstream's SDL build loads from the JSON) the record built from the world's JSON.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reached {
    /// A compiled record.
    Compiled(CompiledRecord),
    /// The JSON's record.
    Json,
}

/// A tournament of World 1996-97 to start: how upstream reaches it, the world's competition
/// and source for it.
pub(crate) struct Started9697 {
    /// How upstream reaches it.
    pub(crate) reached: Reached,
    /// The world's competition.
    pub(crate) competition: CompetitionRef,
    /// Whether the career's copy is started.
    pub(crate) source: Source,
}

/// Every tournament of World 1996-97: the career's from their records and copies, the two the
/// oracle binds, the others (the Copa) from the JSON.
pub(crate) fn tournaments(world: &World) -> Vec<Started9697> {
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
pub(crate) fn json_record(world: &World, competition: CompetitionRef) -> TournamentBytes {
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

/// The teams the checks of World 1996-97 give: each selected team priced by its ordinal, the
/// controls in turn; the career club the player-coach, priced highest.
pub(crate) fn world_teams() -> (Vec<SelectedTeam>, SelectedTeam) {
    let controls = [0u8, 1, 2, 3];
    let teams = (0..SELECTED_TEAMS)
        .map(|ordinal| SelectedTeam {
            prices: [u8::try_from(ordinal).expect("160"); 16],
            controls: controls[ordinal % controls.len()],
        })
        .collect();
    let career_team = SelectedTeam {
        prices: [u8::MAX; 16],
        controls: u8::from(TeamControls::PlayerCoach),
    };
    (teams, career_team)
}

/// A tournament of World 1996-97 started on both sides.
pub(crate) struct BothStarted {
    /// The world's tournament.
    pub(crate) resolved: WorldTournament,
    /// The port's state after.
    pub(crate) state: TournamentState,
    /// The port's competition under way after.
    pub(crate) competition: RunningCompetition,
    /// The port's start.
    pub(crate) result: Result<(), StartError>,
    /// Upstream's images after, or its halt.
    pub(crate) upstream: Result<Started, u16>,
    /// The port's generators after (upstream's are the oracle's).
    pub(crate) rng: Rng,
}

/// `tournament` of `world` started on both sides, as upstream starts it (a compiled record, or
/// its JSON's), with `teams` and `career_team`: the state's teams the record's seeds backwards
/// (so the seeds order them), then the selected teams; the competition under way cleared; the
/// second generators from 0.
pub(crate) fn start_9697(
    oracle: &mut Oracle,
    world: &World,
    tournament: &Started9697,
    teams: &[SelectedTeam],
    career_team: SelectedTeam,
) -> BothStarted {
    let resolved = WorldTournament::resolve(world, tournament.competition, tournament.source)
        .expect("a tournament of the world");
    let start = resolved.start(Pitch::Seasonal);
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
                teams: teams.to_vec(),
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
            teams: teams.to_vec(),
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
        selected_loader(teams, &career_team),
        &mut rng,
    );
    BothStarted {
        resolved,
        state: ours,
        competition: ours_competition,
        result,
        upstream,
        rng,
    }
}
