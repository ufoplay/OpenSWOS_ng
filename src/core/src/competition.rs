//! The generic competition engine (docs/PLAN.md M5): leagues, cups and tournaments as the
//! loaded world states them. It grows unit by unit; today the round robins, the league
//! tables, the teams' values, the draws, a competition under way's state and a league started.

pub mod draw;
pub mod league;
pub mod running;
pub mod schedule;
pub mod table;
pub mod value;
