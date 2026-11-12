#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

mod code;
pub mod convert;
pub mod ids;
pub mod model;
pub mod numbers;
pub mod text;

use openswos_common::{Exporter, Importer};
use openswos_team_resource::{Team, TeamFile};
use thiserror::Error;

pub use convert::{EntryError, EntryIds};
pub use ids::{
    TEAM_ID_NAMESPACE, WORLD_1996_97_TAG, derived_player_uuid, derived_team_uuid, team_checksum,
};
pub use model::{FileKind, PlayerEntry, TeamChecksum, TeamEntry, TeamHeader, TeamJsonFile};
pub use numbers::{JsonByte, JsonWord};
pub use text::TrailingNewline;

/// A team JSON file and the one thing about its text the model does not hold: whether it
/// ends with a newline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamJsonDocument {
    /// The file.
    pub file: TeamJsonFile,
    /// Whether its text ends with a newline.
    pub trailing_newline: TrailingNewline,
}

/// Why a team JSON file does not decode, convert or encode.
#[derive(Debug, Error)]
pub enum TeamJsonError {
    /// Not valid JSON, or not a team file.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The `count` key disagrees with the teams.
    #[error("count says {count} teams, the file has {teams}")]
    CountMismatch {
        /// The `count` key.
        count: usize,
        /// The teams in the file.
        teams: usize,
    },
    /// A team entry does not convert.
    #[error("team {index}: {source}")]
    Team {
        /// The team's index in the file.
        index: usize,
        /// Why.
        source: EntryError,
    },
}

impl TeamJsonFile {
    /// The file's teams, typed.
    ///
    /// # Errors
    ///
    /// When a team entry does not convert.
    pub fn to_team_file(&self) -> Result<TeamFile, TeamJsonError> {
        let teams = self
            .teams
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                entry
                    .to_team()
                    .map_err(|source| TeamJsonError::Team { index, source })
            })
            .collect::<Result<_, _>>()?;
        Ok(TeamFile { teams })
    }

    /// `file` as JSON, as OpenSWOS Setup makes it from a binary team file: the country is
    /// its first team's file number, `country_name` is written when given, and every team
    /// gets the ids `ids` gives it. Numbers are written unsigned, and what the game does not
    /// read is not written ([`TeamEntry::from_team`]).
    #[must_use]
    pub fn from_team_file(
        file: &TeamFile,
        country_name: Option<&str>,
        ids: impl Fn(&Team) -> EntryIds,
    ) -> Self {
        let teams: Vec<TeamEntry> = file
            .teams
            .iter()
            .map(|team| TeamEntry::from_team(team, ids(team)))
            .collect();
        Self {
            kind: FileKind::Team,
            country: file.teams.first().map(|team| team.id.file.0),
            country_name: country_name.map(|name| Some(name.to_owned())),
            count: teams.len(),
            teams,
        }
    }
}

/// Whether the team JSON of the world packages this port makes ends with a newline: it
/// does, as in the World 2026-27 package.
pub const WORLD_PACKAGE_TRAILING_NEWLINE: TrailingNewline = TrailingNewline::Present;

/// The team JSON format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamJsonFormat;

impl Importer for TeamJsonFormat {
    type Model = TeamJsonDocument;
    type Error = TeamJsonError;

    fn import(bytes: &[u8]) -> Result<TeamJsonDocument, TeamJsonError> {
        let file: TeamJsonFile = serde_json::from_slice(bytes)?;
        if file.count != file.teams.len() {
            return Err(TeamJsonError::CountMismatch {
                count: file.count,
                teams: file.teams.len(),
            });
        }
        Ok(TeamJsonDocument {
            file,
            trailing_newline: TrailingNewline::of(bytes),
        })
    }
}

impl Exporter for TeamJsonFormat {
    type Model = TeamJsonDocument;
    type Error = TeamJsonError;

    fn export(model: &TeamJsonDocument) -> Result<Vec<u8>, TeamJsonError> {
        Ok(text::to_text(&model.file, model.trailing_newline)?.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "{\n  \"type\": \"team\",\n  \"country\": null,\n  \"country_name\": null,\n  \"count\": 0,\n  \"teams\": []\n}";

    #[test]
    fn an_empty_file_round_trips() {
        let document = TeamJsonFormat::import(EMPTY.as_bytes()).expect("valid");
        assert_eq!(document.file.country_name, Some(None));
        assert_eq!(document.trailing_newline, TrailingNewline::Absent);
        assert_eq!(
            TeamJsonFormat::export(&document).expect("exports"),
            EMPTY.as_bytes()
        );
        assert_eq!(
            document.file.to_team_file().expect("converts"),
            TeamFile::default()
        );
    }

    #[test]
    fn a_wrong_count_fails() {
        let text = EMPTY.replace("\"count\": 0", "\"count\": 1");
        assert!(matches!(
            TeamJsonFormat::import(text.as_bytes()),
            Err(TeamJsonError::CountMismatch { count: 1, teams: 0 })
        ));
    }

    #[test]
    fn invalid_json_fails() {
        assert!(matches!(
            TeamJsonFormat::import(b"{"),
            Err(TeamJsonError::Json(_))
        ));
    }
}
