//! SWOS 2020's AMIGA (60FPS) style: the Amiga's game at 60 frames a second.

use openswos_math::DeltaScale;

use super::{amiga, at_60_hz};
use crate::style::{GameStyle, Release};
use crate::values::{BallPhysics, Goalkeeping, Timers, dive_deltas};

/// SWOS 2020's AMIGA (60FPS) style (its style 3; upstream `GAMEPLAY_AMIGA60`): the Amiga's
/// game, goalkeepers included, at 60 frames a second.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Amiga60Hz;

/// Its id (`[gameplay] style = amiga60`).
const ID: &str = "amiga60";

/// Its number (upstream `GAMEPLAY_AMIGA60`).
const CODE: u8 = 3;

/// Its name (upstream `Gameplay_StyleName`).
const NAME: &str = "AMIGA60";

/// The Amiga's goalkeepers, their reach table a step above the Amiga's (upstream
/// src/dos_gameplay.c `ApplyGameStyle`, as SWOS 2020's style 3).
const GOALKEEPING: Goalkeeping = Goalkeeping {
    dive_deltas: dive_deltas(0x38000),
    ..amiga::GOALKEEPING
};

/// The Amiga's ball at 60 frames a second.
const BALL: BallPhysics = BallPhysics {
    gravity: at_60_hz::GRAVITY,
    ..amiga::BALL
};

impl GameStyle for Amiga60Hz {
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
        Release::Amiga
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
