#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod countries;
pub mod cups;
pub mod file;
pub mod leagues;
pub mod shared;
pub mod tables;

use openswos_common::json::{NonAscii, TrailingNewline, to_text};
use openswos_common::{Exporter, Importer};
use thiserror::Error;

pub use countries::{Berths, Country, CountryCompetitions, CountryList, SeedBucket};
pub use cups::{Cup, CupKind, Playoff, Round, Tournament, TournamentKind};
pub use file::{
    CareerFile, CompetitionsFile, CountriesFile, CountryCompetitionsFile, CupsFile,
    LeagueBlocksFile, LeaguesFile, PlayoffsFile, QualifyingFile, SeedBucketsFile, TournamentsFile,
};
pub use leagues::{Continent, Division, InvalidContinent, League, NewCountry};
pub use shared::{
    CompetitionHeader, CompetitionType, InvalidCompetitionType, InvalidRuleBits, Names, RoundRules,
    RuleBits, SeedCountry, SymbolId, TeamSeed,
};
pub use tables::{
    BlockLeague, CareerContest, CompCupEntry, InvalidMarker, LeagueBlock, Marker, QualifyingGroup,
};

/// How upstream writes these files: characters beyond ASCII as they are
/// (`ensure_ascii=False`, scripts/export_competitions.py and scripts/world_import.py).
pub const NON_ASCII: NonAscii = NonAscii::Raw;

/// A competition JSON file and the one thing about its text the model does not hold:
/// whether it ends with a newline (upstream's do not, World 2026-27's do).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompetitionsDocument {
    /// The file.
    pub file: CompetitionsFile,
    /// Whether its text ends with a newline.
    pub trailing_newline: TrailingNewline,
}

/// Why a competition JSON file does not decode or encode.
#[derive(Debug, Error)]
pub enum CompetitionsJsonError {
    /// Not valid JSON, or not a competition file.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The `count` key disagrees with the records.
    #[error("count says {count} records, the file has {records}")]
    CountMismatch {
        /// The `count` key.
        count: usize,
        /// The records in the file.
        records: usize,
    },
}

/// The competition JSON format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetitionsJsonFormat;

impl Importer for CompetitionsJsonFormat {
    type Model = CompetitionsDocument;
    type Error = CompetitionsJsonError;

    fn import(bytes: &[u8]) -> Result<CompetitionsDocument, CompetitionsJsonError> {
        let file: CompetitionsFile = serde_json::from_slice(bytes)?;
        if let Some((count, records)) = file.count()
            && count != records
        {
            return Err(CompetitionsJsonError::CountMismatch { count, records });
        }
        Ok(CompetitionsDocument {
            file,
            trailing_newline: TrailingNewline::of(bytes),
        })
    }
}

impl Exporter for CompetitionsJsonFormat {
    type Model = CompetitionsDocument;
    type Error = CompetitionsJsonError;

    fn export(model: &CompetitionsDocument) -> Result<Vec<u8>, CompetitionsJsonError> {
        Ok(to_text(&model.file, NON_ASCII, model.trailing_newline)?.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrong_count_is_reported() {
        let text = br#"{"type":"career_index","count":2,"contests":[]}"#;
        let error = CompetitionsJsonFormat::import(text).expect_err("a wrong count");
        assert!(matches!(
            error,
            CompetitionsJsonError::CountMismatch {
                count: 2,
                records: 0
            }
        ));
        assert_eq!(error.to_string(), "count says 2 records, the file has 0");
    }

    #[test]
    fn writes_upstreams_layout() {
        let text = "{\n  \"type\": \"career_index\",\n  \"count\": 0,\n  \"contests\": []\n}";
        let document = CompetitionsJsonFormat::import(text.as_bytes()).expect("valid");
        assert_eq!(document.trailing_newline, TrailingNewline::Absent);
        assert_eq!(
            CompetitionsJsonFormat::export(&document).ok(),
            Some(text.as_bytes().to_vec())
        );
    }

    #[test]
    fn writes_names_beyond_ascii_as_they_are() {
        let text = "{\n  \"type\": \"countries\",\n  \"count\": 1,\n  \"countries\": [\n    {\n      \"number\": 66,\n      \"name\": \"TÜRKIYE\",\n      \"define\": \"CN_TURKEY\",\n      \"team_file\": null\n    }\n  ]\n}\n";
        let document = CompetitionsJsonFormat::import(text.as_bytes()).expect("valid");
        assert_eq!(document.trailing_newline, TrailingNewline::Present);
        assert_eq!(
            CompetitionsJsonFormat::export(&document).ok(),
            Some(text.as_bytes().to_vec())
        );
    }
}
