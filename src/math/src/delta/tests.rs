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
