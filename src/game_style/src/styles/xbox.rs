//! SWOS 2020's XBOX style: 60 frames a second, the Amiga's ball, the DOS goalkeepers.

use openswos_math::DeltaScale;

use super::{amiga, at_60_hz};
use crate::style::{GameStyle, Release};
use crate::values::{BallPhysics, Goalkeeping, Timers, dive_deltas};

/// SWOS 2020's XBOX style (its style 4; upstream `GAMEPLAY_XBOX`): the Amiga's game at 60
/// frames a second with the DOS goalkeeper code, its keepers' values its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Xbox60Hz;

/// Its id (`[gameplay] style = xbox`).
const ID: &str = "xbox";

/// Its number (upstream `GAMEPLAY_XBOX`).
const CODE: u8 = 2;

/// Its name (upstream `Gameplay_StyleName`).
const NAME: &str = "XBOX";

/// The XBOX goalkeepers (upstream src/dos_gameplay.c `ApplyGameStyle`, SWOS 2020's "XBOX GK
/// Values"): the DOS game's save distance and catch height, the reach table two steps above
/// the DOS one, and SWOS 2020's sign fix of the loose ball's range.
const GOALKEEPING: Goalkeeping = Goalkeeping {
    save_distance: 16,
    catch_height: 27,
    dive_deltas: dive_deltas(0x40000),
    loose_ball_min_delta_z: 0x8000,
};

/// The Amiga's ball at 60 frames a second.
const BALL: BallPhysics = BallPhysics {
    gravity: at_60_hz::GRAVITY,
    ..amiga::BALL
};

impl GameStyle for Xbox60Hz {
    fn id(&self) -> &'static str {
        ID
    }

    fn code(&self) -> u8 {
        CODE
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn frames_per_second(&self) -> u16 {
        at_60_hz::FRAMES_PER_SECOND
    }

    fn release(&self) -> Release {
        Release::Amiga
    }

    fn goalkeepers(&self) -> Release {
        Release::Dos
    }

    fn formations(&self) -> Release {
        Release::Amiga
    }

    fn goalkeeping(&self) -> &'static Goalkeeping {
        &GOALKEEPING
    }

    fn ball(&self) -> &'static BallPhysics {
        &BALL
    }

    fn timers(&self) -> &'static Timers {
        &at_60_hz::TIMERS
    }

    fn delta_scale(&self) -> DeltaScale {
        at_60_hz::DELTA_SCALE
    }
}
