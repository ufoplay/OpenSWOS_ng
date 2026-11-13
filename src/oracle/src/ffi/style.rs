//! src/dos_gameplay.c: the game styles.

use std::ffi::CStr;

use super::bindings::dos_gameplay_c as style_c;
use super::stubs::{self, NO_FORMATIONS};
use crate::style::{GsTimers, StyleGlobals};

/// Upstream's number of the Amiga's style.
const AMIGA: i32 = 0;

/// The tweak bit of the stronger keepers (upstream include/tweaks.h).
pub const TWEAK_STRONGER_GOALKEEPER_AI: u32 = style_c::TWEAK_STRONGER_GOALKEEPER_AI;

/// Upstream's number of styles (`GAMEPLAY_STYLES`): each style's number is below it.
pub const GAMEPLAY_STYLES: u32 = style_c::GAMEPLAY_STYLES;

/// Resets the unit's globals to upstream's values: the Amiga's style, whose values are the
/// globals' defaults (include/dos_gameplay.h), with no tweak on; then forgets the formations
/// it selected.
pub(super) fn reset() {
    set_style(AMIGA);
    apply_game_style();
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { stubs::FORMATIONS = NO_FORMATIONS };
}

/// Turns on the tweaks of `tweaks`, as `Tweaks_On` reports them.
pub(crate) fn set_tweaks(tweaks: u32) {
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { stubs::TWEAKS = tweaks };
}

/// `Gameplay_SetStyle(style)`.
pub(crate) fn set_style(style: i32) {
    // SAFETY: writes globals the lock's holder owns; any number is defined (it falls back to
    // the Amiga's).
    unsafe { style_c::Gameplay_SetStyle(style) };
}

/// `ApplyGameStyle()`.
pub(crate) fn apply_game_style() {
    // SAFETY: writes globals the lock's holder owns, within their sizes.
    unsafe { style_c::ApplyGameStyle() };
}

/// The globals the two set, read back.
pub(crate) fn globals() -> StyleGlobals {
    // SAFETY: plain reads of globals the lock's holder owns, the arrays copied whole; the
    // style's name is a static string.
    unsafe {
        let style = style_c::_gGameplayStyle;
        let name = CStr::from_ptr(style_c::Gameplay_StyleName(style));
        StyleGlobals {
            style,
            option: style_c::_gameplayStyleOption,
            dos_gameplay: stubs::_gDosGameplay != 0,
            dos_keepers: style_c::_gDosKeepers != 0,
            frames_per_second: style_c::DosGameplay_TargetFps(),
            name: name.to_string_lossy().into_owned(),
            dos_formations: (stubs::FORMATIONS != NO_FORMATIONS).then_some(stubs::FORMATIONS != 0),
            keeper_save_distance: { style_c::_kKeeperSaveDistance }[0],
            goalkeeper_dive_deltas: style_c::_kGoalkeeperDiveDeltas,
            keeper_loose_ball_min_delta_z: style_c::_gsKeeperLooseBallMinDeltaZ,
            ball_ground_constant: { style_c::_kBallGroundConstant }[0],
            ball_air_constant: { style_c::_kBallAirConstant }[0],
            gravity_constant: { style_c::_kGravityConstant }[0],
            pitch_ball_speed_influence: { style_c::_kPitchBallSpeedInfluence }
                .map(u16::cast_signed),
            timers: GsTimers {
                frames_per_game_second: style_c::_gsFramesPerGameSecond,
                end_game_counter: style_c::_gsEndGameCounter,
                ball_defensive_z: style_c::_gsBallDefensiveZ,
                penalty_timer: style_c::_gsPenaltyTimer,
                penalties_keeper_free: style_c::_gsPenaltiesKeeperFree,
                halftime_stoppage: style_c::_gsHalftimeStoppage,
                ball_out_of_game_timer: style_c::_gsBallOutOfGameTimer,
                result_auto_advance: style_c::_gsResultAutoAdvance,
                kickoff_stoppage: style_c::_gsKickoffStoppage,
                stoppage_event_reload: style_c::_gsStoppageEventReload,
                result_halftime_length: style_c::_gsResultHalftimeLength,
                result_break_length: style_c::_gsResultBreakLength,
                subs_movement_delay: style_c::_gsSubsMovementDelay,
                camera_leaving_subs: style_c::_gsCameraLeavingSubs,
                player_down_timer: style_c::_gsPlayerDownTimer,
                throw_in_hide_result: style_c::_gsThrowInHideResult,
                stoppage_shower_pan: style_c::_gsStoppageShowerPan,
                stoppage_half_ended: style_c::_gsStoppageHalfEnded,
                stoppage_halftime: style_c::_gsStoppageHalftime,
                stoppage_game_over: style_c::_gsStoppageGameOver,
            },
            move_delta_scale: (style_c::_gMoveDeltaScaleNum, style_c::_gMoveDeltaScaleDen),
        }
    }
}
