//! Upstream's data tables `world_extract` reads into its packages (src/strings_data.c,
//! src/swos_amiga_data_imports.c, src/competition_data.c), and World 2026-27's baked install
//! over them (src/world_data.c). Only that install writes them, once a process
//! (`tables::world_2627`): they are never reset.

use std::ffi::{CStr, c_char, c_void};

use super::bindings::{
    competition_data_c, strings_data_c, swos_amiga_data_imports_c, world_data_c,
};
use super::stubs;
use crate::tables::{BakedWorld, CountryEntry, TournamentRecord};

/// The entries of `_countriesTable` (src/strings_data.c).
const COUNTRIES_TABLE_LEN: usize = 262;

/// The entries of `_teamsCountryNumbers` (src/swos_amiga_data_imports.c): one a team file.
pub(crate) const TEAM_FILES: usize = 256;

/// The entries of `_careerCompetitionIdRemapTable` (src/competition_data.c): 65 countries and
/// the end byte.
const SEASON_COUNTRIES_LEN: usize = 66;

/// What ends the season end's countries.
const END_OF_COUNTRIES: u8 = 0xFF;

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

/// World 2026-27's baked install over 96/97's tables (upstream src/world_data.c
/// `WorldData2627_InstallBaked`, run as `World_InstallBaked` runs it at start-up), and what it
/// leaves: the team files' bases, the countries' table, and its accessors' answers. The
/// install writes 96/97's globals for good: the caller runs it once a process. Upstream's halt
/// (a world past the global team numbers) is its reason code.
pub(crate) fn install_world_2627() -> Result<BakedWorld, u16> {
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { stubs::WORLD = stubs::WORLD_2627 };
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
    // SAFETY: plain reads of globals the lock's holder owns.
    let (fixture_rounds, league_phases) = unsafe { (stubs::FIXTURE_ROUNDS, stubs::LEAGUE_PHASES) };
    Ok(BakedWorld {
        bases: team_file_bases().to_vec(),
        countries,
        season_countries,
        second_files,
        squad_numbers,
        fixture_rounds,
        league_phases,
        league_phase_cups: league_phase_cups(),
    })
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
                let comp = header.comp;
                let names = record.name.map(|name| {
                    (!name.is_null()).then(|| CStr::from_ptr(name).to_bytes().to_vec())
                });
                TournamentRecord {
                    comp_id: comp.compId,
                    kind: comp.type_,
                    team_number: comp.teamNumber,
                    begin_month: comp.beginMonth,
                    end_month: comp.endMonth,
                    reserved1: comp.reserved1,
                    is_playoff: comp.isPlayoff,
                    reserved2: comp.reserved2,
                    seed_reorder_flag: comp.seedReorderFlag,
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
                    names: names.to_vec(),
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
