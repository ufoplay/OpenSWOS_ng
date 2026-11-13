//! SWOS 2020's MOON SOCCER style: the Amiga's, with a moon's gravity.

use openswos_math::DeltaScale;

use super::amiga::{self, Amiga50Hz};
use crate::style::{GameStyle, Release};
use crate::values::{BallPhysics, Goalkeeping, Timers};

/// SWOS 2020's MOON SOCCER style (its style 2; upstream `GAMEPLAY_MOON`): the Amiga's game
/// at 50 frames a second, but for its gravity and its goalkeepers' save distance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Moon50Hz;

/// Its id (`[gameplay] style = moon`).
const ID: &str = "moon";

/// Its number (upstream `GAMEPLAY_MOON`).
const CODE: u8 = 4;

/// Its name (upstream `Gameplay_StyleName`).
const NAME: &str = "MOON";

/// The Amiga's goalkeepers, saving at SWOS 2020's style 2 distance (upstream
/// src/dos_gameplay.c `ApplyGameStyle`).
const GOALKEEPING: Goalkeeping = Goalkeeping {
    save_distance: 16,
    ..amiga::GOALKEEPING
};

/// The Amiga's ball under a moon's gravity: it stays up about 1.8 times as long (upstream
/// `ApplyGameStyle`, SWOS 2020's moon).
const BALL: BallPhysics = BallPhysics {
    gravity: 2560,
    ..amiga::BALL
};

impl GameStyle for Moon50Hz {
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
        Amiga50Hz.frames_per_second()
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
        &amiga::TIMERS
    }

    fn delta_scale(&self) -> DeltaScale {
        DeltaScale::UNSCALED
    }
}
