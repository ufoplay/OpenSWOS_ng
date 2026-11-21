//! `rules/season.json`: how a season ends. Today the countries it goes through; the steps join
//! with the season end's port (M5).

use openswos_common::TeamFileNumber;
use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/season.json`.
///
/// ```json
/// {
///   "end": {
///     "countries": [0, 1, 76, …],
///     "own_country_first": true
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Season {
    /// The season's end.
    pub end: SeasonEnd,
}

/// The season's end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonEnd {
    /// The countries whose competitions end the season and send clubs to the European cups,
    /// in its order (upstream `_careerCompetitionIdRemapTable`, src/competition_data.c; a
    /// world's `World_SeasonCountries`). The order is the draws' and the places'.
    pub countries: Vec<TeamFileNumber>,
    /// Whether a club coach's own country ends its season first, then the others in their
    /// order (upstream career_season.c `Career_AdvanceToNextSeason_UpdateCompetitions`).
    pub own_country_first: bool,
}

impl RulesFile for Season {
    const FILE_NAME: &'static str = "season.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    #[test]
    fn it_reads_and_writes_back() {
        let text = "{\n  \"end\": {\n    \"countries\": [\n      0,\n      1,\n      76\n    ],\n    \
                    \"own_country_first\": true\n  }\n}\n";
        let season: Season = read(text.as_bytes()).expect("a season");
        assert_eq!(
            season.end.countries,
            [TeamFileNumber(0), TeamFileNumber(1), TeamFileNumber(76)]
        );
        assert!(season.end.own_country_first);
        assert_eq!(write(&season), text.as_bytes());
    }

    #[test]
    fn a_country_is_a_byte() {
        assert!(
            read::<Season>(br#"{"end": {"countries": [256], "own_country_first": true}}"#).is_err()
        );
    }
}
