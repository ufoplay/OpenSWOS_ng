//! A JSON file of a package with its text's layout, so it writes back byte for byte: Python's
//! `json.dump`, its indentation, how it writes characters beyond ASCII and whether it ends
//! with a newline, each kept as read (`openswos_common::json`).

use openswos_common::json::{Indent, NonAscii, TrailingNewline, to_text};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A package's JSON file: its content and its layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonFile<T> {
    /// The file's content.
    pub content: T,
    /// Its indentation.
    pub indent: Indent,
    /// Its characters beyond ASCII.
    pub non_ascii: NonAscii,
    /// Whether its text ends with a newline.
    pub trailing_newline: TrailingNewline,
}

impl<T: DeserializeOwned> JsonFile<T> {
    /// The file in `bytes`, its layout as read.
    ///
    /// # Errors
    ///
    /// When the bytes are not JSON, or not a `T`.
    pub fn read(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        Ok(Self {
            content: serde_json::from_slice(bytes)?,
            indent: Indent::of(bytes),
            non_ascii: NonAscii::of(bytes),
            trailing_newline: TrailingNewline::of(bytes),
        })
    }
}

impl<T: Serialize> JsonFile<T> {
    /// The file's bytes, in its layout.
    ///
    /// # Errors
    ///
    /// When the content does not serialise as JSON.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        Ok(to_text(
            &self.content,
            self.indent,
            self.non_ascii,
            self.trailing_newline,
        )?
        .into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_writes_back_in_its_layout() {
        // (a `Value`'s keys are sorted)
        for text in [
            "{\n  \"list\": [\n    1\n  ],\n  \"name\": \"PELÉ\"\n}\n",
            "{\n \"list\": [],\n \"name\": \"PEL\\u00c9\"\n}",
        ] {
            let file: JsonFile<serde_json::Value> = JsonFile::read(text.as_bytes()).expect("JSON");
            assert_eq!(file.to_bytes().expect("serializes"), text.as_bytes());
        }
        assert!(JsonFile::<serde_json::Value>::read(b"{").is_err());
    }
}
