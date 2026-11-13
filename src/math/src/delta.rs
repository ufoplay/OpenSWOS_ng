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
mod tests {
    use proptest::prelude::*;

    use super::{
        DOS_SHIFTS, Delta, DeltaScale, DeltaScaleError, MAX_NUMERATOR, NO_DIRECTION, Point, TURN,
        calculate_delta,
    };
    use crate::tables::TURN_STEPS;

    /// A known answer: from, to, speed, then the direction and the steps.
    struct Vector {
        speed: i16,
        from: (i16, i16),
        to: (i16, i16),
        direction: Option<u8>,
        x: i32,
        y: i32,
    }

    /// The DOS routine's known answers (upstream src/replay_test.c `k_deltaVectors`, from
    /// swos-port's `kCalculateDeltaXAndYTestData`): scaled by 41/64.
    const DOS_VECTORS: [Vector; 10] = [
        Vector {
            speed: 1168,
            from: (396, 373),
            to: (757, 447),
            direction: Some(0x46),
            x: 0x17048,
            y: 0x349d,
        },
        Vector {
            speed: 1168,
            from: (757, 447),
            to: (396, 373),
            direction: Some(0xc6),
            x: 0xfffe_8ccd_u32.cast_signed(),
            y: 0xffff_c878_u32.cast_signed(),
        },
        Vector {
            speed: 0,
            from: (396, 373),
            to: (757, 447),
            direction: Some(0x46),
            x: 0,
            y: 0,
        },
        Vector {
            speed: 0,
            from: (757, 447),
            to: (396, 373),
            direction: Some(0xc6),
            x: 0,
            y: 0,
        },
        Vector {
            speed: 1168,
            from: (396, 373),
            to: (396, 373),
            direction: None,
            x: 0,
            y: 0,
        },
        Vector {
            speed: 1168,
            from: (396, 373),
            to: (396, 373),
            direction: None,
            x: 0,
            y: 0,
        },
        Vector {
            speed: 500,
            from: (200, 300),
            to: (200, 100),
            direction: Some(0x00),
            x: 0,
            y: 0xffff_5fd8_u32.cast_signed(),
        },
        Vector {
            speed: 500,
            from: (200, 100),
            to: (200, 300),
            direction: Some(0x80),
            x: 0,
            y: 0x0000_9ee9,
        },
        Vector {
            speed: 1024,
            from: (0, 0),
            to: (1, 1),
            direction: Some(0x60),
            x: 0x0000_e6a0,
            y: 0x0000_e6a0,
        },
        Vector {
            speed: 1024,
            from: (1, 1),
            to: (0, 0),
            direction: Some(0xe0),
            x: 0xffff_16d0_u32.cast_signed(),
            y: 0xffff_16d0_u32.cast_signed(),
        },
    ];

    /// A speed for the tests.
    const SPEED: i16 = 1000;

    /// A distance for the tests.
    const DISTANCE: i16 = 100;

    /// The table's sine at a quarter turn, down to a byte: 32767 >> 8.
    const FULL_STEP: i32 = 127;

    /// The table's sine at three quarters of a turn, down to a byte: -32767 >> 8.
    const FULL_STEP_BACK: i32 = -128;

    /// Up, right, down and left, in 256 steps.
    const COMPASS: [u8; 4] = [0, 64, 128, 192];

    fn point((x, y): (i16, i16)) -> Point {
        Point { x, y }
    }

    fn scale() -> impl Strategy<Value = DeltaScale> {
        (-MAX_NUMERATOR..=MAX_NUMERATOR, any::<i16>())
            .prop_filter_map("a denominator", |(numerator, denominator)| {
                DeltaScale::new(numerator, denominator).ok()
            })
    }

    #[test]
    fn the_dos_routines_known_answers() {
        for (number, vector) in DOS_VECTORS.iter().enumerate() {
            let delta = calculate_delta(
                point(vector.from),
                point(vector.to),
                vector.speed,
                DeltaScale::DOS,
            );
            let expected = Delta {
                direction: vector.direction,
                x: vector.x,
                y: vector.y,
            };
            assert_eq!(delta, expected, "vector {number}");
        }
    }

    #[test]
    fn the_compass_points() {
        let origin = Point::default();
        let steps = [
            ((0, -DISTANCE), (0, FULL_STEP_BACK)),
            ((DISTANCE, 0), (FULL_STEP, 0)),
            ((0, DISTANCE), (0, FULL_STEP)),
            ((-DISTANCE, 0), (FULL_STEP_BACK, 0)),
        ];
        for (direction, (to, (x, y))) in COMPASS.into_iter().zip(steps) {
            let delta = calculate_delta(origin, point(to), SPEED, DeltaScale::UNSCALED);
            let speed = i32::from(SPEED);
            let expected = Delta {
                direction: Some(direction),
                x: x * speed,
                y: y * speed,
            };
            assert_eq!(delta, expected, "towards {to:?}");
        }
    }

    #[test]
    fn no_step_has_upstreams_raw_direction() {
        let here = point((DISTANCE, DISTANCE));
        let delta = calculate_delta(here, here, SPEED, DeltaScale::DOS);
        assert_eq!(delta.raw_direction(), NO_DIRECTION);
        let delta = calculate_delta(Point::default(), here, SPEED, DeltaScale::DOS);
        assert_eq!(
            delta.raw_direction(),
            u16::from(delta.direction.expect("a step"))
        );
    }

    #[test]
    fn a_difference_upstream_leaves_undefined_has_its_true_size() {
        // -32768 along x: taken as 32768, halved like any other.
        let from = point((i16::MAX, 0));
        let to = point((-1, 0));
        let delta = calculate_delta(from, to, SPEED, DeltaScale::UNSCALED);
        assert_eq!(delta.direction, Some(COMPASS[3]));
        let delta = calculate_delta(point((0, i16::MAX)), point((0, -1)), SPEED, DeltaScale::DOS);
        assert_eq!(delta.direction, Some(COMPASS[0]));
    }

    #[test]
    fn the_turn_is_the_tables_byte_offsets() {
        assert_eq!(usize::from(TURN), TURN_STEPS << 1);
    }

    #[test]
    fn a_scale_is_checked() {
        assert_eq!(
            DeltaScale::new(MAX_NUMERATOR + 1, 1),
            Err(DeltaScaleError::Numerator(MAX_NUMERATOR + 1))
        );
        assert_eq!(
            DeltaScale::new(-MAX_NUMERATOR - 1, 1),
            Err(DeltaScaleError::Numerator(-MAX_NUMERATOR - 1))
        );
        assert_eq!(DeltaScale::new(1, 0), Err(DeltaScaleError::ZeroDenominator));
        let scale = DeltaScale::new(-MAX_NUMERATOR, -1).expect("a scale");
        assert_eq!(
            (scale.numerator(), scale.denominator()),
            (-MAX_NUMERATOR, -1)
        );
    }

    proptest! {
        #[test]
        fn only_the_same_points_have_no_direction(
            from: (i16, i16), to: (i16, i16), speed: i16, scale in scale()
        ) {
            let delta = calculate_delta(point(from), point(to), speed, scale);
            prop_assert_eq!(delta.direction.is_none(), from == to);
            if from == to {
                prop_assert_eq!((delta.x, delta.y), (0, 0));
            }
        }

        #[test]
        fn the_dos_scale_shifts(from: (i16, i16), to: (i16, i16), speed: i16) {
            let unscaled = calculate_delta(point(from), point(to), speed, DeltaScale::UNSCALED);
            let dos = calculate_delta(point(from), point(to), speed, DeltaScale::DOS);
            let shifted = |step: i32| {
                step - DOS_SHIFTS.iter().map(|shift| step >> shift).sum::<i32>()
            };
            prop_assert_eq!(dos.direction, unscaled.direction);
            prop_assert_eq!((dos.x, dos.y), (shifted(unscaled.x), shifted(unscaled.y)));
        }

        #[test]
        fn any_other_scale_multiplies_then_divides(
            from: (i16, i16), to: (i16, i16), speed: i16, scale in scale()
        ) {
            prop_assume!(scale != DeltaScale::DOS);
            let unscaled = calculate_delta(point(from), point(to), speed, DeltaScale::UNSCALED);
            let scaled = calculate_delta(point(from), point(to), speed, scale);
            let by = |step: i32| {
                if scale.numerator() == scale.denominator() {
                    step
                } else {
                    step * i32::from(scale.numerator()) / i32::from(scale.denominator())
                }
            };
            prop_assert_eq!(scaled.direction, unscaled.direction);
            prop_assert_eq!((scaled.x, scaled.y), (by(unscaled.x), by(unscaled.y)));
        }

        #[test]
        fn the_way_back_is_half_a_turn_round(from: (i16, i16), to: (i16, i16)) {
            // Where the differences do not wrap, each one is the other's negation.
            prop_assume!(from != to);
            prop_assume!(from.0.checked_sub(to.0).is_some() && from.1.checked_sub(to.1).is_some());
            prop_assume!(to.0.checked_sub(from.0).is_some() && to.1.checked_sub(from.1).is_some());
            let there = calculate_delta(point(from), point(to), SPEED, DeltaScale::UNSCALED);
            let back = calculate_delta(point(to), point(from), SPEED, DeltaScale::UNSCALED);
            let half_turn = COMPASS[2];
            prop_assert_eq!(
                back.direction,
                there.direction.map(|direction| direction.wrapping_add(half_turn))
            );
        }
    }
}
