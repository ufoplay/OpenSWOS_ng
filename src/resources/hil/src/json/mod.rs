//! OpenSWOS's highlights (`.HIL`) as JSON (upstream src/save_format_json.c
//! `SaveJson_WriteHighlights`, `SaveJson_LoadHighlights`): the game's highlights buffer
//! member by member, in the saves' JSON. The buffer (a 2254-byte big-endian header, then a
//! 19000-byte slot a goal) is the Amiga's layout, which is not read here (D9): the file is
//! the model.
//!
//! The writer puts a member a line, none spaced, and `num_goals` with the five numbers
//! after it on one line.

mod scene;
mod team;

use openswos_common::{Exporter, FixedName, Importer};
use openswos_save_json_resource::{
    FileKind, SchemaVersion, UpstreamLayout, from_save_bytes, to_save_bytes,
};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

use crate::team_game::TeamGame;

pub use scene::{
    CAMERA_X_MAX, JsonFrame, JsonScene, JsonSceneError, JsonSprite, SCENE_LEN,
    SPRITE_COORDINATE_MAX, SPRITE_INDEX_MAX,
};

/// Bytes of the competition's name, and of its round (upstream `HilFileHeader.game_name`).
pub const JSON_GAME_NAME_LEN: usize = 100;

/// The members the writer puts on the line of the member before: the numbers after
/// `num_goals`.
const INLINE_MEMBERS: &[&str] = &[
    "team_1_goals",
    "team_2_goals",
    "pitch_number",
    "max_substitutes",
    "replay_fps",
];

/// A name field as the writer writes it: its bytes up to the first NUL, a character each
/// (upstream `emit_str_at`); read back NUL-padded (`load_str_at`).
mod name {
    use openswos_common::FixedName;
    use serde::Serializer;
    use serde::de::{self, Deserialize, Deserializer};

    /// Writes the name up to its first NUL. Faithful to the original: the bytes after it
    /// are not written.
    pub(super) fn serialize<const N: usize, S: Serializer>(
        name: &FixedName<N>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&name.text())
    }

    /// Reads a name, NUL-padded; longer than its field or holding a NUL, it is no name.
    pub(super) fn deserialize<'de, const N: usize, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<FixedName<N>, D::Error> {
        let text = String::deserialize(deserializer)?;
        FixedName::from_text(&text).map_err(de::Error::custom)
    }
}

/// The file kind: `highlights`, the only one a highlights file has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct HighlightsKind;

impl Serialize for HighlightsKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(FileKind::Highlights.name())
    }
}

impl<'de> Deserialize<'de> for HighlightsKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let kind = String::deserialize(deserializer)?;
        if kind == FileKind::Highlights.name() {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "file kind {kind:?}: not highlights"
            )))
        }
    }
}

/// OpenSWOS's highlights: the header's values, then a scene a goal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonHilFile {
    /// The schema version.
    pub schema_version: SchemaVersion,
    /// The file kind.
    pub file_kind: HighlightsKind,
    /// The header's first long word, as the buffer held it.
    pub signature: u32,
    /// The header's second long word, as the buffer held it.
    pub max_size: u32,
    /// The team at the top.
    #[serde(with = "team")]
    pub top_team: TeamGame,
    /// The team at the bottom.
    #[serde(with = "team")]
    pub bottom_team: TeamGame,
    /// The competition's name.
    #[serde(with = "name")]
    pub game_name: FixedName<JSON_GAME_NAME_LEN>,
    /// The competition's round.
    #[serde(with = "name")]
    pub game_round: FixedName<JSON_GAME_NAME_LEN>,
    /// The goals, a scene each: `scenes`' length.
    pub num_goals: u16,
    /// The first team's final goals.
    pub team_1_goals: u16,
    /// The second team's final goals.
    pub team_2_goals: u16,
    /// The pitch.
    pub pitch_number: u16,
    /// The most substitutes.
    pub max_substitutes: u16,
    /// The rate the match was played at, frames a second (70 DOS, 50 Amiga); absent in
    /// files written before it was kept, which upstream plays at 50.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_fps: Option<u32>,
    /// The scenes, a goal each.
    pub scenes: Vec<JsonScene>,
}

impl JsonHilFile {
    /// The writer's layout: a member a line, none spaced, the numbers after `num_goals` on
    /// its line.
    pub const LAYOUT: UpstreamLayout = UpstreamLayout {
        spaced_members: FileKind::Highlights.spaced_members(),
        inline: INLINE_MEMBERS,
    };

    /// Fails when the goals' count disagrees with the scenes, or a scene is not one a slot
    /// holds.
    fn check(&self) -> Result<(), JsonHilError> {
        if usize::from(self.num_goals) != self.scenes.len() {
            return Err(JsonHilError::Scenes {
                stated: self.num_goals,
                scenes: self.scenes.len(),
            });
        }
        self.scenes
            .iter()
            .enumerate()
            .try_for_each(|(index, scene)| {
                scene.check().map_err(|source| JsonHilError::Scene {
                    scene: index,
                    source,
                })
            })
    }
}

/// Why JSON highlights do not read or write.
#[derive(Debug, Error)]
pub enum JsonHilError {
    /// Not the saves' JSON, or not highlights.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// `num_goals` disagrees with `scenes`.
    #[error("num_goals says {stated}, the file holds {scenes} scenes")]
    Scenes {
        /// The count stated.
        stated: u16,
        /// The scenes held.
        scenes: usize,
    },
    /// A scene is not one a slot holds.
    #[error("scene {scene}: {source}")]
    Scene {
        /// The scene.
        scene: usize,
        /// Why.
        source: JsonSceneError,
    },
}

/// The JSON highlights format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonHilFormat;

impl Importer for JsonHilFormat {
    type Model = JsonHilFile;
    type Error = JsonHilError;

    fn import(bytes: &[u8]) -> Result<JsonHilFile, JsonHilError> {
        let file: JsonHilFile = from_save_bytes(bytes)?;
        file.check()?;
        Ok(file)
    }
}

impl Exporter for JsonHilFormat {
    type Model = JsonHilFile;
    type Error = JsonHilError;

    fn export(model: &JsonHilFile) -> Result<Vec<u8>, JsonHilError> {
        model.check()?;
        Ok(to_save_bytes(model, &JsonHilFile::LAYOUT)?)
    }
}

#[cfg(test)]
mod tests;
