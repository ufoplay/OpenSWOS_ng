#![doc = include_str!("../readme.md")]
// The original game has no floating point: every decision is integer arithmetic, and
// results must be bit-exact on every platform.
#![deny(clippy::float_arithmetic)]

pub mod data;
pub mod validation;
pub mod world;

#[cfg(test)]
mod fixture;

pub use data::GameData;
pub use validation::{Invalid, Location, Problem, ValidationError};
pub use world::World;
