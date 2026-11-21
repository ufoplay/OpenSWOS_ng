//! A completed world's rules (docs/WORLD_INVENTORY.md §4): what its baked install leaves in
//! upstream's tables, read through the oracle ([`BakedWorld`]), the national files grown to
//! the world's as upstream's SDL build grows them, and the facts upstream keeps in code.

use std::collections::{BTreeMap, BTreeSet};

use openswos_common::TeamFileNumber;
use openswos_competitions_json_resource::{Continent, CountriesFile};
use openswos_oracle::tables::BakedWorld;
use openswos_team_json_resource::TeamJsonDocument;
use openswos_world_rules_resource::{
    Calendar, CountryDetail, CountryDetails, Fixtures, OddRoundRobin, Rules, Season, SeasonEnd,
    SecondFile, TeamFiles,
};

use super::GLOBAL_TEAM_NUMBERS;
use crate::error::ExtractError;
use crate::extract::OWN_COUNTRY_FIRST;

/// The year World 2026-27's first season starts (upstream src/world.c
/// `kFirstYear[WORLD_2627]`: a `static` table the oracle cannot link).
pub(crate) const FIRST_YEAR_2627: u16 = 2026;

/// Whether a world's tournaments may have odd-sized groups: yes, in every world but 96/97
/// (upstream diy_competition.c `InitDIYLeagueFromRecord` refuses them in 96/97 alone).
const WORLD_ODD_TOURNAMENT_GROUPS: bool = true;

/// The national teams' files: countries 80 to 85 (Europe, Africa, South America, North
/// America, Asia, Oceania; upstream save_format_json.c `kNationalFiles`).
const NATIONAL_FILES: std::ops::RangeInclusive<u8> = 80..=85;

/// World 2026-27's rules: its first year; each country of `countries`' continent and the
/// team files' bases as its baked install leaves them, the national files grown to its
/// rosters' (`teams`); its season end's countries; its fixture behaviours.
pub(super) fn world_2627(
    countries: &CountriesFile,
    teams: &BTreeMap<String, TeamJsonDocument>,
    baked: &BakedWorld,
) -> Result<Rules, ExtractError> {
    let mut bases = baked.bases.clone();
    fit_national_files(&mut bases, |file| national_count(teams, file))?;
    Ok(Rules {
        calendar: Some(Calendar {
            first_year: FIRST_YEAR_2627,
        }),
        country_details: Some(country_details(countries, baked)?),
        team_files: Some(TeamFiles {
            bases,
            second_files: baked
                .second_files
                .iter()
                .map(|&(country, file)| SecondFile {
                    country: TeamFileNumber(country),
                    file: TeamFileNumber(file),
                })
                .collect(),
            squad_numbers: baked.squad_numbers,
        }),
        season: Some(Season {
            end: SeasonEnd {
                countries: baked
                    .season_countries
                    .iter()
                    .copied()
                    .map(TeamFileNumber)
                    .collect(),
                own_country_first: OWN_COUNTRY_FIRST,
            },
        }),
        fixtures: Some(Fixtures {
            odd_round_robin: if baked.fixture_rounds {
                OddRoundRobin::RoundsWithRest
            } else {
                OddRoundRobin::OriginalPairTables
            },
            odd_tournament_groups: WORLD_ODD_TOURNAMENT_GROUPS,
        }),
    })
}

/// Each country of `countries`' continent, from the countries' table the install leaves.
fn country_details(
    countries: &CountriesFile,
    baked: &BakedWorld,
) -> Result<CountryDetails, ExtractError> {
    let numbers: BTreeSet<u8> = countries
        .countries
        .iter()
        .map(|country| country.number.0)
        .collect();
    let countries = numbers
        .into_iter()
        .map(|number| {
            let entry = baked
                .countries
                .iter()
                .find(|(found, _)| *found == number)
                .map(|(_, entry)| entry)
                .ok_or(ExtractError::NoCountryEntry(number))?;
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

/// The teams of the world's national file `file` (its roster whose country is `file`), if it
/// has one.
fn national_count(teams: &BTreeMap<String, TeamJsonDocument>, file: u8) -> Option<usize> {
    teams
        .values()
        .find(|document| document.file.country == Some(file))
        .map(|document| document.file.teams.len())
}

/// Each national file's range of global team numbers grown to the world's national teams,
/// every later file's base moving by the difference (upstream save_format_json.c
/// `fit_world_national_files`, the SDL build's: a world may hold more national teams than
/// 96/97, World 2026-27 the 211 FIFA members). Never shrunk.
fn fit_national_files(
    bases: &mut [u16],
    count: impl Fn(u8) -> Option<usize>,
) -> Result<(), ExtractError> {
    for file in NATIONAL_FILES {
        let index = usize::from(file);
        let (Some(&base), Some(&next)) = (bases.get(index), bases.get(index + 1)) else {
            continue;
        };
        let range = next.wrapping_sub(base);
        let Some(teams) = count(file).and_then(|teams| u16::try_from(teams).ok()) else {
            continue;
        };
        if teams <= range {
            continue;
        }
        let growth = teams - range;
        for later in bases.iter_mut().skip(index + 1) {
            *later = later.wrapping_add(growth);
        }
        if let Some(&last) = bases.last()
            && last > GLOBAL_TEAM_NUMBERS
        {
            return Err(ExtractError::TooManyTeamNumbers(last));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use openswos_oracle::tables::CountryEntry;

    use super::*;

    /// Ten global numbers a file.
    fn bases() -> Vec<u16> {
        (0..256u16).map(|file| file * 10).collect()
    }

    #[test]
    fn a_national_file_grows_to_the_worlds_teams_and_moves_the_rest() {
        let mut grown = bases();
        fit_national_files(&mut grown, |file| match file {
            80 => Some(15),
            81 => Some(4),
            _ => None,
        })
        .expect("within the numbers");
        assert_eq!(grown[80], 800);
        assert_eq!(grown[81], 815, "Europe's 15 teams");
        assert_eq!(grown[82] - grown[81], 10, "Africa's 4 fit: never shrunk");
        assert_eq!(grown[255], 2555);
    }

    #[test]
    fn past_the_global_numbers_is_an_error() {
        let mut grown = bases();
        let error = fit_national_files(&mut grown, |file| (file == 80).then_some(1000))
            .expect_err("past 2560");
        assert!(error.to_string().contains("2560"), "{error}");
    }

    fn entry(continent: u8) -> CountryEntry {
        CountryEntry {
            continent,
            name: b"X".to_vec(),
            adjective: b"X".to_vec(),
        }
    }

    fn baked(countries: Vec<(u8, CountryEntry)>) -> BakedWorld {
        BakedWorld {
            bases: bases(),
            countries,
            season_countries: vec![8, 47],
            second_files: Vec::new(),
            squad_numbers: true,
            fixture_rounds: true,
            league_phases: true,
            league_phase_cups: Vec::new(),
            cups: Vec::new(),
        }
    }

    fn countries(numbers: &[u8]) -> CountriesFile {
        CountriesFile {
            count: numbers.len(),
            countries: numbers
                .iter()
                .map(|&number| openswos_competitions_json_resource::Country {
                    number: TeamFileNumber(number),
                    name: "X".to_owned(),
                    define: "CN_X".to_owned(),
                    team_file: None,
                })
                .collect(),
        }
    }

    #[test]
    fn the_rules_are_the_installs_and_the_worlds() {
        let rules = world_2627(
            &countries(&[8, 47]),
            &BTreeMap::new(),
            &baked(vec![(8, entry(0)), (47, entry(0))]),
        )
        .expect("rules");
        assert_eq!(
            rules.calendar.map(|calendar| calendar.first_year),
            Some(2026)
        );
        assert_eq!(
            rules.country_details.map(|details| details.countries.len()),
            Some(2)
        );
        assert_eq!(
            rules.season.map(|season| season.end.countries),
            Some(vec![TeamFileNumber(8), TeamFileNumber(47)])
        );
        assert_eq!(
            rules.fixtures,
            Some(Fixtures {
                odd_round_robin: OddRoundRobin::RoundsWithRest,
                odd_tournament_groups: true
            })
        );
        assert_eq!(
            rules.team_files.map(|files| files.squad_numbers),
            Some(true)
        );
    }

    #[test]
    fn a_country_the_table_lacks_is_named() {
        let error = world_2627(
            &countries(&[8, 47]),
            &BTreeMap::new(),
            &baked(vec![(8, entry(0))]),
        )
        .expect_err("no entry");
        assert!(error.to_string().contains("country 47"), "{error}");
    }
}
