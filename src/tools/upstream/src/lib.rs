//! The migration ledger tool (docs/PLAN.md §8).
//!
//! `upstream.toml` records which upstream OpenSWOS commit the port is migrated against,
//! and the state of every upstream file it covers. This crate reads and writes it, keeps
//! it in line with an upstream checkout, and checks it.

pub mod check;
pub mod commands;
pub mod error;
pub mod fetch;
pub mod git;
pub mod ledger;
pub mod mark;
pub mod status;
pub mod sync;
pub mod tracked;
pub mod workspace;
