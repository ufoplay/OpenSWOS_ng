//! The generic competition engine (docs/PLAN.md M5): leagues, cups and tournaments as the
//! loaded world states them. It grows unit by unit; today the round robins, the league
//! tables, the teams' values, the draws, a competition under way's state, a league and a cup
//! started, and a tournament's state.

pub mod cup;
pub mod draw;
pub mod league;
pub mod running;
pub mod schedule;
pub mod start;
pub mod table;
pub mod tournament;
pub mod value;
