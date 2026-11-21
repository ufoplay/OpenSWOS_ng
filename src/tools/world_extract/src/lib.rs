//! `world_extract`: builds the world packages (docs/PLAN.md §5.2, M4 #5).
//!
//! - [`assemble`]: World 1996-97, the committed package (`data/worlds/1996-97/`) with the
//!   rosters and the pool of your copy of the PC game ([`game`]);
//! - [`extract`]: World 1996-97's committed package, extracted from the upstream checkout and
//!   its compiled tables (the oracle);
//! - [`fetch`]: World 2026-27, downloaded and checked against the upstream pin;
//! - [`complete`]: World 2026-27 completed: its download over World 1996-97, its rules from
//!   its baked install (the oracle);
//! - [`check`]: every package a valid world with valid game data (`openswos_core`) before it
//!   is written;
//! - [`output`]: where packages go (`target/cache/worlds/`), and writing them;
//! - [`commands`]: the commands, whole.

pub mod assemble;
pub mod check;
pub mod commands;
pub mod complete;
pub mod error;
pub mod extract;
pub mod fetch;
pub mod game;
pub mod output;
