//! `openswos_math` against upstream's src/math68k.c and include/math68k.h: every angle of
//! the table in every quadrant, any two points at any speed and scale, the 68000 shifts.
//! Without upstream's checkout the oracle is not built and these tests are not either
//! (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

use openswos_math::{
    DeltaScale, MAX_NUMERATOR, Point, calculate_delta, shift_left_16, shift_left_32,
};
use openswos_oracle::Oracle;
use openswos_oracle::math::{DeltaCall, Movement, Undefined};
use proptest::prelude::*;

/// The differences the angle table is read at without halving: below 32 either way.
const TABLE_REACH: i16 = 31;

/// The movement scale of the 60 Hz styles (upstream src/dos_gameplay.c `ApplyGameStyle`:
/// 50/60), scaled by multiplying then dividing.
const AT_60_HZ: (i16, i16) = (50, 60);

/// Speeds for the sweep: none, a player's, the largest, a negative one.
const SPEEDS: [i16; 4] = [0, 1168, i16::MAX, i16::MIN];

/// A point the sweep measures from.
const ORIGIN: Point = Point { x: 336, y: 449 };

fn scales() -> [DeltaScale; 3] {
    [
        DeltaScale::UNSCALED,
        DeltaScale::DOS,
        DeltaScale::new(AT_60_HZ.0, AT_60_HZ.1).expect("a scale"),
    ]
}

fn scale() -> impl Strategy<Value = DeltaScale> {
    prop_oneof![
        Just(DeltaScale::UNSCALED),
        Just(DeltaScale::DOS),
        (-MAX_NUMERATOR..=MAX_NUMERATOR, any::<i16>()).prop_filter_map(
            "a denominator",
            |(numerator, denominator)| DeltaScale::new(numerator, denominator).ok()
        ),
    ]
}

/// Both sides' answers from `from` to `to`, or `None` where upstream is undefined.
fn both(from: Point, to: Point, speed: i16, scale: DeltaScale) -> Option<(Movement, Movement)> {
    let mut oracle = Oracle::lock();
    let call = DeltaCall {
        dest_x: to.x,
        dest_y: to.y,
        src_x: from.x,
        src_y: from.y,
        speed,
        scale_numerator: scale.numerator(),
        scale_denominator: scale.denominator(),
    };
    let upstream = match oracle.calculate_delta_x_and_y(call) {
        Ok(movement) => movement,
        Err(Undefined::Difference) => return None,
        Err(error) => panic!("a valid scale refused: {error}"),
    };
    let delta = calculate_delta(from, to, speed, scale);
    let port = Movement {
        direction: delta.raw_direction(),
        delta_x: delta.x,
        delta_y: delta.y,
    };
    Some((upstream, port))
}

#[test]
fn every_angle_in_every_quadrant_as_upstream() {
    for dx in -TABLE_REACH..=TABLE_REACH {
        for dy in -TABLE_REACH..=TABLE_REACH {
            let to = Point {
                x: ORIGIN.x + dx,
                y: ORIGIN.y + dy,
            };
            for speed in SPEEDS {
                for scale in scales() {
                    let (upstream, port) = both(ORIGIN, to, speed, scale).expect("a defined call");
                    assert_eq!(port, upstream, "({dx}, {dy}) at {speed}, {scale:?}");
                }
            }
        }
    }
}

proptest! {
    #[test]
    fn any_two_points_as_upstream(
        from: (i16, i16), to: (i16, i16), speed: i16, scale in scale()
    ) {
        let from = Point { x: from.0, y: from.1 };
        let to = Point { x: to.0, y: to.1 };
        if let Some((upstream, port)) = both(from, to, speed, scale) {
            prop_assert_eq!(port, upstream);
        }
    }

    #[test]
    fn the_32_bit_shift_as_upstream(value: i32, count in 0..i32::BITS) {
        let oracle = Oracle::lock();
        let upstream = oracle.shift_left_32(value, count.cast_signed());
        prop_assert_eq!(Ok(shift_left_32(value, count)), upstream);
    }

    #[test]
    fn the_16_bit_shift_as_upstream(value: i16, count in 0..i16::BITS) {
        let oracle = Oracle::lock();
        let upstream = oracle.shift_left_16(value, count.cast_signed());
        prop_assert_eq!(Ok(shift_left_16(value, count)), upstream);
    }
}

#[test]
fn undefined_calls_are_not_made() {
    let mut oracle = Oracle::lock();
    let call = DeltaCall {
        dest_x: -1,
        dest_y: 0,
        src_x: i16::MAX,
        src_y: 0,
        speed: 1,
        scale_numerator: 1,
        scale_denominator: 1,
    };
    assert_eq!(
        oracle.calculate_delta_x_and_y(call),
        Err(Undefined::Difference)
    );
    let call = DeltaCall {
        dest_x: 0,
        scale_denominator: 0,
        ..call
    };
    assert_eq!(oracle.calculate_delta_x_and_y(call), Err(Undefined::Scale));
    let width = i32::BITS.cast_signed();
    assert_eq!(oracle.shift_left_32(1, width), Err(Undefined::ShiftCount));
    assert_eq!(oracle.shift_left_32(1, -1), Err(Undefined::ShiftCount));
    let width = i16::BITS.cast_signed();
    assert_eq!(oracle.shift_left_16(1, width), Err(Undefined::ShiftCount));
}
