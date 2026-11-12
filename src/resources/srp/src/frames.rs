//! `FRMS`: the frames as drawn (upstream src/replay_v2.c `write_head`,
//! `ReplayRec_CaptureSprites`). Big-endian. Each frame is a 20-byte head, then its sprites.
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 2 | the sprites (at most 78) |
//! | 2 | 4 + 4 | the camera's X and Y, 16.16 fixed point |
//! | 10 | 2 + 2 | the two teams' goals |
//! | 14 | 2 | the crowd's animation state |
//! | 16 | 2 | the minute on the clock |
//! | 18 | 2 | the animated ad's line + 1 (0 in a recording from before it was kept) |
//!
//! A sprite is 6 bytes: its image, then its world X and Y with its graphic's centre and its
//! height taken out, so playback only subtracts the camera.

use openswos_common::{ByteReader, ByteWriter};

use crate::error::ChunkError;

/// The most sprites a frame holds (upstream `MAX_FRAME_SPRITES`, `_sortedSprites`).
pub const MAX_FRAME_SPRITES: usize = 78;

/// Bytes of a frame's head (upstream `FRAME_HEAD_BYTES`).
pub const FRAME_HEAD_LEN: usize = 20;

/// Bytes of a sprite (upstream `SPRITE_BYTES`).
pub const SPRITE_LEN: usize = 6;

/// A sprite of a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SrpSprite {
    /// Its image.
    pub image: u16,
    /// Its X in the world, less its graphic's centre.
    pub x: i16,
    /// Its Y in the world, less its graphic's centre and its height.
    pub y: i16,
}

/// A frame as drawn.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SrpFrame {
    /// The camera's X, 16.16 fixed point.
    pub camera_x: i32,
    /// The camera's Y, 16.16 fixed point.
    pub camera_y: i32,
    /// The first team's goals.
    pub team1_goals: u16,
    /// The second team's goals.
    pub team2_goals: u16,
    /// The crowd's animation state.
    pub crowd: u16,
    /// The minute on the clock.
    pub minute: u16,
    /// The animated ad's line + 1; 0 in older recordings.
    pub ad_line: u16,
    /// The sprites, in drawing order (the first at the back).
    pub sprites: Vec<SrpSprite>,
}

/// The frames of a `FRMS` chunk's content.
///
/// # Errors
///
/// When a frame is cut short or holds more than 78 sprites.
pub fn decode_frames(data: &[u8]) -> Result<Vec<SrpFrame>, ChunkError> {
    let mut reader = ByteReader::new(data);
    let mut frames = Vec::new();
    while !reader.is_at_end() {
        let count = usize::from(reader.read_u16_be()?);
        if count > MAX_FRAME_SPRITES {
            return Err(ChunkError::TooManySprites {
                frame: frames.len(),
                sprites: count,
            });
        }
        let mut frame = SrpFrame {
            camera_x: reader.read_i32_be()?,
            camera_y: reader.read_i32_be()?,
            team1_goals: reader.read_u16_be()?,
            team2_goals: reader.read_u16_be()?,
            crowd: reader.read_u16_be()?,
            minute: reader.read_u16_be()?,
            ad_line: reader.read_u16_be()?,
            sprites: Vec::with_capacity(count),
        };
        for _ in 0..count {
            frame.sprites.push(SrpSprite {
                image: reader.read_u16_be()?,
                x: reader.read_i16_be()?,
                y: reader.read_i16_be()?,
            });
        }
        frames.push(frame);
    }
    Ok(frames)
}

/// The `FRMS` chunk's content.
///
/// # Errors
///
/// When a frame holds more than 78 sprites.
pub fn encode_frames(frames: &[SrpFrame]) -> Result<Vec<u8>, ChunkError> {
    let mut writer = ByteWriter::new();
    for (index, frame) in frames.iter().enumerate() {
        let SrpFrame {
            camera_x,
            camera_y,
            team1_goals,
            team2_goals,
            crowd,
            minute,
            ad_line,
            sprites,
        } = frame;
        let count = u16::try_from(sprites.len())
            .ok()
            .filter(|count| usize::from(*count) <= MAX_FRAME_SPRITES)
            .ok_or(ChunkError::TooManySprites {
                frame: index,
                sprites: sprites.len(),
            })?;
        writer.write_u16_be(count);
        writer.write_i32_be(*camera_x);
        writer.write_i32_be(*camera_y);
        for word in [team1_goals, team2_goals, crowd, minute, ad_line] {
            writer.write_u16_be(*word);
        }
        for SrpSprite { image, x, y } in sprites {
            writer.write_u16_be(*image);
            writer.write_i16_be(*x);
            writer.write_i16_be(*y);
        }
    }
    Ok(writer.into_bytes())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into bytes the tests build
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let frames = vec![
            SrpFrame {
                camera_x: 176 << 16 | 0x8000,
                camera_y: -1,
                team1_goals: 1,
                team2_goals: 0,
                crowd: 3,
                minute: 45,
                ad_line: 2,
                sprites: vec![SrpSprite {
                    image: 300,
                    x: -5,
                    y: 700,
                }],
            },
            SrpFrame::default(),
        ];
        let data = encode_frames(&frames).expect("encodes");
        assert_eq!(data.len(), 2 * FRAME_HEAD_LEN + SPRITE_LEN);
        assert_eq!(decode_frames(&data).expect("decodes"), frames);
    }

    #[test]
    fn too_many_sprites_or_a_cut_frame_do_not_read() {
        let mut data = encode_frames(&[SrpFrame::default()]).expect("encodes");
        data[1] = 79;
        assert!(matches!(
            decode_frames(&data),
            Err(ChunkError::TooManySprites {
                frame: 0,
                sprites: 79
            })
        ));
        data[1] = 1;
        assert!(matches!(decode_frames(&data), Err(ChunkError::Read(_))));
        let frame = SrpFrame {
            sprites: vec![SrpSprite::default(); MAX_FRAME_SPRITES + 1],
            ..SrpFrame::default()
        };
        assert!(encode_frames(&[frame]).is_err());
    }
}
