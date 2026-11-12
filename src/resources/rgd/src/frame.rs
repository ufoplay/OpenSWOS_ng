//! A frame: the DOS game's state after one frame of the match (swos-port `FrameV1p0` and
//! its later additions). It is kept as its bytes; its fields are read by name through the
//! schema (`crate::schema`), and the frame's own scalars have accessors.
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 4 | the frame's number |
//! | 4 | 2 + 2 | player 1's and player 2's controls |
//! | 8, 164 | 145 + 11 | the top and bottom teams' general info and players |
//! | 320, 2024 | 1704 | the top and bottom teams' in-game data |
//! | 3728, 3742 | 14 | the top and bottom teams' statistics |
//! | 3756 | 4 + 4 | the camera's position (16.16 fixed point) |
//! | 3764 | 1 + 1 | its velocities |
//! | 3766 | 4 | the clock's digits |
//! | 3770 | 4 | the two teams' goals, the game's state, its player state |
//! | 3774 | 33 × 110 | the sprites |
//! | 7404 | 71 | 1.3+: the bench's state |
//! | 7475 | 6 | 1.4+: the key, fire blocked, the stats and result timers |
//! | 7481 | 4 | 1.5+: the two tap timeouts |

use bitflags::bitflags;

use crate::schema::{CAMERA_CLOCK, FieldType, FrameField, GAME_TIME_DIGITS, OFF_CAMERA};
use crate::version::FRAME_V1P0_LEN;

/// Where a frame has player 1's controls.
const PLAYER1_CONTROLS: usize = 4;

/// Where it has player 2's.
const PLAYER2_CONTROLS: usize = 6;

/// Where the camera has its Y, after its X.
const CAMERA_Y: usize = 4;

/// Where the camera has its X velocity.
const CAMERA_X_VELOCITY: usize = 8;

/// Where it has its Y velocity.
const CAMERA_Y_VELOCITY: usize = 9;

/// Where the clock is followed by the goals and the game's state.
const CLOCK_END: usize = OFF_CAMERA + CAMERA_CLOCK + GAME_TIME_DIGITS;

/// The clock's tens of minutes are worth ten.
const TENS: u16 = 10;

/// Its hundreds, a hundred.
const HUNDREDS: u16 = 100;

bitflags! {
    /// A player's controls in a frame (swos-port's bitfield).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Controls: u16 {
        /// Up.
        const UP = 1;
        /// Down.
        const DOWN = 2;
        /// Left.
        const LEFT = 4;
        /// Right.
        const RIGHT = 8;
        /// Fire.
        const FIRE = 32;
        /// The bench.
        const BENCH = 64;
        /// Escape.
        const ESCAPE = 128;
        // Bits no control uses are kept as read.
        const _ = !0;
    }
}

/// A frame of a recording.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RgdFrame {
    bytes: Vec<u8>,
}

impl RgdFrame {
    /// The frame of `bytes`, which are its version's frame size (at least a 1.0 frame's).
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        debug_assert!(bytes.len() >= FRAME_V1P0_LEN);
        Self { bytes }
    }

    /// Its bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn array<const N: usize>(&self, offset: usize) -> [u8; N] {
        self.bytes
            .get(offset..offset + N)
            .and_then(|bytes| bytes.try_into().ok())
            .expect("a frame holds a 1.0 frame's fields")
    }

    fn byte(&self, offset: usize) -> u8 {
        let [byte] = self.array(offset);
        byte
    }

    /// Its number.
    #[must_use]
    pub fn frame_no(&self) -> u32 {
        u32::from_le_bytes(self.array(0))
    }

    /// Player 1's controls.
    #[must_use]
    pub fn player1_controls(&self) -> Controls {
        Controls::from_bits_retain(u16::from_le_bytes(self.array(PLAYER1_CONTROLS)))
    }

    /// Player 2's controls.
    #[must_use]
    pub fn player2_controls(&self) -> Controls {
        Controls::from_bits_retain(u16::from_le_bytes(self.array(PLAYER2_CONTROLS)))
    }

    /// The camera's X: the view's left edge, 16.16 fixed point.
    #[must_use]
    pub fn camera_x(&self) -> i32 {
        i32::from_le_bytes(self.array(OFF_CAMERA))
    }

    /// The camera's Y: the view's top edge, 16.16 fixed point.
    #[must_use]
    pub fn camera_y(&self) -> i32 {
        i32::from_le_bytes(self.array(OFF_CAMERA + CAMERA_Y))
    }

    /// The camera's X velocity.
    #[must_use]
    pub fn camera_x_velocity(&self) -> i8 {
        self.byte(OFF_CAMERA + CAMERA_X_VELOCITY).cast_signed()
    }

    /// The camera's Y velocity.
    #[must_use]
    pub fn camera_y_velocity(&self) -> i8 {
        self.byte(OFF_CAMERA + CAMERA_Y_VELOCITY).cast_signed()
    }

    /// The clock's digits: 0, hundreds, tens and units of minutes.
    #[must_use]
    pub fn game_time(&self) -> [u8; GAME_TIME_DIGITS] {
        self.array(OFF_CAMERA + CAMERA_CLOCK)
    }

    /// The clock's minutes.
    #[must_use]
    pub fn minutes(&self) -> u16 {
        let [_, hundreds, tens, units] = self.game_time().map(u16::from);
        units + TENS * tens + HUNDREDS * hundreds
    }

    /// The first team's goals.
    #[must_use]
    pub fn team1_goals(&self) -> u8 {
        self.byte(CLOCK_END)
    }

    /// The second team's goals.
    #[must_use]
    pub fn team2_goals(&self) -> u8 {
        self.byte(CLOCK_END + 1)
    }

    /// The game's state.
    #[must_use]
    pub fn game_state(&self) -> u8 {
        self.byte(CLOCK_END + 2)
    }

    /// The game's player state.
    #[must_use]
    pub fn game_state_pl(&self) -> u8 {
        self.byte(CLOCK_END + 3)
    }

    /// The value of `field`, if the frame holds it (a field of a later version does not).
    #[must_use]
    pub fn value(&self, field: &FrameField) -> Option<i64> {
        let bytes = self
            .bytes
            .get(field.offset..field.offset + field.field_type.byte_len())?;
        Some(match field.field_type {
            FieldType::U8 => i64::from(*bytes.first()?),
            FieldType::S8 => i64::from(bytes.first()?.cast_signed()),
            FieldType::U16 => i64::from(u16::from_le_bytes(bytes.try_into().ok()?)),
            FieldType::S16 => i64::from(i16::from_le_bytes(bytes.try_into().ok()?)),
            FieldType::U32 => i64::from(u32::from_le_bytes(bytes.try_into().ok()?)),
            FieldType::S32 => i64::from(i32::from_le_bytes(bytes.try_into().ok()?)),
        })
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into a frame the tests build
mod tests {
    use super::*;
    use crate::schema::{Level, frame_fields};
    use crate::version::RgdVersion;

    fn frame() -> RgdFrame {
        let mut bytes = vec![0; FRAME_V1P0_LEN];
        bytes[0..4].copy_from_slice(&7u32.to_le_bytes());
        bytes[4..6].copy_from_slice(&(Controls::UP | Controls::FIRE).bits().to_le_bytes());
        bytes[OFF_CAMERA..OFF_CAMERA + 4].copy_from_slice(&(176i32 << 16).to_le_bytes());
        bytes[OFF_CAMERA + 8] = 0xFF;
        bytes[OFF_CAMERA + 10..OFF_CAMERA + 14].copy_from_slice(&[0, 1, 2, 3]);
        bytes[OFF_CAMERA + 14] = 3;
        RgdFrame::new(bytes)
    }

    #[test]
    fn reads_the_frames_scalars() {
        let frame = frame();
        assert_eq!(frame.frame_no(), 7);
        assert_eq!(frame.player1_controls(), Controls::UP | Controls::FIRE);
        assert_eq!(frame.camera_x() >> 16, 176);
        assert_eq!(frame.camera_x_velocity(), -1);
        assert_eq!(frame.minutes(), 123);
        assert_eq!(frame.team1_goals(), 3);
    }

    #[test]
    fn fields_read_by_name() {
        let frame = frame();
        let fields = frame_fields(RgdVersion::new(1, 5).expect("a version"), Level::Scalars);
        let value = |name: &str| {
            let field = fields
                .iter()
                .find(|field| field.name == name)
                .expect("a field");
            frame.value(field)
        };
        assert_eq!(value("frame_no"), Some(7));
        assert_eq!(value("camera_x_vel"), Some(-1));
        assert_eq!(value("game_time[3]"), Some(3));
        assert_eq!(value("team1_goals"), Some(3));
        assert_eq!(value("user_key"), None, "a 1.0 frame has no 1.4 words");
    }
}
