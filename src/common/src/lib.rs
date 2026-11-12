#![doc = include_str!("../readme.md")]
// This crate parses untrusted bytes: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod byte_enum;
pub mod bytes;
pub mod format;
pub mod name;
pub mod nibble;
pub mod position;
pub mod tactics;
pub mod team_id;

pub use bytes::{ByteReader, ByteWriter, ReadError};
pub use format::{Exporter, Importer};
pub use name::{FixedName, NameError};
pub use nibble::{InvalidNibble, Nibble};
pub use position::{InvalidPosition, Position};
pub use tactics::{InvalidTactics, Tactics};
pub use team_id::{TeamFileNumber, TeamId, TeamOrdinal};
