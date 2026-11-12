//! The country tables: `countries.json` (the master list), `country_competitions.json`
//! (each country's competitions) and `seed_buckets.json` (the UEFA Cup berths).

use openswos_common::TeamFileNumber;
use serde::{Deserialize, Serialize};

use crate::shared::SymbolId;

/// A country of the master list (upstream `_allCountriesList` and the continents).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Country {
    /// Its number: its team file.
    pub number: TeamFileNumber,
    /// Its name.
    pub name: String,
    /// Its `CN_*` name.
    pub define: String,
    /// Its team JSON, `null` when it has none.
    pub team_file: Option<String>,
}

/// A country's competitions (upstream `_competitionsTable[country]`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountryCompetitions {
    /// Its number.
    pub number: TeamFileNumber,
    /// Its name.
    pub name: String,
    /// Its table's id upstream: a country table of the C data, or a region's qualifying
    /// group.
    pub table: SymbolId,
    /// Its leagues, when it has a table of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leagues: Option<Vec<SymbolId>>,
    /// Its cups, likewise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cups: Option<Vec<SymbolId>>,
}

/// A coefficient tier of the UEFA Cup berths (upstream `EuroCupQualSeedBucketBlock`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedBucket {
    /// Its upstream array's name (`bucket15`).
    pub tier: String,
    /// Its berths in all.
    pub size: u8,
    /// Its countries' berths, in draw order.
    pub allocation: Vec<Berths>,
}

/// A country's berths in a tier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Berths {
    /// The country's name.
    pub country: String,
    /// Its number.
    pub number: TeamFileNumber,
    /// Its berths.
    pub berths: u8,
}

/// Another country list of upstream's C data (none today).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountryList {
    /// Its id.
    pub id: SymbolId,
    /// Its countries.
    pub countries: Vec<Country>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_country_may_have_no_team_file() {
        for text in [
            r#"{"number":0,"name":"ALBANIA","define":"CN_ALBANIA","team_file":"teams/ALBANIA.json"}"#,
            r#"{"number":72,"name":"CUSTOMS","define":"CN_CUSTOMS","team_file":null}"#,
        ] {
            let country: Country = serde_json::from_str(text).expect("a country");
            assert_eq!(serde_json::to_string(&country).ok(), Some(text.to_owned()));
        }
    }

    #[test]
    fn a_country_table_may_list_no_competitions() {
        for text in [
            r#"{"number":0,"name":"ALBANIA","table":"_albaniaTable","leagues":["_albanianLeague"],"cups":[]}"#,
            r#"{"number":80,"name":"EUROPE","table":"_europeTable"}"#,
        ] {
            let table: CountryCompetitions = serde_json::from_str(text).expect("a table");
            assert_eq!(serde_json::to_string(&table).ok(), Some(text.to_owned()));
        }
    }

    #[test]
    fn a_bucket_lists_berths_in_order() {
        let text = r#"{"tier":"bucket15","size":15,"allocation":[{"country":"GERMANY","number":19,"berths":4}]}"#;
        let bucket: SeedBucket = serde_json::from_str(text).expect("a bucket");
        assert_eq!(
            bucket.allocation.first().map(|berths| berths.berths),
            Some(4)
        );
        assert_eq!(serde_json::to_string(&bucket).ok(), Some(text.to_owned()));
    }
}
