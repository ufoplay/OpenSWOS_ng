//! World 1996-97's committed package (`data/worlds/1996-97/`), extracted from the upstream
//! checkout (docs/PLAN.md §5.2): upstream's competition files and career params as they are,
//! and the rules read from upstream's compiled 96/97 tables through the oracle, so no table is
//! transcribed by hand. A fact upstream keeps in code rather than in a table is stated here, a
//! constant citing the code.

use std::collections::BTreeSet;
use std::path::Path;

use openswos_common::{Exporter, TeamFileNumber};
use openswos_competitions_json_resource::{CompetitionsFile, Continent};
use openswos_oracle::tables;
use openswos_world_package_resource::{
    CAREER_PARAMS_JSON, COMPETITIONS_FOLDER, Fingerprint, PackageFiles, RULES_FOLDER, WORLD_INI,
    WorldIni, WorldIniFormat, WorldPackage,
};
use openswos_world_rules_resource::{
    Calendar, CountryDetail, CountryDetails, Fixtures, Rules, Season, SeasonEnd, TeamFiles,
};

use crate::assemble::identity;
use crate::error::ExtractError;
use crate::schedules;

/// Upstream's 96/97 data files, in its checkout.
const DATA_JSON: &str = "bin/data_json";

/// The year World 1996-97's first season starts (upstream src/world.c
/// `kFirstYear[WORLD_9697]`: a `static` table the oracle cannot link).
const FIRST_YEAR: u16 = 1996;

/// Whether a club coach's own country ends its season first (upstream career_season.c
/// `Career_AdvanceToNextSeason_UpdateCompetitions`, which processes the club's country, then
/// every other in the list).
pub(crate) const OWN_COUNTRY_FIRST: bool = true;

/// Whether careers keep the players' own squad numbers: not in 96/97 (upstream
/// src/world_baked.c, `World_SquadNumbers` answers 0 outside a world).
const SQUAD_NUMBERS: bool = false;

/// Whether a tournament's groups may be odd-sized in 96/97: no (upstream diy_competition.c
/// `InitDIYLeagueFromRecord` panics 0x1313 in 96/97 only).
const ODD_TOURNAMENT_GROUPS: bool = false;

/// World 1996-97's committed package, extracted from the upstream checkout in `checkout`:
/// its competition files and career params, and its rules.
///
/// # Errors
///
/// When the oracle is not built, a file of the checkout does not read, or a table lacks what
/// the world needs.
pub fn world_1996_97(checkout: &Path) -> Result<WorldPackage, ExtractError> {
    // The oracle first: without it there is nothing to extract.
    let bases = tables::team_file_bases()?;
    let data = checkout.join(DATA_JSON);
    let mut files = PackageFiles::default();
    let competitions = data.join(COMPETITIONS_FOLDER);
    for entry in read_dir(&competitions)? {
        let path = entry
            .map_err(|source| ExtractError::Read {
                path: competitions.clone(),
                source,
            })?
            .path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        files.insert(&format!("{COMPETITIONS_FOLDER}/{name}"), read(&path)?)?;
    }
    let params = read(&data.join(CAREER_PARAMS_JSON))?;
    files.insert(&format!("{RULES_FOLDER}/{CAREER_PARAMS_JSON}"), params)?;
    let ini = WorldIni {
        identity: identity(),
        fingerprint: Fingerprint::of(&files),
    };
    let ini = WorldIniFormat::export(&ini).unwrap_or_else(|never| match never {});
    files.insert(WORLD_INI, ini)?;
    let opened = WorldPackage::from_files(&files)?;
    if !opened.ignored.is_empty() {
        return Err(ExtractError::UnknownFiles {
            dir: competitions,
            paths: opened.ignored,
        });
    }
    let mut package = opened.package;
    package.rules = rules(&package, bases)?;
    Ok(package)
}

/// World 1996-97's rules: the tables read through the oracle, the facts stated here.
fn rules(package: &WorldPackage, bases: Vec<u16>) -> Result<Rules, ExtractError> {
    let countries = country_details(&country_numbers(package)?)?;
    let season_countries = tables::season_countries()?
        .into_iter()
        .map(TeamFileNumber)
        .collect();
    // The original's round robins: upstream installs real rounds only for a world
    // (`FixtureRounds_Install`, include/fixture_rounds.h).
    let schedules = schedules::schedules(&tables::fixture_schedules()?);
    let euro_championships = schedules::euro_championships(
        &package.competitions,
        &tables::euro_championship_fixtures()?,
        &schedules,
    )?;
    Ok(Rules {
        calendar: Some(Calendar {
            first_year: FIRST_YEAR,
        }),
        country_details: Some(countries),
        team_files: Some(TeamFiles {
            bases,
            second_files: Vec::new(),
            squad_numbers: SQUAD_NUMBERS,
        }),
        season: Some(Season {
            end: SeasonEnd {
                countries: season_countries,
                own_country_first: OWN_COUNTRY_FIRST,
            },
        }),
        fixtures: Some(Fixtures {
            odd_tournament_groups: ODD_TOURNAMENT_GROUPS,
            schedules,
            competition_schedules: vec![euro_championships],
        }),
    })
}

/// The numbers of the countries `package`'s `countries.json` lists, each once, in order.
fn country_numbers(package: &WorldPackage) -> Result<BTreeSet<u8>, ExtractError> {
    package
        .competitions
        .values()
        .find_map(|document| match &document.file {
            CompetitionsFile::Countries(file) => Some(
                file.countries
                    .iter()
                    .map(|country| country.number.0)
                    .collect(),
            ),
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
        .ok_or(ExtractError::NoCountries)
}

/// Each of `numbers`' continent, from upstream's `_countriesTable`.
fn country_details(numbers: &BTreeSet<u8>) -> Result<CountryDetails, ExtractError> {
    let countries = numbers
        .iter()
        .map(|&number| {
            let entry = tables::country(number)?.ok_or(ExtractError::NoCountryEntry(number))?;
            let continent =
                Continent::try_from(entry.continent).map_err(|_| ExtractError::NoContinent {
                    number,
                    code: entry.continent,
                })?;
            Ok(CountryDetail {
                number: TeamFileNumber(number),
                continent,
            })
        })
        .collect::<Result<_, ExtractError>>()?;
    Ok(CountryDetails { countries })
}

/// The committed package's files: every file of `package` but `WORLD.ini`, whose
/// fingerprint would cover the rosters only the assembled package holds.
///
/// # Errors
///
/// When a file does not encode.
pub fn committed_files(package: &WorldPackage) -> Result<PackageFiles, ExtractError> {
    let mut files = PackageFiles::default();
    for (path, bytes) in package.to_files()?.iter() {
        if path != WORLD_INI {
            files.insert(path, bytes.to_vec())?;
        }
    }
    Ok(files)
}

/// The entries of the folder `dir`.
fn read_dir(dir: &Path) -> Result<std::fs::ReadDir, ExtractError> {
    std::fs::read_dir(dir).map_err(|source| ExtractError::Read {
        path: dir.to_path_buf(),
        source,
    })
}

/// The bytes of the file `path`.
fn read(path: &Path) -> Result<Vec<u8>, ExtractError> {
    std::fs::read(path).map_err(|source| ExtractError::Read {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use openswos_test_utils::world_1996_97_path;

    use super::*;
    use crate::assemble::committed;

    #[test]
    fn the_countries_are_those_countries_json_lists_once_each() {
        let package = committed(&world_1996_97_path()).expect("the committed package");
        let numbers = country_numbers(&package).expect("countries.json");
        assert_eq!(numbers.len(), 71, "BRAZIL listed twice counts once");
        assert!(numbers.contains(&8) && numbers.contains(&85));
        let mut package = package;
        package.competitions.remove("countries.json");
        assert!(matches!(
            country_numbers(&package),
            Err(ExtractError::NoCountries)
        ));
    }

    #[test]
    fn the_committed_files_have_no_ini() {
        let package = committed(&world_1996_97_path()).expect("the committed package");
        let files = committed_files(&package).expect("the files");
        assert!(files.get(WORLD_INI).is_none());
        assert!(files.get("rules/career_params.json").is_some());
        assert!(files.get("rules/season.json").is_some());
        assert_eq!(files.in_folder(COMPETITIONS_FOLDER).count(), 10);
    }

    #[test]
    fn a_checkout_without_its_data_is_named() {
        let empty = tempfile::tempdir().expect("a temp dir");
        match world_1996_97(empty.path()) {
            Err(ExtractError::Oracle(_)) => {}
            Err(error) => assert!(error.to_string().contains("bin/data_json"), "{error}"),
            Ok(_) => panic!("no data"),
        }
    }
}
