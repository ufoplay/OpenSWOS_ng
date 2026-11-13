//! `openswos_game_style` against upstream's src/dos_gameplay.c: every style, with and without
//! the stronger keepers, sets what `Gameplay_SetStyle` and `ApplyGameStyle` set; a number
//! upstream does not know plays the default. Without upstream's checkout the oracle is not
//! built and these tests are not either (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

use openswos_game_style::{
    BallPhysics, DEFAULT, GameStyle, Goalkeeping, KeeperReach, Release, STYLES, Timers, by_code,
};
use openswos_oracle::Oracle;
use openswos_oracle::style::{
    GAMEPLAY_STYLES, GsTimers, StyleGlobals, TWEAK_STRONGER_GOALKEEPER_AI,
};

/// Every reach, with the tweaks that give it.
const REACHES: [(KeeperReach, u32); 2] = [
    (KeeperReach::Standard, 0),
    (KeeperReach::Stronger, TWEAK_STRONGER_GOALKEEPER_AI),
];

/// What upstream sets for `style` at `reach`, as the port says.
fn expected(style: &dyn GameStyle, reach: KeeperReach) -> StyleGlobals {
    let Goalkeeping {
        save_distance,
        catch_height,
        dive_deltas: _,
        loose_ball_min_delta_z,
    } = *style.goalkeeping();
    let BallPhysics {
        ground_friction,
        air_friction,
        gravity,
        pitch_friction,
    } = *style.ball();
    let Timers {
        frames_per_game_second,
        end_game_counter,
        penalty_timer,
        penalties_keeper_free,
        halftime_stoppage,
        ball_out_of_game_timer,
        result_auto_advance,
        kickoff_stoppage,
        stoppage_event_reload,
        result_halftime_length,
        result_break_length,
        subs_movement_delay,
        camera_leaving_subs,
        player_down_timer,
        throw_in_hide_result,
        stoppage_shower_pan,
        stoppage_half_ended,
        stoppage_halftime,
        stoppage_game_over,
    } = *style.timers();
    let is_dos = |release: Release| match release {
        Release::Amiga => false,
        Release::Dos => true,
    };
    StyleGlobals {
        style: i32::from(style.code()),
        option: u16::from(style.code()),
        dos_gameplay: is_dos(style.release()),
        dos_keepers: is_dos(style.goalkeepers()),
        frames_per_second: i32::from(style.frames_per_second()),
        name: style.name().to_owned(),
        dos_formations: Some(is_dos(style.formations())),
        keeper_save_distance: save_distance,
        goalkeeper_dive_deltas: style.dive_deltas(reach),
        keeper_loose_ball_min_delta_z: loose_ball_min_delta_z,
        ball_ground_constant: ground_friction,
        ball_air_constant: air_friction,
        gravity_constant: gravity,
        pitch_ball_speed_influence: pitch_friction,
        timers: GsTimers {
            frames_per_game_second,
            end_game_counter,
            ball_defensive_z: catch_height,
            penalty_timer,
            penalties_keeper_free,
            halftime_stoppage,
            ball_out_of_game_timer,
            result_auto_advance,
            kickoff_stoppage,
            stoppage_event_reload,
            result_halftime_length,
            result_break_length,
            subs_movement_delay,
            camera_leaving_subs,
            player_down_timer,
            throw_in_hide_result,
            stoppage_shower_pan,
            stoppage_half_ended,
            stoppage_halftime,
            stoppage_game_over,
        },
        move_delta_scale: (
            style.delta_scale().numerator(),
            style.delta_scale().denominator(),
        ),
    }
}

/// What upstream sets for its style `number` with `tweaks` on.
fn upstream(number: i32, tweaks: u32) -> StyleGlobals {
    let mut oracle = Oracle::lock();
    oracle.set_tweaks(tweaks);
    oracle.set_style(number);
    oracle.apply_game_style();
    oracle.style_globals()
}

#[test]
fn every_style_sets_what_upstream_sets() {
    for style in STYLES {
        for (reach, tweaks) in REACHES {
            assert_eq!(
                upstream(i32::from(style.code()), tweaks),
                expected(style, reach),
                "{} {reach:?}",
                style.id()
            );
        }
    }
}

#[test]
fn a_number_upstream_does_not_know_plays_the_default() {
    let past = i32::try_from(GAMEPLAY_STYLES).expect("a few styles");
    for number in [-1, past, i32::from(u8::MAX), i32::MAX, i32::MIN] {
        assert_eq!(
            upstream(number, 0),
            expected(DEFAULT, KeeperReach::Standard),
            "style {number}"
        );
        if let Ok(code) = u8::try_from(number) {
            assert!(by_code(code).is_none(), "style {number}");
        }
    }
}

#[test]
fn the_lock_resets_to_the_amigas_style() {
    {
        let mut oracle = Oracle::lock();
        oracle.set_tweaks(TWEAK_STRONGER_GOALKEEPER_AI);
        oracle.set_style(i32::from(openswos_game_style::Dos70Hz.code()));
        oracle.apply_game_style();
    }
    let oracle = Oracle::lock();
    let globals = oracle.style_globals();
    // The reset applied the Amiga's style, and forgot the formations it selected.
    let amiga = StyleGlobals {
        dos_formations: None,
        ..expected(DEFAULT, KeeperReach::Standard)
    };
    assert_eq!(globals, amiga);
}
