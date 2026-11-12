#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod dos;
pub mod error;
pub mod header;
pub mod json;
pub mod scene;
pub mod team_game;

pub use dos::{DosHilFile, DosHilFormat, DosRplFile, DosRplFormat};
pub use error::DosReplayError;
pub use header::{DOS_HEADER_LEN, DOS_SIGNATURE, DosReplayHeader, GAME_NAME_LEN, RESERVED_LEN};
pub use json::{
    CAMERA_X_MAX, HighlightsKind, JSON_GAME_NAME_LEN, JsonFrame, JsonHilError, JsonHilFile,
    JsonHilFormat, JsonScene, JsonSceneError, JsonSprite, SCENE_LEN, SPRITE_COORDINATE_MAX,
    SPRITE_INDEX_MAX,
};
pub use scene::{
    COORDINATE_MAX, COORDINATE_MIN, ReplayFrame, ReplaySprite, SPRITE_IMAGE_MAX, Scene, SceneError,
};
pub use team_game::{
    DOS_TEAM_GAME_LEN, DosKit, DosPlayerGame, DosTeamGame, FULL_NAME_LEN, PLAYER_GAME_LEN,
    SHORT_NAME_LEN, TEAM_GAME_LEN, TEAM_GAME_NAME_LEN, TEAM_GAME_PLAYERS, TEAM_GAME_TAIL_LEN,
    TeamGame, WordOrder,
};
