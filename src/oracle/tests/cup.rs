//! `openswos_core::competition::cup` against upstream's src/diy_competition.c
//! (`InitDIYCupFromRecord`): from any image, any cup's or play-off's record started alike, byte
//! for byte (the C's name pointer, an address, aside), and odd teams refused as upstream halts;
//! and every knockout of World 1996-97, resolved from its package (`WorldKnockout`), started as
//! upstream starts its compiled records with its own tables. A cup played (`decide`,
//! `play_cup_match`, `next_round`: upstream's `ProcessDIYKnockoutMatchResultAndCareerUpdates`,
//! `DIY_PlayCupMatchAndAdvance`, `DIY_UpdateCompetitionProgress`, its results cached): any tie
//! decided alike, the same parts of its results asked; any next round alike; any cup's record
//! and every knockout of World 1996-97 played through to its winner alike, match by match; the
//! round names alike. Without upstream's checkout the oracle is not built and these tests are
//! not either (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

mod given;
mod sim_setup;
mod simulated;

use std::sync::LazyLock;

use given::Given;
use openswos_common::Nibble;
use openswos_common::json::JsonByte;
use openswos_competitions_json_resource::{
    CompetitionHeader, CompetitionType, Cup, CupKind, Playoff, RoundRules, SymbolId,
};
use openswos_core::competition::cup::{
    CupFixture, CupPlayError, CupRecord, CupStart, StartError, WorldKnockout, decide, next_round,
    play_cup_match, round_name, stage_name, start_cup,
};
use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition};
use openswos_core::competition::start::{LoadedTeam, Pitch};
use openswos_core::competition::tournament::{STATE_LEN, TournamentState};
use openswos_core::sim::{Simulation, formation_orders};
use openswos_core::world::{CompetitionRef, Source, TableContent, World};
use openswos_oracle::Oracle;
use openswos_oracle::cup::{CompiledCupSetup, CupSetup, KnockoutBytes, Layout, SelectedTeam};
use openswos_oracle::cup_play::{CachedResults, CupMatchSetup, Taken};
use openswos_oracle::draw::Prices;
use openswos_oracle::league::CompiledRecord;
use openswos_oracle::rng::RandState;
use openswos_oracle::tables::country;
use openswos_rng::{Generator, Rng};
use openswos_team_resource::TeamControls;
use openswos_test_utils::world_1996_97_path;
use openswos_world_package_resource::{PackageFiles, WorldPackage};
use proptest::collection::vec;
use proptest::prelude::*;
use sim_setup::core_tactics;
use simulated::{
    CAREER_PLACE, CAREER_TEAM, Recorded, Sim, england, generators_alike, squads_alike, variant,
};

/// The name pointer's bytes in the image (an address: the C's own, not compared).
const NAME_POINTER: std::ops::Range<usize> = 40..48;

/// The name upstream's cup reset writes (src/swos_amiga_strings.c `_aDiyCup`).
const DIY_CUP: &[u8] = b"DIY CUP";

/// A cup's name when its record names none (`_aCup_0`).
const CUP: &[u8] = b"CUP";

/// The session's pitch type for the season's (upstream `_pitchType` -2).
const SEASONAL: u16 = 0xFFFE;

/// The selected teams the stubbed loader gives, by ordinal.
const SELECTED_TEAMS: usize = 160;

/// The country of the career club's team numbers in the stubbed loader.
const CAREER_COUNTRY: u8 = 0xFF;

/// A knockout's type (`CompetitionType::Cup`): upstream's `comp_name_table` finds a cup's
/// names by it.
const KNOCKOUT: u8 = 1;

/// The header byte that tells a play-off's record from a cup's (`isPlayoff`): upstream's
/// `comp_name_table` finds the names by it.
const IS_PLAYOFF: usize = 6;

/// The competition ids a record takes here: below World 2026-27's continental contests, which
/// `continental_record` names by id (the Copa, 162, and up).
const IDS: std::ops::RangeInclusive<u8> = 0..=161;

/// A cup's teams here: 1 to 64, the most it holds (an odd number halts).
const CUP_TEAMS: std::ops::RangeInclusive<i8> = 1..=64;

/// A play-off's teams here: 1 to 7, the brackets its two round rules cover (more read its
/// names' address as rules).
const PLAYOFF_TEAMS: std::ops::RangeInclusive<i8> = 1..=7;

/// The round rules a cup's record holds.
const CUP_RULES: usize = 6;

/// The round rules a play-off's record holds.
const PLAYOFF_RULES: usize = 2;

/// The team controls the teams take: upstream's four (`TEAM_NOT_SELECTED`..`TEAM_COACH`).
const CONTROLS: std::ops::RangeInclusive<u8> = 0..=3;

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

/// A name: ASCII letters and spaces, up to past the 35 the buffer takes.
fn name() -> impl Strategy<Value = Vec<u8>> {
    vec(prop_oneof![b'A'..=b'Z', Just(b' ')], 0..=40)
}

/// A header's nine bytes: the id within [`IDS`], a knockout's type, the play-off byte as the
/// layout's, the rest any.
fn header(layout: Layout) -> impl Strategy<Value = [u8; 9]> {
    (IDS, any::<[u8; 6]>()).prop_map(move |(id, [team, begin, end, a, b, c])| {
        let mut header = [id, KNOCKOUT, team, begin, end, a, 0, b, c];
        header[IS_PLAYOFF] = u8::from(layout == Layout::Playoff);
        header
    })
}

/// A knockout's record of `layout`: its teams within `teams`, `rules` round rules.
fn knockout(
    layout: Layout,
    teams: std::ops::RangeInclusive<i8>,
    rules: usize,
) -> impl Strategy<Value = KnockoutBytes> {
    (
        header(layout),
        (any::<i8>(), teams, any::<i8>(), any::<u8>(), any::<i8>()),
        vec(any::<i8>(), rules),
        [
            proptest::option::weighted(0.8, name()),
            proptest::option::of(name()),
        ],
    )
        .prop_map(
            move |(header, (overrides, teams, away, subs, base), round_rules, names)| {
                KnockoutBytes {
                    layout,
                    header,
                    num_rounds_override: overrides,
                    teams,
                    away_goals_mode: away,
                    substitutes: subs,
                    round_string_base: base,
                    round_rules,
                    names,
                }
            },
        )
}

/// A cup's record or a play-off's.
fn record() -> impl Strategy<Value = KnockoutBytes> {
    prop_oneof![
        knockout(Layout::Cup, CUP_TEAMS, CUP_RULES),
        knockout(Layout::Playoff, PLAYOFF_TEAMS, PLAYOFF_RULES),
    ]
}

/// A selected team: any prices, one of upstream's controls.
fn team() -> impl Strategy<Value = SelectedTeam> {
    (any::<Prices>(), CONTROLS).prop_map(|(prices, controls)| SelectedTeam { prices, controls })
}

/// The core's record of `bytes`, through the competition files' models.
fn core_record(bytes: &KnockoutBytes) -> CupRecord {
    let [id, kind, team, begin, end, a, b, c, d] = bytes.header;
    let comp = CompetitionHeader {
        comp_id: JsonByte::from(id),
        kind: CompetitionType::try_from(kind).expect("a type"),
        team_number: JsonByte::from(team),
        begin_month: JsonByte::from(begin),
        end_month: JsonByte::from(end),
        name_table_offset: JsonByte::from(a),
        reserved: JsonByte::from(b),
        team_list_offset: JsonByte::from(c),
        seed_reorder_flag: JsonByte::from(d),
    };
    let byte = |value: i8| JsonByte::from(value.cast_unsigned());
    let nibble = |value: u8| Nibble::try_from(value).expect("a nibble");
    let rules: Vec<RoundRules> = bytes
        .round_rules
        .iter()
        .map(|&rule| RoundRules::from(rule.cast_unsigned()))
        .collect();
    match bytes.layout {
        Layout::Cup => CupRecord::from_cup(&Cup {
            id: SymbolId("_cup".to_owned()),
            kind: CupKind::Cup,
            comp,
            num_rounds_override: byte(bytes.num_rounds_override),
            teams: byte(bytes.teams),
            away_goals_mode: byte(bytes.away_goals_mode),
            substitutes: nibble(bytes.substitutes >> 4),
            points_for_win: nibble(bytes.substitutes & 0x0F),
            round_string_base: byte(bytes.round_string_base),
            round_rule_flags: rules,
            name: Vec::new(),
            team_seed_list: Vec::new(),
        }),
        Layout::Playoff => CupRecord::from_playoff(&Playoff {
            id: SymbolId("_playoff".to_owned()),
            comp,
            num_rounds_override: byte(bytes.num_rounds_override),
            teams: byte(bytes.teams),
            away_goals_mode: byte(bytes.away_goals_mode),
            substitutes: nibble(bytes.substitutes >> 4),
            max_subs: nibble(bytes.substitutes & 0x0F),
            round_string_base: byte(bytes.round_string_base),
            round_rule_flags: rules,
            name: Vec::new(),
        }),
    }
    .expect("a knockout")
}

/// The core's view of a selected team.
fn loaded(identity: i32, team: &SelectedTeam) -> LoadedTeam {
    LoadedTeam {
        identity,
        controls: TeamControls::try_from(team.controls).expect("a control"),
        prices: team.prices,
    }
}

/// The loader the stub is: the career club for country 0xFF, else the selected team of the
/// ordinal.
fn loader<'a>(
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

proptest! {
    #[test]
    fn a_cup_starts_as_upstream_starts_it(
        before in vec(any::<u8>(), IMAGE_LEN),
        record in record(),
        (national, continental) in any::<(bool, bool)>(),
        pitch_type in prop_oneof![Just(SEASONAL), any::<u16>()],
        teams in vec(team(), SELECTED_TEAMS),
        career_team in team(),
        state in any::<(u8, u8, u8)>(),
    ) {
        let before: Box<[u8; IMAGE_LEN]> = before.into_boxed_slice().try_into().expect("an image");
        let adjective = country(record.header[2]).expect("built").map(|entry| entry.adjective);
        let setup = CupSetup {
            before: before.clone(),
            record: record.clone(),
            national,
            continental,
            pitch_type,
            teams: teams.clone(),
            career_team,
        };
        let (seed, xor_index, xor_key) = state;
        let mut oracle = Oracle::lock();
        oracle.set_rand2_state(RandState { seed, xor_index, xor_key });
        let upstream = oracle.start_cup(&setup);
        let mut rng = Rng::default();
        rng.secondary = Generator { seed, xor_index, xor_key };
        let core_record = core_record(&record);
        // `comp_name_table`: the first name, else the default.
        let name = match &record.names[0] {
            Some(name) => name.clone(),
            None => CUP.to_vec(),
        };
        let prefix = adjective.filter(|_| !national);
        let start = CupStart {
            record: &core_record,
            name: &name,
            prefix: prefix.as_deref(),
            default_name: DIY_CUP,
            continental,
            pitch: if pitch_type == SEASONAL { Pitch::Seasonal } else { Pitch::Type(pitch_type.cast_signed()) },
        };
        let mut ours = RunningCompetition::from_image(&before);
        let started = start_cup(&mut ours, &start, loader(&teams, &career_team), &mut rng);
        match upstream {
            Err(halt) => {
                prop_assert_eq!(halt, 0x1313);
                prop_assert!(matches!(started, Err(StartError::OddTeams(_))), "{:?}", started);
            }
            Ok(after) => {
                prop_assert!(started.is_ok(), "{:?}", started);
                let mut theirs = *after;
                theirs[NAME_POINTER].copy_from_slice(&before[NAME_POINTER]);
                for (offset, (&mine, &upstream)) in ours.image().iter().zip(theirs.iter()).enumerate() {
                    prop_assert_eq!(mine, upstream, "byte {}", offset);
                }
                prop_assert_eq!(
                    (rng.secondary.seed, rng.secondary.xor_index, rng.secondary.xor_key),
                    {
                        let state = oracle.rand2_state();
                        (state.seed, state.xor_index, state.xor_key)
                    }
                );
            }
        }
    }
}

/// The fewest knockouts of World 1996-97 the compiled check reaches: the country tables' cups
/// (51), the career's knockouts from their records and copies (the Cup Winners' Cup and the
/// UEFA Cup, 4), the play-offs (7).
const MIN_COMPILED_KNOCKOUTS: usize = 62;

/// A knockout of World 1996-97 upstream compiles: how the oracle reaches it, and the world's
/// competition and source for it.
struct Compiled {
    /// Upstream's record.
    record: CompiledRecord,
    /// The world's competition.
    competition: CompetitionRef,
    /// Whether the career's copy is started.
    source: Source,
}

/// Whether `competition` is one of `world`'s knockouts.
fn is_knockout(world: &World, competition: CompetitionRef) -> bool {
    WorldKnockout::resolve(world, competition, Source::Record).is_ok()
}

/// Every knockout of World 1996-97 upstream's tables reach: each country table's cups, each
/// career contest played as a knockout, from its record and from its copy, and each play-off.
fn compiled_knockouts(world: &World) -> Vec<Compiled> {
    let mut records = Vec::new();
    for table in world.country_tables() {
        let TableContent::Own { cups, .. } = &table.content else {
            continue;
        };
        for (slot, &cup) in cups.iter().enumerate() {
            let competition = CompetitionRef::Cup(cup);
            if is_knockout(world, competition) {
                records.push(Compiled {
                    record: CompiledRecord::CountryCup {
                        country: table.number.0,
                        slot,
                    },
                    competition,
                    source: Source::Record,
                });
            }
        }
    }
    for contest in world.career() {
        let Some(competition) = contest.competition else {
            continue;
        };
        if !is_knockout(world, competition) {
            continue;
        }
        let comp_id = world.header(competition).comp_id.byte();
        for (source, copy) in [(Source::Record, false), (Source::CareerCopy, true)] {
            records.push(Compiled {
                record: CompiledRecord::CareerContest { comp_id, copy },
                competition,
                source,
            });
        }
    }
    for playoff in world.playoffs() {
        let competition = world
            .competition(&playoff.record.id)
            .expect("the world's play-off");
        records.push(Compiled {
            record: CompiledRecord::Bound(playoff.record.comp.comp_id.byte()),
            competition,
            source: Source::Record,
        });
    }
    records
}

#[test]
fn world_1996_97s_knockouts_start_as_upstreams_compiled_records() {
    let world = &*WORLD;
    let records = compiled_knockouts(world);
    assert!(
        records.len() >= MIN_COMPILED_KNOCKOUTS,
        "{} records",
        records.len()
    );
    // The teams told apart by ordinal: their prices, and every control in turn.
    let controls: Vec<u8> = CONTROLS.collect();
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
    // The image's cup teams the selected teams in order.
    let mut image = RunningCompetition::default();
    for ordinal in 0..=u8::MAX >> 2 {
        image.set_cup_team(usize::from(ordinal), [0, ordinal]);
    }
    let before = Box::new(*image.image());
    let mut oracle = Oracle::lock();
    let mut started_alike = 0;
    for compiled in &records {
        oracle.set_rand2_state(RandState {
            seed: 0,
            xor_index: 0,
            xor_key: 0,
        });
        let upstream = oracle
            .start_compiled_cup(&CompiledCupSetup {
                before: before.clone(),
                record: compiled.record,
                pitch_type: SEASONAL,
                teams: teams.clone(),
                career_team,
            })
            .unwrap_or_else(|| panic!("upstream has no {:?}", compiled.record));
        let resolved = WorldKnockout::resolve(world, compiled.competition, compiled.source)
            .expect("a knockout of the world");
        let mut rng = Rng::default();
        rng.secondary = Generator::default();
        let mut ours = RunningCompetition::from_image(&before);
        let started = start_cup(
            &mut ours,
            &resolved.start(Pitch::Seasonal),
            loader(&teams, &career_team),
            &mut rng,
        );
        let id = world.competition_id(compiled.competition).as_str();
        match upstream {
            Err(halt) => {
                assert_eq!(halt, 0x1313, "{id}");
                assert!(
                    matches!(started, Err(StartError::OddTeams(_))),
                    "{id}: {started:?}"
                );
            }
            Ok(after) => {
                assert!(started.is_ok(), "{id}: {started:?}");
                let mut theirs = *after;
                theirs[NAME_POINTER].copy_from_slice(&before[NAME_POINTER]);
                for (offset, (&mine, &upstream)) in
                    ours.image().iter().zip(theirs.iter()).enumerate()
                {
                    assert_eq!(
                        mine, upstream,
                        "{id} ({:?}): byte {offset}",
                        compiled.source
                    );
                }
                started_alike += 1;
            }
        }
    }
    assert!(
        started_alike >= MIN_COMPILED_KNOCKOUTS,
        "{started_alike} started"
    );
}

/// Match `number`'s results, drawn from `seed`: goals 0 to 3 (decisive from the 200th match, so
/// replays end), extra time 0 to 2, a shoot-out 0 to 9.
fn results_of(seed: u64, number: usize) -> CachedResults {
    let mut state =
        seed ^ (u64::try_from(number).expect("a match") + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut next = |bound: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        u16::try_from(state % bound).expect("small")
    };
    let normal = if number >= 200 {
        (1, 0)
    } else {
        (next(4), next(4))
    };
    CachedResults {
        normal,
        extra: (next(3), next(3)),
        penalties: (next(10), next(10)),
    }
}

/// The two images and what was taken, alike.
fn assert_played_alike(
    ours: &RunningCompetition,
    our_state: &TournamentState,
    theirs: &[u8; IMAGE_LEN],
    their_state: &[u8; STATE_LEN],
    what: &str,
) -> Result<(), TestCaseError> {
    let mut theirs = *theirs;
    theirs[NAME_POINTER].copy_from_slice(&ours.image()[NAME_POINTER]);
    for (offset, (&mine, &upstream)) in ours.image().iter().zip(theirs.iter()).enumerate() {
        prop_assert_eq!(mine, upstream, "{}: byte {}", what, offset);
    }
    prop_assert_eq!(our_state.image(), their_state, "{}: the state", what);
    Ok(())
}

/// The most matches a cup plays here before it is called endless.
const MAX_MATCHES: usize = 1000;

/// Whether the port refuses where upstream runs past the bracket: a round of no ties (a cup
/// whose rounds are more than its bracket's), or a draw's order never drawn (a cup whose rounds
/// are fewer, its replays searching the order: upstream past it, or its counts overwritten
/// from it).
fn past_bracket(error: CupPlayError) -> bool {
    matches!(
        error,
        CupPlayError::NoTies | CupPlayError::NotInOrder(_) | CupPlayError::TiesPastGrid(_)
    )
}

/// How a cup played through ended.
#[derive(Debug)]
enum Ended {
    /// Finished, after this many matches.
    Finished(usize),
    /// Refused by the port where upstream runs past its bracket ([`past_bracket`]).
    Refused(CupPlayError),
}

/// Plays the cup started in `ours` and `theirs` (with `state`) to its end, as the career does:
/// its matches until a round is complete, then the next round unless it is finished; each step
/// alike on both sides, the results of match `n` `results_of(seed, n)`, or the `simulation`'s
/// and upstream's (the squads and the generators alike after each match too).
#[expect(
    clippy::too_many_arguments,
    reason = "both sides' cups and their results"
)]
fn play_through(
    oracle: &mut Oracle,
    mut ours: RunningCompetition,
    mut theirs: Box<[u8; IMAGE_LEN]>,
    teams: &[SelectedTeam],
    career_team: SelectedTeam,
    rng: &mut Rng,
    seed: u64,
    mut simulation: Option<&mut Sim<'_>>,
) -> Result<Ended, TestCaseError> {
    let mut our_state = TournamentState::default();
    let mut their_state = Box::new([0; STATE_LEN]);
    for number in 0..MAX_MATCHES {
        let results = results_of(seed, number);
        let loaded = loader(teams, &career_team);
        let (played, taken) = match simulation.as_deref_mut() {
            Some(simulation) => {
                let mut recorded = Recorded::new(simulation);
                let played = play_cup_match(&mut ours, &mut our_state, &mut recorded, loaded, rng);
                (played, recorded.taken)
            }
            None => {
                let mut given = Given::new(results);
                let played = play_cup_match(&mut ours, &mut our_state, &mut given, loaded, rng);
                (played, given.taken)
            }
        };
        if let Err(error) = played
            && past_bracket(error)
        {
            return Ok(Ended::Refused(error));
        }
        prop_assert!(played.is_ok(), "match {}: {:?}", number, played);
        let (after, state_after, their_taken) = oracle.play_cup_match(&CupMatchSetup {
            competition: theirs,
            state: their_state,
            teams: teams.to_vec(),
            career_team,
            results: simulation.is_none().then_some(results),
        });
        (theirs, their_state) = (after, state_after);
        prop_assert_eq!(taken, their_taken, "match {}: the parts taken", number);
        assert_played_alike(
            &ours,
            &our_state,
            &theirs,
            &their_state,
            &format!("match {number}"),
        )?;
        if let Some(simulation) = simulation.as_deref() {
            squads_alike(oracle, &simulation.squads)
                .and_then(|()| generators_alike(oracle, rng))
                .map_err(|error| TestCaseError::fail(format!("match {number}: {error}")))?;
        }
        if ours.round_complete() == 0 {
            continue;
        }
        if ours.competition_finished() != 0 {
            return Ok(Ended::Finished(number + 1));
        }
        let advanced = next_round(&mut ours);
        if let Err(error) = advanced
            && past_bracket(error)
        {
            return Ok(Ended::Refused(error));
        }
        prop_assert!(advanced.is_ok(), "after match {}: {:?}", number, advanced);
        theirs = oracle.next_round(&theirs, &their_state);
        assert_played_alike(
            &ours,
            &our_state,
            &theirs,
            &their_state,
            &format!("after match {number}"),
        )?;
    }
    Err(TestCaseError::fail("a cup that never ends"))
}

/// A cup's state for a tie: any image, its round's rules and its tie's formats any of upstream's
/// codes.
fn tie_state() -> impl Strategy<Value = [u8; IMAGE_LEN]> {
    (
        vec(any::<u8>(), IMAGE_LEN),
        (
            0..=5i16,
            0..=3u16,
            0..=3u16,
            0..=3u16,
            0..=2u16,
            0..=2u16,
            0..=3i16,
        ),
    )
        .prop_map(
            |(bytes, (round, extra, penalties, format, legs, replay, away))| {
                let image: [u8; IMAGE_LEN] = bytes.try_into().expect("an image");
                let mut competition = RunningCompetition::from_image(&image);
                competition.set_current_round_index(round);
                let round = usize::try_from(round).expect("0 to 5");
                competition.set_cup_round_extra_time(round, extra);
                competition.set_cup_round_penalties(round, penalties);
                competition.set_round_format(format);
                competition.set_competition_format(legs);
                competition.set_group_section_flag(replay);
                competition.set_play_each_team(away);
                *competition.image()
            },
        )
}

/// A pair of goals or penalties: mostly small, sometimes past a byte (the tie is decided on
/// their low bytes).
fn pair() -> impl Strategy<Value = (u16, u16)> {
    prop_oneof![3 => (0..5u16, 0..5u16), 1 => any::<(u16, u16)>()]
}

/// A match's results.
fn cached() -> impl Strategy<Value = CachedResults> {
    (pair(), pair(), pair()).prop_map(|(normal, extra, penalties)| CachedResults {
        normal,
        extra,
        penalties,
    })
}

proptest! {
    #[test]
    fn a_tie_is_decided_as_upstream_decides_it(
        image in tie_state(),
        first_leg in (0..5u16, 0..5u16),
        results in cached(),
    ) {
        let competition = RunningCompetition::from_image(&image);
        let fixture = CupFixture { home_slot: 0, away_slot: 1, home: 0, away: 1 };
        let mut given = Given { results, taken: Taken::default() };
        let ours = decide(&competition, fixture, first_leg, &mut given, &mut Rng::default()).expect("decided");
        let teams: Vec<SelectedTeam> = (0..2).map(|_| SelectedTeam { prices: [0; 16], controls: 1 }).collect();
        let theirs = Oracle::lock().decide_tie(
            &CupMatchSetup {
                competition: Box::new(image),
                state: Box::new([0; STATE_LEN]),
                teams,
                career_team: SelectedTeam { prices: [0; 16], controls: 1 },
                results: Some(results),
            },
            first_leg,
        );
        prop_assert_eq!(
            (ours.goals, ours.aggregate, ours.penalties, u16::from(ours.flags.0)),
            (theirs.goals, theirs.aggregate, theirs.penalties, theirs.flags)
        );
        prop_assert_eq!(given.taken, theirs.taken, "the same parts asked");
    }

    #[test]
    fn a_cup_plays_through_as_upstream_plays_it(
        before in vec(any::<u8>(), IMAGE_LEN),
        mut record in record(),
        // Most cups their bracket's rounds (0), some another number.
        overrides in prop_oneof![3 => Just(0i8), 1 => -2..=8i8],
        teams in vec(team(), SELECTED_TEAMS),
        career_team in team(),
        rand in any::<(u8, u8, u8)>(),
        seed in any::<u64>(),
    ) {
        let before: Box<[u8; IMAGE_LEN]> = before.into_boxed_slice().try_into().expect("an image");
        record.num_rounds_override = overrides;
        let core_record = core_record(&record);
        let name = record.names[0].clone().unwrap_or_else(|| CUP.to_vec());
        let start = CupStart {
            record: &core_record,
            name: &name,
            prefix: None,
            default_name: DIY_CUP,
            continental: false,
            pitch: Pitch::Seasonal,
        };
        let (seed2, xor_index, xor_key) = rand;
        let mut rng = Rng::default();
        rng.secondary = Generator { seed: seed2, xor_index, xor_key };
        let mut ours = RunningCompetition::from_image(&before);
        prop_assume!(start_cup(&mut ours, &start, loader(&teams, &career_team), &mut rng).is_ok());
        let mut oracle = Oracle::lock();
        oracle.set_rand2_state(RandState { seed: seed2, xor_index, xor_key });
        let theirs = oracle
            .start_cup(&CupSetup {
                before: before.clone(),
                record: record.clone(),
                national: true,
                continental: false,
                pitch_type: SEASONAL,
                teams: teams.clone(),
                career_team,
            })
            .expect("started");
        let ended = play_through(&mut oracle, ours, theirs, &teams, career_team, &mut rng, seed, None)?;
        if let Ended::Refused(error) = ended {
            prop_assert!(past_bracket(error), "{:?}", error);
        }
    }

    #[test]
    fn the_next_round_is_upstream_s(
        image in vec(any::<u8>(), IMAGE_LEN),
        pairs in 1..=32i16,
        flags in vec(any::<u8>(), 32),
        order_seed in any::<u64>(),
        (finished, legs, format, replay, undecided, round) in (0..=1i16, 0..=2u16, 0..=3u16, 0..=1u16, 0..=4u16, 0..=4i16),
        has_groups in 0..=1i16,
    ) {
        let image: [u8; IMAGE_LEN] = image.try_into().expect("an image");
        let mut competition = RunningCompetition::from_image(&image);
        competition.set_match_pairs_per_round(pairs);
        competition.set_total_matches_in_round(pairs);
        for (tie, &flag) in flags.iter().enumerate() {
            competition.set_cup_round_result(tie, flag);
        }
        // The draw's order: a permutation of the ties, then the rest as they are.
        let count = usize::try_from(pairs).expect("1 to 32");
        let mut order: Vec<u8> = (0..u8::try_from(count).expect("32")).collect();
        let mut state = order_seed | 1;
        for place in (1..count).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            order.swap(place, usize::try_from(state % u64::try_from(place + 1).expect("33")).expect("a place"));
        }
        for (place, &index) in order.iter().enumerate() {
            competition.set_shuffle_order(place, index);
        }
        competition.set_competition_finished(finished);
        competition.set_competition_format(legs);
        competition.set_round_format(format);
        competition.set_group_section_flag(replay);
        competition.set_second_leg_counter(undecided);
        competition.set_current_round_index(round);
        competition.set_has_groups(has_groups);
        let image = *competition.image();
        let next = next_round(&mut competition);
        prop_assume!(next.is_ok(), "{:?}", next);
        let theirs = Oracle::lock().next_round(&image, &[0; STATE_LEN]);
        prop_assert_eq!(competition.image(), &*theirs);
    }

    #[test]
    fn the_round_names_are_upstream_s(
        (round, rounds, base) in any::<(u16, u16, u16)>(),
        (stage, groups, stages, teams, third_place) in (0..8u16, 0..3u16, 0..8u16, prop_oneof![Just(4u16), Just(8u16), any::<u16>()], 0..=1u16),
    ) {
        let mut oracle = Oracle::lock();
        prop_assert_eq!(round_name(round, rounds, base), oracle.round_name(round, rounds, base));
        prop_assert_eq!(
            stage_name(stage, groups, stages, teams, third_place != 0),
            oracle.stage_name(stage, groups, stages, teams, third_place)
        );
    }
}

#[test]
fn world_1996_97s_knockouts_play_through_as_upstream_s() {
    let world = &*WORLD;
    let records = compiled_knockouts(world);
    let controls: Vec<u8> = CONTROLS.collect();
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
    let mut image = RunningCompetition::default();
    for ordinal in 0..=u8::MAX >> 2 {
        image.set_cup_team(usize::from(ordinal), [0, ordinal]);
    }
    let before = Box::new(*image.image());
    let mut oracle = Oracle::lock();
    let mut played = 0;
    for (number, compiled) in records.iter().enumerate() {
        oracle.set_rand2_state(RandState {
            seed: 0,
            xor_index: 0,
            xor_key: 0,
        });
        let theirs = oracle
            .start_compiled_cup(&CompiledCupSetup {
                before: before.clone(),
                record: compiled.record,
                pitch_type: SEASONAL,
                teams: teams.clone(),
                career_team,
            })
            .expect("upstream's record");
        let Ok(theirs) = theirs else {
            continue;
        };
        let resolved = WorldKnockout::resolve(world, compiled.competition, compiled.source)
            .expect("a knockout of the world");
        let mut rng = Rng::default();
        rng.secondary = Generator::default();
        let mut ours = RunningCompetition::from_image(&before);
        start_cup(
            &mut ours,
            &resolved.start(Pitch::Seasonal),
            loader(&teams, &career_team),
            &mut rng,
        )
        .expect("started");
        let id = world.competition_id(compiled.competition).as_str();
        let seed = u64::try_from(number).expect("a few");
        let ended = play_through(
            &mut oracle,
            ours,
            theirs,
            &teams,
            career_team,
            &mut rng,
            seed,
            None,
        )
        .unwrap_or_else(|error| panic!("{id} ({:?}): {error}", compiled.source));
        let Ended::Finished(matches) = ended else {
            panic!("{id}: {ended:?}");
        };
        assert!(matches > 0, "{id}: no match");
        played += 1;
    }
    assert!(played >= MIN_COMPILED_KNOCKOUTS, "{played} played");
}

#[test]
fn world_1996_97s_knockouts_simulate_through_as_upstream_s() {
    let world = &*WORLD;
    let records = compiled_knockouts(world);
    let mut oracle = Oracle::lock();
    let tactics_records = oracle.compiled_tactics();
    let tactics = core_tactics(&tactics_records);
    let orders = formation_orders(&tactics);
    // The image's teams the selected teams in order, the career club second.
    let mut image = RunningCompetition::default();
    for ordinal in 0..=u8::MAX >> 2 {
        image.set_cup_team(usize::from(ordinal), [0, ordinal]);
    }
    image.set_cup_team(CAREER_PLACE, CAREER_TEAM);
    let before = Box::new(*image.image());
    let mut played = 0;
    for (number, compiled) in records.iter().enumerate() {
        let england = england();
        let (teams, career_team) = england.selected_teams();
        oracle.set_rand2_state(RandState {
            seed: 0,
            xor_index: 0,
            xor_key: 0,
        });
        let theirs = oracle
            .start_compiled_cup(&CompiledCupSetup {
                before: before.clone(),
                record: compiled.record,
                pitch_type: SEASONAL,
                teams: teams.clone(),
                career_team,
            })
            .expect("upstream's record");
        let Ok(theirs) = theirs else {
            continue;
        };
        let resolved = WorldKnockout::resolve(world, compiled.competition, compiled.source)
            .expect("a knockout of the world");
        let mut rng = Rng::default();
        rng.secondary = Generator::default();
        let mut ours = RunningCompetition::from_image(&before);
        start_cup(
            &mut ours,
            &resolved.start(Pitch::Seasonal),
            loader(&teams, &career_team),
            &mut rng,
        )
        .expect("started");
        let id = world.competition_id(compiled.competition).as_str();
        let variant = variant(number);
        oracle.set_simulation(&variant.oracle_session(&england, &tactics_records));
        let mut simulation = Simulation::new(
            england.squads.clone(),
            variant.session(&tactics, &orders),
            england.loader(),
        );
        let seed = u64::try_from(number).expect("a few");
        let ended = play_through(
            &mut oracle,
            ours,
            theirs,
            &teams,
            career_team,
            &mut rng,
            seed,
            Some(&mut simulation),
        )
        .unwrap_or_else(|error| panic!("{id} ({:?}): {error}", compiled.source));
        let Ended::Finished(matches) = ended else {
            panic!("{id}: {ended:?}");
        };
        assert!(matches > 0, "{id}: no match");
        played += 1;
    }
    assert!(played >= MIN_COMPILED_KNOCKOUTS, "{played} played");
}
