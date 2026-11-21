//! `rules/country_details.json`: what the world knows of each country beyond its competitions
//! (its continent).

use openswos_common::TeamFileNumber;
use openswos_common::json::code;
use openswos_competitions_json_resource::Continent;
use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/country_details.json`: one entry a country, by number.
///
/// ```json
/// {
///   "countries": [
///     {
///       "number": 0,
///       "continent": 0
///     }
///   ]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountryDetails {
    /// The countries.
    pub countries: Vec<CountryDetail>,
}

/// A country's details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountryDetail {
    /// Its number (its team file's).
    pub number: TeamFileNumber,
    /// Its continent, by its code (upstream `_countriesTable[n][0]`, src/strings_data.c): the
    /// European cups take only European countries' clubs (career_eurocups.c).
    #[serde(with = "code")]
    pub continent: Continent,
}

impl RulesFile for CountryDetails {
    const FILE_NAME: &'static str = "country_details.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    #[test]
    fn it_reads_and_writes_back() {
        let text = "{\n  \"countries\": [\n    {\n      \"number\": 0,\n      \"continent\": 0\n    \
                    },\n    {\n      \"number\": 42,\n      \"continent\": 5\n    }\n  ]\n}\n";
        let details: CountryDetails = read(text.as_bytes()).expect("details");
        assert_eq!(
            details.countries,
            [
                CountryDetail {
                    number: TeamFileNumber(0),
                    continent: Continent::Europe
                },
                CountryDetail {
                    number: TeamFileNumber(42),
                    continent: Continent::Africa
                }
            ]
        );
        assert_eq!(write(&details), text.as_bytes());
    }

    #[test]
    fn a_code_that_is_no_continent_does_not_read() {
        let error = read::<CountryDetails>(br#"{"countries": [{"number": 0, "continent": 6}]}"#)
            .expect_err("no continent 6");
        assert!(error.to_string().contains("continent"), "{error}");
    }
}
