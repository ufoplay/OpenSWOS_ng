//! Why a recording does not read or write.

use openswos_common::ReadError;
use openswos_tac_resource::TacticsError;
use openswos_team_resource::RecordError;
use thiserror::Error;

use crate::version::{RgdVersion, VersionError};

/// Why a recording does not read or write.
#[derive(Debug, Error)]
pub enum RgdError {
    /// The data ended early.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The stream could not be read (or decompressed).
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The version is not one this crate reads.
    #[error(transparent)]
    Version(#[from] VersionError),
    /// A header team does not read.
    #[error("header team: {0}")]
    Team(#[from] RecordError),
    /// A user's tactic does not read.
    #[error("user tactics: {0}")]
    Tactics(#[from] TacticsError),
    /// The header's 1.2 additions disagree with its version.
    #[error("version {}.{}: the header's 1.2 fields are present exactly from 1.2", .0.major, .0.minor)]
    HeaderVersion(RgdVersion),
    /// The data ends inside a frame.
    #[error("{0} bytes after the last whole frame")]
    PartialFrame(usize),
    /// A frame is not its version's size.
    #[error("a frame of {len} bytes; version {}.{}'s are {expected}", .version.major, .version.minor)]
    FrameLength {
        /// The frame's bytes.
        len: usize,
        /// The version's.
        expected: usize,
        /// The version.
        version: RgdVersion,
    },
}
