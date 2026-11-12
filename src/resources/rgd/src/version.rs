//! A recording's version and the sizes it gives (upstream docs/TESTS.md §2): a `.RGD` has
//! no index and no frame lengths, so the header's first two bytes decide everything.
//!
//! | version | header | frame |
//! |---|---:|---:|
//! | 1.0, 1.1 | 3610 | 7404 |
//! | 1.2 | 3617 | unresolved |
//! | 1.3 | 3617 | 7475 (+ the bench's 71) |
//! | 1.4 | 3617 | 7481 (+ 6) |
//! | 1.5 | 3617 | 7485 (+ 4) |

use thiserror::Error;

/// The only major version.
const MAJOR: u8 = 1;

/// The minor version from which the header has its career and substitutes fields.
const MINOR_HEADER_V1P2: u8 = 2;

/// The minor version from which frames carry the bench's state.
const MINOR_BENCH: u8 = 3;

/// The minor version from which frames carry the key and timers words.
const MINOR_V1P4: u8 = 4;

/// The minor version from which frames carry the tap timeouts.
const MINOR_V1P5: u8 = 5;

/// The newest minor version.
const MINOR_LATEST: u8 = MINOR_V1P5;

/// Bytes of a 1.0/1.1 header (`Header`).
pub const HEADER_V1P0_LEN: usize = 3610;

/// Bytes of a 1.2+ header (`HeaderV1p2`).
pub const HEADER_V1P2_LEN: usize = 3617;

/// Bytes of a 1.0 frame (`FrameV1p0`), the base of every later one.
pub const FRAME_V1P0_LEN: usize = 7404;

/// Bytes of the bench's state (`BenchDataV1`, 1.3+).
pub const BENCH_LEN: usize = 71;

/// Bytes of the 1.4 additions: the key, fire blocked, the stats and result timers.
pub const V1P4_LEN: usize = 6;

/// Bytes of the 1.5 additions: the two tap timeouts.
pub const V1P5_LEN: usize = 4;

/// A recording's version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RgdVersion {
    /// Major: 1.
    pub major: u8,
    /// Minor: 0..=5.
    pub minor: u8,
}

/// Why a version is not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum VersionError {
    /// No such version.
    #[error("version {}.{} is no .RGD version (1.0 to 1.5)", .0.major, .0.minor)]
    Unknown(RgdVersion),
    /// Version 1.2: its frame size is unknown. The corpus's one file of it matches no
    /// header and frame sizes (upstream docs/TESTS.md §2, "Open").
    #[error("version 1.2's frames are not known: upstream could not resolve them either")]
    Unresolved,
}

impl RgdVersion {
    /// The version, if it is one.
    ///
    /// # Errors
    ///
    /// When it is not 1.0 to 1.5.
    pub fn new(major: u8, minor: u8) -> Result<Self, VersionError> {
        let version = Self { major, minor };
        if major == MAJOR && minor <= MINOR_LATEST {
            Ok(version)
        } else {
            Err(VersionError::Unknown(version))
        }
    }

    /// Whether the header has the 1.2 fields (career size, season, substitutes).
    #[must_use]
    pub fn has_header_v1p2(self) -> bool {
        self.minor >= MINOR_HEADER_V1P2
    }

    /// Whether frames carry the bench's state.
    #[must_use]
    pub fn has_bench(self) -> bool {
        self.minor >= MINOR_BENCH
    }

    /// Whether frames carry the 1.4 words.
    #[must_use]
    pub fn has_v1p4(self) -> bool {
        self.minor >= MINOR_V1P4
    }

    /// Whether frames carry the 1.5 words.
    #[must_use]
    pub fn has_v1p5(self) -> bool {
        self.minor >= MINOR_V1P5
    }

    /// Bytes of the header.
    #[must_use]
    pub fn header_len(self) -> usize {
        if self.has_header_v1p2() {
            HEADER_V1P2_LEN
        } else {
            HEADER_V1P0_LEN
        }
    }

    /// Bytes of a frame.
    ///
    /// # Errors
    ///
    /// For 1.2, whose frames are not known.
    pub fn frame_len(self) -> Result<usize, VersionError> {
        if self.minor == MINOR_HEADER_V1P2 {
            return Err(VersionError::Unresolved);
        }
        let mut len = FRAME_V1P0_LEN;
        for (has, extra) in [
            (self.has_bench(), BENCH_LEN),
            (self.has_v1p4(), V1P4_LEN),
            (self.has_v1p5(), V1P5_LEN),
        ] {
            if has {
                len += extra;
            }
        }
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sizes_are_upstreams() {
        for (minor, header, frame) in [
            (0, 3610, Ok(7404)),
            (1, 3610, Ok(7404)),
            (2, 3617, Err(VersionError::Unresolved)),
            (3, 3617, Ok(7475)),
            (4, 3617, Ok(7481)),
            (5, 3617, Ok(7485)),
        ] {
            let version = RgdVersion::new(1, minor).expect("a version");
            assert_eq!(version.header_len(), header, "1.{minor}");
            assert_eq!(version.frame_len(), frame, "1.{minor}");
        }
    }

    #[test]
    fn other_versions_are_not_read() {
        for (major, minor) in [(0, 5), (2, 0), (1, 6)] {
            assert!(matches!(
                RgdVersion::new(major, minor),
                Err(VersionError::Unknown(_))
            ));
        }
    }
}
