//! What SWOS 2020's two 60 Hz styles share: the Amiga's game at 60 frames a second.

use openswos_math::DeltaScale;

use crate::values::Timers;

/// The 60 Hz styles' frames a second (upstream `DosGameplay_TargetFps`).
pub(super) const FRAMES_PER_SECOND: u16 = 60;

/// The Amiga's gravity a frame × 50/60 (upstream src/dos_gameplay.c `ApplyGameStyle`).
pub(super) const GRAVITY: u32 = 3840;

/// Every step × 50/60, multiplied then divided as C truncates (upstream `ApplyGameStyle`;
/// SWOS 2020's `D1 * 50 / 60`).
pub(super) const DELTA_SCALE: DeltaScale = match DeltaScale::new(50, 60) {
    Ok(scale) => scale,
    Err(_) => panic!("50/60 is a scale"),
};

/// The Amiga's timers × 1.2, 60 frames a second for 50 (upstream `ApplyGameStyle`, the XBOX
/// column: SWOS 2020's style 4 where it has a value, the same × 1.2 where it does not). The
/// bench menu's delay stays the Amiga's.
pub(super) const TIMERS: Timers = Timers {
    frames_per_game_second: 59,
    end_game_counter: 60,
    penalty_timer: 120,
    penalties_keeper_free: 60,
    halftime_stoppage: 420,
    ball_out_of_game_timer: 600,
    result_auto_advance: 720,
    kickoff_stoppage: 900,
    stoppage_event_reload: 60,
    result_halftime_length: 300,
    result_break_length: 180,
    subs_movement_delay: 2,
    camera_leaving_subs: 60,
    player_down_timer: 60,
    throw_in_hide_result: 60,
    stoppage_shower_pan: 120,
    stoppage_half_ended: 300,
    stoppage_halftime: 840,
    stoppage_game_over: 1800,
};
