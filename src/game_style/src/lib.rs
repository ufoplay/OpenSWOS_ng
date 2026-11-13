#![doc = include_str!("../readme.md")]
// The original game has no floating point: every decision is integer arithmetic, and
// results must be bit-exact on every platform.
#![deny(clippy::float_arithmetic)]

mod registry;
mod style;
mod styles;
mod values;

pub use registry::{DEFAULT, STYLES, by_code, by_id};
pub use style::{GameStyle, KeeperReach, Release};
pub use styles::{Amiga50Hz, Amiga60Hz, Dos70Hz, Moon50Hz, Xbox60Hz};
pub use values::{
    BallPhysics, DIVE_DELTA_STEP, DIVE_DELTAS, Goalkeeping, PITCH_TYPES, STRONGER_KEEPER_STEP,
    Timers, dive_deltas,
};
