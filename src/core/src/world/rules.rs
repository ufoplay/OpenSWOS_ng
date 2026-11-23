//! The world's rules (`rules/`, docs/WORLD_INVENTORY.md §4) that core reads so far: its
//! calendar, its countries' continents and adjectives, how its clubs are numbered, its season
//! end's countries, its fixture behaviours, its texts and its classes of competition. Each is
//! checked against the world's countries.

use std::collections::BTreeMap;

use openswos_common::{TeamFileNumber, TeamId};
use openswos_competitions_json_resource::Continent;
use openswos_world_package_resource::RULES_FOLDER;
use openswos_world_rules_resource::{
    Calendar, CompetitionClasses, CountryDetails, Draws, Fixtures, Rules, RulesFile, Season,
    SeasonEnd, SecondFile, Strings, TeamFiles,
};

use super::countries::Countries;
use super::texts::bytes;
use crate::validation::{Field, Location, Problem, Problems};

/// The team files a world numbers its clubs over: one a file number.
pub const TEAM_FILES: usize = 256;

/// How the world numbers its clubs across its team files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamNumbering {
    /// Each team file's first global team number, by file number, never going down.
    bases: [u16; TEAM_FILES],
    /// The countries over two team files.
    pub second_files: Vec<SecondFile>,
    /// Whether players keep their own squad numbers.
    pub squad_numbers: bool,
}

impl TeamNumbering {
    /// The first global team number of team file `file`.
    #[must_use]
    pub fn base(&self, file: TeamFileNumber) -> u16 {
        self.bases[usize::from(file.0)]
    }

    /// The global team number of `team`: its file's base plus its place in the file
    /// (upstream `SetTeamGlobalNumbers`). Faithful to the original: it wraps as a 16-bit
    /// word.
    #[must_use]
    pub fn global_number(&self, team: TeamId) -> u16 {
        self.base(team.file).wrapping_add(u16::from(team.ordinal.0))
    }
}

/// The rules files core reads, or `None` when a required one is missing (each reported).
pub(crate) struct RulesFiles<'a> {
    /// `calendar.json`.
    pub(crate) calendar: &'a Calendar,
    /// `country_details.json`.
    pub(crate) country_details: &'a CountryDetails,
    /// `team_files.json`.
    pub(crate) team_files: &'a TeamFiles,
    /// `season.json`.
    pub(crate) season: &'a Season,
    /// `fixtures.json`.
    pub(crate) fixtures: &'a Fixtures,
    /// `strings.json`.
    pub(crate) strings: &'a Strings,
    /// `competition_classes.json`.
    pub(crate) competition_classes: &'a CompetitionClasses,
    /// `draws.json`.
    pub(crate) draws: &'a Draws,
}

/// The path of a rules file.
pub(super) fn path<T: RulesFile>() -> String {
    format!("{RULES_FOLDER}/{}", T::FILE_NAME)
}

/// Where a field of a rules file is.
pub(super) fn at<T: RulesFile>(field: &Field) -> Location {
    Location::field(&path::<T>(), field)
}

/// `file`, reporting it missing when it is `None`.
fn required<'a, T: RulesFile>(file: Option<&'a T>, problems: &mut Problems) -> Option<&'a T> {
    if file.is_none() {
        problems.push(Location::file(&path::<T>()), Problem::MissingFile);
    }
    file
}

/// The rules files of `rules`, or `None` when a required one is missing.
pub(crate) fn pick<'a>(rules: &'a Rules, problems: &mut Problems) -> Option<RulesFiles<'a>> {
    let calendar = required(rules.calendar.as_ref(), problems);
    let country_details = required(rules.country_details.as_ref(), problems);
    let team_files = required(rules.team_files.as_ref(), problems);
    let season = required(rules.season.as_ref(), problems);
    let fixtures = required(rules.fixtures.as_ref(), problems);
    let strings = required(rules.strings.as_ref(), problems);
    let competition_classes = required(rules.competition_classes.as_ref(), problems);
    let draws = required(rules.draws.as_ref(), problems);
    Some(RulesFiles {
        calendar: calendar?,
        country_details: country_details?,
        team_files: team_files?,
        season: season?,
        fixtures: fixtures?,
        strings: strings?,
        competition_classes: competition_classes?,
        draws: draws?,
    })
}

/// What the world knows of a country beyond its competitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Details {
    /// Its continent.
    pub(crate) continent: Continent,
    /// Its adjective, a byte a character.
    pub(crate) adjective: Vec<u8>,
}

/// Each country's details: one entry for each country of the world, and none for another
/// number; each adjective a byte a character.
pub(crate) fn country_details(
    details: &CountryDetails,
    countries: &Countries,
    problems: &mut Problems,
) -> Option<BTreeMap<TeamFileNumber, Details>> {
    let field = Field::key("countries");
    let mut checked = BTreeMap::new();
    let mut listed_at = BTreeMap::new();
    let mut valid = true;
    for (index, detail) in details.countries.iter().enumerate() {
        let entry = field.index(index);
        let location = at::<CountryDetails>(&entry.then("number"));
        let adjective = bytes(&detail.adjective).map_err(|problem| {
            problems.push(at::<CountryDetails>(&entry.then("adjective")), problem);
        });
        if countries.get(detail.number).is_none() {
            problems.push(location, Problem::UnknownCountry(detail.number));
            valid = false;
        } else if let Some(first) = listed_at.insert(detail.number, location.clone()) {
            problems.push(
                location,
                Problem::RepeatedCountry {
                    number: detail.number,
                    first,
                },
            );
            valid = false;
        } else if let Ok(adjective) = adjective {
            let continent = detail.continent;
            checked.insert(
                detail.number,
                Details {
                    continent,
                    adjective,
                },
            );
        } else {
            valid = false;
        }
    }
    for country in countries.iter() {
        if !listed_at.contains_key(&country.number) {
            problems.push(
                at::<CountryDetails>(&field),
                Problem::NoCountryDetails(country.number),
            );
            valid = false;
        }
    }
    valid.then_some(checked)
}

/// How the world numbers its clubs: 256 bases never going down, second files of countries
/// numbered as no country is.
pub(crate) fn team_numbering(
    files: &TeamFiles,
    countries: &Countries,
    problems: &mut Problems,
) -> Option<TeamNumbering> {
    let mut valid = true;
    let bases_field = Field::key("bases");
    let bases: Option<[u16; TEAM_FILES]> = files.bases.as_slice().try_into().ok();
    if bases.is_none() {
        problems.push(
            at::<TeamFiles>(&bases_field),
            Problem::WrongCount {
                expected: TEAM_FILES,
                found: files.bases.len(),
            },
        );
        valid = false;
    }
    for (index, pair) in files.bases.windows(2).enumerate() {
        if let &[previous, value] = pair
            && value < previous
        {
            problems.push(
                at::<TeamFiles>(&bases_field.index(index + 1)),
                Problem::Descending { value, previous },
            );
            valid = false;
        }
    }
    let second_field = Field::key("second_files");
    for (index, second) in files.second_files.iter().enumerate() {
        let entry = second_field.index(index);
        if countries.get(second.country).is_none() {
            problems.push(
                at::<TeamFiles>(&entry.then("country")),
                Problem::UnknownCountry(second.country),
            );
            valid = false;
        }
        if countries.get(second.file).is_some() {
            problems.push(
                at::<TeamFiles>(&entry.then("file")),
                Problem::FileTaken(second.file),
            );
            valid = false;
        }
    }
    let bases = bases.filter(|_| valid)?;
    Some(TeamNumbering {
        bases,
        second_files: files.second_files.clone(),
        squad_numbers: files.squad_numbers,
    })
}

/// The season end's countries: each a country of the world, each once.
pub(crate) fn season_end(
    season: &Season,
    countries: &Countries,
    problems: &mut Problems,
) -> Option<SeasonEnd> {
    let field = Field::key("end").then("countries");
    let mut listed_at = BTreeMap::new();
    let mut valid = true;
    for (index, &number) in season.end.countries.iter().enumerate() {
        let location = at::<Season>(&field.index(index));
        if countries.get(number).is_none() {
            problems.push(location, Problem::UnknownCountry(number));
            valid = false;
        } else if let Some(first) = listed_at.insert(number, location.clone()) {
            problems.push(location, Problem::RepeatedCountry { number, first });
            valid = false;
        }
    }
    valid.then(|| season.end.clone())
}

#[cfg(test)]
mod tests {
    use openswos_common::{Importer, TeamOrdinal};
    use openswos_competitions_json_resource::{CompetitionsFile, CompetitionsJsonFormat};
    use openswos_world_rules_resource::CountryDetail;

    use super::*;

    /// Albania (0), England (8), listed in a `countries.json`.
    fn countries() -> Countries {
        let document = CompetitionsJsonFormat::import(
            br#"{"type": "countries", "count": 2, "countries": [
                {"number": 0, "name": "ALBANIA", "define": "CN_ALBANIA", "team_file": "teams/ALBANIA.json"},
                {"number": 8, "name": "ENGLAND", "define": "CN_ENGLAND", "team_file": "teams/ENGLAND.json"}
            ]}"#,
        )
        .expect("a countries file");
        let CompetitionsFile::Countries(file) = document.file else {
            panic!("a countries file");
        };
        let mut problems = Problems::default();
        let countries = Countries::build(&file, &mut problems);
        problems.finish(Some(countries)).expect("valid countries")
    }

    fn detail(number: u8, continent: Continent) -> CountryDetail {
        CountryDetail {
            number: TeamFileNumber(number),
            continent,
            adjective: format!("ADJECTIVE {number}"),
        }
    }

    fn checked<T>(check: impl FnOnce(&mut Problems) -> Option<T>) -> Result<T, String> {
        let mut problems = Problems::default();
        let value = check(&mut problems);
        problems
            .finish(value)
            .map_err(|invalid| invalid.to_string())
    }

    fn numbering(bases: Vec<u16>, second_files: Vec<SecondFile>) -> TeamFiles {
        TeamFiles {
            bases,
            second_files,
            squad_numbers: false,
        }
    }

    #[test]
    fn every_country_has_its_continent_and_adjective() {
        let details = CountryDetails {
            countries: vec![detail(8, Continent::Europe), detail(0, Continent::Asia)],
        };
        let details =
            checked(|problems| country_details(&details, &countries(), problems)).expect("valid");
        assert_eq!(
            details.get(&TeamFileNumber(0)),
            Some(&Details {
                continent: Continent::Asia,
                adjective: b"ADJECTIVE 0".to_vec()
            })
        );
    }

    #[test]
    fn an_adjective_beyond_a_byte_a_character_is_reported() {
        let mut england = detail(8, Continent::Europe);
        england.adjective = "ENGLISH ✓".to_owned();
        let details = CountryDetails {
            countries: vec![detail(0, Continent::Europe), england],
        };
        let error = checked(|problems| country_details(&details, &countries(), problems))
            .expect_err("not a byte");
        assert_eq!(
            error,
            "rules/country_details.json: countries[1].adjective: '✓' is no character of the \
             game's texts (one byte each, not NUL)"
        );
    }

    #[test]
    fn a_country_missing_repeated_or_unknown_is_reported() {
        let details = CountryDetails {
            countries: vec![
                detail(0, Continent::Europe),
                detail(0, Continent::Asia),
                detail(9, Continent::Europe),
            ],
        };
        let error = checked(|problems| country_details(&details, &countries(), problems))
            .expect_err("three problems");
        assert_eq!(
            error,
            "rules/country_details.json: countries[1].number: country 0 is listed again (first \
             at rules/country_details.json: countries[0].number)\n\
             rules/country_details.json: countries[2].number: country 9 is not a country of the \
             world\n\
             rules/country_details.json: countries: country 8 has no entry"
        );
    }

    #[test]
    fn a_club_is_numbered_from_its_files_base() {
        let mut bases = vec![0; TEAM_FILES];
        for (base, index) in bases.iter_mut().zip(0..) {
            *base = index * 10;
        }
        let files = numbering(bases, Vec::new());
        let numbering =
            checked(|problems| team_numbering(&files, &countries(), problems)).expect("valid");
        assert_eq!(numbering.base(TeamFileNumber(8)), 80);
        let team = TeamId {
            file: TeamFileNumber(8),
            ordinal: TeamOrdinal(3),
        };
        assert_eq!(numbering.global_number(team), 83);
    }

    #[test]
    fn bases_must_be_256_never_going_down() {
        let error = checked(|problems| {
            team_numbering(
                &numbering(vec![0, 16, 10], Vec::new()),
                &countries(),
                problems,
            )
        })
        .expect_err("short and going down");
        assert_eq!(
            error,
            "rules/team_files.json: bases: 3 entries, not 256\n\
             rules/team_files.json: bases[2]: 10 is below the one before (16)"
        );
    }

    #[test]
    fn a_second_file_is_a_countrys_on_a_free_number() {
        let files = numbering(
            vec![0; TEAM_FILES],
            vec![
                SecondFile {
                    country: TeamFileNumber(8),
                    file: TeamFileNumber(197),
                },
                SecondFile {
                    country: TeamFileNumber(9),
                    file: TeamFileNumber(0),
                },
            ],
        );
        let error = checked(|problems| team_numbering(&files, &countries(), problems))
            .expect_err("one wrong");
        assert_eq!(
            error,
            "rules/team_files.json: second_files[1].country: country 9 is not a country of the \
             world\n\
             rules/team_files.json: second_files[1].file: file 0 is a country's own"
        );
    }

    #[test]
    fn the_season_ends_through_countries_of_the_world_once_each() {
        let season = |numbers: &[u8]| Season {
            end: SeasonEnd {
                countries: numbers.iter().copied().map(TeamFileNumber).collect(),
                own_country_first: true,
            },
        };
        let end = checked(|problems| season_end(&season(&[8, 0]), &countries(), problems))
            .expect("valid");
        assert_eq!(end.countries, [TeamFileNumber(8), TeamFileNumber(0)]);
        let error = checked(|problems| season_end(&season(&[8, 8, 5]), &countries(), problems))
            .expect_err("repeated and unknown");
        assert_eq!(
            error,
            "rules/season.json: end.countries[1]: country 8 is listed again (first at \
             rules/season.json: end.countries[0])\n\
             rules/season.json: end.countries[2]: country 5 is not a country of the world"
        );
    }

    #[test]
    fn the_rules_files_are_required() {
        let error =
            checked(|problems| pick(&Rules::default(), problems).map(|_| ())).expect_err("nothing");
        assert_eq!(
            error,
            "rules/calendar.json: the file is missing\n\
             rules/country_details.json: the file is missing\n\
             rules/team_files.json: the file is missing\n\
             rules/season.json: the file is missing\n\
             rules/fixtures.json: the file is missing\n\
             rules/strings.json: the file is missing\n\
             rules/competition_classes.json: the file is missing\n\
             rules/draws.json: the file is missing"
        );
        let rules = Rules {
            calendar: Some(Calendar { first_year: 1996 }),
            country_details: Some(CountryDetails {
                countries: Vec::new(),
            }),
            team_files: Some(numbering(Vec::new(), Vec::new())),
            season: Some(Season {
                end: SeasonEnd {
                    countries: Vec::new(),
                    own_country_first: true,
                },
            }),
            fixtures: Some(Fixtures {
                odd_tournament_groups: true,
                schedules: Vec::new(),
                competition_schedules: Vec::new(),
            }),
            strings: Some(Strings {
                symbols: BTreeMap::new(),
                league_reset_name: String::new(),
                cup_reset_name: String::new(),
                tournament_reset_name: String::new(),
                division_names: Vec::new(),
                cup_name: String::new(),
                tournament_name: String::new(),
                cup_round_names: Vec::new(),
            }),
            competition_classes: Some(CompetitionClasses {
                national_teams: Vec::new(),
                continental: Vec::new(),
            }),
            draws: Some(Draws {
                own_draws: Vec::new(),
            }),
        };
        let picked = checked(|problems| pick(&rules, problems)).expect("picked");
        assert!(picked.fixtures.odd_tournament_groups);
    }
}
