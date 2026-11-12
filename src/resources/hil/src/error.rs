//! Why a DOS replay does not read or write.

use openswos_common::ReadError;
use thiserror::Error;

use crate::scene::SceneError;

/// Why a DOS replay does not read or write.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DosReplayError {
    /// The data ended early.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The first long word is not a DOS replay's signature (OpenSWOS's own `.HIL` is
    /// big-endian, or JSON).
    #[error("signature {0}: not a DOS replay (8)")]
    Signature(u32),
    /// A scene does not read or encode.
    #[error("scene {index}: {error}")]
    Scene {
        /// The scene.
        index: usize,
        /// Why.
        error: SceneError,
    },
    /// A `.HIL` with no scenes, or whose scenes do not divide its bytes.
    #[error("{scenes} scenes do not divide the {bytes} bytes after the header")]
    Slots {
        /// The header's scenes.
        scenes: u16,
        /// The bytes after the header.
        bytes: usize,
    },
    /// A `.HIL`'s scenes disagree with its header, or are not one slot size.
    #[error(
        "the header says {stated} scenes of one slot size; the file holds {scenes} (of {sizes:?} bytes)"
    )]
    Scenes {
        /// The header's count.
        stated: u16,
        /// The scenes held.
        scenes: usize,
        /// Their sizes.
        sizes: Vec<usize>,
    },
    /// A raw part is not its size.
    #[error("{part}: {len} bytes, not {expected}")]
    PartLength {
        /// The part.
        part: &'static str,
        /// Its bytes.
        len: usize,
        /// The bytes it has.
        expected: usize,
    },
}
