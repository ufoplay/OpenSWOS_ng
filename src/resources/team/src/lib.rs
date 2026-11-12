#![doc = include_str!("../readme.md")]
// This crate parses untrusted bytes: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod file;
pub mod player;
pub mod skill;
pub mod team;

pub use file::{TeamFile, TeamFileError, TeamFileFormat};
pub use player::{
    Face, Form, FormLevel, GoalCounts, InvalidFace, InvalidPlayerFlags, Nationality, NotAFormLevel,
    PLAYER_NAME_LEN, PLAYER_RECORD_LEN, Player, PlayerFlags, PlayerRuntime, PriceId,
};
pub use skill::{
    InvalidNibble, InvalidSkill, InvalidSkillLevel, Nibble, Skill, SkillLevel, Skills,
};
pub use team::{
    COACH_NAME_LEN, Division, GlobalTeamNumber, InvalidKitColour, InvalidPlayerValues,
    InvalidShirtType, InvalidTactics, InvalidTeamControls, Kit, KitColour, MatchFlags,
    PLAYERS_PER_TEAM, PlayerValues, RecordError, ShirtType, TEAM_HEADER_LEN, TEAM_NAME_LEN,
    TEAM_RECORD_LEN, Tactics, Team, TeamControls, TeamRuntime,
};
