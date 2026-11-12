#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod document;
pub mod formatter;
pub mod layout;
pub mod text;
pub mod value;

pub use document::{
    FILE_KIND_KEY, FileKind, SCHEMA_VERSION, SCHEMA_VERSION_KEY, SaveDocument, SaveJsonError,
    SaveJsonFormat, SchemaVersion,
};
pub use formatter::{SaveFormatter, to_save_bytes};
pub use layout::{Compact, Layout, MemberGaps, TopLevelLayout, UpstreamLayout};
pub use text::{from_save_bytes, to_standard_json};
pub use value::{SaveString, SaveValue};
