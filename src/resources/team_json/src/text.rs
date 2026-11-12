//! The text layout of upstream's files: Python's `json.dump(indent=2)` with
//! `ensure_ascii=True`. That means:
//! - one value a line, indented by two spaces;
//! - `": "` after a key;
//! - `[]` / `{}` for an empty container;
//! - every character outside space..`~` escaped as `\uXXXX` (lower-case hex).
//!
//! OpenSWOS Setup writes no trailing newline, the World 2026-27 import writes one.

use std::fmt::Write as _;

use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};

/// The indentation of one nesting level.
const INDENT: &[u8] = b"  ";

/// The last character written as it is: the end of printable ASCII (`~`). Every character
/// above it is escaped, as `ensure_ascii` does (serde_json already escapes the ones below
/// space).
const LAST_PLAIN_CHAR: char = '~';

/// Whether a text ends with a newline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailingNewline {
    /// It does not (OpenSWOS Setup's files).
    Absent,
    /// It does (the World 2026-27 import's files).
    Present,
}

impl TrailingNewline {
    /// Whether `bytes` end with a newline.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        if bytes.ends_with(b"\n") {
            Self::Present
        } else {
            Self::Absent
        }
    }
}

/// `value` laid out as upstream lays it out.
///
/// # Errors
///
/// When `value` cannot be serialised as JSON (a map with non-string keys).
pub fn to_text<T: Serialize>(
    value: &T,
    newline: TrailingNewline,
) -> Result<String, serde_json::Error> {
    let mut bytes = Vec::new();
    let mut serializer =
        Serializer::with_formatter(&mut bytes, PrettyFormatter::with_indent(INDENT));
    value.serialize(&mut serializer)?;
    let pretty = String::from_utf8(bytes).expect("serde_json writes UTF-8");
    let mut text = escape_non_ascii(&pretty);
    match newline {
        TrailingNewline::Present => text.push('\n'),
        TrailingNewline::Absent => {}
    }
    Ok(text)
}

/// `text` with every character above `~` escaped as `\uXXXX`, UTF-16 units as JSON does.
/// Such characters only occur inside JSON strings: the layout itself is ASCII.
fn escape_non_ascii(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if character <= LAST_PLAIN_CHAR {
            escaped.push(character);
        } else {
            let mut units = [0u16; 2];
            for unit in character.encode_utf16(&mut units) {
                write!(escaped, "\\u{unit:04x}").expect("writing to a String cannot fail");
            }
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lays_out_like_python() {
        let value = json!({ "type": "team", "count": 2, "list": [0, 1], "empty": [], "nested": { "a": null } });
        let text = to_text(&value, TrailingNewline::Absent).expect("serializes");
        assert_eq!(
            text,
            "{\n  \"count\": 2,\n  \"empty\": [],\n  \"list\": [\n    0,\n    1\n  ],\n  \"nested\": {\n    \"a\": null\n  },\n  \"type\": \"team\"\n}"
        );
    }

    #[test]
    fn escapes_what_ensure_ascii_escapes() {
        let text =
            to_text(&json!("ÉIRE\u{7f}\u{1}\n\"~"), TrailingNewline::Absent).expect("serializes");
        assert_eq!(text, "\"\\u00c9IRE\\u007f\\u0001\\n\\\"~\"");
    }

    #[test]
    fn escapes_beyond_the_basic_plane_as_surrogates() {
        let text = to_text(&json!("\u{1F600}"), TrailingNewline::Absent).expect("serializes");
        assert_eq!(text, "\"\\ud83d\\ude00\"");
    }

    #[test]
    fn adds_the_trailing_newline_when_asked() {
        assert_eq!(
            to_text(&json!(1), TrailingNewline::Present).expect("serializes"),
            "1\n"
        );
        assert_eq!(TrailingNewline::of(b"{}\n"), TrailingNewline::Present);
        assert_eq!(TrailingNewline::of(b"{}"), TrailingNewline::Absent);
    }
}
