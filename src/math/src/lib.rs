#![doc = include_str!("../readme.md")]
// The original game has no floating point: every decision is integer arithmetic, and
// results must be bit-exact on every platform.
#![deny(clippy::float_arithmetic)]

mod delta;
mod m68k;
mod tables;

pub use delta::{
    Delta, DeltaScale, DeltaScaleError, MAX_NUMERATOR, NO_DIRECTION, Point, calculate_delta,
};
pub use m68k::{shift_left_16, shift_left_32};
