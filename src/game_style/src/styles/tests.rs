//! The relations between the styles their documents state; each value is checked against
//! upstream by the oracle.

use openswos_math::DeltaScale;

use super::{Amiga50Hz, Amiga60Hz, Dos70Hz, Moon50Hz, Xbox60Hz};
use crate::style::{GameStyle, Release};
use crate::values::{BallPhysics, Goalkeeping};

/// The 60 Hz movement scale: 50 frames' steps over 60.
const AT_60_HZ: (i16, i16) = (50, 60);

#[test]
fn the_dos_game_runs_its_own_code_scaled_by_41_64() {
    let style = Dos70Hz;
    assert_eq!(
        (style.release(), style.goalkeepers(), style.formations()),
        (Release::Dos, Release::Dos, Release::Dos)
    );
    assert_eq!(style.delta_scale(), DeltaScale::DOS);
    assert!(style.frames_per_second() > Amiga50Hz.frames_per_second());
}

#[test]
fn xbox_is_the_amigas_game_with_the_dos_keepers() {
    let style = Xbox60Hz;
    assert_eq!(
        (style.release(), style.goalkeepers(), style.formations()),
        (Release::Amiga, Release::Dos, Release::Amiga)
    );
    let BallPhysics {
        ground_friction,
        air_friction,
        gravity: _,
        pitch_friction,
    } = *style.ball();
    let amiga = Amiga50Hz.ball();
    assert_eq!(
        (ground_friction, air_friction, pitch_friction),
        (
            amiga.ground_friction,
            amiga.air_friction,
            amiga.pitch_friction
        )
    );
    let dos = Dos70Hz.goalkeeping();
    assert_eq!(
        (
            style.goalkeeping().save_distance,
            style.goalkeeping().catch_height
        ),
        (dos.save_distance, dos.catch_height)
    );
}

#[test]
fn the_60_hz_styles_share_their_clock_scale_and_gravity() {
    let scale = DeltaScale::new(AT_60_HZ.0, AT_60_HZ.1).expect("a scale");
    for style in [&Xbox60Hz as &dyn GameStyle, &Amiga60Hz] {
        assert_eq!(style.delta_scale(), scale, "{}", style.id());
        assert_eq!(style.timers(), Xbox60Hz.timers());
        assert_eq!(style.ball().gravity, Xbox60Hz.ball().gravity);
        assert_eq!(style.frames_per_second(), Xbox60Hz.frames_per_second());
    }
}

#[test]
fn amiga60_is_the_amigas_game_but_for_its_reach() {
    let Goalkeeping {
        save_distance,
        catch_height,
        dive_deltas,
        loose_ball_min_delta_z,
    } = *Amiga60Hz.goalkeeping();
    let amiga = Amiga50Hz.goalkeeping();
    assert_eq!(
        (save_distance, catch_height, loose_ball_min_delta_z),
        (
            amiga.save_distance,
            amiga.catch_height,
            amiga.loose_ball_min_delta_z
        )
    );
    assert_ne!(dive_deltas, amiga.dive_deltas);
    assert_eq!(Amiga60Hz.goalkeepers(), Release::Amiga);
}

#[test]
fn moon_is_the_amigas_but_for_its_gravity_and_save_distance() {
    let (moon, amiga) = (Moon50Hz, Amiga50Hz);
    assert_eq!(moon.timers(), amiga.timers());
    assert_eq!(moon.delta_scale(), amiga.delta_scale());
    assert_eq!(moon.frames_per_second(), amiga.frames_per_second());
    let ball = BallPhysics {
        gravity: amiga.ball().gravity,
        ..*moon.ball()
    };
    assert_eq!(&ball, amiga.ball());
    assert!(moon.ball().gravity < amiga.ball().gravity);
    let goalkeeping = Goalkeeping {
        save_distance: amiga.goalkeeping().save_distance,
        ..*moon.goalkeeping()
    };
    assert_eq!(&goalkeeping, amiga.goalkeeping());
}
