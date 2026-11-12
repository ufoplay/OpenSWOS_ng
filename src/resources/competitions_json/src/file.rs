//! A competition JSON file: which one it is says its `type` key.

use serde::{Deserialize, Serialize};

use crate::countries::{Country, CountryCompetitions, CountryList, SeedBucket};
use crate::cups::{Cup, Playoff, Tournament};
use crate::leagues::League;
use crate::tables::{CareerContest, LeagueBlock, QualifyingGroup};

/// A competition JSON file, by its `type`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompetitionsFile {
    /// `leagues.json`: `"competition_leagues"`.
    #[serde(rename = "competition_leagues")]
    Leagues(LeaguesFile),
    /// `cups.json`: `"competition_cups"`.
    #[serde(rename = "competition_cups")]
    Cups(CupsFile),
    /// `tournaments.json`: `"competition_tournaments"`.
    #[serde(rename = "competition_tournaments")]
    Tournaments(TournamentsFile),
    /// `playoffs.json`: `"playoffs"`.
    #[serde(rename = "playoffs")]
    Playoffs(PlayoffsFile),
    /// `qualifying.json`: `"qualifying_groups"`.
    #[serde(rename = "qualifying_groups")]
    Qualifying(QualifyingFile),
    /// `league_blocks.json`: `"league_blocks"`.
    #[serde(rename = "league_blocks")]
    LeagueBlocks(LeagueBlocksFile),
    /// `career.json`: `"career_index"`.
    #[serde(rename = "career_index")]
    Career(CareerFile),
    /// `country_competitions.json`: `"country_competitions"`.
    #[serde(rename = "country_competitions")]
    CountryCompetitions(CountryCompetitionsFile),
    /// `countries.json`: `"countries"`.
    #[serde(rename = "countries")]
    Countries(CountriesFile),
    /// `seed_buckets.json`: `"seed_buckets"`.
    #[serde(rename = "seed_buckets")]
    SeedBuckets(SeedBucketsFile),
}

impl CompetitionsFile {
    /// The file's `count` key and the records it counts, for the files that have one.
    #[must_use]
    pub fn count(&self) -> Option<(usize, usize)> {
        match self {
            Self::Leagues(file) => Some((file.count, file.leagues.len())),
            Self::Cups(file) => Some((file.count, file.competitions.len())),
            Self::Tournaments(file) => Some((file.count, file.competitions.len())),
            Self::Playoffs(file) => Some((file.count, file.playoffs.len())),
            Self::Qualifying(file) => Some((file.count, file.groups.len())),
            Self::LeagueBlocks(file) => Some((file.count, file.groups.len())),
            Self::Career(file) => Some((file.count, file.contests.len())),
            Self::CountryCompetitions(file) => Some((file.count, file.countries.len())),
            Self::Countries(file) => Some((file.count, file.countries.len())),
            Self::SeedBuckets(_) => None,
        }
    }
}

/// `leagues.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaguesFile {
    /// The number of leagues.
    pub count: usize,
    /// The leagues.
    pub leagues: Vec<League>,
}

/// `cups.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CupsFile {
    /// The number of cups.
    pub count: usize,
    /// The cups.
    pub competitions: Vec<Cup>,
}

/// `tournaments.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TournamentsFile {
    /// The number of tournaments.
    pub count: usize,
    /// The tournaments.
    pub competitions: Vec<Tournament>,
}

/// `playoffs.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayoffsFile {
    /// The number of play-offs.
    pub count: usize,
    /// The play-offs.
    pub playoffs: Vec<Playoff>,
}

/// `qualifying.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifyingFile {
    /// The number of groups.
    pub count: usize,
    /// The groups.
    pub groups: Vec<QualifyingGroup>,
}

/// `league_blocks.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeagueBlocksFile {
    /// The number of blocks.
    pub count: usize,
    /// The blocks.
    pub groups: Vec<LeagueBlock>,
}

/// `career.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CareerFile {
    /// The number of contests.
    pub count: usize,
    /// The contests.
    pub contests: Vec<CareerContest>,
}

/// `country_competitions.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountryCompetitionsFile {
    /// The number of countries.
    pub count: usize,
    /// The countries.
    pub countries: Vec<CountryCompetitions>,
}

/// `countries.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountriesFile {
    /// The number of countries.
    pub count: usize,
    /// The countries.
    pub countries: Vec<Country>,
}

/// `seed_buckets.json`: it has no `count`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedBucketsFile {
    /// What the file is, for whoever reads it.
    #[serde(rename = "_comment")]
    pub comment: String,
    /// The UEFA Cup berths, by coefficient tier.
    pub euro_cup_qual_seed_buckets: Vec<SeedBucket>,
    /// Other country lists of upstream's C data (none today).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_lists: Option<Vec<CountryList>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_type_key_selects_the_file_and_comes_first() {
        let text = r#"{"type":"career_index","count":1,"contests":[{"competition":"_euroCup","competition_copy":"_euroCupCopy"}]}"#;
        let file: CompetitionsFile = serde_json::from_str(text).expect("a career index");
        assert!(matches!(file, CompetitionsFile::Career(_)));
        assert_eq!(file.count(), Some((1, 1)));
        assert_eq!(serde_json::to_string(&file).ok(), Some(text.to_owned()));
    }

    #[test]
    fn unknown_types_and_keys_are_rejected() {
        assert!(serde_json::from_str::<CompetitionsFile>(r#"{"type":"team","count":0}"#).is_err());
        assert!(
            serde_json::from_str::<CompetitionsFile>(
                r#"{"type":"career_index","count":0,"contests":[],"extra":1}"#
            )
            .is_err()
        );
    }

    #[test]
    fn seed_buckets_have_no_count() {
        let text = r#"{"type":"seed_buckets","_comment":"berths","euro_cup_qual_seed_buckets":[]}"#;
        let file: CompetitionsFile = serde_json::from_str(text).expect("seed buckets");
        assert_eq!(file.count(), None);
        assert_eq!(serde_json::to_string(&file).ok(), Some(text.to_owned()));
    }
}
