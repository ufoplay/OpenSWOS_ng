//! The JSON's shape, field for field, in upstream's order (upstream
//! `tools/swosdata/lib/team_json.c`, `emit_team`/`emit_player`). Serialising it in that order
//! reproduces upstream's files byte for byte; [`crate::convert`] turns it into the typed
//! team model, checking every range. Codes (position, kit colours, tactics...) and skills
//! are already the typed model's types, written as their numbers.

use openswos_common::{Position, Tactics};
use openswos_team_resource::{
    Face, KitColour, MatchFlags, PLAYERS_PER_TEAM, PlayerFlags, PlayerValues, ShirtType, Skill,
    TeamControls,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use openswos_common::json::{JsonByte, JsonWord, code, present};

/// Competition slots of a player's goal counters (upstream `goalStats[4]`): league,
/// domestic cup, league cup, continental cup.
pub const GOAL_STATS_LEN: usize = 4;

/// Hex digits of a team checksum: the first 8 bytes of a SHA-1.
pub const CHECKSUM_HEX_LEN: usize = 16;

/// A team file in JSON (`"type": "team"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamJsonFile {
    /// Always `"team"`.
    #[serde(rename = "type")]
    pub kind: FileKind,
    /// The team file's number: its first team's (`null` with no team).
    pub country: Option<u8>,
    /// The country's name. `None`: the key is absent (World 2026-27's pools and guests);
    /// `Some(None)`: `null` (a file with no team).
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub country_name: Option<Option<String>>,
    /// The number of teams.
    pub count: usize,
    /// The teams.
    pub teams: Vec<TeamEntry>,
}

/// The kind of a team JSON file. Only team files carry ids; the European-cup pools and the
/// transfer pool (`"tmd"`, `"pool"`) are other formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileKind {
    /// `"team"`.
    #[serde(rename = "team")]
    Team,
}

/// One team and its squad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamEntry {
    /// The team's id.
    pub uuid: Uuid,
    /// What a match plays the team by, hashed (docs/TEAM_IDS.md §2).
    pub checksum: TeamChecksum,
    /// The team record's header.
    pub team: TeamHeader,
    /// The squad.
    pub players: [PlayerEntry; PLAYERS_PER_TEAM],
}

/// A team checksum: 16 lower-case hex digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TeamChecksum(String);

/// A string that is not a team checksum.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("`{0}` is not a team checksum (16 lower-case hex digits)")]
pub struct InvalidChecksum(pub String);

impl TeamChecksum {
    /// The checksum's hex digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TeamChecksum {
    type Error = InvalidChecksum;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let valid = value.len() == CHECKSUM_HEX_LEN
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidChecksum(value))
        }
    }
}

impl From<TeamChecksum> for String {
    fn from(checksum: TeamChecksum) -> Self {
        checksum.0
    }
}

/// A team record's header, field for field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(missing_docs)] // the binary fields of `openswos_team_resource::Team`, under upstream's names
pub struct TeamHeader {
    pub country_number: u8,
    pub team_ordinal: u8,
    pub global_team_number: JsonWord,
    #[serde(with = "code")]
    pub team_controls: TeamControls,
    pub team_name: String,
    pub writes_zero_here: u8,
    #[serde(with = "code")]
    pub and_with_0xfe: MatchFlags,
    #[serde(with = "code")]
    pub tactics: Tactics,
    pub league: u8,
    #[serde(with = "code")]
    pub primary_shirt_type: ShirtType,
    #[serde(with = "code")]
    pub primary_stripes_color: KitColour,
    #[serde(with = "code")]
    pub primary_basic_color: KitColour,
    #[serde(with = "code")]
    pub primary_shorts_color: KitColour,
    #[serde(with = "code")]
    pub primary_socks_color: KitColour,
    #[serde(with = "code")]
    pub secondary_shirt_type: ShirtType,
    #[serde(with = "code")]
    pub secondary_stripes_color: KitColour,
    #[serde(with = "code")]
    pub secondary_basic_color: KitColour,
    #[serde(with = "code")]
    pub secondary_shorts_color: KitColour,
    #[serde(with = "code")]
    pub secondary_socks_color: KitColour,
    pub coach_name: String,
    #[serde(with = "code")]
    pub player_values_dirty: PlayerValues,
    pub player_numbers: [u8; PLAYERS_PER_TEAM],
}

/// A player record, field for field, its packed bytes split as upstream splits them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(missing_docs)] // the binary fields of `openswos_team_resource::Player`, under upstream's names
pub struct PlayerEntry {
    pub uuid: Uuid,
    pub nationality: JsonByte,
    pub cached_match_value: JsonByte,
    pub shirt_number: JsonByte,
    pub player_name: String,
    /// Bits 5-7 of the position-and-face byte.
    #[serde(with = "code")]
    pub position: Position,
    /// Bits 3-4 of the position-and-face byte.
    #[serde(with = "code")]
    pub face: Face,
    /// Bits 0-2 of the position-and-face byte.
    #[serde(with = "code")]
    pub flags: PlayerFlags,
    /// Bits 0-1 of the cards-and-injuries byte (the JSON's name).
    pub cards: u8,
    /// Bits 2-4 of the cards-and-injuries byte (the JSON's name).
    pub injury: u8,
    /// Bits 5-7 of the cards-and-injuries byte (the JSON's name; the game code reads its
    /// injuries from these bits).
    pub status_bucket: u8,
    /// The low nibble of the passing byte, like every other skill. Upstream's writer writes
    /// the whole byte; its high nibble is unused, 0 in every file, so the texts agree.
    #[serde(with = "code")]
    pub passing: Skill,
    #[serde(with = "code")]
    pub velocity: Skill,
    #[serde(with = "code")]
    pub heading: Skill,
    #[serde(with = "code")]
    pub tackling: Skill,
    #[serde(with = "code")]
    pub control: Skill,
    #[serde(with = "code")]
    pub speed: Skill,
    #[serde(with = "code")]
    pub finishing: Skill,
    pub price: JsonByte,
    pub goal_stats: [u8; GOAL_STATS_LEN],
    pub form: JsonByte,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_are_16_lower_case_hex_digits() {
        assert!(TeamChecksum::try_from("84a7cb97403a00a9".to_owned()).is_ok());
        assert!(TeamChecksum::try_from("84A7CB97403A00A9".to_owned()).is_err());
        assert!(TeamChecksum::try_from("84a7cb97".to_owned()).is_err());
        assert_eq!(
            TeamChecksum::try_from("84a7cb97403a00a9".to_owned())
                .map(|checksum| checksum.as_str().to_owned()),
            Ok("84a7cb97403a00a9".to_owned())
        );
    }

    #[test]
    fn the_country_name_may_be_absent_null_or_given() {
        let base = "\"type\": \"team\", \"country\": null, \"count\": 0, \"teams\": []";
        let absent: TeamJsonFile = serde_json::from_str(&format!("{{{base}}}")).expect("valid");
        assert_eq!(absent.country_name, None);
        let null: TeamJsonFile =
            serde_json::from_str(&format!("{{{base}, \"country_name\": null}}")).expect("valid");
        assert_eq!(null.country_name, Some(None));
        let named: TeamJsonFile =
            serde_json::from_str(&format!("{{{base}, \"country_name\": \"FIJI\"}}"))
                .expect("valid");
        assert_eq!(named.country_name, Some(Some("FIJI".to_owned())));
    }

    #[test]
    fn unknown_keys_and_kinds_are_rejected() {
        assert!(
            serde_json::from_str::<TeamJsonFile>(
                "{\"type\": \"tmd\", \"country\": null, \"count\": 0, \"teams\": []}"
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<TeamJsonFile>(
                "{\"type\": \"team\", \"country\": null, \"count\": 0, \"teams\": [], \"extra\": 1}"
            )
            .is_err()
        );
    }
}
