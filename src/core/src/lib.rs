#![doc = include_str!("../readme.md")]
// The original game has no floating point: every decision is integer arithmetic, and
// results must be bit-exact on every platform.
#![deny(clippy::float_arithmetic)]
