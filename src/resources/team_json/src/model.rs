//! The JSON's shape, field for field, in upstream's order (upstream
//! `tools/swosdata/lib/team_json.c`, `emit_team`/`emit_player`; a world's additions,
//! `scripts/world_import.py` `team`/`player`). Serialising it in that order reproduces
//! upstream's files byte for byte; [`crate::convert`] turns it into the typed team model,
//! checking every range. Codes (position, kit colours, tactics...) and skills are already the
//! typed model's types, written as their numbers.

use openswos_common::{Position, Tactics};
use openswos_team_resource::{
    Face, KitColour, LookBits, MatchFlags, PLAYERS_PER_TEAM, PlayerFlags, PlayerValues, ShirtType,
    Skill, TeamControls,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;
use uuid::Uuid;

use openswos_common::json::{JsonByte, JsonWord, code, present};

/// Competition slots of a player's goal counters (upstream `goalStats[4]`): league,
/// domestic cup, league cup, continental cup.
pub const GOAL_STATS_LEN: usize = 4;

/// Hex digits of a team checksum: the first 8 bytes of a SHA-1.
pub const CHECKSUM_HEX_LEN: usize = 16;

/// The first year of birth the game takes (upstream `DataJson_LoadTeamBorn`: `y > 1800`).
pub const FIRST_KNOWN_BIRTH_YEAR: u16 = 1801;

/// The last year of birth the game takes (upstream `DataJson_LoadTeamBorn`: `y < 2200`).
pub const LAST_KNOWN_BIRTH_YEAR: u16 = 2199;

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
    /// A world club's players from place 17 on (the SWOS 96/97 Mod's long benches' 17..30),
    /// the same fields and ids as `players`, numbered after them; not in the checksum (a match
    /// plays the 16). `None`: the key is absent (every club without them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_players: Option<Vec<PlayerEntry>>,
}

/// A player's year of birth, as written (a world's B teams' players: the SWOS 96/97 Mod's
/// `BAGE.nnn`, for the call-ups' under-23 rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BirthYear(pub u16);

impl BirthYear {
    /// The year, when the game takes it ([`FIRST_KNOWN_BIRTH_YEAR`] to
    /// [`LAST_KNOWN_BIRTH_YEAR`]); `None`: unknown to it (upstream: 0).
    #[must_use]
    pub fn known(self) -> Option<u16> {
        (FIRST_KNOWN_BIRTH_YEAR..=LAST_KNOWN_BIRTH_YEAR)
            .contains(&self.0)
            .then_some(self.0)
    }
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

/// Where a player's `face` (and `looks`) come among its keys: the text's, not the record's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PlayerKeyOrder {
    /// The record's: `player_name`, `looks`, `position`, `face` (OpenSWOS Setup's
    /// `emit_player`, `scripts/world_import.py`).
    #[default]
    Record,
    /// `player_name`, `face`, `looks`, `position`: World 2026-27's new national teams'
    /// players (upstream `scripts/worldgen/nations.py`, which takes the face from the player's
    /// club first).
    FaceFirst,
}

/// A player record, field for field, its packed bytes split as upstream splits them. Written
/// in its [`PlayerKeyOrder`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(missing_docs)] // the binary fields of `openswos_team_resource::Player`, under upstream's names
pub struct PlayerEntry {
    /// The order its keys are written in: the file's ([`crate::TeamJsonFormat`] reads it).
    #[serde(skip)]
    pub key_order: PlayerKeyOrder,
    pub uuid: Uuid,
    pub nationality: JsonByte,
    pub cached_match_value: JsonByte,
    pub shirt_number: JsonByte,
    pub player_name: String,
    /// The SWOS 96/97 Mod's look (the record's byte 25, `openswos_team_resource::looks`):
    /// written only when the name leaves room for it and it is a look. `None`: absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub looks: Option<LookBits>,
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
    /// The year of birth (a world's B teams' players); `None`: absent.
    #[serde(default)]
    pub born: Option<BirthYear>,
}

/// A code written as its byte (as `openswos_common::json::code` writes one).
struct AsByte<T>(T);

impl<T: Copy + Into<u8>> Serialize for AsByte<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.0.into())
    }
}

/// The keys every player has.
const PLAYER_KEYS: usize = 21;

impl Serialize for PlayerEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self {
            key_order,
            uuid,
            nationality,
            cached_match_value,
            shirt_number,
            player_name,
            looks,
            position,
            face,
            flags,
            cards,
            injury,
            status_bucket,
            passing,
            velocity,
            heading,
            tackling,
            control,
            speed,
            finishing,
            price,
            goal_stats,
            form,
            born,
        } = self;
        let keys = PLAYER_KEYS + usize::from(looks.is_some()) + usize::from(born.is_some());
        let mut player = serializer.serialize_struct("PlayerEntry", keys)?;
        player.serialize_field("uuid", uuid)?;
        player.serialize_field("nationality", nationality)?;
        player.serialize_field("cached_match_value", cached_match_value)?;
        player.serialize_field("shirt_number", shirt_number)?;
        player.serialize_field("player_name", player_name)?;
        match key_order {
            PlayerKeyOrder::Record => {
                if let Some(looks) = looks {
                    player.serialize_field("looks", looks)?;
                }
                player.serialize_field("position", &AsByte(*position))?;
                player.serialize_field("face", &AsByte(*face))?;
            }
            PlayerKeyOrder::FaceFirst => {
                player.serialize_field("face", &AsByte(*face))?;
                if let Some(looks) = looks {
                    player.serialize_field("looks", looks)?;
                }
                player.serialize_field("position", &AsByte(*position))?;
            }
        }
        player.serialize_field("flags", &AsByte(*flags))?;
        player.serialize_field("cards", cards)?;
        player.serialize_field("injury", injury)?;
        player.serialize_field("status_bucket", status_bucket)?;
        player.serialize_field("passing", &AsByte(*passing))?;
        player.serialize_field("velocity", &AsByte(*velocity))?;
        player.serialize_field("heading", &AsByte(*heading))?;
        player.serialize_field("tackling", &AsByte(*tackling))?;
        player.serialize_field("control", &AsByte(*control))?;
        player.serialize_field("speed", &AsByte(*speed))?;
        player.serialize_field("finishing", &AsByte(*finishing))?;
        player.serialize_field("price", price)?;
        player.serialize_field("goal_stats", goal_stats)?;
        player.serialize_field("form", form)?;
        if let Some(born) = born {
            player.serialize_field("born", born)?;
        }
        player.end()
    }
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
    fn a_year_of_birth_is_known_within_the_games_range() {
        assert_eq!(BirthYear(1999).known(), Some(1999));
        assert_eq!(BirthYear(FIRST_KNOWN_BIRTH_YEAR).known(), Some(1801));
        assert_eq!(BirthYear(LAST_KNOWN_BIRTH_YEAR).known(), Some(2199));
        assert_eq!(BirthYear(1800).known(), None);
        assert_eq!(BirthYear(2200).known(), None);
        assert_eq!(BirthYear(0).known(), None);
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
