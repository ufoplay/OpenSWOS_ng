#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod convert;
pub mod ids;
pub mod model;

use openswos_common::json::to_text;
use openswos_common::{Exporter, Importer};
use openswos_team_resource::{Team, TeamFile};
use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use thiserror::Error;

pub use convert::{EntryError, EntryExtras, EntryIds, ExtraPlayer};
pub use ids::{
    TEAM_ID_NAMESPACE, WORLD_1996_97_TAG, derived_player_uuid, derived_team_uuid, players_checksum,
    team_checksum,
};
pub use model::{
    BirthYear, FIRST_KNOWN_BIRTH_YEAR, FileKind, LAST_KNOWN_BIRTH_YEAR, PlayerEntry,
    PlayerKeyOrder, TeamChecksum, TeamEntry, TeamHeader, TeamJsonFile,
};
pub use openswos_common::json::{Indent, JsonByte, JsonWord, NonAscii, TrailingNewline};

/// A team JSON file and what about its text the model does not hold: its indentation, how it
/// writes characters beyond ASCII and whether it ends with a newline (each player's key order
/// is the player's: [`PlayerKeyOrder`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamJsonDocument {
    /// The file.
    pub file: TeamJsonFile,
    /// Its indentation: two spaces (OpenSWOS Setup, `scripts/world_import.py`), one in World
    /// 2026-27's sealed package (`scripts/worldgen/seal.py` rewrites every file).
    pub indent: Indent,
    /// Its characters beyond ASCII: escaped (Setup), or as they are (the sealed package).
    pub non_ascii: NonAscii,
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

/// The indentation of the team JSON of the world packages this port makes: two spaces, as
/// upstream's import writes a world's clubs (`scripts/world_import.py`, `indent=2`; World
/// 2026-27's sealed package is rewritten with one, `scripts/worldgen/seal.py`).
pub const WORLD_PACKAGE_INDENT: Indent = Indent::Two;

/// How the team JSON of the world packages this port makes writes characters beyond ASCII:
/// escaped, as OpenSWOS Setup does (`ensure_ascii`; the names of the PC game's files are
/// ASCII, so it writes them the same either way).
pub const WORLD_PACKAGE_NON_ASCII: NonAscii = NonAscii::Escaped;

/// The key orders of a file's players, read beside the model (which reads keys in any order).
#[derive(Deserialize)]
struct KeyOrders {
    teams: Vec<TeamKeyOrders>,
}

/// A team's players' key orders.
#[derive(Deserialize)]
struct TeamKeyOrders {
    players: Vec<PlayerKeys>,
    #[serde(default)]
    extra_players: Option<Vec<PlayerKeys>>,
}

/// A player's key order: [`PlayerKeyOrder::FaceFirst`] when its `face` comes before its
/// `position`.
struct PlayerKeys(PlayerKeyOrder);

impl<'de> Deserialize<'de> for PlayerKeys {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(PlayerKeysVisitor)
    }
}

struct PlayerKeysVisitor;

impl<'de> Visitor<'de> for PlayerKeysVisitor {
    type Value = PlayerKeys;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a player")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<PlayerKeys, A::Error> {
        let mut order = PlayerKeyOrder::Record;
        let mut position_seen = false;
        while let Some(key) = map.next_key::<String>()? {
            map.next_value::<IgnoredAny>()?;
            match key.as_str() {
                "position" => position_seen = true,
                "face" if !position_seen => order = PlayerKeyOrder::FaceFirst,
                _ => {}
            }
        }
        Ok(PlayerKeys(order))
    }
}

/// Gives the file's players the key orders their text has.
fn apply_key_orders(file: &mut TeamJsonFile, orders: KeyOrders) {
    for (team, orders) in file.teams.iter_mut().zip(orders.teams) {
        for (player, PlayerKeys(order)) in team.players.iter_mut().zip(orders.players) {
            player.key_order = order;
        }
        if let (Some(players), Some(orders)) = (team.extra_players.as_mut(), orders.extra_players) {
            for (player, PlayerKeys(order)) in players.iter_mut().zip(orders) {
                player.key_order = order;
            }
        }
    }
}

/// The team JSON format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamJsonFormat;

impl Importer for TeamJsonFormat {
    type Model = TeamJsonDocument;
    type Error = TeamJsonError;

    fn import(bytes: &[u8]) -> Result<TeamJsonDocument, TeamJsonError> {
        let mut file: TeamJsonFile = serde_json::from_slice(bytes)?;
        if file.count != file.teams.len() {
            return Err(TeamJsonError::CountMismatch {
                count: file.count,
                teams: file.teams.len(),
            });
        }
        apply_key_orders(&mut file, serde_json::from_slice(bytes)?);
        Ok(TeamJsonDocument {
            file,
            indent: Indent::of(bytes),
            non_ascii: NonAscii::of(bytes),
            trailing_newline: TrailingNewline::of(bytes),
        })
    }
}

impl Exporter for TeamJsonFormat {
    type Model = TeamJsonDocument;
    type Error = TeamJsonError;

    fn export(model: &TeamJsonDocument) -> Result<Vec<u8>, TeamJsonError> {
        Ok(to_text(
            &model.file,
            model.indent,
            model.non_ascii,
            model.trailing_newline,
        )?
        .into_bytes())
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

    /// World 2026-27's sealed dialect: one space, characters as they are, a national team's
    /// player face first, a look, a year of birth, players 17 on. It round-trips byte for byte.
    #[test]
    #[allow(clippy::indexing_slicing)] // fixed indexes into a fixed-size squad
    fn the_sealed_packages_dialect_round_trips() {
        use openswos_team_resource::{LookBits, TeamFileFormat};
        let bytes = std::fs::read(
            openswos_test_utils::assets_path()
                .join("TEAM")
                .join("TEAM.085"),
        )
        .expect("the committed fixture");
        let teams = TeamFileFormat::import(&bytes).expect("a team file");
        let mut file = TeamJsonFile::from_team_file(&teams, Some("OCEANIA"), |team| {
            EntryIds::derived(WORLD_1996_97_TAG, team)
        });
        let entry = file.teams.first_mut().expect("an entry");
        entry.players[0].key_order = PlayerKeyOrder::FaceFirst;
        entry.players[0].looks = Some(LookBits::BALD);
        entry.players[1].born = Some(BirthYear(2005));
        entry.players[2].player_name = "JOS\u{c9}".to_owned();
        entry.extra_players = Some(vec![entry.players[0].clone()]);
        let document = TeamJsonDocument {
            file,
            indent: Indent::One,
            non_ascii: NonAscii::Raw,
            trailing_newline: TrailingNewline::Present,
        };
        let text = TeamJsonFormat::export(&document).expect("exports");
        let read = TeamJsonFormat::import(&text).expect("reads back");
        assert_eq!(read, document);
        assert_eq!(TeamJsonFormat::export(&read).expect("exports"), text);
        let text = String::from_utf8(text).expect("UTF-8");
        assert!(text.starts_with("{\n \"type\": \"team\","));
        assert!(text.contains("\"JOS\u{c9}\""));
        let first = text.find("\"player_name\"").expect("a player");
        let (face, looks, position) = (
            text[first..].find("\"face\"").expect("a face"),
            text[first..].find("\"looks\": 1").expect("a look"),
            text[first..].find("\"position\"").expect("a position"),
        );
        assert!(face < looks && looks < position, "face first");
        assert!(text.contains("\"born\": 2005"));
        assert_eq!(
            read.file.teams[0].extras().map(|extras| extras.born[1]),
            Ok(Some(BirthYear(2005)))
        );
    }
}
