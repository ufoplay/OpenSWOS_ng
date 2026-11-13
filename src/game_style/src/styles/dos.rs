//! The DOS game's style: the PC release, 70 frames a second.

use openswos_math::DeltaScale;

use crate::style::{GameStyle, Release};
use crate::values::{BallPhysics, Goalkeeping, Timers, dive_deltas};

/// The DOS game's style (upstream `GAMEPLAY_DOS`): its physics, timers, goalkeepers and
/// formations, pre-scaled so it plays like the Amiga's at more frames a second, and its own
/// code where the releases differ.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Dos70Hz;

/// Its id (`[gameplay] style = dos`).
const ID: &str = "dos";

/// Its number (upstream `GAMEPLAY_DOS`).
const CODE: u8 = 1;

/// Its name (upstream `Gameplay_StyleName`).
const NAME: &str = "DOS";

/// Its frames a second (upstream `DosGameplay_TargetFps`).
const FRAMES_PER_SECOND: u16 = 70;

/// The DOS goalkeepers (upstream src/dos_gameplay.c `ApplyGameStyle`, the DOS column, from
/// SWOS 2020's `setGameStyleVariables` and the PC release's disassembly): they react at a
/// shorter range, the whole reach table a step lower.
const GOALKEEPING: Goalkeeping = Goalkeeping {
    save_distance: 16,
    catch_height: 27,
    dive_deltas: dive_deltas(0x28000),
    loose_ball_min_delta_z: -0x8000,
};

/// The DOS ball (as [`GOALKEEPING`]).
const BALL: BallPhysics = BallPhysics {
    ground_friction: 13,
    air_friction: 4,
    gravity: 3291,
    pitch_friction: [-3, 4, 1, 0, 0, -1, -1],
};

/// The DOS timers (as [`GOALKEEPING`]): mostly the Amiga's × 1.1, each one checked against
/// the PC release; its match clock measured on a recording (70 frames a game second's
/// reload, where SWOS 2020 says 54).
const TIMERS: Timers = Timers {
    frames_per_game_second: 70,
    end_game_counter: 55,
    penalty_timer: 110,
    penalties_keeper_free: 55,
    halftime_stoppage: 385,
    ball_out_of_game_timer: 550,
    result_auto_advance: 660,
    kickoff_stoppage: 825,
    stoppage_event_reload: 55,
    result_halftime_length: 275,
    result_break_length: 165,
    subs_movement_delay: 4,
    camera_leaving_subs: 55,
    player_down_timer: 55,
    throw_in_hide_result: 55,
    stoppage_shower_pan: 110,
    stoppage_half_ended: 275,
    stoppage_halftime: 770,
    stoppage_game_over: 1650,
};

impl GameStyle for Dos70Hz {
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
        FRAMES_PER_SECOND
    }

    fn release(&self) -> Release {
        Release::Dos
    }

    fn goalkeepers(&self) -> Release {
        Release::Dos
    }

    fn formations(&self) -> Release {
        Release::Dos
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
        DeltaScale::DOS
    }
}
