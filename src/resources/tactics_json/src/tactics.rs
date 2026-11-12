//! A built-in tactic as upstream's tactics JSON holds it: a straight serialisation of the
//! 370-byte record (scripts/export_tactics.py writes it, src/tactics_load_json.c and
//! scripts/emit_tactics.py read it).
//!
//! ```text
//! {
//!  "id": "442",
//!  "label": "4-4-2",
//!  "name_bytes": ["0x34", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"],
//!  "players": [
//!   [
//!    ["0x10", "0x21", "0x31", "0x31", "0x42", "0x13", "0x22"],
//!    … 5 groups of 7 bytes …
//!   ],
//!   … 10 players …
//!  ],
//!  "pair_ids": [0, -1, -1, 0, 1, -1, -1, 1, -1, -1],
//!  "ball_out_of_play": 0
//! }
//! ```

use std::convert::Infallible;

use openswos_common::json::JsonByte;
use openswos_common::{ByteReader, ByteWriter, Exporter, Importer};
use openswos_tac_resource::{
    OUTFIELD_PLAYERS, TACTICS_NAME_LEN, TACTICS_RECORD_LEN, TacticsError, TeamTactics,
};
use serde::Deserialize;
use thiserror::Error;

/// Groups of a player's 35 position bytes in the JSON: the C struct's
/// `positions[5][7]` (upstream `PlayerPositions`, include/game_types.h), which the JSON
/// keeps. The bytes mean 7 bands of 5 ball areas (`PlayerSpots`): the grouping is the
/// declaration's, not the pitch's.
pub const POSITION_GROUPS: usize = 5;

/// Bytes in each group of a player's position bytes.
pub const POSITION_GROUP_LEN: usize = 7;

/// Bytes of a player's positions: one per ball area.
const POSITION_BYTES: usize = POSITION_GROUPS * POSITION_GROUP_LEN;

/// One level of indentation: upstream writes `json.dumps(indent=1)`.
const INDENT: &str = " ";

/// The prefix of a byte written in hex, as upstream's exporter writes it (`"0x%02X"`).
const HEX_PREFIX: &str = "0x";

/// The prefix in capitals, which the loaders also read (`strtol`, Python's `int(x, 0)`).
const HEX_PREFIX_CAPITAL: &str = "0X";

/// The most hex digits a byte takes.
const HEX_DIGITS: usize = 2;

/// The radix of hex.
const HEX_RADIX: u32 = 16;

/// The signs a decimal byte may start with.
const SIGNS: [char; 2] = ['+', '-'];

/// A built-in tactic's JSON file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TacticsJson {
    /// The preset's id (`442`). The game finds the file by its name, not by this.
    pub id: String,
    /// The preset's label (`4-4-2`), which the game does not read.
    pub label: String,
    /// The tactic.
    pub tactics: TeamTactics,
}

/// Why a tactics JSON file does not decode.
#[derive(Debug, Error)]
pub enum TacticsJsonError {
    /// Not valid JSON, or not a tactics file.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Its bytes are not a tactic.
    #[error(transparent)]
    Record(#[from] TacticsError),
}

/// A record byte as the JSON spells it. Upstream's exporter writes `"0x34"`; its loaders
/// (`strtol(…, 0)`, Python's `int(…, 0)`) also read `"0X34"`, `"0x4"`, `"52"`, `"-1"` or a
/// number. Every spelling is written back as `"0x34"`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "ByteSpelling")]
struct SpelledByte(u8);

/// A byte's spelling, as read.
#[derive(Deserialize)]
#[serde(untagged)]
enum ByteSpelling {
    /// `"0x34"`, `"52"`.
    Text(String),
    /// `52`, or `-1` for `0xFF`.
    Number(JsonByte),
}

/// A string that is not a byte.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("`{0}` is not a byte (0xHH, or a number)")]
struct NotAByte(String);

impl TryFrom<ByteSpelling> for SpelledByte {
    type Error = NotAByte;

    fn try_from(spelling: ByteSpelling) -> Result<Self, Self::Error> {
        match spelling {
            ByteSpelling::Number(number) => Ok(Self(number.byte())),
            ByteSpelling::Text(text) => spelled_byte(&text).map(Self).ok_or(NotAByte(text)),
        }
    }
}

/// The byte `text` spells, as both loaders read it.
fn spelled_byte(text: &str) -> Option<u8> {
    if let Some(digits) = text
        .strip_prefix(HEX_PREFIX)
        .or_else(|| text.strip_prefix(HEX_PREFIX_CAPITAL))
    {
        let hex = (1..=HEX_DIGITS).contains(&digits.len())
            && digits.chars().all(|digit| digit.is_ascii_hexdigit());
        return hex
            .then(|| u8::from_str_radix(digits, HEX_RADIX).ok())
            .flatten();
    }
    // A leading zero makes octal to strtol and an error to Python: not read.
    let digits = text.strip_prefix(SIGNS).unwrap_or(text);
    let decimal = digits.chars().all(|digit| digit.is_ascii_digit())
        && (digits.len() == 1 || !digits.is_empty() && !digits.starts_with('0'));
    decimal
        .then(|| text.parse::<i16>().ok())
        .flatten()
        .and_then(|number| JsonByte::try_from(number).ok())
        .map(JsonByte::byte)
}

/// The file, as read.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Text {
    id: String,
    label: String,
    name_bytes: [SpelledByte; TACTICS_NAME_LEN],
    players: [[[SpelledByte; POSITION_GROUP_LEN]; POSITION_GROUPS]; OUTFIELD_PLAYERS],
    /// Upstream writes them signed (`-1` for `0xFF`); either sign reads.
    pair_ids: [JsonByte; OUTFIELD_PLAYERS],
    ball_out_of_play: u8,
}

impl TryFrom<Text> for TacticsJson {
    type Error = TacticsError;

    fn try_from(text: Text) -> Result<Self, Self::Error> {
        let Text {
            id,
            label,
            name_bytes,
            players,
            pair_ids,
            ball_out_of_play,
        } = text;
        let mut writer = ByteWriter::with_capacity(TACTICS_RECORD_LEN);
        writer.write_bytes(&name_bytes.map(|byte| byte.0));
        for group in players.iter().flatten() {
            writer.write_bytes(&group.map(|byte| byte.0));
        }
        writer.write_bytes(&pair_ids.map(JsonByte::byte));
        writer.write_u8(ball_out_of_play);
        let bytes = writer.into_bytes();
        let mut reader = ByteReader::new(&bytes);
        let tactics = TeamTactics::read(&mut reader)?;
        reader.finish()?;
        Ok(Self { id, label, tactics })
    }
}

/// A record's fields, as bytes.
struct Fields {
    name: [u8; TACTICS_NAME_LEN],
    players: [[u8; POSITION_BYTES]; OUTFIELD_PLAYERS],
    pairs: [u8; OUTFIELD_PLAYERS],
    ball_out_of_play: u8,
}

impl Fields {
    fn of(tactics: &TeamTactics) -> Self {
        const WHOLE: &str = "a tactics record holds every field";
        let mut writer = ByteWriter::with_capacity(TACTICS_RECORD_LEN);
        tactics.write(&mut writer);
        let bytes = writer.into_bytes();
        let mut reader = ByteReader::new(&bytes);
        let name = reader.read_array().expect(WHOLE);
        let mut players = [[0; POSITION_BYTES]; OUTFIELD_PLAYERS];
        for player in &mut players {
            *player = reader.read_array().expect(WHOLE);
        }
        Self {
            name,
            players,
            pairs: reader.read_array().expect(WHOLE),
            ball_out_of_play: reader.read_u8().expect(WHOLE),
        }
    }
}

/// `bytes` as one line: `["0x34", "0x00"]`.
fn hex_line(bytes: &[u8]) -> String {
    let items: Vec<String> = bytes
        .iter()
        .map(|byte| format!("\"{HEX_PREFIX}{byte:02X}\""))
        .collect();
    format!("[{}]", items.join(", "))
}

/// `items` as an array of one line each, its bracket opening on a line at `depth`.
fn block(items: &[String], depth: usize) -> String {
    let inner = INDENT.repeat(depth + 1);
    let lines: Vec<String> = items.iter().map(|item| format!("{inner}{item}")).collect();
    format!("[\n{}\n{}]", lines.join(",\n"), INDENT.repeat(depth))
}

/// `text` as a JSON string, as Python's `json.dumps(ensure_ascii=False)` writes it.
fn string(text: &str) -> String {
    serde_json::to_string(text).expect("a string serialises")
}

/// The file's text: upstream's exporter's layout (`json.dumps(indent=1,
/// ensure_ascii=False)`, the innermost arrays on one line, a newline at the end).
///
/// Upstream joins the innermost arrays with a regular expression over the whole text,
/// which would also rewrite an id or label holding `[a,b]`. None does; this writer only
/// touches the arrays.
fn to_text(model: &TacticsJson) -> String {
    let member = 1;
    let Fields {
        name,
        players,
        pairs,
        ball_out_of_play,
    } = Fields::of(&model.tactics);
    let players: Vec<String> = players
        .iter()
        .map(|player| {
            let (groups, _) = player.as_chunks::<POSITION_GROUP_LEN>();
            let groups: Vec<String> = groups.iter().map(|group| hex_line(group)).collect();
            block(&groups, member + 1)
        })
        .collect();
    let pairs: Vec<String> = pairs
        .iter()
        .map(|pair| pair.cast_signed().to_string())
        .collect();
    let members = [
        ("id", string(&model.id)),
        ("label", string(&model.label)),
        ("name_bytes", hex_line(&name)),
        ("players", block(&players, member)),
        ("pair_ids", format!("[{}]", pairs.join(", "))),
        ("ball_out_of_play", ball_out_of_play.to_string()),
    ];
    let lines: Vec<String> = members
        .iter()
        .map(|(key, value)| format!("{}\"{key}\": {value}", INDENT.repeat(member)))
        .collect();
    format!("{{\n{}\n}}\n", lines.join(",\n"))
}

/// The tactics JSON format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticsJsonFormat;

impl Importer for TacticsJsonFormat {
    type Model = TacticsJson;
    type Error = TacticsJsonError;

    fn import(bytes: &[u8]) -> Result<TacticsJson, TacticsJsonError> {
        let text: Text = serde_json::from_slice(bytes)?;
        Ok(TacticsJson::try_from(text)?)
    }
}

impl Exporter for TacticsJsonFormat {
    type Model = TacticsJson;
    type Error = Infallible;

    fn export(model: &TacticsJson) -> Result<Vec<u8>, Infallible> {
        Ok(to_text(model).into_bytes())
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size records
mod tests {
    use super::*;
    use openswos_common::{InvalidTactics, Tactics};
    use proptest::prelude::*;

    /// A file of `442.json`'s shape: every position byte `0x12`, the given pairs and
    /// ball-out-of-play slot.
    fn file(name: &str, pairs: &str, ball: &str) -> String {
        let group = format!("[{}]", ["\"0x12\""; POSITION_GROUP_LEN].join(", "));
        let player = format!("[{}]", vec![group; POSITION_GROUPS].join(", "));
        let players = vec![player; OUTFIELD_PLAYERS].join(", ");
        format!(
            r#"{{"id": "442", "label": "4-4-2", "name_bytes": {name}, "players": [{players}], "pair_ids": {pairs}, "ball_out_of_play": {ball}}}"#
        )
    }

    const NAME: &str =
        r#"["0x34", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#;
    const PAIRS: &str = "[0, -1, -1, 0, 1, -1, -1, 1, -1, -1]";

    fn read(text: &str) -> Result<TacticsJson, TacticsJsonError> {
        TacticsJsonFormat::import(text.as_bytes())
    }

    #[test]
    fn reads_the_record_upstream_writes() {
        let tactics = read(&file(NAME, PAIRS, "0")).expect("a tactic").tactics;
        assert_eq!(tactics.name.text(), "442");
        assert_eq!(tactics.name.as_bytes()[5], b'A');
        assert_eq!(tactics.ball_out_of_play, Tactics::FourFourTwo);
        assert_eq!(tactics.pairs.iter().flatten().count(), 4);
        assert_eq!(u8::from(tactics.players[9].0[6][4]), 0x12);
    }

    #[test]
    fn other_spellings_of_a_byte_read_the_same() {
        let name = r#"["0X34", 52, "50", 0, "0x0", "+65", "-0", "0", "0x00"]"#;
        let pairs = "[0, 255, -1, 0, 1, 255, -1, 1, -1, -1]";
        assert_eq!(
            read(&file(name, pairs, "0")).expect("a tactic"),
            read(&file(NAME, PAIRS, "0")).expect("a tactic")
        );
    }

    #[test]
    fn what_is_not_a_byte_does_not_read() {
        for name in [
            r#"["0x134", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"["034", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"["256", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"["-129", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"["", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"["-", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"["0x", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"["0x+1", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"[256, "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00", "0x00"]"#,
            r#"["0x34", "0x34", "0x32", "0x00", "0x00", "0x41", "0x00", "0x00"]"#,
        ] {
            assert!(
                matches!(
                    read(&file(name, PAIRS, "0")),
                    Err(TacticsJsonError::Json(_))
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn the_ball_out_of_play_slot_must_be_a_slot() {
        assert!(matches!(
            read(&file(NAME, PAIRS, "18")),
            Err(TacticsJsonError::Record(TacticsError::BallOutOfPlay(
                InvalidTactics(18)
            )))
        ));
        assert!(matches!(
            read(&file(NAME, PAIRS, "-1")),
            Err(TacticsJsonError::Json(_))
        ));
    }

    #[test]
    fn unknown_keys_do_not_read() {
        let text = file(NAME, PAIRS, "0").replace("\"id\"", "\"extra\": 1, \"id\"");
        assert!(matches!(read(&text), Err(TacticsJsonError::Json(_))));
    }

    #[test]
    fn writes_upstreams_layout() {
        let model = read(&file(NAME, PAIRS, "5")).expect("a tactic");
        let text = String::from_utf8(
            TacticsJsonFormat::export(&model).unwrap_or_else(|never| match never {}),
        )
        .expect("UTF-8");
        let group = format!("[{}]", ["\"0x12\""; POSITION_GROUP_LEN].join(", "));
        let player = format!(
            "  [\n{}\n  ]",
            vec![format!("   {group}"); POSITION_GROUPS].join(",\n")
        );
        let expected = format!(
            "{{\n \"id\": \"442\",\n \"label\": \"4-4-2\",\n \"name_bytes\": {NAME},\n \"players\": [\n{}\n ],\n \"pair_ids\": {PAIRS},\n \"ball_out_of_play\": 5\n}}\n",
            vec![player; OUTFIELD_PLAYERS].join(",\n")
        );
        assert_eq!(text, expected);
    }

    #[test]
    fn labels_beyond_ascii_are_written_as_they_are() {
        let mut model = read(&file(NAME, PAIRS, "0")).expect("a tactic");
        model.label = "4-4-2 \"Düsseldorf\"\n".to_owned();
        let bytes = TacticsJsonFormat::export(&model).unwrap_or_else(|never| match never {});
        let text = String::from_utf8(bytes).expect("UTF-8");
        assert!(
            text.contains(r#" "label": "4-4-2 \"Düsseldorf\"\n","#),
            "{text}"
        );
        assert_eq!(read(&text).expect("reads back"), model);
    }

    fn any_tactics() -> impl Strategy<Value = TeamTactics> {
        (
            proptest::collection::vec(any::<u8>(), TACTICS_RECORD_LEN - 1),
            0..u8::try_from(Tactics::ALL.len()).expect("18 slots"),
        )
            .prop_map(|(mut bytes, ball)| {
                bytes.push(ball);
                TeamTactics::read(&mut ByteReader::new(&bytes)).expect("a tactic")
            })
    }

    proptest! {
        #[test]
        fn every_tactic_round_trips(tactics in any_tactics(), id in ".*", label in ".*") {
            let model = TacticsJson { id, label, tactics };
            let bytes = TacticsJsonFormat::export(&model).unwrap_or_else(|never| match never {});
            prop_assert_eq!(read(std::str::from_utf8(&bytes).expect("UTF-8")).expect("reads back"), model);
        }
    }
}
