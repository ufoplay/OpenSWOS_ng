//! `openswos_core::competition::cup` against upstream's src/diy_competition.c
//! (`InitDIYCupFromRecord`): from any image, any cup's or play-off's record started alike, byte
//! for byte (the C's name pointer, an address, aside), and odd teams refused as upstream halts;
//! and every knockout of World 1996-97, resolved from its package (`WorldKnockout`), started as
//! upstream starts its compiled records with its own tables. Without upstream's checkout the
//! oracle is not built and these tests are not either (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

use std::sync::LazyLock;

use openswos_common::Nibble;
use openswos_common::json::JsonByte;
use openswos_competitions_json_resource::{
    CompetitionHeader, CompetitionType, Cup, CupKind, Playoff, RoundRules, SymbolId,
};
use openswos_core::competition::cup::{CupRecord, CupStart, StartError, WorldKnockout, start_cup};
use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition};
use openswos_core::competition::start::{LoadedTeam, Pitch};
use openswos_core::world::{CompetitionRef, Source, TableContent, World};
use openswos_oracle::Oracle;
use openswos_oracle::cup::{CompiledCupSetup, CupSetup, KnockoutBytes, Layout, SelectedTeam};
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
