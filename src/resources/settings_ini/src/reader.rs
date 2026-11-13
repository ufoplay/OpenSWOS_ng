//! What the game reads from a settings file: inih's rules (upstream src/ini.c
//! `ini_parse_stream`, configured by include/ini.h), each `name=value` handed to the settings
//! in the file's order (`Settings_Load`, src/swos_settings.c).
//!
//! The rules are ported from inih — Copyright (C) 2009-2025, Ben Hoyt;
//! SPDX-License-Identifier: BSD-3-Clause; <https://github.com/benhoyt/inih>.
//!
//! - A read takes at most 199 bytes (`INI_MAX_LINE`); the rest of a longer line is dropped,
//!   an error. A NUL ends what a read holds.
//! - The first line may start with UTF-8's byte order mark, skipped.
//! - A line is trimmed (`isspace`). Empty, or starting with `;` or `#`: a comment.
//! - Starting with whitespace, after a `name=value`: a continuation, another value of that
//!   name (blank lines and comments between do not end it; a section does).
//! - `[name]`: a section (its name at most 49 bytes). No `]`: an error.
//! - `name=value` or `name:value`: the name and value trimmed, the value up to an inline
//!   comment: a `;` after whitespace. Neither `=` nor `:`: an error.

use crate::text::{Fgets, NEWLINE, c_str, eqi, is_c_space, skip_end, skip_start, truncated};

/// The line buffer (`INI_MAX_LINE`), its NUL included.
const INI_MAX_LINE: usize = 200;

/// The buffer the rest of a long line is read into and dropped (`abyss`).
const ABYSS_LEN: usize = 16;

/// The section's buffer (`MAX_SECTION`), its NUL included.
const MAX_SECTION: usize = 50;

/// The buffer of the last name read (`MAX_NAME`), its NUL included: the name of a
/// continuation line.
const MAX_NAME: usize = 50;

/// UTF-8's byte order mark, skipped at the start of the file (`INI_ALLOW_BOM`).
const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// What starts a comment line (`INI_START_COMMENT_PREFIXES`).
const COMMENT_PREFIXES: &[u8] = b";#";

/// What starts a comment after whitespace (`INI_INLINE_COMMENT_PREFIXES`).
const INLINE_COMMENT_PREFIXES: &[u8] = b";";

/// What opens a section's name.
const SECTION_OPEN: u8 = b'[';

/// What closes it.
const SECTION_CLOSE: &[u8] = b"]";

/// What separates a name from its value.
const NAME_VALUE_SEPARATORS: &[u8] = b"=:";

/// A setting as the game reads it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IniValue {
    /// The section it is in (empty before the first).
    pub section: Vec<u8>,
    /// Its name.
    pub name: Vec<u8>,
    /// Its value.
    pub value: Vec<u8>,
}

impl IniValue {
    /// Whether it is `name` of `section`, as the game compares them (ignoring ASCII case).
    #[must_use]
    pub fn is(&self, section: &str, name: &str) -> bool {
        eqi(&self.section, section.as_bytes()) && eqi(&self.name, name.as_bytes())
    }
}

/// What the game reads from a settings file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct IniRead {
    /// The settings, in the file's order; a name may come more than once, the last one
    /// counting where the game keeps the last.
    pub values: Vec<IniValue>,
    /// The first line inih could not read (from 1, as it counts its reads), if any. The
    /// game reads on regardless.
    pub first_error: Option<usize>,
}

/// The index of the first byte of `bytes` in `chars`, or of an inline comment, or the
/// length (`ini_find_chars_or_comment`).
fn find_chars_or_comment(bytes: &[u8], chars: &[u8]) -> usize {
    let mut was_space = false;
    for (at, &byte) in bytes.iter().enumerate() {
        if chars.contains(&byte) || (was_space && INLINE_COMMENT_PREFIXES.contains(&byte)) {
            return at;
        }
        was_space = is_c_space(byte);
    }
    bytes.len()
}

/// `bytes` up to an inline comment, trimmed at the end.
fn before_comment(bytes: &[u8]) -> &[u8] {
    skip_end(
        bytes
            .get(..find_chars_or_comment(bytes, &[]))
            .unwrap_or_default(),
        is_c_space,
    )
}

/// The state of a read: the section, and the name a continuation line continues.
#[derive(Default)]
struct Reading {
    section: Vec<u8>,
    previous_name: Vec<u8>,
    read: IniRead,
}

impl Reading {
    /// Records an error on line `line`, unless one came before.
    fn error(&mut self, line: usize) {
        self.read.first_error.get_or_insert(line);
    }

    /// Hands a setting to the settings.
    fn value(&mut self, name: &[u8], value: &[u8]) {
        self.read.values.push(IniValue {
            section: self.section.clone(),
            name: name.to_vec(),
            value: value.to_vec(),
        });
    }

    /// Reads line `number`: `trimmed` is it trimmed, `indented` whether bytes were skipped
    /// from its start.
    fn line(&mut self, number: usize, trimmed: &[u8], indented: bool) {
        match trimmed.split_first() {
            None => {}
            Some((first, _)) if COMMENT_PREFIXES.contains(first) => {}
            Some(_) if indented && !self.previous_name.is_empty() => {
                let name = self.previous_name.clone();
                self.value(&name, before_comment(trimmed));
            }
            Some((&SECTION_OPEN, rest)) => {
                let end = find_chars_or_comment(rest, SECTION_CLOSE);
                match rest.split_at_checked(end) {
                    Some((name, close)) if close.starts_with(SECTION_CLOSE) => {
                        self.section = truncated(name, MAX_SECTION - 1).to_vec();
                        self.previous_name.clear();
                    }
                    Some(_) | None => self.error(number),
                }
            }
            Some(_) => {
                let end = find_chars_or_comment(trimmed, NAME_VALUE_SEPARATORS);
                match trimmed.split_at_checked(end) {
                    Some((name, separated)) => match separated.split_first() {
                        Some((separator, value)) if NAME_VALUE_SEPARATORS.contains(separator) => {
                            let name = skip_end(name, is_c_space);
                            // Cut at the comment first: a `;` right after the separator's
                            // spaces is one.
                            let value = skip_start(before_comment(value), is_c_space);
                            self.previous_name = truncated(name, MAX_NAME - 1).to_vec();
                            self.value(name, value);
                        }
                        Some(_) | None => self.error(number),
                    },
                    None => self.error(number),
                }
            }
        }
    }
}

/// What the game reads from the settings file `bytes`.
#[must_use]
pub fn read(bytes: &[u8]) -> IniRead {
    let mut lines = Fgets::new(bytes);
    let mut reading = Reading::default();
    let mut number = 0;
    while let Some(buffer) = lines.read(INI_MAX_LINE - 1) {
        let line = c_str(buffer);
        number += 1;
        if line.len() == INI_MAX_LINE - 1 && line.last() != Some(&NEWLINE) {
            // A line longer than the buffer: the rest of it is dropped.
            while let Some(abyss) = lines.read(ABYSS_LEN - 1) {
                reading.error(number);
                if c_str(abyss).last() == Some(&NEWLINE) {
                    break;
                }
            }
        }
        let start = if number == 1 {
            line.strip_prefix(BOM).unwrap_or(line)
        } else {
            line
        };
        let skipped = skip_start(start, is_c_space);
        // Upstream's `start > line`: a byte order mark or whitespace was skipped.
        let indented = skipped.len() < line.len();
        reading.line(number, skip_end(skipped, is_c_space), indented);
    }
    reading.read
}

#[cfg(test)]
mod tests;
