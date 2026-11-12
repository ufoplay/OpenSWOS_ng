#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod error;
pub mod file;
pub mod frame;
pub mod header;
pub mod reader;
pub mod schema;
pub mod version;

pub use error::RgdError;
pub use file::{RgdFile, RgdFormat};
pub use frame::{Controls, RgdFrame};
pub use header::{HeaderV1p2, RgdHeader, USER_TACTICS};
pub use reader::RgdReader;
pub use schema::{Compare, FieldType, FrameField, Level, PointerKind, frame_fields};
pub use version::{RgdVersion, VersionError};
