#![doc = include_str!("../readme.md")]
// The original game has no floating point: every draw is integer arithmetic.
#![deny(clippy::float_arithmetic)]

mod generator;
mod rng;
mod table;

pub use generator::Generator;
pub use rng::{NothingSaved, Rng, SAVE_STACK_FULL_REASON, SAVE_STACK_LEN, SaveStackFull};
pub use table::{RANDOM_TABLE, RANDOM_TABLE_LEN, random_byte};
