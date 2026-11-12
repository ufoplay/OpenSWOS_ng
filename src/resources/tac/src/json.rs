//! OpenSWOS's tactics as JSON (`.TAC`; upstream src/save_format_json.c
//! `write_tactics_buf`, `load_tactics_buf`): the tactics record ([`TeamTactics`]) member
//! by member, in the saves' JSON (the `save_json` crate). A member a line, none spaced.
//!
//! | member | value |
//! |---|---|
//! | `schema_version`, `file_kind` | 3, `tactics` |
//! | `name` | the name, up to its first NUL |
//! | `player_positions` | each outfield player's 35 position bytes |
//! | `pair_ids` | the 10 pair bytes |
//! | `ball_out_of_play_tactics` | the slot's byte |

use openswos_common::{ByteReader, ByteWriter, Exporter, FixedName, Importer, NameError};
use openswos_save_json_resource::{
    FileKind, SchemaVersion, UpstreamLayout, from_save_bytes, to_save_bytes,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::tactics::{
    BALL_AREA_COLUMNS, BALL_AREA_ROWS, OUTFIELD_PLAYERS, PairId, TACTICS_NAME_LEN,
    TACTICS_RECORD_LEN, TacticsError, TeamTactics,
};

/// The bytes of a player's positions: a byte a ball area (upstream `PlayerPositions`).
const PLAYER_POSITIONS_LEN: usize = BALL_AREA_COLUMNS * BALL_AREA_ROWS;

/// The writer's layout: a member a line, none spaced.
const LAYOUT: UpstreamLayout = UpstreamLayout {
    spaced_members: FileKind::Tactics.spaced_members(),
    inline: &[],
};

/// The file's members, as written.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TacText {
    schema_version: SchemaVersion,
    file_kind: String,
    name: String,
    player_positions: Vec<Vec<u8>>,
    pair_ids: Vec<u8>,
    ball_out_of_play_tactics: u8,
}

/// Why JSON tactics do not read.
#[derive(Debug, Error)]
pub enum JsonTacError {
    /// Not the saves' JSON, or a member missing, unknown or of another type.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Another file kind.
    #[error("file kind {0:?}: not tactics")]
    Kind(String),
    /// A name longer than its field, or holding a NUL.
    #[error("name: {0}")]
    Name(#[from] NameError),
    /// An array of another length than the record's.
    #[error("{field}: {len} values, the record holds {expected}")]
    Length {
        /// The member.
        field: &'static str,
        /// The values held.
        len: usize,
        /// The record's.
        expected: usize,
    },
    /// The record's checks: the ball-out-of-play slot is not one.
    #[error(transparent)]
    Tactics(#[from] TacticsError),
}

/// Fails when `len` values are not the record's `expected`.
fn check_len(field: &'static str, len: usize, expected: usize) -> Result<(), JsonTacError> {
    if len == expected {
        Ok(())
    } else {
        Err(JsonTacError::Length {
            field,
            len,
            expected,
        })
    }
}

impl TacText {
    /// The record the members give, as the loader fills it.
    fn tactics(&self) -> Result<TeamTactics, JsonTacError> {
        if self.file_kind != FileKind::Tactics.name() {
            return Err(JsonTacError::Kind(self.file_kind.clone()));
        }
        let name = FixedName::<TACTICS_NAME_LEN>::from_text(&self.name)?;
        let mut writer = ByteWriter::with_capacity(TACTICS_RECORD_LEN);
        writer.write_bytes(name.as_bytes());
        check_len(
            "player_positions",
            self.player_positions.len(),
            OUTFIELD_PLAYERS,
        )?;
        for positions in &self.player_positions {
            check_len("player_positions", positions.len(), PLAYER_POSITIONS_LEN)?;
            writer.write_bytes(positions);
        }
        check_len("pair_ids", self.pair_ids.len(), OUTFIELD_PLAYERS)?;
        writer.write_bytes(&self.pair_ids);
        writer.write_u8(self.ball_out_of_play_tactics);
        Ok(TeamTactics::read(&mut ByteReader::new(
            &writer.into_bytes(),
        ))?)
    }
}

impl From<&TeamTactics> for TacText {
    /// The members the writer writes. Faithful to the original: the name's bytes after its
    /// first NUL are not written.
    fn from(tactics: &TeamTactics) -> Self {
        let TeamTactics {
            name,
            players,
            pairs,
            ball_out_of_play,
        } = tactics;
        Self {
            schema_version: SchemaVersion,
            file_kind: FileKind::Tactics.name().to_owned(),
            name: name.text(),
            player_positions: players
                .iter()
                .map(|spots| spots.0.iter().flatten().copied().map(u8::from).collect())
                .collect(),
            pair_ids: pairs.map(PairId::byte).to_vec(),
            ball_out_of_play_tactics: u8::from(*ball_out_of_play),
        }
    }
}

/// The JSON tactics format: a tactic, as OpenSWOS's tactics editor saves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonTacFormat;

impl Importer for JsonTacFormat {
    type Model = TeamTactics;
    type Error = JsonTacError;

    fn import(bytes: &[u8]) -> Result<TeamTactics, JsonTacError> {
        from_save_bytes::<TacText>(bytes)?.tactics()
    }
}

impl Exporter for JsonTacFormat {
    type Model = TeamTactics;
    type Error = JsonTacError;

    fn export(model: &TeamTactics) -> Result<Vec<u8>, JsonTacError> {
        Ok(to_save_bytes(&TacText::from(model), &LAYOUT)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text as `write_tactics_buf` prints it: player `p`'s area `a` holds `p + a`, the
    /// pairs `0xFF` but two, the ball-out-of-play slot 3 (5-3-2).
    fn writer_text(name: &str) -> String {
        writer_text_of(name, OUTFIELD_PLAYERS)
    }

    /// [`writer_text`] with `players` position arrays.
    fn writer_text_of(name: &str, players: usize) -> String {
        let positions: Vec<String> = (0..players)
            .map(|player| {
                let bytes: Vec<String> = (0..PLAYER_POSITIONS_LEN)
                    .map(|area| (player + area).to_string())
                    .collect();
                format!("[{}]", bytes.join(","))
            })
            .collect();
        format!(
            concat!(
                "{{\n  \"schema_version\":3,\n  \"file_kind\":\"tactics\",\n  ",
                "\"name\":\"{name}\",\n  \"player_positions\":[{positions}],\n  ",
                "\"pair_ids\":[1,1,255,255,255,255,255,255,255,255],\n  ",
                "\"ball_out_of_play_tactics\":3\n}}\n"
            ),
            name = name,
            positions = positions.join(","),
        )
    }

    #[test]
    fn the_writers_text_round_trips() {
        let text = writer_text("MY \\\"442\\\"");
        let tactics = JsonTacFormat::import(text.as_bytes()).expect("reads");
        assert_eq!(tactics.name.text(), "MY \"442\"");
        let pairs: Vec<Option<u8>> = tactics.pairs.iter().map(|p| p.map(PairId::value)).collect();
        assert_eq!(pairs.get(..3), Some(&[Some(1), Some(1), None][..]));
        assert_eq!(
            JsonTacFormat::export(&tactics).expect("writes"),
            text.as_bytes()
        );
    }

    #[test]
    fn a_name_is_written_up_to_its_nul() {
        let mut tactics = JsonTacFormat::import(writer_text("442").as_bytes()).expect("reads");
        tactics.name = FixedName::from_bytes(*b"442\0JUNK!");
        let text = JsonTacFormat::export(&tactics).expect("writes");
        assert_eq!(text, writer_text("442").as_bytes());
    }

    #[test]
    fn what_is_not_a_record_does_not_read() {
        let text = writer_text("442");
        for (from, to, why) in [
            ("\"tactics\"", "\"career\"", "not tactics"),
            ("\"442\"", "\"0123456789\"", "has 10 characters"),
            ("[1,1,255", "[1,255", "pair_ids: 9 values"),
            ("[0,1,2,", "[1,2,", "player_positions: 34 values"),
            (
                "\"ball_out_of_play_tactics\":3",
                "\"ball_out_of_play_tactics\":99",
                "ball out of play",
            ),
            (
                "\"ball_out_of_play_tactics\":3",
                "\"ball_out_of_play_tactics\":3,\"x\":0",
                "unknown field",
            ),
        ] {
            let changed = text.replacen(from, to, 1);
            assert_ne!(changed, text, "{from}");
            let error = JsonTacFormat::import(changed.as_bytes()).expect_err(to);
            assert!(error.to_string().contains(why), "{to}: {error}");
        }
        let nine = writer_text_of("442", OUTFIELD_PLAYERS - 1);
        let error = JsonTacFormat::import(nine.as_bytes()).expect_err("nine players");
        assert!(
            error.to_string().contains("player_positions: 9 values"),
            "{error}"
        );
    }
}
