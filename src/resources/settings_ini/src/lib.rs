#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod file;
pub mod reader;
pub mod settings;
mod text;
pub mod writer;

pub use file::{IniFile, IniFileFormat};
pub use reader::{IniRead, IniValue, read};
pub use settings::{
    CpuTactics, GameLength, GameOptions, GameplaySettings, GameplayStyle, GeneralSettings,
    ModSettings, PitchType, ReplaySettings, ReplaySpeed, SECTION_INTROS, Settings, Spelled,
    TestingSettings, Tweaks,
};
pub use writer::{
    ADDED_SECTIONS_COMMENT, FRESH_FILE_HEADER, SAVE_MAX_LINES, SectionIntro, Setting, SettingsTable,
};
