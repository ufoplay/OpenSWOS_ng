#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod document;
pub mod knobs;

pub use document::{
    CareerParamsDocument, CareerParamsJsonFormat, Member, NOTE_PREFIX, NotANoteKey, NoteKey,
};
pub use knobs::{PerDivisionKnob, SignedKnob, UnsignedKnob};
