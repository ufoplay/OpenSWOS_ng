//! The game styles (upstream src/dos_gameplay.c): `Gameplay_SetStyle`, `ApplyGameStyle`, and
//! the globals they set.

use crate::Oracle;
use crate::ffi;

pub use crate::ffi::style::{GAMEPLAY_STYLES, TWEAK_STRONGER_GOALKEEPER_AI};

/// The entries of the goalkeeper's reach table (`_kGoalkeeperDiveDeltas[8]`).
pub const DIVE_DELTAS: usize = 8;

/// The entries of the pitch types' ball friction (`_kPitchBallSpeedInfluence[7]`).
pub const PITCH_TYPES: usize = 7;

/// The `_gs*` timers and thresholds `ApplyGameStyle` writes, by their upstream names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct GsTimers {
    pub frames_per_game_second: i16,
    pub end_game_counter: i16,
    pub ball_defensive_z: i16,
    pub penalty_timer: i16,
    pub penalties_keeper_free: i16,
    pub halftime_stoppage: i16,
    pub ball_out_of_game_timer: i16,
    pub result_auto_advance: i16,
    pub kickoff_stoppage: i16,
    pub stoppage_event_reload: i16,
    pub result_halftime_length: i16,
    pub result_break_length: i16,
    pub subs_movement_delay: i16,
    pub camera_leaving_subs: i16,
    pub player_down_timer: i16,
    pub throw_in_hide_result: i16,
    pub stoppage_shower_pan: i16,
    pub stoppage_half_ended: i16,
    pub stoppage_halftime: i16,
    pub stoppage_game_over: i16,
}

/// What `Gameplay_SetStyle` and `ApplyGameStyle` set, read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleGlobals {
    /// The style's number (`_gGameplayStyle`).
    pub style: i32,
    /// The options menu's mirror of it (`_gameplayStyleOption`).
    pub option: u16,
    /// Whether the DOS release's code runs (`_gDosGameplay`).
    pub dos_gameplay: bool,
    /// Whether the DOS goalkeeper code runs (`_gDosKeepers`).
    pub dos_keepers: bool,
    /// The match's frames a second (`DosGameplay_TargetFps()`).
    pub frames_per_second: i32,
    /// The style's name (`Gameplay_StyleName(_gGameplayStyle)`).
    pub name: String,
    /// The formations `ApplyGameStyle` selected (`TacticsData_SelectRelease`'s argument): the
    /// DOS release's or not; `None` when not called.
    pub dos_formations: Option<bool>,
    /// `_kKeeperSaveDistance`.
    pub keeper_save_distance: u16,
    /// `_kGoalkeeperDiveDeltas`.
    pub goalkeeper_dive_deltas: [u32; DIVE_DELTAS],
    /// `_gsKeeperLooseBallMinDeltaZ`.
    pub keeper_loose_ball_min_delta_z: i32,
    /// `_kBallGroundConstant`.
    pub ball_ground_constant: u16,
    /// `_kBallAirConstant`.
    pub ball_air_constant: u16,
    /// `_kGravityConstant`.
    pub gravity_constant: u32,
    /// `_kPitchBallSpeedInfluence`, its words read as signed.
    pub pitch_ball_speed_influence: [i16; PITCH_TYPES],
    /// The `_gs*` timers.
    pub timers: GsTimers,
    /// The movement scale (`_gMoveDeltaScaleNum`, `_gMoveDeltaScaleDen`).
    pub move_delta_scale: (i16, i16),
}

impl Oracle {
    /// Turns on the tweaks of `tweaks` (include/tweaks.h's bits), as `Tweaks_On` reports
    /// them.
    pub fn set_tweaks(&mut self, tweaks: u32) {
        ffi::style::set_tweaks(tweaks);
    }

    /// `Gameplay_SetStyle(style)`: the style's number and the flags that follow it. A number
    /// upstream does not know sets the Amiga's.
    pub fn set_style(&mut self, style: i32) {
        ffi::style::set_style(style);
    }

    /// `ApplyGameStyle()`: writes the style's constants.
    pub fn apply_game_style(&mut self) {
        ffi::style::apply_game_style();
    }

    /// The globals the two set.
    #[must_use]
    pub fn style_globals(&self) -> StyleGlobals {
        ffi::style::globals()
    }
}
