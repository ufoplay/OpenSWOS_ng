//! Writing saves: mjson's escapes and raw bytes, and a layout's whitespace, as a
//! `serde_json` formatter.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::ser::{CharEscape, Formatter, Serializer};

use crate::layout::TopLevelLayout;

/// `value` written as upstream writes a save, its top-level object laid out by `layout`.
///
/// # Errors
///
/// When `value` holds a character past U+00FF (no byte), a float, or a map with keys that
/// are not strings.
pub fn to_save_bytes<T: Serialize, L: TopLevelLayout + ?Sized>(
    value: &T,
    layout: &L,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = layout.before_open().to_vec();
    let mut serializer = Serializer::with_formatter(&mut bytes, SaveFormatter::new(layout));
    value.serialize(&mut serializer)?;
    bytes.extend_from_slice(layout.after_close());
    Ok(bytes)
}

/// The writer's escaping, and a layout's whitespace, as a `serde_json` formatter.
///
/// A top-level key is held back until it is complete, so that the layout can place the
/// whitespace before it by its name.
#[derive(Debug)]
pub struct SaveFormatter<'a, L: ?Sized> {
    layout: &'a L,
    /// The nesting depth: 1 inside the top-level object.
    depth: usize,
    /// The top-level members begun so far.
    members: usize,
    /// The current top-level key, quotes included, while it is written.
    key: Option<Vec<u8>>,
    /// The current top-level key, without its quotes.
    name: Vec<u8>,
}

impl<'a, L: TopLevelLayout + ?Sized> SaveFormatter<'a, L> {
    /// A formatter laying the top-level object out by `layout`.
    #[must_use]
    pub fn new(layout: &'a L) -> Self {
        Self {
            layout,
            depth: 0,
            members: 0,
            key: None,
            name: Vec::new(),
        }
    }

    fn at_top_level(&self) -> bool {
        self.depth == 1
    }

    /// The place of the current top-level member.
    fn index(&self) -> usize {
        self.members.saturating_sub(1)
    }

    /// Writes `bytes`, or holds them back with the key being written.
    fn put<W: ?Sized + Write>(&mut self, writer: &mut W, bytes: &[u8]) -> io::Result<()> {
        match &mut self.key {
            Some(key) => {
                key.extend_from_slice(bytes);
                Ok(())
            }
            None => writer.write_all(bytes),
        }
    }
}

impl<L: TopLevelLayout + ?Sized> Formatter for SaveFormatter<'_, L> {
    fn write_f32<W: ?Sized + Write>(&mut self, _writer: &mut W, value: f32) -> io::Result<()> {
        Err(no_fractions(f64::from(value)))
    }

    fn write_f64<W: ?Sized + Write>(&mut self, _writer: &mut W, value: f64) -> io::Result<()> {
        Err(no_fractions(value))
    }

    fn begin_string<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.put(writer, b"\"")
    }

    fn end_string<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.put(writer, b"\"")
    }

    fn write_string_fragment<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        let mut bytes = Vec::with_capacity(fragment.len());
        for character in fragment.chars() {
            let byte = u8::try_from(u32::from(character)).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{character:?} is past U+00FF: no byte of a save's string"),
                )
            })?;
            bytes.push(byte);
        }
        self.put(writer, &bytes)
    }

    fn write_char_escape<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        char_escape: CharEscape,
    ) -> io::Result<()> {
        let raw;
        let escaped: &[u8] = match char_escape {
            CharEscape::Quote => b"\\\"",
            CharEscape::ReverseSolidus => b"\\\\",
            CharEscape::Solidus => b"/",
            CharEscape::Backspace => b"\\b",
            CharEscape::FormFeed => b"\\f",
            CharEscape::LineFeed => b"\\n",
            CharEscape::CarriageReturn => b"\\r",
            CharEscape::Tab => b"\\t",
            // Faithful to the original: mjson writes the other control characters raw.
            CharEscape::AsciiControl(byte) => {
                raw = [byte];
                &raw
            }
        };
        self.put(writer, escaped)
    }

    fn begin_object<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.depth += 1;
        writer.write_all(b"{")
    }

    fn end_object<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        if self.at_top_level() && self.members > 0 {
            writer.write_all(self.layout.before_close(self.members))?;
        }
        self.depth -= 1;
        writer.write_all(b"}")
    }

    fn begin_object_key<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if self.at_top_level() {
            if !first {
                writer.write_all(self.layout.after_value(self.index(), &self.name))?;
                writer.write_all(b",")?;
            }
            self.members += 1;
            self.key = Some(Vec::new());
            Ok(())
        } else if first {
            Ok(())
        } else {
            writer.write_all(b",")
        }
    }

    fn end_object_key<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        if let Some(key) = self.key.take() {
            self.name = key
                .strip_prefix(b"\"")
                .and_then(|key| key.strip_suffix(b"\""))
                .unwrap_or(&key)
                .to_vec();
            let index = self.index();
            writer.write_all(self.layout.before_key(index, &self.name))?;
            writer.write_all(&key)?;
            writer.write_all(self.layout.after_key(index, &self.name))?;
        }
        Ok(())
    }

    fn begin_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b":")?;
        if self.at_top_level() {
            writer.write_all(self.layout.after_colon(self.index(), &self.name))?;
        }
        Ok(())
    }

    fn begin_array<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.depth += 1;
        writer.write_all(b"[")
    }

    fn end_array<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.depth -= 1;
        writer.write_all(b"]")
    }

    fn begin_array_value<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b",")
        }
    }
}

fn no_fractions(value: f64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{value} is not an integer: saves hold integers"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Layout, UpstreamLayout};
    use crate::text::from_save_bytes;
    use serde_json::{Value, json};

    #[test]
    fn writes_mjsons_escapes_and_raw_bytes() {
        let value = json!(["M\u{dc}LLER\u{1}\u{7f}\"\\/\u{8}\u{c}\n\r\t"]);
        let bytes = to_save_bytes(&value, &Layout::default()).expect("writes");
        assert_eq!(
            bytes,
            b"[\"M\xdcLLER\x01\x7f\\\"\\\\/\\b\\f\\n\\r\\t\"]".to_vec()
        );
        assert_eq!(from_save_bytes::<Value>(&bytes).expect("reads back"), value);
    }

    #[derive(Serialize)]
    struct Save {
        schema_version: u8,
        file_kind: &'static str,
        teams: Vec<Value>,
        scores: Vec<u8>,
    }

    fn save() -> Save {
        Save {
            schema_version: 3,
            file_kind: "c\u{e9}reer",
            teams: vec![json!({ "a": [1, -2], "b": {} }), json!([])],
            scores: vec![1],
        }
    }

    #[test]
    fn lays_out_the_top_level_by_a_rule() {
        let rule = UpstreamLayout {
            spaced_members: 2,
            inline: &["scores"],
        };
        let bytes = to_save_bytes(&save(), &rule).expect("writes");
        assert_eq!(
            bytes,
            b"{\n  \"schema_version\": 3,\n  \"file_kind\": \"c\xe9reer\",\n  \"teams\":[{\"a\":[1,-2],\"b\":{}},[]],\"scores\":[1]\n}\n".to_vec()
        );
    }

    #[test]
    fn the_layout_read_is_the_layout_written() {
        let text = b" {\"schema_version\" : 3,\"file_kind\":\t\"c\xe9reer\" ,\n \"teams\":[{\"a\":[1,-2],\"b\":{}},[]],\"scores\":[1]  }\r\n";
        let layout = Layout::of(text).expect("a layout");
        assert_eq!(
            to_save_bytes(&save(), &layout).expect("writes"),
            text.to_vec()
        );
    }

    #[test]
    fn what_has_no_byte_does_not_write() {
        assert!(to_save_bytes(&json!(["\u{100}"]), &Layout::default()).is_err());
        assert!(to_save_bytes(&json!([1.5]), &Layout::default()).is_err());
    }
}
