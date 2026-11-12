#![doc = include_str!("../readme.md")]
// This crate parses untrusted bytes: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod file;
pub mod player;
pub mod team;

pub use file::{TeamFile, TeamFileError, TeamFileFormat};
pub use player::{
    Face, GoalCounts, InvalidFace, InvalidSkill, Nationality, PLAYER_NAME_LEN, PLAYER_RECORD_LEN,
    Player, PlayerFlags, PlayerRuntime, PriceId, RawSkill, Skills,
};
pub use team::{
    COACH_NAME_LEN, Division, GlobalTeamNumber, InvalidShirtType, Kit, KitColour, PLAYERS_PER_TEAM,
    RecordError, ShirtType, TEAM_HEADER_LEN, TEAM_NAME_LEN, TEAM_RECORD_LEN, TacticsIndex, Team,
    TeamRuntime,
};
