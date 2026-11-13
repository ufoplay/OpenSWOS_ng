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
mod tests {
    //! Texts printed as upstream's writer prints them (`SaveJson_WriteHighlights`,
    //! `emit_team_game`, `emit_scene`) read and write back byte for byte.

    use super::*;
    use crate::team_game::{TEAM_GAME_NAME_LEN, TEAM_GAME_PLAYERS};

    /// The header's signature in the texts.
    const SIGNATURE: u32 = 0x1234_5678;

    /// The buffer's size in the texts: the header (2254 bytes) and a scene.
    const MAX_SIZE: u32 = 21_254;

    /// Where the end word of the texts' scene leaves off: two leading sprites (8 bytes), a
    /// frame's three words (12), its sprite (4), the end word (4).
    const EOS_OFFSET: u32 = 28;

    /// A byte of the game's character set that is not ASCII (`Á`, Latin-1).
    const LATIN_1_BYTE: u8 = 0xC1;

    /// The placeholder replaced with [`LATIN_1_BYTE`] in the texts.
    const PLACEHOLDER: u8 = b'~';

    /// A kit word written unsigned: `-1` in the record.
    const UNSIGNED_WORD: u16 = u16::MAX;

    /// A player's object, its bytes signed (`field_7` is -1).
    fn player_text(index: usize) -> String {
        format!(
            concat!(
                r#"{{"skip":0,"player_index":{index},"goals_scored":0,"shirt_number":{shirt},"#,
                r#""position":2,"face":1,"is_injured":0,"field_7":-1,"field_8":0,"previous_cards":0,"#,
                r#""num_yellow_cards":0,"field_b":0,"short_name":"P{index}","passing":3,"#,
                r#""shooting":4,"heading":5,"tackling":6,"ball_control":7,"speed":-8,"finishing":9,"#,
                r#""goalie_skill":0,"injuries_bits":0,"half_played":1,"face_2":1,"#,
                r#""full_name":"PLAYER NUMBER {index}"}}"#
            ),
            index = index,
            shirt = index + 1,
        )
    }

    /// A team's member, as `emit_team_game` prints it.
    fn team_text(key: &str, name: &str) -> String {
        let players: Vec<String> = (0..TEAM_GAME_PLAYERS).map(player_text).collect();
        format!(
            concat!(
                r#""{key}":{{"kit":{{"pr_shirt_type":1,"pr_shirt_col":{unsigned},"#,
                r#""pr_stripes_col":3,"pr_shorts_col":4,"pr_socks_col":5,"sec_shirt_type":0,"#,
                r#""sec_shirt_col":7,"sec_stripes":8,"sec_shorts":9,"sec_socks":10}},"#,
                r#""marked_player":65535,"team_name":"{name}","reserved1":0,"#,
                r#""num_own_goals":200,"reserved3":255,"players":[{players}]}}"#
            ),
            key = key,
            unsigned = UNSIGNED_WORD,
            name = name,
            players = players.join(","),
        )
    }

    /// A scene as `emit_scene` prints it: sprites before the first marker (a made-up frame),
    /// a frame of one sprite, the end word, the slot's bytes after it.
    fn scene_text() -> String {
        let trailing: Vec<String> = (EOS_OFFSET..SCENE_LEN)
            .map(|offset| (offset % 251).to_string())
            .collect();
        format!(
            concat!(
                r#"{{"frames":[{{"cam_x":0,"cam_y":0,"team_1_goals":0,"team_2_goals":0,"#,
                r#""anim_patterns_state":0,"_synthetic_no_frame_marker":true,"sprites":["#,
                r#"{{"index":4095,"x":1023,"y":0}},{{"index":1,"x":2,"y":3}}]}},"#,
                r#"{{"cam_x":32767,"cam_y":65534,"team_1_goals":1,"team_2_goals":0,"#,
                r#""anim_patterns_state":7,"sprites":[{{"index":300,"x":160,"y":100}}]}}],"#,
                r#""eos_offset":{eos},"trailing":[{trailing}]}}"#
            ),
            eos = EOS_OFFSET,
            trailing = trailing.join(","),
        )
    }

    /// A file as `SaveJson_WriteHighlights` prints it; `replay_fps` only when given (files
    /// before it have none).
    fn writer_text(replay_fps: Option<u32>) -> Vec<u8> {
        let fps = replay_fps
            .map(|fps| format!(r#","replay_fps":{fps}"#))
            .unwrap_or_default();
        let text = format!(
            concat!(
                "{{\n  \"schema_version\":3,\n  \"file_kind\":\"highlights\",\n  ",
                "\"signature\":{signature},\n  \"max_size\":{max_size},\n  ",
                "{top},\n  {bottom},\n  ",
                "\"game_name\":\"CUP \\\"{placeholder}\\\" \\\\ 1\",\n  \"game_round\":\"FINAL\",\n  ",
                "\"num_goals\":1,\"team_1_goals\":1,\"team_2_goals\":0,\"pitch_number\":3,",
                "\"max_substitutes\":2{fps},\n  ",
                "\"scenes\":[{scene}]\n}}\n"
            ),
            signature = SIGNATURE,
            max_size = MAX_SIZE,
            top = team_text("top_team", "M~LAGA"),
            bottom = team_text("bottom_team", "SEVENTEEN BYTES!!"),
            placeholder = char::from(PLACEHOLDER),
            fps = fps,
            scene = scene_text(),
        );
        text.into_bytes()
            .into_iter()
            .map(|byte| {
                if byte == PLACEHOLDER {
                    LATIN_1_BYTE
                } else {
                    byte
                }
            })
            .collect()
    }

    /// The writer's text with the first `from` replaced by `to`.
    fn replaced(from: &str, to: &str) -> Vec<u8> {
        let text = writer_text(Some(70));
        let at = text
            .windows(from.len())
            .position(|window| window == from.as_bytes())
            .expect("in the text");
        let (head, tail) = text.split_at(at);
        let tail = tail.get(from.len()..).expect("after it");
        [head, to.as_bytes(), tail].concat()
    }

    fn read(bytes: &[u8]) -> JsonHilFile {
        let file = JsonHilFormat::import(bytes).expect("reads");
        assert_eq!(JsonHilFormat::export(&file).expect("writes"), bytes);
        file
    }

    #[test]
    fn the_writers_text_round_trips() {
        let file = read(&writer_text(Some(70)));
        assert_eq!(
            (file.signature, file.max_size, file.replay_fps),
            (SIGNATURE, MAX_SIZE, Some(70))
        );
        assert_eq!(file.game_name.text_bytes(), b"CUP \"\xC1\" \\ 1");
        let top = &file.top_team;
        assert_eq!(top.name.text_bytes(), b"M\xC1LAGA");
        assert_eq!(
            (
                top.primary_kit.shirt_colour,
                top.marked_player,
                top.num_own_goals
            ),
            (-1, -1, -56)
        );
        assert_eq!((top.field_27, top.field_29), (0, u8::MAX));
        let player = top.players.last().expect("16 players");
        assert_eq!(
            (player.player_index, player.field_7, player.speed),
            (15, -1, -8)
        );
        assert_eq!(player.full_name.text(), "PLAYER NUMBER 15");
        assert_eq!(file.bottom_team.name.text(), "SEVENTEEN BYTES!!");
        let scene = file.scenes.first().expect("a scene");
        let [leading, frame] = scene.frames.as_slice() else {
            panic!("two frames");
        };
        assert_eq!(leading.synthetic_no_frame_marker, Some(true));
        assert_eq!(leading.sprites.len(), 2);
        assert_eq!((frame.cam_x, frame.cam_y), (CAMERA_X_MAX, u16::MAX - 1));
        assert_eq!(scene.eos_offset, EOS_OFFSET);
        assert_eq!(scene.trailing.first(), Some(&28));
    }

    #[test]
    fn a_file_before_replay_fps_round_trips() {
        let file = read(&writer_text(None));
        assert_eq!(file.replay_fps, None);
    }

    #[test]
    fn a_team_is_written_with_its_names_up_to_the_nul() {
        let mut file = read(&writer_text(Some(50)));
        let mut name = [0; TEAM_GAME_NAME_LEN];
        name.get_mut(..6).expect("fits").copy_from_slice(b"AB\0JNK");
        file.top_team.name = FixedName::from_bytes(name);
        let text = JsonHilFormat::export(&file).expect("writes");
        let back = JsonHilFormat::import(&text).expect("reads");
        assert_eq!(back.top_team.name.as_bytes().get(..3), Some(&b"AB\0"[..]));
        assert!(back.top_team.name.is_canonical());
    }

    #[test]
    fn the_goals_are_the_scenes() {
        let mut file = read(&writer_text(Some(70)));
        file.num_goals = 2;
        assert!(matches!(
            JsonHilFormat::export(&file),
            Err(JsonHilError::Scenes {
                stated: 2,
                scenes: 1
            })
        ));
        assert!(matches!(
            JsonHilFormat::import(&replaced("\"num_goals\":1,", "\"num_goals\":0,")),
            Err(JsonHilError::Scenes { .. })
        ));
    }

    #[test]
    fn a_scene_beyond_its_slot_does_not_read() {
        assert!(matches!(
            JsonHilFormat::import(&replaced("\"eos_offset\":28,", "\"eos_offset\":32,")),
            Err(JsonHilError::Scene {
                scene: 0,
                source: JsonSceneError::Trailing { .. }
            })
        ));
    }

    #[test]
    fn other_files_and_members_do_not_read() {
        for (from, to, why) in [
            ("\"highlights\"", "\"career\"", "not highlights"),
            (
                "\"schema_version\":3",
                "\"schema_version\":2",
                "schema version 2",
            ),
            (
                "\"reserved3\":255",
                "\"reserved3\":255,\"extra\":0",
                "unknown field `extra`",
            ),
            (
                "\"team_name\":\"SEVENTEEN BYTES!!\"",
                "\"team_name\":\"EIGHTEEN BYTES!!!!\"",
                "has 18 characters",
            ),
            ("\"num_own_goals\":200", "\"num_own_goals\":256", "256"),
        ] {
            let error = JsonHilFormat::import(&replaced(from, to)).expect_err(to);
            assert!(error.to_string().contains(why), "{to}: {error}");
        }
    }
}
