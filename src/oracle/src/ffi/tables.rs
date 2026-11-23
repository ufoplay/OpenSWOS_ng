//! Upstream's data tables `world_extract` reads into its packages (src/strings_data.c,
//! src/swos_amiga_data_imports.c, src/competition_data.c), and World 2026-27's baked install
//! over them (src/world_data.c). Only that install writes them, once a process
//! (`tables::world_2627`): they are never reset.

use std::ffi::{CStr, c_char, c_void};

use super::bindings::{
    competition_data_c, fixture_data_c, fixture_rounds_c, string_symbols_c, strings_data_c,
    swos_amiga_data_imports_c, swos_amiga_ptr_imports_c, swos_amiga_strings_c,
    swos_data_bulk_imports_c, world_data_c,
};
use super::stubs::{self, LIST_END};
use crate::tables::{
    BakedWorld, CompetitionBase, CompetitionLists, CompiledString, CountryEntry, CupRecord,
    DefaultNames, FixtureSchedule, ListedRecord, OwnDrawTables, TournamentRecord, WorldCupEntry,
};

/// The entries of `_countriesTable` (src/strings_data.c).
const COUNTRIES_TABLE_LEN: usize = 262;

/// The entries of `_teamsCountryNumbers` (src/swos_amiga_data_imports.c): one a team file.
pub(crate) const TEAM_FILES: usize = 256;

/// The entries of `_careerCompetitionIdRemapTable` (src/competition_data.c): 65 countries and
/// the end byte.
const SEASON_COUNTRIES_LEN: usize = 66;

/// What ends the season end's countries.
const END_OF_COUNTRIES: u8 = 0xFF;

/// The sizes upstream has a round robin for: 2 to 24 clubs (`_fixtureSchedulePtrTable[23]`).
pub(crate) const SCHEDULE_SIZES: usize = 23;

/// The smallest of them.
const FIRST_SCHEDULE_SIZE: u8 = 2;

/// The bytes of `_euroChampFixtureData`.
const EURO_CHAMPIONSHIP_FIXTURES_LEN: usize = 28;

/// The European cups a world's league phases replace: the Champions', Cup Winners' and UEFA
/// Cup's (upstream src/league_phase.c `LeaguePhase_InstallCups`).
const EUROPEAN_CUPS: usize = 3;

/// The country of number `number` in `_countriesTable`, if it has an entry.
pub(crate) fn country(number: usize) -> Option<CountryEntry> {
    // SAFETY: upstream defines the table with its 262 entries; a non-null entry points at
    // one of upstream's static strings (`"\000ALBANIA\000ALBANIAN"`): the continent byte,
    // then the name and the adjective, each ended by a NUL.
    unsafe {
        let table = (&raw const strings_data_c::_countriesTable)
            .cast::<[*mut c_void; COUNTRIES_TABLE_LEN]>()
            .read();
        let entry = table.get(number).copied()?;
        if entry.is_null() {
            return None;
        }
        let bytes = entry.cast::<u8>();
        let continent = bytes.read();
        let name = CStr::from_ptr(bytes.add(1).cast::<c_char>());
        let adjective = CStr::from_ptr(bytes.add(1 + name.to_bytes_with_nul().len()).cast());
        Some(CountryEntry {
            continent,
            name: name.to_bytes().to_vec(),
            adjective: adjective.to_bytes().to_vec(),
        })
    }
}

/// Each team file's first global team number.
pub(crate) fn team_file_bases() -> [u16; TEAM_FILES] {
    // SAFETY: upstream defines the table with its 256 entries.
    unsafe {
        (&raw const swos_amiga_data_imports_c::_teamsCountryNumbers)
            .cast::<[u16; TEAM_FILES]>()
            .read()
    }
}

/// The countries of the season end, in its order, without the end byte.
pub(crate) fn season_countries() -> Vec<u8> {
    // SAFETY: upstream defines the table with its 66 bytes.
    let table = unsafe {
        (&raw const competition_data_c::_careerCompetitionIdRemapTable)
            .cast::<[u8; SEASON_COUNTRIES_LEN]>()
            .read()
    };
    table
        .into_iter()
        .take_while(|&country| country != END_OF_COUNTRIES)
        .collect()
}

/// Points `_fixtureSchedulePtrTable` at the original's round robins, as upstream's data
/// section does (src/swos_amiga_ptr_imports.c): a world's install may have pointed its odd
/// ones at its rounds.
fn point_at_original_schedules() {
    macro_rules! originals {
        ($($table:ident),+ $(,)?) => {
            [$((&raw const fixture_data_c::$table).cast::<u8>()),+]
        };
    }
    let originals: [*const u8; SCHEDULE_SIZES] = originals!(
        _fixtureSchedule2Teams,
        _fixtureSchedule3Teams,
        _fixtureSchedule4Teams,
        _fixtureSchedule5Teams,
        _fixtureSchedule6Teams,
        _fixtureSchedule7Teams,
        _fixtureSchedule8Teams,
        _fixtureSchedule9Teams,
        _fixtureSchedule10Teams,
        _fixtureSchedule11Teams,
        _fixtureSchedule12Teams,
        _fixtureSchedule13Teams,
        _fixtureSchedule14Teams,
        _fixtureSchedule15Teams,
        _fixtureSchedule16Teams,
        _fixtureSchedule17Teams,
        _fixtureSchedule18Teams,
        _fixtureSchedule19Teams,
        _fixtureSchedule20Teams,
        _fixtureSchedule21Teams,
        _fixtureSchedule22Teams,
        _fixtureSchedule23Teams,
        _fixtureSchedule24Teams,
    );
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { swos_amiga_ptr_imports_c::_fixtureSchedulePtrTable = originals };
}

/// The round robins `_fixtureSchedulePtrTable` points at, each with whether it plays real
/// rounds (`FixtureRounds_Real`).
fn schedules() -> Vec<FixtureSchedule> {
    // SAFETY: a plain read of a global the lock's holder owns; each entry points at a table of
    // `teams * (teams - 1)` bytes (the original's, or `FixtureRounds_Install`'s rounds).
    let table = unsafe { swos_amiga_ptr_imports_c::_fixtureSchedulePtrTable };
    (FIRST_SCHEDULE_SIZE..)
        .zip(table)
        .map(|(teams, pointer)| {
            let len = usize::from(teams) * usize::from(teams - 1);
            // SAFETY: as above.
            let bytes = unsafe { std::slice::from_raw_parts(pointer, len) };
            FixtureSchedule {
                teams,
                // SAFETY: the function reads its own flag.
                rounds: unsafe { fixture_rounds_c::FixtureRounds_Real(i32::from(teams)) } != 0,
                pairs: bytes
                    .as_chunks()
                    .0
                    .iter()
                    .map(|&[home, away]| (home, away))
                    .collect(),
            }
        })
        .collect()
}

/// The original's round robins.
pub(crate) fn original_schedules() -> Vec<FixtureSchedule> {
    point_at_original_schedules();
    schedules()
}

/// The bytes of `_euroChampFixtureData`.
pub(crate) fn euro_championship_fixtures() -> Vec<u8> {
    // SAFETY: upstream defines the table with its 28 bytes.
    let table = unsafe {
        (&raw const swos_data_bulk_imports_c::_euroChampFixtureData)
            .cast::<[i8; EURO_CHAMPIONSHIP_FIXTURES_LEN]>()
            .read()
    };
    table.iter().map(|&byte| byte.cast_unsigned()).collect()
}

/// The own draws' tables, as compiled.
pub(crate) fn own_draw_tables() -> OwnDrawTables {
    // SAFETY: plain reads of upstream's arrays, of their declared lengths.
    unsafe {
        OwnDrawTables {
            order_8: (&raw const swos_data_bulk_imports_c::_diyShuffleOrder8Teams).read().to_vec(),
            order_4: (&raw const swos_data_bulk_imports_c::_diyShuffleOrder4Teams).read().to_vec(),
            tournament_order_8: (&raw const swos_data_bulk_imports_c::_diyTournamentShuffleOrder8).read().to_vec(),
            group_patterns: (&raw const swos_data_bulk_imports_c::_diyGroupPatternLookup).read().to_vec(),
            best_placed_map: (&raw const swos_data_bulk_imports_c::_diyLeagueTeamShuffleMap).read().to_vec(),
        }
    }
}

/// Every compiled string `_stringSymbols` lists, in its order: its symbol's name and its text.
fn string_table() -> Vec<(String, *const c_void)> {
    // SAFETY: upstream defines `_stringSymbols` with `_stringSymbolsCount` entries (the
    // binding declares it of none); each name is a static string ended by a NUL.
    unsafe {
        let count = usize::try_from(string_symbols_c::_stringSymbolsCount).unwrap_or_default();
        let first =
            (&raw const string_symbols_c::_stringSymbols).cast::<string_symbols_c::StringSymbol>();
        (0..count)
            .map(|index| {
                let entry = first.add(index).read();
                let name = CStr::from_ptr(entry.name).to_string_lossy().into_owned();
                (name, entry.addr.cast_const())
            })
            .collect()
    }
}

/// The text of the string at `address`, up to its NUL.
///
/// # Safety
///
/// `address` points at a string ended by a NUL.
unsafe fn text_at(address: *const c_void) -> Vec<u8> {
    // SAFETY: the caller's.
    unsafe { CStr::from_ptr(address.cast::<c_char>()) }
        .to_bytes()
        .to_vec()
}

/// Every compiled string `_stringSymbols` lists (src/string_symbols.c), in its order: its
/// symbol's name and its text.
pub(crate) fn string_symbols() -> Vec<(String, Vec<u8>)> {
    string_table()
        .into_iter()
        // SAFETY: each listed address is a compiled string, ended by a NUL.
        .map(|(name, address)| (name, unsafe { text_at(address) }))
        .collect()
}

/// The string at `address`, by its symbol's name when `table` lists it.
///
/// # Safety
///
/// `address` points at a string ended by a NUL.
unsafe fn compiled_string(
    table: &[(String, *const c_void)],
    address: *const c_void,
) -> CompiledString {
    CompiledString {
        symbol: table
            .iter()
            .find(|&&(_, listed)| listed == address)
            .map(|(name, _)| name.clone()),
        // SAFETY: the caller's.
        text: unsafe { text_at(address) },
    }
}

/// The names a league, a cup and a tournament fall back on: the resets' (`_aDiyLeague`,
/// `_aDiyCup`, `_aDiyTournament`), each division's (`_divisionStringTable`), a cup's
/// (`_aCup_0`) and a tournament's (`_aLeague_1`), a cup's rounds' (`_cupRoundStringTable`).
pub(crate) fn default_names() -> DefaultNames {
    let table = string_table();
    // SAFETY: plain reads of globals the lock's holder owns; each is a compiled string, ended by
    // a NUL (the division table's entries point at upstream's static strings).
    unsafe {
        let divisions = swos_amiga_ptr_imports_c::_divisionStringTable;
        let cup_rounds = swos_amiga_ptr_imports_c::_cupRoundStringTable;
        DefaultNames {
            league_reset: compiled_string(
                &table,
                (&raw const swos_amiga_strings_c::_aDiyLeague).cast(),
            ),
            cup_reset: compiled_string(&table, (&raw const swos_amiga_strings_c::_aDiyCup).cast()),
            tournament_reset: compiled_string(
                &table,
                (&raw const swos_amiga_strings_c::_aDiyTournament).cast(),
            ),
            divisions: divisions
                .iter()
                .map(|&name| compiled_string(&table, name.cast_const()))
                .collect(),
            cup: compiled_string(&table, (&raw const swos_amiga_strings_c::_aCup_0).cast()),
            tournament: compiled_string(
                &table,
                (&raw const swos_amiga_strings_c::_aLeague_1).cast(),
            ),
            cup_rounds: cup_rounds
                .iter()
                .map(|&name| compiled_string(&table, name.cast_const()))
                .collect(),
        }
    }
}

/// The records a list of upstream's names (`entries`, up to its end), each by its compId and
/// whether it is a career's copy of the record (`_careerContests`): a copy is blank until a
/// career makes it, so its id is its template's.
///
/// # Safety
///
/// Each entry before the end points at a compiled record or at a copy `_careerContests` pairs.
unsafe fn listed_records(entries: &[*mut c_void]) -> Vec<ListedRecord> {
    // SAFETY: a plain read of a global the lock's holder owns.
    let contests = unsafe { competition_data_c::_careerContests };
    entries
        .iter()
        .take_while(|&&entry| entry.addr() != LIST_END)
        .map(|&entry| {
            let template = contests
                .iter()
                .find(|contest| {
                    !contest.competitionCopy.is_null() && contest.competitionCopy.cast() == entry
                })
                .map(|contest| contest.competition.cast::<c_void>());
            let record = template
                .unwrap_or(entry)
                .cast::<competition_data_c::Competition>();
            ListedRecord {
                // SAFETY: the caller's: a record starts with its base, compId first.
                comp_id: unsafe { record.read_unaligned() }.compId,
                copy: template.is_some(),
            }
        })
        .collect()
}

/// The national teams' competitions (`_nationalTeamCompetitionsAndQualifications1`) and the
/// contests of the continental suspensions' track
/// (`_europeanCupsandNationalTeamCompetitionsAndQualifications`).
pub(crate) fn competition_lists() -> CompetitionLists {
    // SAFETY: plain reads of globals the lock's holder owns; upstream's lists hold compiled
    // records and copies `_careerContests` pairs, up to their end.
    unsafe {
        let national = swos_amiga_ptr_imports_c::_nationalTeamCompetitionsAndQualifications1;
        let continental =
            swos_amiga_ptr_imports_c::_europeanCupsandNationalTeamCompetitionsAndQualifications;
        CompetitionLists {
            national_teams: listed_records(&national),
            continental: listed_records(&continental),
        }
    }
}

/// World 2026-27's baked install over 96/97's tables (upstream src/world_data.c
/// `WorldData2627_InstallBaked`, run as `World_InstallBaked` runs it at start-up), and what it
/// leaves: the team files' bases, the countries' table, and its accessors' answers. The
/// install writes 96/97's globals for good: the caller runs it once a process. Upstream's halt
/// (a world past the global team numbers) is its reason code.
pub(crate) fn install_world_2627() -> Result<BakedWorld, u16> {
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { stubs::WORLD = stubs::WORLD_2627 };
    point_at_original_schedules();
    // SAFETY: the install writes upstream's tables within their sizes; its externs are the
    // compiled data's and the stubs.
    super::halting(|| unsafe { world_data_c::WorldData2627_InstallBaked() })?;
    let countries = (0..=u8::MAX)
        .filter_map(|number| country(usize::from(number)).map(|entry| (number, entry)))
        .collect();
    // SAFETY: the accessors read the generated tables; the season's list is a static array
    // ended by its end byte, read up to a team file's worth of numbers.
    let (season_countries, second_files, squad_numbers) = unsafe {
        let list = world_data_c::WorldData2627_SeasonCountries();
        let season = if list.is_null() {
            Vec::new()
        } else {
            (0..TEAM_FILES)
                .map(|index| list.add(index).read())
                .take_while(|&country| country != END_OF_COUNTRIES)
                .collect()
        };
        let second_files = (0..=u8::MAX)
            .filter_map(|country| {
                let file = world_data_c::WorldData2627_SecondFile(country);
                (file != 0).then_some((country, file))
            })
            .collect();
        (
            season,
            second_files,
            world_data_c::WorldData2627_SquadNumbers() != 0,
        )
    };
    // SAFETY: a plain read of a global the lock's holder owns.
    let league_phases = unsafe { stubs::LEAGUE_PHASES };
    Ok(BakedWorld {
        bases: team_file_bases().to_vec(),
        countries,
        season_countries,
        second_files,
        squad_numbers,
        schedules: schedules(),
        league_phases,
        league_phase_cups: league_phase_cups(),
        cups: world_cups(),
    })
}

/// A record's base as upstream compiles it.
fn base(comp: world_data_c::Competition) -> CompetitionBase {
    CompetitionBase {
        comp_id: comp.compId,
        kind: comp.type_,
        team_number: comp.teamNumber,
        begin_month: comp.beginMonth,
        end_month: comp.endMonth,
        reserved1: comp.reserved1,
        is_playoff: comp.isPlayoff,
        reserved2: comp.reserved2,
        seed_reorder_flag: comp.seedReorderFlag,
    }
}

/// A name's text, none for a null pointer.
///
/// # Safety
///
/// A non-null `name` points at a string ended by a NUL.
unsafe fn name_text(name: *const c_char) -> Option<Vec<u8>> {
    // SAFETY: the caller's.
    (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }.to_bytes().to_vec())
}

/// World 2026-27's national and league cups, in their order, none outside a world.
fn world_cups() -> Vec<WorldCupEntry> {
    let mut count = 0;
    // SAFETY: the accessor writes its count and returns the static array of that many cups
    // (or null). Each cup's record is a packed `CompetitionCup`, read unaligned; its names are
    // static strings ended by a NUL; its `n` clubs start at the record's seed list, a list
    // past the record's 32 continuing in the struct around it (`kWorld2627_englishCupExt`),
    // as upstream reads them (src/world_cups.c `WorldCups_Install`).
    unsafe {
        let cups = world_data_c::WorldData2627_Cups(&raw mut count);
        if cups.is_null() {
            return Vec::new();
        }
        (0..usize::try_from(count).unwrap_or_default())
            .map(|index| {
                let cup = cups.add(index).read();
                let record = cup.record.cast::<world_data_c::CompetitionCup>();
                let read = record.read_unaligned();
                let header = read.header;
                let seeds = (&raw const (*record).teamSeedList).cast::<world_data_c::TeamId>();
                WorldCupEntry {
                    country: cup.country,
                    slot: cup.slot,
                    record: CupRecord {
                        comp: base(header.comp),
                        num_rounds_override: header.numRoundsOverride,
                        teams: header.teams,
                        away_goals_mode: header.awayGoalsMode,
                        subs_and_points_for_win: header.subsAndPointsForWin,
                        round_string_base: header.roundStringBase,
                        round_rule_flags: read.roundRuleFlags.to_vec(),
                        names: read.name.map(|name| name_text(name)).to_vec(),
                        team_seed_list: (0..usize::from(cup.n))
                            .map(|club| {
                                let team = seeds.add(club).read_unaligned();
                                (team.countryNumber, team.teamOrdinal)
                            })
                            .collect(),
                    },
                }
            })
            .collect()
    }
}

/// The league-phase records of World 2026-27's European cups, none without them.
fn league_phase_cups() -> Vec<TournamentRecord> {
    let mut places: *const [u8; 4] = std::ptr::null();
    let mut count = 0;
    // SAFETY: the accessor writes its two out-parameters and returns the static array of the
    // three records (or null); each record is read whole, unaligned (the struct is packed),
    // and its names are static strings ended by a NUL.
    unsafe {
        let cups = world_data_c::WorldData2627_LeaguePhaseCups(&raw mut places, &raw mut count);
        if cups.is_null() {
            return Vec::new();
        }
        (0..EUROPEAN_CUPS)
            .map(|index| {
                let record = cups.add(index).read_unaligned();
                let header = record.header;
                TournamentRecord {
                    comp: base(header.comp),
                    seeding_type: header.seedingType,
                    menu_state: header.menuState,
                    play_each_team: header.playEachTeam,
                    away_goals_mode: header.awayGoalsMode,
                    subs_and_points_for_win: header.subsAndPointsForWin,
                    round_string_base: header.roundStringBase,
                    rounds: header
                        .rounds
                        .iter()
                        .map(|round| [round.teamCount, round.groupCount, round.teamGroupCount])
                        .collect(),
                    reserved: header.reserved,
                    round_rule_flags: record.roundRuleFlags.to_vec(),
                    names: record.name.map(|name| name_text(name)).to_vec(),
                    team_seed_list: record
                        .teamSeedList
                        .iter()
                        .map(|team| (team.countryNumber, team.teamOrdinal))
                        .collect(),
                }
            })
            .collect()
    }
}
