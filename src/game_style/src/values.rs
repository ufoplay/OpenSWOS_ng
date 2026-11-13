//! The constants a style sets, grouped as the match reads them (upstream src/dos_gameplay.c
//! `ApplyGameStyle`, which writes them into globals at kick-off).

/// The entries of the goalkeeper's reach table.
pub const DIVE_DELTAS: usize = 8;

/// The step between two entries of the reach table, 16.16 fixed point (upstream
/// `ApplyGameStyle`: `+ i * 0x8000`).
pub const DIVE_DELTA_STEP: u32 = 0x8000;

/// How much further the Amiga's goalkeepers reach with the stronger keepers tweak: the
/// whole table a step lower (upstream `ApplyGameStyle`, SWOS 2020's
/// `--stronger-goalkeeper-ai`).
pub const STRONGER_KEEPER_STEP: u32 = DIVE_DELTA_STEP;

/// The pitch types the ball's friction is set for: frozen, muddy, wet, soft, normal, dry,
/// hard.
pub const PITCH_TYPES: usize = 7;

/// The goalkeepers' constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Goalkeeping {
    /// How near the ball must come for the keeper to dive or save (upstream
    /// `_kKeeperSaveDistance`).
    pub save_distance: u16,
    /// The ball's height under which the keeper catches it (`_gsBallDefensiveZ`).
    pub catch_height: i16,
    /// The keeper's reach table, from its first entry by [`DIVE_DELTA_STEP`], 16.16 fixed
    /// point (`_kGoalkeeperDiveDeltas`). [`crate::GameStyle::dive_deltas`] applies the
    /// stronger keepers tweak.
    pub dive_deltas: [u32; DIVE_DELTAS],
    /// The lowest vertical speed of a slow, low ball near the keeper that he goes for, 16.16
    /// fixed point; the highest is `0x8000` (`_gsKeeperLooseBallMinDeltaZ`).
    pub loose_ball_min_delta_z: i32,
}

/// The ball's physics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BallPhysics {
    /// The speed the ball loses each frame on the ground (upstream `_kBallGroundConstant`).
    pub ground_friction: u16,
    /// The speed the ball loses each frame in the air, instead (`_kBallAirConstant`).
    pub air_friction: u16,
    /// The vertical speed the ball loses each frame, 16.16 fixed point (`_kGravityConstant`).
    pub gravity: u32,
    /// What each pitch type, frozen to hard, adds to the ground friction while nobody has the
    /// ball (`_kPitchBallSpeedInfluence`).
    pub pitch_friction: [i16; PITCH_TYPES],
}

/// The match clock and the stoppage timers, in frames (upstream's `_gs*` globals, each
/// named after the release's two values: `GS_<DOS>_<Amiga>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timers {
    /// The match clock's reload: every frame the clock takes the game length's step, and an
    /// in-game second passes each time it adds this back (`_gsFramesPerGameSecond`).
    pub frames_per_game_second: i16,
    /// The reload of the last minute's counter (`_gsEndGameCounter`).
    pub end_game_counter: i16,
    /// How long a penalty waits before the next one (`_gsPenaltyTimer`).
    pub penalty_timer: i16,
    /// In a penalty shoot-out, when the goalkeeper is let go (`_gsPenaltiesKeeperFree`).
    pub penalties_keeper_free: i16,
    /// How long a result other than half or full time stays up (`_gsHalftimeStoppage`).
    pub halftime_stoppage: i16,
    /// How long the ball stays out of play (`_gsBallOutOfGameTimer`).
    pub ball_out_of_game_timer: i16,
    /// How long the half or full time result stays up (`_gsResultAutoAdvance`).
    pub result_auto_advance: i16,
    /// The kick-off's stoppage (`_gsKickoffStoppage`).
    pub kickoff_stoppage: i16,
    /// The reload of the stoppage event timer (`_gsStoppageEventReload`).
    pub stoppage_event_reload: i16,
    /// How long the half time result banner stays up (`_gsResultHalftimeLength`).
    pub result_halftime_length: i16,
    /// How long a break's result banner stays up (`_gsResultBreakLength`).
    pub result_break_length: i16,
    /// How long a held direction waits before the bench menu's cursor steps again
    /// (`_gsSubsMovementDelay`). The DOS game doubled it: a feel change, not the frame rate.
    pub subs_movement_delay: i16,
    /// How long the camera takes to come back from the bench (`_gsCameraLeavingSubs`).
    pub camera_leaving_subs: i16,
    /// How long a player stays down after a tackle or a jumping header
    /// (`_gsPlayerDownTimer`).
    pub player_down_timer: i16,
    /// How long a throw-in is left alone before the result is hidden
    /// (`_gsThrowInHideResult`).
    pub throw_in_hide_result: i16,
    /// The pause while the camera pans to the showers, and before extra time
    /// (`_gsStoppageShowerPan`).
    pub stoppage_shower_pan: i16,
    /// The pause after the first half, and while the players leave the pitch
    /// (`_gsStoppageHalfEnded`).
    pub stoppage_half_ended: i16,
    /// The pause going to half time (`_gsStoppageHalftime`).
    pub stoppage_halftime: i16,
    /// The pause at the end of the game (`_gsStoppageGameOver`).
    pub stoppage_game_over: i16,
}

/// The reach table from its first entry, a [`DIVE_DELTA_STEP`] an entry.
#[must_use]
pub const fn dive_deltas(first: u32) -> [u32; DIVE_DELTAS] {
    let mut table = [0; DIVE_DELTAS];
    let mut entry = 0;
    let mut delta = first;
    while entry < DIVE_DELTAS {
        table[entry] = delta;
        delta += DIVE_DELTA_STEP;
        entry += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::{DIVE_DELTA_STEP, DIVE_DELTAS, dive_deltas};

    /// The Amiga's first entry (upstream src/swos_amiga_data_imports.c).
    const AMIGA_FIRST: u32 = 0x30000;

    /// The Amiga's last entry.
    const AMIGA_LAST: u32 = 0x68000;

    #[test]
    fn the_table_steps_from_its_first_entry() {
        let table = dive_deltas(AMIGA_FIRST);
        assert_eq!(
            (table[0], table[DIVE_DELTAS - 1]),
            (AMIGA_FIRST, AMIGA_LAST)
        );
        assert!(
            table
                .windows(2)
                .all(|pair| pair[1] - pair[0] == DIVE_DELTA_STEP)
        );
    }
}
