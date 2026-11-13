//! src/math68k.c and include/math68k.h. The unit has no globals of its own: the movement
//! scale it reads is stubbed.

use super::bindings::math68k_c::CalculateDeltaXAndY;
use super::bindings::math68k_h::{M68kShiftLeft16, M68kShiftLeft32};
use super::stubs;
use crate::math::{DeltaCall, Movement, Undefined};

/// The largest numerator of the movement scale the oracle calls with: a step is at most 2²²
/// in size (-128 × -32768), and 2²² × 511 still fits in an `int`; past it the multiplication
/// could overflow, which is undefined.
const MAX_SCALE_NUMERATOR: u16 = 511;

/// `CalculateDeltaXAndY(...)`, its scale in the two globals. A difference of -32768 after
/// wrapping makes upstream index its table before its start, and a scale past
/// [`MAX_SCALE_NUMERATOR`] or over 0 makes its arithmetic undefined: neither is called.
pub(crate) fn calculate_delta_x_and_y(call: DeltaCall) -> Result<Movement, Undefined> {
    let DeltaCall {
        dest_x,
        dest_y,
        src_x,
        src_y,
        speed,
        scale_numerator,
        scale_denominator,
    } = call;
    if dest_x.wrapping_sub(src_x) == i16::MIN || dest_y.wrapping_sub(src_y) == i16::MIN {
        return Err(Undefined::Difference);
    }
    if scale_numerator.unsigned_abs() > MAX_SCALE_NUMERATOR || scale_denominator == 0 {
        return Err(Undefined::Scale);
    }
    let (mut delta_x, mut delta_y) = (0, 0);
    // SAFETY: writes the scale, globals the lock's holder owns; the call is defined for these
    // arguments, its tables indexed within them, and writes the two steps it is given.
    let direction = unsafe {
        stubs::_gMoveDeltaScaleNum = scale_numerator;
        stubs::_gMoveDeltaScaleDen = scale_denominator;
        CalculateDeltaXAndY(
            dest_x,
            dest_y,
            src_x,
            src_y,
            speed,
            &raw mut delta_x,
            &raw mut delta_y,
        )
    };
    Ok(Movement {
        direction,
        delta_x,
        delta_y,
    })
}

/// `M68kShiftLeft32(value, count)`. A count outside 0..32 is undefined: not called.
pub(crate) fn shift_left_32(value: i32, count: i32) -> Result<i32, Undefined> {
    if !(0..32).contains(&count) {
        return Err(Undefined::ShiftCount);
    }
    // SAFETY: a pure function, defined for this count.
    Ok(unsafe { M68kShiftLeft32(value, count) })
}

/// `M68kShiftLeft16(value, count)`. A count outside 0..16 can overflow the `int` the word is
/// shifted in, which is undefined: not called.
pub(crate) fn shift_left_16(value: i16, count: i32) -> Result<i16, Undefined> {
    if !(0..16).contains(&count) {
        return Err(Undefined::ShiftCount);
    }
    // SAFETY: a pure function, defined for this count.
    Ok(unsafe { M68kShiftLeft16(value, count) })
}
