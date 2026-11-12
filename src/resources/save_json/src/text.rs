//! Reading the JSON upstream's save writer produces (mjson's printer, src/mjson.c, and the
//! writers of src/save_format_json.c).
//!
//! - **Strings are bytes.** `mjson_print_str` escapes `\b \f \n \r \t \\ \"` and writes
//!   every other byte as it is: a byte past `0x7F` (a name in the game's character set),
//!   or a control character, raw. A save need not be UTF-8, nor strict JSON.
//! - **Numbers are integers** (`%d`, `%u`, `%lu`).
//!
//! Reading maps the text to standard JSON for `serde_json`: each byte past `0x7F` becomes
//! the character of the same code (U+0080..U+00FF), each raw control character in a
//! string a `\u00XX` escape. Writing maps back (`SaveFormatter`).

use std::io::Write as _;

use serde::de::DeserializeOwned;

/// The first byte past ASCII: from here on, a byte is a character of its own code.
const FIRST_NON_ASCII: u8 = 0x80;

/// The first byte that is not a control character.
const FIRST_PRINTABLE: u8 = 0x20;

/// `bytes` as standard JSON text: bytes past ASCII as the characters of their codes, raw
/// control characters inside strings escaped.
#[must_use]
pub fn to_standard_json(bytes: &[u8]) -> Vec<u8> {
    let mut text = Vec::with_capacity(bytes.len());
    let mut in_string = false;
    let mut escaping = false;
    for &byte in bytes {
        if byte >= FIRST_NON_ASCII {
            let mut utf8 = [0; 2];
            text.extend_from_slice(char::from(byte).encode_utf8(&mut utf8).as_bytes());
            escaping = false;
            continue;
        }
        if in_string {
            if escaping {
                escaping = false;
            } else if byte == b'\\' {
                escaping = true;
            } else if byte == b'"' {
                in_string = false;
            } else if byte < FIRST_PRINTABLE {
                write!(text, "\\u{byte:04x}").expect("writing to a Vec cannot fail");
                continue;
            }
        } else if byte == b'"' {
            in_string = true;
        }
        text.push(byte);
    }
    text
}

/// A value read from a save's bytes.
///
/// # Errors
///
/// When the text is not JSON, or not a `T`.
pub fn from_save_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(&to_standard_json(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn bytes_past_ascii_and_raw_controls_read() {
        let bytes = b"{\"name\":\"M\xdcLLER\x01\\n\"}";
        let value: Value = from_save_bytes(bytes).expect("reads");
        assert_eq!(value, json!({ "name": "M\u{dc}LLER\u{1}\n" }));
    }

    #[test]
    fn controls_outside_strings_are_left_to_the_parser() {
        assert_eq!(to_standard_json(b"{\"a\":\n1}"), b"{\"a\":\n1}".to_vec());
        assert!(from_save_bytes::<Value>(b"{\"a\":\x011}").is_err());
    }

    #[test]
    fn an_escaped_quote_does_not_end_a_string() {
        let value: Value = from_save_bytes(b"[\"a\\\"\x02\"]").expect("reads");
        assert_eq!(value, json!(["a\"\u{2}"]));
    }
}
