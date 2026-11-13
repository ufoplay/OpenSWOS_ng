//! The Amiga's style: the definitive release, 50 frames a second.

use openswos_math::DeltaScale;

use crate::style::{GameStyle, Release};
use crate::values::{BallPhysics, Goalkeeping, Timers, dive_deltas};

/// The Amiga's style (upstream `GAMEPLAY_AMIGA`): the default, and what the original's
/// globals hold before any style is applied.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Amiga50Hz;

/// Its id (`[gameplay] style = amiga`).
const ID: &str = "amiga";

/// Its name (upstream `Gameplay_StyleName`).
const NAME: &str = "AMIGA";

/// Its frames a second (upstream `DosGameplay_TargetFps`).
const FRAMES_PER_SECOND: u16 = 50;

/// The Amiga's goalkeepers (upstream src/dos_gameplay.c `ApplyGameStyle`, the Amiga column;
/// the globals' defaults in src/swos_amiga_data_imports.c and src/dos_gameplay.c).
pub(super) const GOALKEEPING: Goalkeeping = Goalkeeping {
    save_distance: 24,
    catch_height: 22,
    dive_deltas: dive_deltas(0x30000),
    loose_ball_min_delta_z: -0x8000,
};

/// The Amiga's ball (as [`GOALKEEPING`]).
pub(super) const BALL: BallPhysics = BallPhysics {
    ground_friction: 16,
    air_friction: 10,
    gravity: 4608,
    pitch_friction: [-2, 2, 3, 0, 0, -1, -1],
};

/// The Amiga's timers (as [`GOALKEEPING`]; SWOS2.s).
pub(super) const TIMERS: Timers = Timers {
    frames_per_game_second: 49,
    end_game_counter: 50,
    penalty_timer: 100,
    penalties_keeper_free: 50,
    halftime_stoppage: 350,
    ball_out_of_game_timer: 500,
    result_auto_advance: 600,
    kickoff_stoppage: 750,
    stoppage_event_reload: 50,
    result_halftime_length: 250,
    result_break_length: 150,
    subs_movement_delay: 2,
    camera_leaving_subs: 50,
    player_down_timer: 50,
    throw_in_hide_result: 50,
    stoppage_shower_pan: 100,
    stoppage_half_ended: 250,
    stoppage_halftime: 700,
    stoppage_game_over: 1500,
};

impl GameStyle for Amiga50Hz {
    fn id(&self) -> &'static str {
        ID
    }

    fn code(&self) -> u8 {
        0
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn frames_per_second(&self) -> u16 {
        FRAMES_PER_SECOND
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
        &TIMERS
    }

    fn delta_scale(&self) -> DeltaScale {
        DeltaScale::UNSCALED
    }
}
