#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod file;
pub mod json;
pub mod tactics;

pub use file::{TacFile, TacFileFormat};
pub use json::{JsonTacError, JsonTacFormat};
pub use tactics::{
    BALL_AREA_COLUMNS, BALL_AREA_ROWS, NotAPairId, OUTFIELD_PLAYERS, PairId, PlayerSpots, Spot,
    TACTICS_NAME_LEN, TACTICS_RECORD_LEN, TacticsError, TeamTactics,
};
