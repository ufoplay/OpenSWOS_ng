//! World 2026-27, completed (docs/PLAN.md §5.2, docs/WORLD_INVENTORY.md §9): its download laid
//! over World 1996-97's committed package as upstream lays it at start-up, once and ahead of
//! time, and its rules from its baked install. The result is self-contained: the engine loads
//! it alone.
//!
//! What it holds so far:
//!
//! - the download's identity, mod, rosters (its 107 hold every 96/97 country), national teams,
//!   B teams and stadiums, as they are;
//! - its foreign players' pool, unpacked (the caller unpacks it with upstream's decoder);
//! - 96/97's competition files, its leagues replaced by the world's and its new leagues with
//!   their countries and tables ([`competitions`]), its European cups the league phases
//!   ([`european_cups`]), its national and league cups the world's ([`national_cups`]);
//! - 96/97's career params, which upstream plays every world with (D12);
//! - the five rules files of M4 #5c1, and the texts and classes of M5 #4b2 ([`rules`]).
//!
//! Its other rules join with the units that read them (its cups' draws: `national_cups.json`,
//! M5).

mod competitions;
mod european_cups;
mod national_cups;
mod records;
mod rules;

pub use national_cups::{CupRow, cup_rows};

use openswos_competitions_json_resource::CompetitionsFile;
use openswos_oracle::tables::BakedWorld;
use openswos_poolplyr_resource::PoolFile;
use openswos_world_package_resource::WorldPackage;

use crate::error::ExtractError;

/// The global team numbers there are, in upstream's worlds build (include/swos_types.h
/// `GLOBAL_TEAM_NUMBERS`, 2,000 without the worlds): past it, upstream halts (0x8F02).
pub const GLOBAL_TEAM_NUMBERS: u16 = 2560;

/// The world's leagues' file.
const LEAGUES: &str = "leagues.json";

/// World 2026-27 completed: its `download` over World 1996-97's committed package `base`,
/// with the rules, the European cups and the national cups its baked install (`baked`) leaves
/// (its cups named by `cup_rows`, upstream's table of them: [`cup_rows`]), and its foreign
/// players' `pool`, unpacked.
///
/// # Errors
///
/// When the download has no leagues, `base` lacks a competition file the merge edits, a new
/// country has no roster, `cup_rows` and the baked cups disagree, a cup's country has no
/// table, or the tables lack what the world needs.
pub fn world_2627(
    base: &WorldPackage,
    download: &WorldPackage,
    baked: &BakedWorld,
    cup_rows: &[CupRow],
    pool: Option<PoolFile>,
) -> Result<WorldPackage, ExtractError> {
    let Some(CompetitionsFile::Leagues(world_leagues)) = download
        .competitions
        .get(LEAGUES)
        .map(|document| &document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(LEAGUES));
    };
    let mut competitions = base.competitions.clone();
    competitions::merge(&mut competitions, world_leagues, &download.teams)?;
    european_cups::install(&mut competitions, &baked.league_phase_cups, &download.teams)?;
    national_cups::install(&mut competitions, &baked.cups, cup_rows, &download.teams)?;
    let countries = competitions
        .values()
        .find_map(|document| match &document.file {
            CompetitionsFile::Countries(file) => Some(file),
            CompetitionsFile::Leagues(_)
            | CompetitionsFile::Cups(_)
            | CompetitionsFile::Tournaments(_)
            | CompetitionsFile::Playoffs(_)
            | CompetitionsFile::Qualifying(_)
            | CompetitionsFile::LeagueBlocks(_)
            | CompetitionsFile::Career(_)
            | CompetitionsFile::CountryCompetitions(_)
            | CompetitionsFile::SeedBuckets(_) => None,
        })
        .ok_or(ExtractError::NoCountries)?;
    let continental = crate::texts::continental_records(&competitions);
    let draws = crate::draws::draws(&competitions)?;
    let rules = rules::world_2627(
        countries,
        &download.teams,
        baked,
        &base.rules,
        continental,
        draws,
    )?;
    Ok(WorldPackage {
        identity: download.identity.clone(),
        manifest: download.manifest.clone(),
        teams: download.teams.clone(),
        competitions,
        national_teams: download.national_teams.clone(),
        bteams: download.bteams.clone(),
        stadiums: download.stadiums.clone(),
        career_params: base.career_params.clone(),
        rules,
        pool,
    })
}
