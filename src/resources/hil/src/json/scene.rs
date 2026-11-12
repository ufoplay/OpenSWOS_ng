//! A scene of the JSON highlights (upstream `emit_scene`, `load_scene_slice`): the frames
//! of a 19000-byte slot of big-endian long words, as the writer decodes them, and the
//! slot's bytes after its end word.
//!
//! | word | meaning |
//! |---|---|
//! | bit 31 set | a frame's marker: camera X (15 bits) << 16 \| camera Y (16 bits) |
//! | next | the score: the first team's goals << 16 \| the second team's |
//! | next | the animation state << 16 \| the same again (the copy is not written) |
//! | bit 31 clear | a sprite: index (12 bits) << 20 \| X (10 bits) << 10 \| Y (10 bits) |
//! | `0xFFFFFFFF` | the end |

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Bytes of a scene's slot (upstream `HIL_SCENE_SIZE`).
pub const SCENE_LEN: u32 = 19_000;

/// The largest camera X: the 15 bits below a marker's bit 31.
pub const CAMERA_X_MAX: u16 = 0x7FFF;

/// The largest sprite index: 12 bits.
pub const SPRITE_INDEX_MAX: u16 = 0xFFF;

/// The largest sprite coordinate: 10 bits, unsigned (the writer masks them so).
pub const SPRITE_COORDINATE_MAX: u16 = 0x3FF;

/// A sprite of a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonSprite {
    /// Its image (0..=4095).
    pub index: u16,
    /// Its X (0..=1023).
    pub x: u16,
    /// Its Y (0..=1023).
    pub y: u16,
}

/// A frame of a scene.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonFrame {
    /// The camera's X (0..=32767).
    pub cam_x: u16,
    /// The camera's Y.
    pub cam_y: u16,
    /// The first team's goals.
    pub team_1_goals: u16,
    /// The second team's goals.
    pub team_2_goals: u16,
    /// The animation state: its word's high half. Faithful to the original: the low half,
    /// a copy of it, is not written; the loader copies the high half into it.
    pub anim_patterns_state: u16,
    /// `true` on the frame the writer makes up for sprites before a scene's first marker
    /// (its other values 0, the loader writes no marker for it); absent on every other
    /// frame. Kept as read.
    #[serde(
        rename = "_synthetic_no_frame_marker",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub synthetic_no_frame_marker: Option<bool>,
    /// The sprites, in drawing order.
    pub sprites: Vec<JsonSprite>,
}

/// A scene: its frames, and the slot's bytes after its end word.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonScene {
    /// The frames.
    pub frames: Vec<JsonFrame>,
    /// Where the bytes after the end word start: just past it; the slot's size when it
    /// holds no end word.
    pub eos_offset: u32,
    /// The slot's bytes from `eos_offset` to its end (what the slot held before), kept as
    /// written.
    pub trailing: Vec<u8>,
}

/// Why a scene is not one a slot holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum JsonSceneError {
    /// The end is past the slot.
    #[error("eos_offset {0} is past the scene's {SCENE_LEN} bytes")]
    EndPastSlot(u32),
    /// The trailing bytes are not the slot's bytes after the end.
    #[error("{trailing} trailing bytes; the slot has {expected} after eos_offset")]
    Trailing {
        /// The trailing bytes held.
        trailing: usize,
        /// The slot's bytes after the end.
        expected: u32,
    },
    /// A value wider than its bits: the loader would keep only some of them.
    #[error("frame {frame}: {field} {value} does not fit its bits (at most {max})")]
    TooWide {
        /// The frame.
        frame: usize,
        /// The value's key.
        field: &'static str,
        /// The value.
        value: u16,
        /// Its bits' largest value.
        max: u16,
    },
    /// A frame whose marker is the end word: the scene would end there.
    #[error("frame {frame}: its marker reads as the scene's end")]
    MarkerIsEnd {
        /// The frame.
        frame: usize,
    },
}

impl JsonScene {
    /// Fails when the scene is not one a slot holds: its end past the slot, its trailing
    /// bytes not the rest of the slot, a value wider than its bits, a marker reading as the
    /// end.
    ///
    /// # Errors
    ///
    /// The first such fault.
    pub fn check(&self) -> Result<(), JsonSceneError> {
        let expected = SCENE_LEN
            .checked_sub(self.eos_offset)
            .ok_or(JsonSceneError::EndPastSlot(self.eos_offset))?;
        if usize::try_from(expected).ok() != Some(self.trailing.len()) {
            return Err(JsonSceneError::Trailing {
                trailing: self.trailing.len(),
                expected,
            });
        }
        self.frames
            .iter()
            .enumerate()
            .try_for_each(|(index, frame)| frame.check(index))
    }
}

impl JsonFrame {
    /// Fails when a value is wider than its bits, or the marker is the end word.
    fn check(&self, frame: usize) -> Result<(), JsonSceneError> {
        let fits = |field, value, max| {
            if value > max {
                Err(JsonSceneError::TooWide {
                    frame,
                    field,
                    value,
                    max,
                })
            } else {
                Ok(())
            }
        };
        fits("cam_x", self.cam_x, CAMERA_X_MAX)?;
        let has_marker = self.synthetic_no_frame_marker != Some(true);
        if has_marker && self.cam_x == CAMERA_X_MAX && self.cam_y == u16::MAX {
            return Err(JsonSceneError::MarkerIsEnd { frame });
        }
        self.sprites.iter().try_for_each(|sprite| {
            fits("index", sprite.index, SPRITE_INDEX_MAX)?;
            fits("x", sprite.x, SPRITE_COORDINATE_MAX)?;
            fits("y", sprite.y, SPRITE_COORDINATE_MAX)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes after the end word of a scene of one frame and one sprite: the marker,
    /// the score, the animation state, the sprite, the end word.
    const SHORT_SCENE_END: u32 = 20;

    fn scene() -> JsonScene {
        JsonScene {
            frames: vec![JsonFrame {
                cam_x: CAMERA_X_MAX,
                cam_y: 1,
                team_1_goals: 1,
                team_2_goals: 0,
                anim_patterns_state: 2,
                synthetic_no_frame_marker: None,
                sprites: vec![JsonSprite {
                    index: SPRITE_INDEX_MAX,
                    x: SPRITE_COORDINATE_MAX,
                    y: 0,
                }],
            }],
            eos_offset: SHORT_SCENE_END,
            trailing: vec![0; usize::try_from(SCENE_LEN - SHORT_SCENE_END).expect("fits")],
        }
    }

    #[test]
    fn a_scene_fills_its_slot() {
        assert_eq!(scene().check(), Ok(()));
        let full = JsonScene {
            eos_offset: SCENE_LEN,
            trailing: Vec::new(),
            ..scene()
        };
        assert_eq!(full.check(), Ok(()));
        let past = JsonScene {
            eos_offset: SCENE_LEN + 1,
            trailing: Vec::new(),
            ..scene()
        };
        assert_eq!(
            past.check(),
            Err(JsonSceneError::EndPastSlot(SCENE_LEN + 1))
        );
        let mut short = scene();
        short.trailing.pop();
        assert_eq!(
            short.check(),
            Err(JsonSceneError::Trailing {
                trailing: short.trailing.len(),
                expected: SCENE_LEN - SHORT_SCENE_END,
            })
        );
    }

    #[test]
    fn values_fit_their_bits() {
        let mut wide = scene();
        if let Some(frame) = wide.frames.first_mut() {
            frame.cam_x = CAMERA_X_MAX + 1;
        }
        assert_eq!(
            wide.check(),
            Err(JsonSceneError::TooWide {
                frame: 0,
                field: "cam_x",
                value: CAMERA_X_MAX + 1,
                max: CAMERA_X_MAX
            })
        );
        let mut wide = scene();
        if let Some(sprite) = wide.frames.first_mut().and_then(|f| f.sprites.first_mut()) {
            sprite.y = SPRITE_COORDINATE_MAX + 1;
        }
        assert!(matches!(
            wide.check(),
            Err(JsonSceneError::TooWide { field: "y", .. })
        ));
    }

    #[test]
    fn a_marker_never_reads_as_the_end() {
        let mut end = scene();
        if let Some(frame) = end.frames.first_mut() {
            frame.cam_y = u16::MAX;
        }
        assert_eq!(end.check(), Err(JsonSceneError::MarkerIsEnd { frame: 0 }));
        if let Some(frame) = end.frames.first_mut() {
            frame.synthetic_no_frame_marker = Some(true);
        }
        assert_eq!(end.check(), Ok(()));
    }
}
