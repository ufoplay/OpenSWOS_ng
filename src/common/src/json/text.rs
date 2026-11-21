//! The text layout of upstream's JSON files: Python's `json.dump(indent=…)`. That means:
//! - one value a line, indented by two spaces (one in a mod's `mod.json` and World 2026-27's
//!   national teams);
//! - `": "` after a key;
//! - `[]` / `{}` for an empty container;
//! - with `ensure_ascii` (the default), every character outside space..`~` escaped as
//!   `\uXXXX` (lower-case hex); without it, characters beyond ASCII written as they are.
//!
//! Some writers end the file with a newline, others do not.

use std::fmt::Write as _;

use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};

/// One space: `indent=1`.
const ONE_SPACE: &[u8] = b" ";

/// Two spaces: `indent=2`.
const TWO_SPACES: &[u8] = b"  ";

/// The indentation of one nesting level: Python's `indent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indent {
    /// One space (a mod's `mod.json`, upstream scripts/make_world_package.sh).
    One,
    /// Two spaces (the team and competition JSON).
    Two,
}

impl Indent {
    /// The indentation of a text laid out as upstream lays it out: its second line's (the
    /// first key's). [`Indent::Two`] when there is none to see (an empty container).
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let second_line = bytes
            .iter()
            .position(|&byte| byte == b'\n')
            .and_then(|end| bytes.get(end + 1..))
            .unwrap_or_default();
        let spaces = second_line.iter().take_while(|&&byte| byte == b' ').count();
        if spaces == ONE_SPACE.len() {
            Self::One
        } else {
            Self::Two
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Self::One => ONE_SPACE,
            Self::Two => TWO_SPACES,
        }
    }
}

/// The last character written as it is: the end of printable ASCII (`~`). Every character
/// above it is escaped, as `ensure_ascii` does (serde_json already escapes the ones below
/// space).
const LAST_PLAIN_CHAR: char = '~';

/// Whether a text ends with a newline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailingNewline {
    /// It does not (OpenSWOS Setup's files, upstream's `bin/data_json/`).
    Absent,
    /// It does (the World 2026-27 import's files).
    Present,
}

/// How characters beyond ASCII are written: Python's `ensure_ascii`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonAscii {
    /// As `\uXXXX` escapes (`ensure_ascii=True`, the default: the team JSON).
    Escaped,
    /// As they are, in UTF-8 (`ensure_ascii=False`: the competition JSON).
    Raw,
}

impl NonAscii {
    /// How a text wrote characters beyond ASCII: [`NonAscii::Raw`] when it holds any as they
    /// are. A text of ASCII alone reads [`NonAscii::Escaped`], which writes it the same.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        if bytes.is_ascii() {
            Self::Escaped
        } else {
            Self::Raw
        }
    }
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
    indent: Indent,
    non_ascii: NonAscii,
    newline: TrailingNewline,
) -> Result<String, serde_json::Error> {
    let mut bytes = Vec::new();
    let mut serializer =
        Serializer::with_formatter(&mut bytes, PrettyFormatter::with_indent(indent.bytes()));
    value.serialize(&mut serializer)?;
    let pretty = String::from_utf8(bytes).expect("serde_json writes UTF-8");
    let mut text = match non_ascii {
        NonAscii::Escaped => escape_non_ascii(&pretty),
        NonAscii::Raw => pretty,
    };
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
    fn the_layout_is_read_from_the_text() {
        assert_eq!(Indent::of(b"{\n \"type\": 1\n}"), Indent::One);
        assert_eq!(Indent::of(b"{\n  \"type\": 1\n}"), Indent::Two);
        assert_eq!(Indent::of(b"{}"), Indent::Two);
        assert_eq!(
            NonAscii::of("{\"a\": \"\\u00e9\"}".as_bytes()),
            NonAscii::Escaped
        );
        assert_eq!(NonAscii::of("{\"a\": \"é\"}".as_bytes()), NonAscii::Raw);
        for (indent, non_ascii) in [
            (Indent::One, NonAscii::Raw),
            (Indent::Two, NonAscii::Escaped),
        ] {
            let text = to_text(
                &json!({ "name": "PELÉ", "list": [1] }),
                indent,
                non_ascii,
                TrailingNewline::Present,
            )
            .expect("serializes");
            assert_eq!(Indent::of(text.as_bytes()), indent);
            assert_eq!(NonAscii::of(text.as_bytes()), non_ascii);
        }
    }

    #[test]
    fn lays_out_like_python() {
        let value = json!({ "type": "team", "count": 2, "list": [0, 1], "empty": [], "nested": { "a": null } });
        let text = to_text(
            &value,
            Indent::Two,
            NonAscii::Escaped,
            TrailingNewline::Absent,
        )
        .expect("serializes");
        assert_eq!(
            text,
            "{\n  \"count\": 2,\n  \"empty\": [],\n  \"list\": [\n    0,\n    1\n  ],\n  \"nested\": {\n    \"a\": null\n  },\n  \"type\": \"team\"\n}"
        );
    }

    #[test]
    fn escapes_what_ensure_ascii_escapes() {
        let text = to_text(
            &json!("ÉIRE\u{7f}\u{1}\n\"~"),
            Indent::Two,
            NonAscii::Escaped,
            TrailingNewline::Absent,
        )
        .expect("serializes");
        assert_eq!(text, "\"\\u00c9IRE\\u007f\\u0001\\n\\\"~\"");
    }

    #[test]
    fn writes_beyond_ascii_raw_when_asked() {
        let text = to_text(
            &json!("SÜPER LIG\u{1}"),
            Indent::Two,
            NonAscii::Raw,
            TrailingNewline::Absent,
        )
        .expect("serializes");
        assert_eq!(text, "\"SÜPER LIG\\u0001\"");
    }

    #[test]
    fn escapes_beyond_the_basic_plane_as_surrogates() {
        let text = to_text(
            &json!("\u{1F600}"),
            Indent::Two,
            NonAscii::Escaped,
            TrailingNewline::Absent,
        )
        .expect("serializes");
        assert_eq!(text, "\"\\ud83d\\ude00\"");
    }

    #[test]
    fn indents_by_one_space_when_asked() {
        let text = to_text(
            &json!({ "targets": ["sdl"] }),
            Indent::One,
            NonAscii::Escaped,
            TrailingNewline::Present,
        )
        .expect("serializes");
        assert_eq!(text, "{\n \"targets\": [\n  \"sdl\"\n ]\n}\n");
    }

    #[test]
    fn adds_the_trailing_newline_when_asked() {
        assert_eq!(
            to_text(
                &json!(1),
                Indent::Two,
                NonAscii::Raw,
                TrailingNewline::Present
            )
            .expect("serializes"),
            "1\n"
        );
        assert_eq!(TrailingNewline::of(b"{}\n"), TrailingNewline::Present);
        assert_eq!(TrailingNewline::of(b"{}"), TrailingNewline::Absent);
    }
}
