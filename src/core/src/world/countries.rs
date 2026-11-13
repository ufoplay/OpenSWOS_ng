//! The world's countries (`countries.json`, upstream's `_allCountriesList`): each a number,
//! which is its team file's, a name, a `CN_*` name, and its roster.

use std::collections::BTreeMap;

use openswos_common::TeamFileNumber;
use openswos_competitions_json_resource::{CountriesFile, Country as CountryRecord, SeedCountry};
use openswos_world_package_resource::TEAMS_FOLDER;

use super::files::Slot;
use crate::validation::{Field, Location, Problem, Problems};

/// The extension of a roster's file.
const ROSTER_EXTENSION: &str = ".json";

/// A country of the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Country {
    /// Its number: its team file's.
    pub number: TeamFileNumber,
    /// Its name.
    pub name: String,
    /// Its `CN_*` name, by which competition files may name it.
    pub define: String,
    /// Its roster's file name in `teams/` (`BRAZIL.json`), if it has one.
    pub roster: Option<String>,
    /// Its entry in `countries.json`: where the problems of its roster are reported.
    listed_at: usize,
}

impl Country {
    /// Where its roster is named.
    pub(crate) fn roster_location(&self) -> Location {
        Location::field(
            &Slot::Countries.path(),
            &Field::key("countries")
                .index(self.listed_at)
                .then("team_file"),
        )
    }
}

/// The world's countries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Countries {
    /// Each country.
    by_number: BTreeMap<TeamFileNumber, Country>,
    /// The numbers in the file's order, repeats included.
    listed: Vec<TeamFileNumber>,
    /// The numbers by `CN_*` name.
    by_define: BTreeMap<String, TeamFileNumber>,
}

impl Countries {
    /// The country numbered `number`.
    #[must_use]
    pub fn get(&self, number: TeamFileNumber) -> Option<&Country> {
        self.by_number.get(&number)
    }

    /// The country whose `CN_*` name is `define`.
    #[must_use]
    pub fn by_define(&self, define: &str) -> Option<&Country> {
        self.by_define
            .get(define)
            .and_then(|number| self.get(*number))
    }

    /// The countries by number.
    pub fn iter(&self) -> impl Iterator<Item = &Country> {
        self.by_number.values()
    }

    /// The countries in the file's order. Faithful to the original: upstream's list starts
    /// with BRAZIL, which it lists again in its place.
    pub fn listed(&self) -> impl Iterator<Item = &Country> {
        self.listed
            .iter()
            .filter_map(|number| self.by_number.get(number))
    }

    /// The country a seeded team's `country` names.
    pub(crate) fn resolve(&self, country: &SeedCountry) -> Result<&Country, Problem> {
        match country {
            SeedCountry::Define(define) => self
                .by_define(define)
                .ok_or_else(|| Problem::UnknownCountryDefine(define.clone())),
            SeedCountry::Number(number) => {
                self.get(*number).ok_or(Problem::UnknownCountry(*number))
            }
        }
    }

    /// Whether `number` is a country, reporting it at `location` when it is not.
    pub(crate) fn check(
        &self,
        number: TeamFileNumber,
        location: impl FnOnce() -> Location,
        problems: &mut Problems,
    ) -> Option<TeamFileNumber> {
        if self.by_number.contains_key(&number) {
            Some(number)
        } else {
            problems.push(location(), Problem::UnknownCountry(number));
            None
        }
    }

    /// The countries `file` lists. A country listed twice must be listed the same; a
    /// conflicting entry is reported, and the first kept.
    pub(crate) fn build(file: &CountriesFile, problems: &mut Problems) -> Self {
        let path = Slot::Countries.path();
        let mut countries = Self::default();
        for (index, record) in file.countries.iter().enumerate() {
            let field = Field::key("countries").index(index);
            let CountryRecord {
                number,
                name,
                define,
                team_file,
            } = record;
            let roster = team_file.as_deref().and_then(|path_in_package| {
                let roster = roster_name(path_in_package);
                if roster.is_none() {
                    problems.push(
                        Location::field(&path, &field.then("team_file")),
                        Problem::NotARosterPath(path_in_package.to_owned()),
                    );
                }
                roster
            });
            let country = Country {
                number: *number,
                name: name.clone(),
                define: define.clone(),
                roster,
                listed_at: index,
            };
            if let Some(first) = countries.by_number.get(number) {
                if !same_listing(first, &country) {
                    problems.push(
                        Location::field(&path, &field),
                        Problem::ConflictingCountry {
                            number: *number,
                            first: Location::field(
                                &path,
                                &Field::key("countries").index(first.listed_at),
                            ),
                        },
                    );
                }
                countries.listed.push(*number);
                continue;
            }
            if let Some(first) = countries.by_define.get(define) {
                problems.push(
                    Location::field(&path, &field.then("define")),
                    Problem::DuplicateDefine {
                        define: define.clone(),
                        first: *first,
                    },
                );
            } else {
                countries.by_define.insert(define.clone(), *number);
            }
            countries.listed.push(*number);
            countries.by_number.insert(*number, country);
        }
        countries
    }
}

/// Whether two entries list a country the same, wherever they are.
fn same_listing(first: &Country, again: &Country) -> bool {
    let Country {
        number,
        name,
        define,
        roster,
        listed_at: _,
    } = first;
    (number, name, define, roster) == (&again.number, &again.name, &again.define, &again.roster)
}

/// The roster file a country's `team_file` names: `teams/<NAME>.json`, a file directly in
/// `teams/` (the folder the package reads rosters from).
fn roster_name(team_file: &str) -> Option<String> {
    team_file
        .strip_prefix(TEAMS_FOLDER)
        .and_then(|rest| rest.strip_prefix('/'))
        .filter(|name| {
            !name.contains('/')
                && name
                    .strip_suffix(ROSTER_EXTENSION)
                    .is_some_and(|stem| !stem.is_empty())
        })
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use openswos_competitions_json_resource::{CountriesFile, Country as CountryRecord};

    use super::*;

    fn record(number: u8, name: &str, team_file: Option<&str>) -> CountryRecord {
        CountryRecord {
            number: TeamFileNumber(number),
            name: name.to_owned(),
            define: format!("CN_{name}"),
            team_file: team_file.map(str::to_owned),
        }
    }

    fn file(countries: Vec<CountryRecord>) -> CountriesFile {
        CountriesFile {
            count: countries.len(),
            countries,
        }
    }

    fn build(countries: Vec<CountryRecord>) -> (Countries, Vec<String>) {
        let mut problems = Problems::default();
        let built = Countries::build(&file(countries), &mut problems);
        let reported = match problems.finish(Some(())) {
            Ok(()) => Vec::new(),
            Err(invalid) => invalid.0.iter().map(ToString::to_string).collect(),
        };
        (built, reported)
    }

    #[test]
    fn a_roster_is_a_file_directly_in_teams() {
        assert_eq!(
            roster_name("teams/BRAZIL.json"),
            Some("BRAZIL.json".to_owned())
        );
        assert_eq!(roster_name("teams/.json"), None);
        assert_eq!(roster_name("teams/a/BRAZIL.json"), None);
        assert_eq!(roster_name("data/TEAM.046"), None);
        assert_eq!(roster_name("teams/BRAZIL.txt"), None);
        assert_eq!(roster_name("teamsBRAZIL.json"), None);
    }

    #[test]
    fn countries_are_found_by_number_and_by_define() {
        let (countries, problems) = build(vec![
            record(46, "BRAZIL", Some("teams/BRAZIL.json")),
            record(0, "ALBANIA", None),
        ]);
        assert!(problems.is_empty(), "{problems:?}");
        let brazil = countries.get(TeamFileNumber(46)).expect("Brazil");
        assert_eq!(brazil.roster.as_deref(), Some("BRAZIL.json"));
        assert_eq!(
            countries
                .by_define("CN_ALBANIA")
                .map(|country| country.number),
            Some(TeamFileNumber(0))
        );
        assert_eq!(countries.get(TeamFileNumber(1)), None);
        assert_eq!(
            countries
                .iter()
                .map(|country| country.number.0)
                .collect::<Vec<_>>(),
            [0, 46]
        );
    }

    #[test]
    fn a_country_listed_again_the_same_is_kept_in_the_order() {
        let brazil = record(46, "BRAZIL", Some("teams/BRAZIL.json"));
        let (countries, problems) = build(vec![brazil.clone(), record(0, "ALBANIA", None), brazil]);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(
            countries
                .listed()
                .map(|country| country.name.as_str())
                .collect::<Vec<_>>(),
            ["BRAZIL", "ALBANIA", "BRAZIL"]
        );
        assert_eq!(countries.iter().count(), 2);
    }

    #[test]
    fn a_country_listed_again_differently_is_reported() {
        let (countries, problems) =
            build(vec![record(46, "BRAZIL", None), record(46, "BRASIL", None)]);
        assert_eq!(
            problems,
            [
                "competitions/countries.json: countries[1]: country 46 is listed again, differently \
             (first at competitions/countries.json: countries[0])"
            ]
        );
        assert_eq!(
            countries
                .get(TeamFileNumber(46))
                .map(|country| country.name.as_str()),
            Some("BRAZIL")
        );
    }

    #[test]
    fn a_define_of_two_countries_and_a_bad_roster_are_reported() {
        let mut twin = record(1, "ALBANIA", Some("ALBANIA.json"));
        twin.define = "CN_ALBANIA".to_owned();
        let (_, problems) = build(vec![record(0, "ALBANIA", None), twin]);
        assert_eq!(
            problems,
            [
                "competitions/countries.json: countries[1].team_file: ALBANIA.json is not a roster \
             of the package (teams/<NAME>.json)",
                "competitions/countries.json: countries[1].define: CN_ALBANIA is country 0's too",
            ]
        );
    }

    #[test]
    fn a_seed_names_its_country_by_define_or_by_number() {
        let (countries, _) = build(vec![record(46, "BRAZIL", None)]);
        let by_define = countries.resolve(&SeedCountry::Define("CN_BRAZIL".to_owned()));
        assert_eq!(
            by_define.map(|country| country.number),
            Ok(TeamFileNumber(46))
        );
        let by_number = countries.resolve(&SeedCountry::Number(TeamFileNumber(46)));
        assert_eq!(
            by_number.map(|country| country.number),
            Ok(TeamFileNumber(46))
        );
        assert_eq!(
            countries.resolve(&SeedCountry::Define("CN_MARS".to_owned())),
            Err(Problem::UnknownCountryDefine("CN_MARS".to_owned()))
        );
        assert_eq!(
            countries.resolve(&SeedCountry::Number(TeamFileNumber(7))),
            Err(Problem::UnknownCountry(TeamFileNumber(7)))
        );
    }

    #[test]
    fn checking_a_number_reports_where_it_is() {
        let (countries, _) = build(vec![record(46, "BRAZIL", None)]);
        let mut problems = Problems::default();
        assert_eq!(
            countries.check(TeamFileNumber(46), || Location::file("x"), &mut problems),
            Some(TeamFileNumber(46))
        );
        assert_eq!(
            countries.check(TeamFileNumber(9), || Location::file("x"), &mut problems),
            None
        );
        let invalid = problems.finish(Some(())).expect_err("one problem");
        assert_eq!(
            invalid.to_string(),
            "x: country 9 is not a country of the world"
        );
    }

    #[test]
    fn a_countrys_roster_is_reported_at_its_entry() {
        let (countries, _) = build(vec![
            record(0, "ALBANIA", None),
            record(46, "BRAZIL", Some("teams/BRAZIL.json")),
        ]);
        let brazil = countries.get(TeamFileNumber(46)).expect("Brazil");
        assert_eq!(
            brazil.roster_location().to_string(),
            "competitions/countries.json: countries[1].team_file"
        );
    }
}
