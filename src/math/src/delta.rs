//! The direction from a point to another, and the step towards it (upstream
//! `CalculateDeltaXAndY`, src/math68k.c).

use thiserror::Error;

use crate::tables::{ANGLE_COEFFICIENTS, ANGLE_STEPS, SINE_COSINE_TABLE};

/// A full turn in the half steps `CalculateDeltaXAndY` turns in: the byte offsets into the
/// word table [`SINE_COSINE_TABLE`], twice its 256 steps.
const TURN: u16 = 512;

/// A quarter turn, in half steps.
const QUARTER_TURN: u16 = 128;

/// Half a turn, in half steps.
const HALF_TURN: u16 = 256;

/// Three quarters of a turn, in half steps.
const THREE_QUARTER_TURN: u16 = HALF_TURN + QUARTER_TURN;

/// The bits of a direction within a turn, in half steps.
const TURN_MASK: u16 = TURN - 1;

/// `asr.w #8`: the table's sine, ±32767, down to -128..=127.
const SINE_SHIFT: u32 = 8;

/// Upstream's direction when there is no step to take: the points are the same.
pub const NO_DIRECTION: u16 = 0xFFFF;

/// The largest numerator of a [`DeltaScale`]: a step is at most 2²² in size (-128 × -32768),
/// and 2²² × 511 still fits in an `i32`. Upstream is undefined past it.
pub const MAX_NUMERATOR: i16 = 511;

/// The DOS game's numerator (upstream src/math68k.c, `ApplyGameStyle`).
const DOS_NUMERATOR: i16 = 41;

/// The DOS game's denominator.
const DOS_DENOMINATOR: i16 = 64;

/// The DOS scale as arithmetic shifts: 41/64 = 1 - 1/4 - 1/16 - 1/32 - 1/64 (upstream
/// src/math68k.c). Each shift floors where a division truncates towards zero, which gives
/// different steps for negative values.
const DOS_SHIFTS: [u32; 4] = [2, 4, 5, 6];

/// A point on the pitch: the whole parts of a sprite's coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Point {
    /// Across the pitch, growing rightwards.
    pub x: i16,
    /// Along the pitch, growing downwards.
    pub y: i16,
}

/// What `CalculateDeltaXAndY` computes: the direction from a point to another, and a step
/// towards it at a speed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Delta {
    /// The direction in 256 steps clockwise: 0 up (towards smaller y), 64 right, 128 down,
    /// 192 left. `None` when the points are the same (upstream [`NO_DIRECTION`]).
    pub direction: Option<u8>,
    /// The step along x, in 16.16 fixed point.
    pub x: i32,
    /// The step along y, in 16.16 fixed point.
    pub y: i32,
}

impl Delta {
    /// The points are the same: no direction, no step.
    const STILL: Self = Self {
        direction: None,
        x: 0,
        y: 0,
    };

    /// The direction as upstream returns it: 0 to 255, or [`NO_DIRECTION`]. Some callers do
    /// arithmetic on it.
    #[must_use]
    pub fn raw_direction(self) -> u16 {
        self.direction.map_or(NO_DIRECTION, u16::from)
    }
}

/// The factor the steps are scaled by, `numerator / denominator`, so that a game running at
/// more frames a second does not play faster (upstream `_gMoveDeltaScaleNum` and
/// `_gMoveDeltaScaleDen`, which `ApplyGameStyle` sets): 41/64 for the DOS game at 70 Hz, none
/// for the Amiga's 50 Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeltaScale {
    numerator: i16,
    denominator: i16,
}

/// A [`DeltaScale`] that `CalculateDeltaXAndY` cannot scale by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DeltaScaleError {
    /// The numerator is beyond ±[`MAX_NUMERATOR`]: a scaled step could overflow.
    #[error("the numerator {0} is beyond ±{MAX_NUMERATOR}: a scaled step could overflow")]
    Numerator(i16),
    /// The denominator is 0.
    #[error("the denominator is 0")]
    ZeroDenominator,
}

impl DeltaScale {
    /// No scale: the Amiga's at 50 Hz.
    pub const UNSCALED: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    /// 41/64, the DOS game's at 70 Hz: done with arithmetic shifts ([`DOS_SHIFTS`]), not by
    /// multiplying and dividing.
    pub const DOS: Self = Self {
        numerator: DOS_NUMERATOR,
        denominator: DOS_DENOMINATOR,
    };

    /// The scale `numerator / denominator`.
    ///
    /// # Errors
    ///
    /// When the numerator is beyond ±[`MAX_NUMERATOR`], or the denominator is 0: upstream's
    /// arithmetic is undefined there.
    pub const fn new(numerator: i16, denominator: i16) -> Result<Self, DeltaScaleError> {
        if numerator.unsigned_abs() > MAX_NUMERATOR.unsigned_abs() {
            Err(DeltaScaleError::Numerator(numerator))
        } else if denominator == 0 {
            Err(DeltaScaleError::ZeroDenominator)
        } else {
            Ok(Self {
                numerator,
                denominator,
            })
        }
    }

    /// The numerator.
    #[must_use]
    pub const fn numerator(self) -> i16 {
        self.numerator
    }

    /// The denominator.
    #[must_use]
    pub const fn denominator(self) -> i16 {
        self.denominator
    }

    /// `step` scaled. Faithful to the original: 41/64 exactly is done with shifts, any other
    /// pair of equal numbers leaves the step, every other scale multiplies then divides.
    fn scale(self, step: i32) -> i32 {
        if self == Self::DOS {
            DOS_SHIFTS
                .iter()
                .fold(step, |scaled, shift| scaled - (step >> shift))
        } else if self.numerator == self.denominator {
            step
        } else {
            step * i32::from(self.numerator) / i32::from(self.denominator)
        }
    }
}

/// The direction from `from` to `to`, and a step towards it at `speed`, scaled by `scale`
/// (upstream `CalculateDeltaXAndY`).
///
/// The differences are halved together until both are below 32, the angle table gives the
/// angle within a quadrant, and the sine table the step: its sine and cosine (down to
/// -128..=127) times the speed. Where upstream is undefined (a difference of exactly -32768
/// after wrapping, whose absolute value does not fit a word) the port takes its true size.
#[must_use]
pub fn calculate_delta(from: Point, to: Point, speed: i16, scale: DeltaScale) -> Delta {
    let dx = to.x.wrapping_sub(from.x);
    let dy = to.y.wrapping_sub(from.y);
    let (mut abs_x, mut abs_y) = (dx.unsigned_abs(), dy.unsigned_abs());
    while usize::from(abs_x) >= ANGLE_STEPS {
        abs_x >>= 1;
        abs_y >>= 1;
    }
    while usize::from(abs_y) >= ANGLE_STEPS {
        abs_x >>= 1;
        abs_y >>= 1;
    }
    let Ok(angle) = u16::try_from(ANGLE_COEFFICIENTS[usize::from(abs_y)][usize::from(abs_x)])
    else {
        return Delta::STILL;
    };
    let angle = angle << 1;
    let turned = match (dx < 0, dy < 0) {
        (false, false) => QUARTER_TURN - angle,
        (false, true) => QUARTER_TURN + angle,
        (true, false) => THREE_QUARTER_TURN + angle,
        (true, true) => THREE_QUARTER_TURN - angle,
    } & TURN_MASK;
    let cosine = SINE_COSINE_TABLE[usize::from(turned >> 1)] >> SINE_SHIFT;
    let sine =
        SINE_COSINE_TABLE[usize::from(((turned + QUARTER_TURN) & TURN_MASK) >> 1)] >> SINE_SHIFT;
    let direction = ((TURN - turned + HALF_TURN) & TURN_MASK) >> 1;
    Delta {
        direction: Some(u8::try_from(direction).expect("half a direction within a turn")),
        x: scale.scale(i32::from(cosine) * i32::from(speed)),
        y: scale.scale(i32::from(sine) * i32::from(speed)),
    }
}

#[cfg(test)]
mod tests;
