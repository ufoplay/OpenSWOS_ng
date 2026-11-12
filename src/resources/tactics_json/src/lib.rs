#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod preset;
pub mod tactics;

pub use preset::{NotAPreset, Preset, PresetFile, Release};
pub use tactics::{
    POSITION_GROUP_LEN, POSITION_GROUPS, TacticsJson, TacticsJsonError, TacticsJsonFormat,
};
