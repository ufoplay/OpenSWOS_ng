//! `openswos_core::competition::league` against upstream's src/diy_competition.c
//! (`InitDIYCompetitionFromTableEntry`): from any image, any division of a league record, or
//! of a cup played as a league, started alike, byte for byte (the C's name pointer, an
//! address, aside), and an odd division refused as upstream halts; and every division of
//! World 1996-97's leagues, resolved from its package (`WorldDivision`), started as upstream
//! starts its compiled records with its own tables. Without upstream's checkout the oracle is
//! not built and these tests are not either (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

use std::sync::LazyLock;

use openswos_common::Nibble;
use openswos_common::json::JsonByte;
use openswos_competitions_json_resource::{
    CompetitionHeader, CompetitionType, Cup, CupKind, Division, League, RoundRules, SymbolId,
};
use openswos_core::competition::league::{
    DivisionStart, LeagueRecord, StartError, WorldDivision, start_division,
};
use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition};
use openswos_core::competition::schedule::Schedules;
use openswos_core::competition::start::{LoadedTeam, Pitch};
use openswos_core::world::{CompetitionRef, Source, TableContent, World};
use openswos_oracle::Oracle;
use openswos_oracle::draw::Prices;
use openswos_oracle::league::{CompiledRecord, CompiledSetup, DivisionSetup, RecordBytes};
use openswos_oracle::rng::RandState;
use openswos_oracle::tables::country;
use openswos_rng::{Generator, Rng};
use openswos_team_resource::TeamControls;
use openswos_test_utils::world_1996_97_path;
use openswos_world_package_resource::{PackageFiles, WorldPackage};
use proptest::collection::vec;
use proptest::prelude::*;

/// The name pointer's bytes in the image (an address: the C's own, not compared).
const NAME_POINTER: std::ops::Range<usize> = 40..48;

/// The name upstream's reset writes (src/swos_amiga_strings.c `_aDiyLeague`).
const DIY_LEAGUE: &[u8] = b"DIY LEAGUE";

/// The session's pitch type for the season's (upstream `_pitchType` -2).
const SEASONAL: u16 = 0xFFFE;

/// The selected teams the stubbed loader gives, by ordinal.
const SELECTED_TEAMS: usize = 160;

/// The country of the career club's team numbers in the stubbed loader.
const CAREER_COUNTRY: u8 = 0xFF;

/// The competition ids a league record takes here: from the customs league's
/// (`COMPID_customsLeague`) to below World 2026-27's continental contests, which
/// `continental_record` names by id.
const LEAGUE_IDS: std::ops::RangeInclusive<u8> = 36..=161;

/// The competition ids a cup played as a league takes (below `COMPID_customsLeague`, as
/// upstream's `comp_is_type0_cup`).
const CUP_IDS: std::ops::RangeInclusive<u8> = 0..=35;

/// The divisions a record has here: four at most, as the default names.
const MAX_DIVISIONS: usize = 4;

/// The clubs of a division here: 1 to 24, the sizes upstream has round robins of (an odd one
/// halts).
const CLUBS: std::ops::RangeInclusive<i8> = 1..=24;

/// A `WORLD.ini` for the committed package, which holds none (its fingerprint, which covers
/// the rosters, is not read here).
const WORLD_INI: &str = "tag = 9697\nname = World 1996-97\nfingerprint = 0000000000000000\n";

/// The most rounds a record states here, as worlds' records do (a random byte makes seasons
/// whose months upstream writes past the competition).
const MAX_ROUNDS: i8 = 4;

/// A record's rounds: -1 (once) or 1 to [`MAX_ROUNDS`]. Not 0: a season of no match days, whose
/// months upstream's loop writes from 0 to 65535, past its memory.
fn rounds() -> impl Strategy<Value = i8> {
    prop_oneof![Just(-1i8), 1..=MAX_ROUNDS]
}

/// World 1996-97, from its committed package.
static WORLD: LazyLock<World> = LazyLock::new(|| {
    let mut files = PackageFiles::read_dir(&world_1996_97_path()).expect("the committed package");
    files
        .insert("WORLD.ini", WORLD_INI.as_bytes().to_vec())
        .expect("a safe path");
    let opened = WorldPackage::from_files(&files).expect("the package opens");
    World::from_package(&opened.package).expect("a valid world")
});

/// World 1996-97's round robins (the original's tables, the ones the oracle's C reads).
static SCHEDULES: LazyLock<Schedules> = LazyLock::new(|| WORLD.schedules().clone());

/// A name: ASCII letters and spaces, up to past the 35 the buffer takes.
fn name() -> impl Strategy<Value = Vec<u8>> {
    vec(prop_oneof![b'A'..=b'Z', Just(b' ')], 0..=40)
}

/// A division's six bytes, its clubs within [`CLUBS`].
fn division() -> impl Strategy<Value = [i8; 6]> {
    (CLUBS, any::<[i8; 5]>()).prop_map(|(clubs, [a, b, c, d, e])| [clubs, a, b, c, d, e])
}

/// A header's nine bytes: the id within `ids`, type 0 (a league), the rest any.
fn header(ids: std::ops::RangeInclusive<u8>) -> impl Strategy<Value = [u8; 9]> {
    (ids, any::<[u8; 7]>())
        .prop_map(|(id, [team, begin, end, a, b, c, d])| [id, 0, team, begin, end, a, b, c, d])
}

/// A record and a division of it.
fn record() -> impl Strategy<Value = (RecordBytes, u16)> {
    let league = (
        header(LEAGUE_IDS),
        (any::<i8>(), rounds(), any::<i8>(), any::<u8>()),
        vec(division(), 1..=MAX_DIVISIONS),
        vec(proptest::option::weighted(0.8, name()), 2 * MAX_DIVISIONS),
    )
        .prop_flat_map(
            |(header, (count, rounds, points, subs), divisions, names)| {
                let len = divisions.len();
                (
                    Just(RecordBytes::League {
                        header,
                        divisions_count: count,
                        rounds,
                        points_for_win: points,
                        substitutes: subs,
                        divisions,
                        names,
                    }),
                    0..u16::try_from(len).expect("4 divisions"),
                )
            },
        );
    let cup = (
        header(CUP_IDS),
        (any::<i8>(), rounds(), any::<i8>(), any::<u8>()),
        CLUBS,
        any::<[i8; 6]>(),
        [
            proptest::option::weighted(0.8, name()),
            proptest::option::of(name()),
        ],
    )
        .prop_map(
            |(header, (overrides, teams, away, subs), base, rules, names)| {
                (
                    RecordBytes::Cup {
                        header,
                        num_rounds_override: overrides,
                        teams,
                        away_goals_mode: away,
                        substitutes: subs,
                        round_string_base: base,
                        round_rules: rules,
                        names,
                    },
                    0,
                )
            },
        );
    prop_oneof![league, cup]
}

/// The core's record of `bytes`, through the competition files' models.
fn core_record(bytes: &RecordBytes) -> LeagueRecord {
    let comp = |header: [u8; 9]| {
        let [id, kind, team, begin, end, a, b, c, d] = header;
        CompetitionHeader {
            comp_id: JsonByte::from(id),
            kind: CompetitionType::try_from(kind).expect("a type"),
            team_number: JsonByte::from(team),
            begin_month: JsonByte::from(begin),
            end_month: JsonByte::from(end),
            name_table_offset: JsonByte::from(a),
            reserved: JsonByte::from(b),
            team_list_offset: JsonByte::from(c),
            seed_reorder_flag: JsonByte::from(d),
        }
    };
    let byte = |value: i8| JsonByte::from(value.cast_unsigned());
    let nibble = |value: u8| Nibble::try_from(value).expect("a nibble");
    match bytes {
        RecordBytes::League {
            header,
            divisions_count,
            rounds,
            points_for_win,
            substitutes,
            divisions,
            ..
        } => LeagueRecord::from_league(&League {
            id: SymbolId("_league".to_owned()),
            comp: comp(*header),
            divisions_count: byte(*divisions_count),
            rounds: byte(*rounds),
            points_for_win: byte(*points_for_win),
            substitutes: nibble(substitutes >> 4),
            max_subs: nibble(substitutes & 0x0F),
            divisions: divisions
                .iter()
                .map(
                    |&[teams, promoted, promoted_po, relegated, relegated_po, link]| Division {
                        teams: byte(teams),
                        promoted: byte(promoted),
                        promoted_po: byte(promoted_po),
                        relegated: byte(relegated),
                        relegated_po: byte(relegated_po),
                        qualifier_link_offset: byte(link),
                    },
                )
                .collect(),
            reserved1: JsonByte::from(0u8),
            name: Vec::new(),
            playoff_links: None,
            promotion_link_data: None,
            new: None,
        }),
        RecordBytes::Cup {
            header,
            num_rounds_override,
            teams,
            away_goals_mode,
            substitutes,
            round_string_base,
            round_rules,
            ..
        } => LeagueRecord::from_cup(&Cup {
            id: SymbolId("_cup".to_owned()),
            kind: CupKind::Cup,
            comp: comp(*header),
            num_rounds_override: byte(*num_rounds_override),
            teams: byte(*teams),
            away_goals_mode: byte(*away_goals_mode),
            substitutes: nibble(substitutes >> 4),
            points_for_win: nibble(substitutes & 0x0F),
            round_string_base: byte(*round_string_base),
            round_rule_flags: round_rules
                .iter()
                .map(|&rule| RoundRules::from(rule.cast_unsigned()))
                .collect(),
            name: Vec::new(),
            team_seed_list: Vec::new(),
        })
        .expect("a cup played as a league"),
    }
}

/// The division's name as upstream picks it (`comp_name_table`): none when the record's
/// table has no first name, then the default; the division's own, two a division.
fn division_name(bytes: &RecordBytes, division: u16, defaults: &[Vec<u8>; 4]) -> Vec<u8> {
    let table: Vec<Option<Vec<u8>>> = match bytes {
        RecordBytes::League { names, .. } => names.clone(),
        RecordBytes::Cup { names, .. } => names.to_vec(),
    };
    match table.first() {
        Some(Some(_)) => table
            .get(2 * usize::from(division))
            .cloned()
            .flatten()
            .unwrap_or_default(),
        Some(None) | None => defaults[usize::from(division)].clone(),
    }
}

fn header_of(bytes: &RecordBytes) -> [u8; 9] {
    match bytes {
        RecordBytes::League { header, .. } | RecordBytes::Cup { header, .. } => *header,
    }
}

proptest! {
    #[test]
    fn a_division_starts_as_upstream_starts_it(
        before in vec(any::<u8>(), IMAGE_LEN),
        (record, division) in record(),
        defaults in [name(), name(), name(), name()],
        (national, continental) in any::<(bool, bool)>(),
        pitch_type in prop_oneof![Just(SEASONAL), any::<u16>()],
        teams in vec(any::<Prices>(), SELECTED_TEAMS),
        career_team in any::<Prices>(),
        state in any::<(u8, u8, u8)>(),
    ) {
        let before: Box<[u8; IMAGE_LEN]> = before.into_boxed_slice().try_into().expect("an image");
        let header = header_of(&record);
        let adjective = country(header[2]).expect("built").map(|entry| entry.adjective);
        let setup = DivisionSetup {
            before: before.clone(),
            record: record.clone(),
            division,
            division_names: defaults.clone(),
            national,
            continental,
            pitch_type,
            teams: teams.clone(),
            career_team,
        };
        let (seed, xor_index, xor_key) = state;
        let mut oracle = Oracle::lock();
        oracle.set_rand2_state(RandState { seed, xor_index, xor_key });
        let upstream = oracle.start_division(&setup);
        let mut rng = Rng::default();
        rng.secondary = Generator { seed, xor_index, xor_key };
        let core_record = core_record(&record);
        let name = division_name(&record, division, &defaults);
        let prefix = adjective.filter(|_| !national);
        let start = DivisionStart {
            record: &core_record,
            division: usize::from(division),
            name: &name,
            prefix: prefix.as_deref(),
            default_name: DIY_LEAGUE,
            continental,
            pitch: if pitch_type == SEASONAL { Pitch::Seasonal } else { Pitch::Type(pitch_type.cast_signed()) },
        };
        let mut ours = RunningCompetition::from_image(&before);
        let started = start_division(&mut ours, &start, &SCHEDULES, |team| {
            let [country, ordinal] = team;
            Some(if country == CAREER_COUNTRY {
                LoadedTeam { identity: -1, controls: TeamControls::Computer, prices: career_team }
            } else {
                let index = usize::from(ordinal) % SELECTED_TEAMS;
                LoadedTeam {
                    identity: i32::try_from(index).expect("160"),
                    controls: TeamControls::Computer,
                    prices: teams[index],
                }
            })
        }, &mut rng);
        match upstream {
            Err(halt) => {
                prop_assert_eq!(halt, 0x1313);
                // Refused alike: a size without real rounds, or (one club) without a table.
                prop_assert!(
                    matches!(started, Err(StartError::NotRounds(_) | StartError::NoSchedule(_))),
                    "{:?}",
                    started
                );
            }
            Ok(after) => {
                prop_assert!(started.is_ok(), "{:?}", started);
                let mut theirs = *after;
                theirs[NAME_POINTER].copy_from_slice(&before[NAME_POINTER]);
                let ours = ours.image();
                for (offset, (&mine, &upstream)) in ours.iter().zip(theirs.iter()).enumerate() {
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

/// The fewest league records of World 1996-97 the compiled check reaches: a league a
/// country's table (66), the career's cups played as leagues, each from its record and its
/// copy.
const MIN_COMPILED_RECORDS: usize = 80;

/// A record of World 1996-97 upstream compiles: how the oracle reaches it, and the world's
/// competition and source for it.
struct Compiled {
    /// Upstream's record.
    record: CompiledRecord,
    /// The world's competition.
    competition: CompetitionRef,
    /// Whether the career's copy is started.
    source: Source,
    /// Its divisions.
    divisions: usize,
}

/// Every league of World 1996-97 upstream's tables reach: each country table's league, and
/// each career contest played as a league, from its record and from its copy.
fn compiled_records(world: &World) -> Vec<Compiled> {
    let mut records = Vec::new();
    for table in world.country_tables() {
        let TableContent::Own { leagues, .. } = &table.content else {
            continue;
        };
        if let Some(&league) = leagues.first() {
            records.push(Compiled {
                record: CompiledRecord::CountryLeague(table.number.0),
                competition: CompetitionRef::League(league),
                source: Source::Record,
                divisions: world.league(league).record.divisions.len(),
            });
        }
    }
    for contest in world.career() {
        let Some(competition) = contest.competition else {
            continue;
        };
        let record = match competition {
            CompetitionRef::Cup(cup) => LeagueRecord::from_cup(&world.cup(cup).record).ok(),
            CompetitionRef::League(_)
            | CompetitionRef::Tournament(_)
            | CompetitionRef::Playoff(_) => None,
        };
        let Some(record) = record else {
            continue;
        };
        for (source, copy) in [(Source::Record, false), (Source::CareerCopy, true)] {
            records.push(Compiled {
                record: CompiledRecord::CareerContest {
                    comp_id: record.comp_id,
                    copy,
                },
                competition,
                source,
                divisions: record.divisions.len(),
            });
        }
    }
    records
}

#[test]
fn world_1996_97s_leagues_start_as_upstreams_compiled_records() {
    let world = &*WORLD;
    let records = compiled_records(world);
    assert!(
        records.len() >= MIN_COMPILED_RECORDS,
        "{} records",
        records.len()
    );
    // The teams' prices told apart by ordinal, the image's team numbers all the first team's.
    let teams: Vec<Prices> = (0..SELECTED_TEAMS)
        .map(|ordinal| [u8::try_from(ordinal).expect("160"); 16])
        .collect();
    let career_team: Prices = [u8::MAX; 16];
    let before = Box::new([0; IMAGE_LEN]);
    let mut oracle = Oracle::lock();
    let mut started_alike = 0;
    for compiled in &records {
        for division in 0..compiled.divisions {
            let index = u16::try_from(division).expect("a few divisions");
            oracle.set_rand2_state(RandState {
                seed: 0,
                xor_index: 0,
                xor_key: 0,
            });
            let upstream = oracle
                .start_compiled_division(&CompiledSetup {
                    before: before.clone(),
                    record: compiled.record,
                    division: index,
                    pitch_type: SEASONAL,
                    teams: teams.clone(),
                    career_team,
                })
                .unwrap_or_else(|| panic!("upstream has no {:?}", compiled.record));
            let resolved =
                WorldDivision::resolve(world, compiled.competition, compiled.source, division)
                    .expect("a division of the world");
            let mut rng = Rng::default();
            rng.secondary = Generator::default();
            let mut ours = RunningCompetition::from_image(&before);
            let started = start_division(
                &mut ours,
                &resolved.start(Pitch::Seasonal),
                world.schedules(),
                |[country, ordinal]| {
                    Some(if country == CAREER_COUNTRY {
                        LoadedTeam {
                            identity: -1,
                            controls: TeamControls::Computer,
                            prices: career_team,
                        }
                    } else {
                        let index = usize::from(ordinal) % SELECTED_TEAMS;
                        let identity = i32::try_from(index).expect("160");
                        LoadedTeam {
                            identity,
                            controls: TeamControls::Computer,
                            prices: teams[index],
                        }
                    })
                },
                &mut rng,
            );
            let id = world.competition_id(compiled.competition).as_str();
            match upstream {
                Err(halt) => {
                    assert_eq!(halt, 0x1313, "{id} {division}");
                    assert!(
                        matches!(started, Err(StartError::NotRounds(_))),
                        "{id} {division}: {started:?}"
                    );
                }
                Ok(after) => {
                    assert!(started.is_ok(), "{id} {division}: {started:?}");
                    let mut theirs = *after;
                    theirs[NAME_POINTER].copy_from_slice(&before[NAME_POINTER]);
                    for (offset, (&mine, &upstream)) in
                        ours.image().iter().zip(theirs.iter()).enumerate()
                    {
                        assert_eq!(
                            mine, upstream,
                            "{id} division {division} ({:?}): byte {offset}",
                            compiled.source
                        );
                    }
                    started_alike += 1;
                }
            }
        }
    }
    assert!(
        started_alike >= MIN_COMPILED_RECORDS,
        "{started_alike} started"
    );
}
