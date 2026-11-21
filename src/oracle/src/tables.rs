//! Upstream's 96/97 data tables, as compiled, for `world_extract`: it writes them into World
//! 1996-97's package, so no table is transcribed by hand (docs/PLAN.md §5.2). Unlike the rest
//! of the crate this module is always built: without the checkout every table is
//! [`Unavailable`], so the tool builds everywhere and says why it cannot extract.

use thiserror::Error;

/// The oracle is not built: no upstream checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the oracle is not built: no upstream checkout (just upstream_fetch)")]
pub struct Unavailable;

/// Why a world's baked install gives nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum InstallError {
    /// The oracle is not built.
    #[error(transparent)]
    Unavailable(#[from] Unavailable),
    /// Upstream halted, with this reason code (0x8F02: the world's clubs past the global
    /// team numbers).
    #[error("upstream halted installing the world: reason {0:#06X}")]
    Halted(u16),
}

/// What World 2026-27's baked install (upstream src/world_data.c) leaves in upstream's tables,
/// and its accessors' answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakedWorld {
    /// Each team file's first global team number, its countries resized to its clubs
    /// (`resize_country`).
    pub bases: Vec<u16>,
    /// Every number with an entry in `_countriesTable`, its new countries' and renames in.
    pub countries: Vec<(u8, CountryEntry)>,
    /// The season end's countries, in its order (`WorldData2627_SeasonCountries`).
    pub season_countries: Vec<u8>,
    /// Each country over two team files, and its second file (`WorldData2627_SecondFile`).
    pub second_files: Vec<(u8, u8)>,
    /// Whether players keep their own squad numbers (`WorldData2627_SquadNumbers`).
    pub squad_numbers: bool,
    /// The round robins of 2 to 24 clubs after it (`_fixtureSchedulePtrTable`: its odd ones
    /// rewritten as real rounds, `FixtureRounds_Install`).
    pub schedules: Vec<FixtureSchedule>,
    /// Whether it plays the European cups as league phases (`LeaguePhase_InstallCups`).
    pub league_phases: bool,
    /// Its European cups' league-phase records (`WorldData2627_LeaguePhaseCups`): the
    /// Champions', Cup Winners' and UEFA Cup's places, in that order.
    pub league_phase_cups: Vec<TournamentRecord>,
    /// Its national and league cups (`WorldData2627_Cups`), in the order `WorldCups_Install`
    /// installs them.
    pub cups: Vec<WorldCupEntry>,
}

/// A round robin as upstream's table points at it (`_fixtureSchedulePtrTable[teams - 2]`): its
/// clubs, whether it plays real rounds (`FixtureRounds_Real`), its pairs in playing order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSchedule {
    /// Its clubs.
    pub teams: u8,
    /// Whether it plays real rounds: every club at most once a match day.
    pub rounds: bool,
    /// One cycle's pairs, `(home, away)` as places among its clubs.
    pub pairs: Vec<(u8, u8)>,
}

/// The base every competition record starts with (include/game_types.h `Competition`, 9
/// bytes): its fields by their C names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetitionBase {
    /// `compId`.
    pub comp_id: u8,
    /// `type`.
    pub kind: i8,
    /// `teamNumber`.
    pub team_number: i8,
    /// `beginMonth`.
    pub begin_month: i8,
    /// `endMonth`.
    pub end_month: i8,
    /// `reserved1` (the JSON's `name_table_offset`).
    pub reserved1: i8,
    /// `isPlayoff` (the JSON's `reserved`).
    pub is_playoff: i8,
    /// `reserved2` (the JSON's `team_list_offset`).
    pub reserved2: i8,
    /// `seedReorderFlag`.
    pub seed_reorder_flag: i8,
}

/// A tournament record as upstream compiles it (include/game_types.h `CompetitionLeaguePhase`,
/// `CompetitionTournament`'s layout): its fields by their C names, its names as their text,
/// its seed list as `(country, ordinal)` pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentRecord {
    /// `header.comp`.
    pub comp: CompetitionBase,
    /// `header.seedingType`.
    pub seeding_type: i8,
    /// `header.menuState`.
    pub menu_state: i8,
    /// `header.playEachTeam`.
    pub play_each_team: i8,
    /// `header.awayGoalsMode`.
    pub away_goals_mode: i8,
    /// `header.subsAndPointsForWin`: substitutes high nibble, a win's points low.
    pub subs_and_points_for_win: i8,
    /// `header.roundStringBase`.
    pub round_string_base: i8,
    /// `header.rounds`: each stage's teams, groups and teams a group.
    pub rounds: Vec<[i8; 3]>,
    /// `header.reserved`.
    pub reserved: i8,
    /// `roundRuleFlags`.
    pub round_rule_flags: Vec<i8>,
    /// `name`: each name's text, `None` for a null pointer.
    pub names: Vec<Option<Vec<u8>>>,
    /// `teamSeedList`: `(country, ordinal)`.
    pub team_seed_list: Vec<(u8, u8)>,
}

/// A cup record as upstream compiles it (include/game_types.h `CompetitionCup`): its fields by
/// their C names, its names as their text, its seed list as `(country, ordinal)` pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CupRecord {
    /// `header.comp`.
    pub comp: CompetitionBase,
    /// `header.numRoundsOverride`.
    pub num_rounds_override: i8,
    /// `header.teams`.
    pub teams: i8,
    /// `header.awayGoalsMode`.
    pub away_goals_mode: i8,
    /// `header.subsAndPointsForWin`: substitutes high nibble, a win's points low.
    pub subs_and_points_for_win: i8,
    /// `header.roundStringBase`.
    pub round_string_base: i8,
    /// `roundRuleFlags`.
    pub round_rule_flags: Vec<i8>,
    /// `name`: each name's text, `None` for a null pointer.
    pub names: Vec<Option<Vec<u8>>>,
    /// `teamSeedList`: `(country, ordinal)`, as many as the record lists.
    pub team_seed_list: Vec<(u8, u8)>,
}

/// A world's national or league cup (include/world_cups.h `WorldCup`): its country, its slot
/// and its record. Its draw's other fields join with the M5 unit that reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCupEntry {
    /// `country`: its country's number.
    pub country: u8,
    /// `slot`: 1 the national cup, 2 the league cup.
    pub slot: u8,
    /// `record`, its seed list the cup's `n` clubs: a list past the record's 32 continues
    /// after it.
    pub record: CupRecord,
}

/// A country's entry in upstream's `_countriesTable` (src/strings_data.c): its continent's
/// code, its name and its adjective, as their bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountryEntry {
    /// Its continent's code (upstream `_countriesTable[n][0]`: 0 Europe, 1 North America, 2
    /// South America, 3 Asia, 4 Oceania, 5 Africa).
    pub continent: u8,
    /// Its name (`ALBANIA`).
    pub name: Vec<u8>,
    /// Its adjective (`ALBANIAN`).
    pub adjective: Vec<u8>,
}

/// The entry of country `number` in `_countriesTable`, if it has one.
///
/// # Errors
///
/// [`Unavailable`] without the checkout.
pub fn country(number: u8) -> Result<Option<CountryEntry>, Unavailable> {
    #[cfg(not(oracle_unavailable))]
    {
        let _held = crate::Oracle::lock();
        Ok(crate::ffi::tables::country(usize::from(number)))
    }
    #[cfg(oracle_unavailable)]
    {
        let _ = number;
        Err(Unavailable)
    }
}

/// Each team file's first global team number (`_teamsCountryNumbers`,
/// src/swos_amiga_data_imports.c): 256 numbers, by file number.
///
/// # Errors
///
/// [`Unavailable`] without the checkout.
pub fn team_file_bases() -> Result<Vec<u16>, Unavailable> {
    #[cfg(not(oracle_unavailable))]
    {
        let _held = crate::Oracle::lock();
        Ok(crate::ffi::tables::team_file_bases().to_vec())
    }
    #[cfg(oracle_unavailable)]
    {
        Err(Unavailable)
    }
}

/// The countries of the season end, in its order (`_careerCompetitionIdRemapTable`,
/// src/competition_data.c), without its end byte.
///
/// # Errors
///
/// [`Unavailable`] without the checkout.
pub fn season_countries() -> Result<Vec<u8>, Unavailable> {
    #[cfg(not(oracle_unavailable))]
    {
        let _held = crate::Oracle::lock();
        Ok(crate::ffi::tables::season_countries())
    }
    #[cfg(oracle_unavailable)]
    {
        Err(Unavailable)
    }
}

/// The original's round robins of 2 to 24 clubs (src/fixture_data.c, through
/// `_fixtureSchedulePtrTable`), each with whether it plays real rounds.
///
/// # Errors
///
/// [`Unavailable`] without the checkout.
pub fn fixture_schedules() -> Result<Vec<FixtureSchedule>, Unavailable> {
    #[cfg(not(oracle_unavailable))]
    {
        let _held = crate::Oracle::lock();
        Ok(crate::ffi::tables::original_schedules())
    }
    #[cfg(oracle_unavailable)]
    {
        Err(Unavailable)
    }
}

/// The bytes of `_euroChampFixtureData` (src/swos_data_bulk_imports.c): the European
/// Championships' group pairs first (diy_competition.c `getCurrentFixtureTeamPtrs`), then data
/// that is no pair.
///
/// # Errors
///
/// [`Unavailable`] without the checkout.
pub fn euro_championship_fixtures() -> Result<Vec<u8>, Unavailable> {
    #[cfg(not(oracle_unavailable))]
    {
        let _held = crate::Oracle::lock();
        Ok(crate::ffi::tables::euro_championship_fixtures())
    }
    #[cfg(oracle_unavailable)]
    {
        Err(Unavailable)
    }
}

/// World 2026-27's baked install, run once a process and remembered: it writes 96/97's tables
/// for good, so afterwards the oracle serves nothing else in this process ([`crate::Oracle`]
/// panics), and a process that needs both runs them apart (a test file of its own).
///
/// # Errors
///
/// [`InstallError`] without the checkout, or when upstream halts.
pub fn world_2627() -> Result<BakedWorld, InstallError> {
    #[cfg(not(oracle_unavailable))]
    {
        use std::sync::OnceLock;
        static BAKED: OnceLock<Result<BakedWorld, u16>> = OnceLock::new();
        BAKED
            .get_or_init(|| {
                let _held = crate::Oracle::lock();
                let baked = crate::ffi::tables::install_world_2627();
                crate::oracle::mark_world_installed();
                baked
            })
            .clone()
            .map_err(InstallError::Halted)
    }
    #[cfg(oracle_unavailable)]
    {
        Err(InstallError::Unavailable(Unavailable))
    }
}
