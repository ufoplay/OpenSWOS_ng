//! Why a chunk's content does not decode or encode.

use openswos_common::ReadError;
use thiserror::Error;

/// Why a chunk's content does not decode or encode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChunkError {
    /// The content is short, or longer than what it holds.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// A frame with more sprites than a frame holds.
    #[error("frame {frame}: {sprites} sprites, at most 78")]
    TooManySprites {
        /// The frame.
        frame: usize,
        /// Its sprites.
        sprites: usize,
    },
    /// A chapter's kind is not one.
    #[error("chapter kind {0}: 1 a goal, 2 a half")]
    ChapterKind(u8),
    /// More entries than the chunk's count holds, or a name longer than its length byte.
    #[error("more than the chunk can count")]
    TooMany,
}
