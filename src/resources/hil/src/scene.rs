//! A replay's scene: frames recorded as little-endian long words (upstream
//! src/dos_replay_load.c). Each frame is a camera marker, the score, an animation state,
//! then its sprites; `0xFFFFFFFF` ends the scene.
//!
//! | word | meaning |
//! |---|---|
//! | bit 31 set | the camera: Y (15 bits) << 16 \| X (16 bits) |
//! | next | the score: the second team's goals << 16 \| the first team's |
//! | next | the animation state |
//! | bit 31 clear | a sprite: image (11 bits) << 20 \| X (10 bits, signed) << 10 \| Y (10 bits, signed) |
//! | `0xFFFFFFFF` | the scene's end |
//!
//! Sprite images are the DOS game's: the Amiga's + 114 (upstream docs/AMIGA_DOS_DIFF.md).
//! In a `.HIL` each scene fills a slot; the bytes after its end are what the slot held
//! before, kept as written.

use thiserror::Error;

/// The word ending a scene.
const SCENE_END: u32 = 0xFFFF_FFFF;

/// The bit marking a camera word.
const CAMERA_MARKER: u32 = 0x8000_0000;

/// The bits of the camera's Y, below the marker.
const CAMERA_Y_MAX: u16 = 0x7FFF;

/// Where a word holds its high half (the camera's Y, the second team's goals).
const HIGH_HALF: u32 = 16;

/// Where a sprite word holds its image.
const SPRITE_IMAGE_SHIFT: u32 = 20;

/// Where it holds its X.
const SPRITE_X_SHIFT: u32 = 10;

/// A sprite's image: 11 bits.
pub const SPRITE_IMAGE_MAX: u16 = 0x7FF;

/// A sprite coordinate's 10 bits.
const COORDINATE_MASK: u32 = 0x3FF;

/// The bits a 10-bit coordinate leaves in an `i16`, for its sign.
const COORDINATE_SIGN_SHIFT: u32 = 6;

/// The smallest sprite coordinate.
pub const COORDINATE_MIN: i16 = -512;

/// The largest sprite coordinate.
pub const COORDINATE_MAX: i16 = 511;

/// Bytes of a word.
const WORD_LEN: usize = 4;

/// A sprite drawn in a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ReplaySprite {
    /// Its image in the DOS sprite bank (0..=2047).
    pub image: u16,
    /// Its X on screen (-512..=511).
    pub x: i16,
    /// Its Y on screen (-512..=511).
    pub y: i16,
}

/// A frame of a scene.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ReplayFrame {
    /// The camera's X.
    pub camera_x: u16,
    /// The camera's Y (0..=32767).
    pub camera_y: u16,
    /// The first team's goals.
    pub team1_goals: u16,
    /// The second team's goals.
    pub team2_goals: u16,
    /// The animation state.
    pub animation: u32,
    /// The sprites, in drawing order.
    pub sprites: Vec<ReplaySprite>,
}

/// A scene: its frames, and what follows its end in its slot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Scene {
    /// The frames.
    pub frames: Vec<ReplayFrame>,
    /// The bytes after the end word, to the slot's end, kept as written.
    pub leftover: Vec<u8>,
}

/// Why bytes are not a scene, or a scene does not encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SceneError {
    /// The scene has no end word.
    #[error("the scene has no end word")]
    NoEnd,
    /// A sprite before any camera word.
    #[error("a sprite before the scene's first camera word")]
    SpriteBeforeCamera,
    /// The scene ends between a camera word and its score or animation state.
    #[error("the scene ends inside a frame's header")]
    IncompleteFrame,
    /// A value does not fit its bits, or would read as the end word.
    #[error("a frame's {0} does not fit its word")]
    OutOfRange(&'static str),
}

/// What the next word is, after the words before it.
enum Expect {
    Camera,
    Score,
    Animation,
}

impl Scene {
    /// The scene in `slot`.
    ///
    /// # Errors
    ///
    /// When it has no end word, a sprite comes before the first camera word, or it ends
    /// inside a frame's header.
    pub fn parse(slot: &[u8]) -> Result<Self, SceneError> {
        let (words, _) = slot.as_chunks::<WORD_LEN>();
        let mut frames: Vec<ReplayFrame> = Vec::new();
        let mut expect = Expect::Camera;
        for (index, word) in words.iter().copied().map(u32::from_le_bytes).enumerate() {
            if word == SCENE_END {
                if !matches!(expect, Expect::Camera) {
                    return Err(SceneError::IncompleteFrame);
                }
                let leftover = slot.get((index + 1) * WORD_LEN..).unwrap_or_default();
                return Ok(Self {
                    frames,
                    leftover: leftover.to_vec(),
                });
            }
            match expect {
                Expect::Score => {
                    let frame = frames.last_mut().ok_or(SceneError::IncompleteFrame)?;
                    frame.team1_goals = low_half(word);
                    frame.team2_goals = high_half(word);
                    expect = Expect::Animation;
                }
                Expect::Animation => {
                    frames
                        .last_mut()
                        .ok_or(SceneError::IncompleteFrame)?
                        .animation = word;
                    expect = Expect::Camera;
                }
                Expect::Camera if word & CAMERA_MARKER != 0 => {
                    frames.push(ReplayFrame {
                        camera_x: low_half(word),
                        camera_y: high_half(word & !CAMERA_MARKER),
                        ..ReplayFrame::default()
                    });
                    expect = Expect::Score;
                }
                Expect::Camera => frames
                    .last_mut()
                    .ok_or(SceneError::SpriteBeforeCamera)?
                    .sprites
                    .push(sprite(word)),
            }
        }
        Err(SceneError::NoEnd)
    }

    /// The scene's bytes: its words, the end word, the leftover.
    ///
    /// # Errors
    ///
    /// When a value does not fit its bits, or a score or animation state would read as
    /// the end word.
    pub fn encode(&self) -> Result<Vec<u8>, SceneError> {
        let mut words = Vec::new();
        for frame in &self.frames {
            let ReplayFrame {
                camera_x,
                camera_y,
                team1_goals,
                team2_goals,
                animation,
                sprites,
            } = frame;
            if *camera_y > CAMERA_Y_MAX {
                return Err(SceneError::OutOfRange("camera Y"));
            }
            words.push(CAMERA_MARKER | (u32::from(*camera_y) << HIGH_HALF) | u32::from(*camera_x));
            let score = (u32::from(*team2_goals) << HIGH_HALF) | u32::from(*team1_goals);
            for (word, what) in [(score, "score"), (*animation, "animation state")] {
                if word == SCENE_END {
                    return Err(SceneError::OutOfRange(what));
                }
                words.push(word);
            }
            for sprite in sprites {
                words.push(sprite_word(*sprite)?);
            }
        }
        words.push(SCENE_END);
        let mut bytes: Vec<u8> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        bytes.extend_from_slice(&self.leftover);
        Ok(bytes)
    }
}

fn low_half(word: u32) -> u16 {
    u16::try_from(word & u32::from(u16::MAX)).expect("a half word")
}

fn high_half(word: u32) -> u16 {
    u16::try_from(word >> HIGH_HALF).expect("a half word")
}

/// A 10-bit coordinate, sign-extended.
fn coordinate(bits: u32) -> i16 {
    let raw = u16::try_from(bits & COORDINATE_MASK).expect("10 bits");
    raw.cast_signed() << COORDINATE_SIGN_SHIFT >> COORDINATE_SIGN_SHIFT
}

fn sprite(word: u32) -> ReplaySprite {
    ReplaySprite {
        image: u16::try_from(word >> SPRITE_IMAGE_SHIFT).expect("11 bits"),
        x: coordinate(word >> SPRITE_X_SHIFT),
        y: coordinate(word),
    }
}

fn sprite_word(sprite: ReplaySprite) -> Result<u32, SceneError> {
    let ReplaySprite { image, x, y } = sprite;
    if image > SPRITE_IMAGE_MAX {
        return Err(SceneError::OutOfRange("sprite image"));
    }
    let bits = |value: i16, what| {
        if (COORDINATE_MIN..=COORDINATE_MAX).contains(&value) {
            Ok(u32::from(value.cast_unsigned()) & COORDINATE_MASK)
        } else {
            Err(SceneError::OutOfRange(what))
        }
    };
    Ok((u32::from(image) << SPRITE_IMAGE_SHIFT)
        | (bits(x, "sprite X")? << SPRITE_X_SHIFT)
        | bits(y, "sprite Y")?)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // frames the tests just built
mod tests {
    use super::*;

    fn words(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    #[test]
    fn reads_frames_and_keeps_the_leftover() {
        let mut slot = words(&[
            CAMERA_MARKER | (592 << 16) | 176,
            (2 << 16) | 1,
            7,
            (1300 << 20) | (((-3i32).cast_unsigned() & 0x3FF) << 10) | 200,
            SCENE_END,
        ]);
        slot.extend_from_slice(&[9, 9, 9]);
        let scene = Scene::parse(&slot).expect("a scene");
        assert_eq!(scene.frames.len(), 1);
        let frame = &scene.frames[0];
        assert_eq!((frame.camera_x, frame.camera_y), (176, 592));
        assert_eq!((frame.team1_goals, frame.team2_goals), (1, 2));
        assert_eq!(frame.animation, 7);
        assert_eq!(
            frame.sprites,
            vec![ReplaySprite {
                image: 1300,
                x: -3,
                y: 200
            }]
        );
        assert_eq!(scene.leftover, vec![9, 9, 9]);
        assert_eq!(scene.encode(), Ok(slot));
    }

    #[test]
    fn malformed_scenes_do_not_read() {
        assert_eq!(
            Scene::parse(&words(&[CAMERA_MARKER, 0, 0])),
            Err(SceneError::NoEnd)
        );
        assert_eq!(
            Scene::parse(&words(&[5, SCENE_END])),
            Err(SceneError::SpriteBeforeCamera)
        );
        assert_eq!(
            Scene::parse(&words(&[CAMERA_MARKER, 0, SCENE_END])),
            Err(SceneError::IncompleteFrame)
        );
    }

    #[test]
    fn values_that_do_not_fit_do_not_encode() {
        let frame = |sprite| Scene {
            frames: vec![ReplayFrame {
                sprites: vec![sprite],
                ..ReplayFrame::default()
            }],
            leftover: Vec::new(),
        };
        assert!(
            frame(ReplaySprite {
                image: 2048,
                x: 0,
                y: 0
            })
            .encode()
            .is_err()
        );
        assert!(
            frame(ReplaySprite {
                image: 0,
                x: 512,
                y: 0
            })
            .encode()
            .is_err()
        );
        let mut scene = frame(ReplaySprite::default());
        scene.frames[0].animation = SCENE_END;
        assert_eq!(
            scene.encode(),
            Err(SceneError::OutOfRange("animation state"))
        );
    }

    #[test]
    fn every_coordinate_round_trips() {
        for value in COORDINATE_MIN..=COORDINATE_MAX {
            let sprite = ReplaySprite {
                image: SPRITE_IMAGE_MAX,
                x: value,
                y: -value.max(-511),
            };
            assert_eq!(super::sprite(sprite_word(sprite).expect("fits")), sprite);
        }
    }
}
