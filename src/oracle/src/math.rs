//! The math (upstream src/math68k.c, include/math68k.h): `CalculateDeltaXAndY` and the 68000
//! shifts.

use thiserror::Error;

use crate::Oracle;
use crate::ffi;

/// The arguments of `CalculateDeltaXAndY`, with the movement scale it reads from two
/// globals (`_gMoveDeltaScaleNum`, `_gMoveDeltaScaleDen`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeltaCall {
    /// The destination's x.
    pub dest_x: i16,
    /// The destination's y.
    pub dest_y: i16,
    /// The source's x.
    pub src_x: i16,
    /// The source's y.
    pub src_y: i16,
    /// The speed.
    pub speed: i16,
    /// The movement scale's numerator.
    pub scale_numerator: i16,
    /// The movement scale's denominator.
    pub scale_denominator: i16,
}

/// What `CalculateDeltaXAndY` returns: its direction, and the steps it writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Movement {
    /// The direction, 0 to 255, or `0xFFFF` for none.
    pub direction: u16,
    /// The step along x.
    pub delta_x: i32,
    /// The step along y.
    pub delta_y: i32,
}

/// A call upstream's C does not define: the oracle does not make it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Undefined {
    /// A difference of -32768 after wrapping: its absolute value does not fit, and upstream
    /// indexes its table before the start.
    #[error("a difference of -32768")]
    Difference,
    /// A movement scale whose arithmetic could overflow, or over 0.
    #[error("a movement scale upstream cannot scale by")]
    Scale,
    /// A shift count past the width.
    #[error("a shift count past the width")]
    ShiftCount,
}

impl Oracle {
    /// `CalculateDeltaXAndY`: the direction from the source to the destination, and the steps
    /// towards it at the speed, scaled.
    ///
    /// # Errors
    ///
    /// When upstream would be undefined: a difference of -32768, or a scale whose numerator
    /// is beyond ±511 or whose denominator is 0.
    pub fn calculate_delta_x_and_y(&mut self, call: DeltaCall) -> Result<Movement, Undefined> {
        ffi::math::calculate_delta_x_and_y(call)
    }

    /// `M68kShiftLeft32`: the value's bits shifted left.
    ///
    /// # Errors
    ///
    /// When the count is outside 0..32.
    pub fn shift_left_32(&self, value: i32, count: i32) -> Result<i32, Undefined> {
        ffi::math::shift_left_32(value, count)
    }

    /// `M68kShiftLeft16`: the word's bits shifted left.
    ///
    /// # Errors
    ///
    /// When the count is outside 0..16.
    pub fn shift_left_16(&self, value: i16, count: i32) -> Result<i16, Undefined> {
        ffi::math::shift_left_16(value, count)
    }
}
