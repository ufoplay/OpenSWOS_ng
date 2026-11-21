//! What every `rules/` file shares: its name, and its text, laid out as upstream lays out its
//! JSON (two spaces, ASCII, a trailing newline: `openswos_common::json`).

use openswos_common::json::{Indent, NonAscii, TrailingNewline, to_text};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The layout of the files' text: upstream's competition JSON's indentation.
const INDENT: Indent = Indent::Two;

/// How the files write characters beyond ASCII: escaped, as Python's `json.dump` does.
const NON_ASCII: NonAscii = NonAscii::Escaped;

/// Whether the files end with a newline: they do, as World 2026-27's package's.
const TRAILING_NEWLINE: TrailingNewline = TrailingNewline::Present;

/// A file of `rules/`.
pub trait RulesFile: Serialize + DeserializeOwned {
    /// Its name in `rules/`.
    const FILE_NAME: &'static str;
}

/// The file `T` in `bytes`. Its keys may come in any order and its text may be laid out in
/// any way; a key the file does not have is an error.
///
/// # Errors
///
/// When `bytes` is not the file's JSON.
pub fn read<T: RulesFile>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(bytes)
}

/// `file`'s text, in the one layout this crate writes: a file it wrote reads and writes back
/// byte for byte.
#[must_use]
pub fn write<T: RulesFile>(file: &T) -> Vec<u8> {
    to_text(file, INDENT, NON_ASCII, TRAILING_NEWLINE)
        .expect("a rules file holds no map keyed by something other than a string")
        .into_bytes()
}
