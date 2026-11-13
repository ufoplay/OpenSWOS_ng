//! Saving settings as the game does (upstream src/swos_settings.c `Settings_Save`,
//! `flush_section_missing`, `copy_value`, `settings_write_full`), from a table of settings:
//! section, key, comment, value (the game's own is `settings_build`).
//!
//! The save reads the file by rules of its own, not inih's: a line is what `fgets` reads into
//! 256 bytes (a longer one is cut in two), at most 400 of them; a section is `[name]` (the
//! name cut to 31 bytes); a pair is `key=value` (`=` only; the key cut to 47 bytes; the value
//! trimmed of blanks, comments included). It changes nothing unless something is to be
//! written: a value the table holds otherwise, a key twice, a key missing. Then:
//!
//! - a changed value replaces the old one after `key=`, its line ending `\n`;
//! - a key's second occurrence goes;
//! - a missing key joins the end of the first block of its section, a missing section the
//!   end of the file, after a comment;
//! - every other line stays as read. Faithful to the original: lines past the 400th are
//!   lost.

use crate::file::IniFile;
use crate::text::{Fgets, NEWLINE, RETURN, c_str, eqi, is_blank, skip_end, skip_start, truncated};

/// The save's line buffer (`lines[][256]`), its NUL included.
const SAVE_LINE_LEN: usize = 256;

/// The most lines the save reads (`lines[400]`).
pub const SAVE_MAX_LINES: usize = 400;

/// The save's section buffer (`cur_sec[32]`), its NUL included.
const SAVE_SECTION_LEN: usize = 32;

/// The save's key buffer (`key[48]`), its NUL included.
const SAVE_KEY_LEN: usize = 48;

/// The save's value buffer (`fileval[300]`), its NUL included.
const SAVE_VALUE_LEN: usize = 300;

/// What opens a section's name.
const SECTION_OPEN: u8 = b'[';

/// What closes it.
const SECTION_CLOSE: u8 = b']';

/// What separates a key from its value.
const EQUALS: u8 = b'=';

/// What starts a comment line.
const COMMENT_STARTS: &[u8] = b";#";

/// The comment a fresh file starts with (`settings_write_full`).
pub const FRESH_FILE_HEADER: &str = concat!(
    "; SWOS \u{2014} display, audio and gameplay settings.\n",
    "; Hand-edit freely: SWOS only updates values you change and adds missing\n",
    "; keys; your comments, blank lines and commented-out keys are preserved.\n",
    "; Booleans accept true/false, on/off, yes/no, 1/0 (case-insensitive).\n",
);

/// The comment before the sections a save adds to a file (`Settings_Save`).
pub const ADDED_SECTIONS_COMMENT: &str =
    "\n; --- keys added by a newer SWOS build (defaults; edit as you like) ---\n";

/// A setting the game writes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Setting {
    /// Its section.
    pub section: String,
    /// Its key.
    pub key: String,
    /// The comment before it, a line or more, written when the setting is written whole
    /// (a fresh file, a missing key); none for a key the one before describes.
    pub comment: Option<String>,
    /// Its value, as bytes (the game copies values byte for byte).
    pub value: Vec<u8>,
    /// Unchanged by the player since the file was read: the file's value stands, whatever
    /// it is (upstream `keep`, from `s_baseline`).
    pub keep: bool,
}

/// What a fresh file writes after a section's header (`section_intro`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectionIntro {
    /// The section.
    pub section: &'static str,
    /// What follows its `]`: spaces and an inline comment.
    pub intro: &'static str,
}

/// The settings a save writes, in the game's order, and the sections' intros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SettingsTable<'a> {
    /// The settings: a key once each.
    pub settings: &'a [Setting],
    /// The sections' intros of a fresh file.
    pub intros: &'a [SectionIntro],
}

/// A line as the save reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SavedLine<'a> {
    /// A section's header: its name, when it has its `]`.
    Section(Option<&'a [u8]>),
    /// `key=value`: the key, and where the `=` is.
    Pair { key: &'a [u8], equals: usize },
    /// A comment, a blank line, a line without `=`.
    Other,
}

impl<'a> SavedLine<'a> {
    fn of(line: &'a [u8]) -> Self {
        let indent = line.iter().take_while(|&&byte| is_blank(byte)).count();
        let text = line.get(indent..).unwrap_or_default();
        match text.split_first() {
            Some((&SECTION_OPEN, rest)) => Self::Section(
                rest.iter()
                    .position(|&byte| byte == SECTION_CLOSE)
                    .and_then(|end| rest.get(..end))
                    .map(|name| truncated(name, SAVE_SECTION_LEN - 1)),
            ),
            None => Self::Other,
            Some((first, _))
                if COMMENT_STARTS.contains(first) || matches!(*first, NEWLINE | RETURN) =>
            {
                Self::Other
            }
            Some(_) => match text.iter().position(|&byte| byte == EQUALS) {
                Some(at) => Self::Pair {
                    key: truncated(
                        skip_end(text.get(..at).unwrap_or_default(), is_blank),
                        SAVE_KEY_LEN - 1,
                    ),
                    equals: indent + at,
                },
                None => Self::Other,
            },
        }
    }
}

/// The value after a pair's `=`, as the save compares it (`copy_value`): blanks skipped, up
/// to the line's end, trailing blanks dropped.
fn file_value(after_equals: &[u8]) -> &[u8] {
    let value = skip_start(after_equals, is_blank);
    let len = value
        .iter()
        .take(SAVE_VALUE_LEN - 1)
        .take_while(|&&byte| byte != NEWLINE && byte != RETURN)
        .count();
    skip_end(value.get(..len).unwrap_or_default(), is_blank)
}

/// The lines the save reads from `bytes`, as C strings.
fn save_lines(bytes: &[u8]) -> Vec<&[u8]> {
    let mut reader = Fgets::new(bytes);
    let mut lines = Vec::new();
    while lines.len() < SAVE_MAX_LINES {
        match reader.read(SAVE_LINE_LEN - 1) {
            Some(line) => lines.push(c_str(line)),
            None => break,
        }
    }
    lines
}

/// Writes `setting` whole: its comment, then `key=value`.
fn write_setting(out: &mut Vec<u8>, setting: &Setting) {
    if let Some(comment) = &setting.comment {
        out.extend_from_slice(comment.as_bytes());
        out.push(NEWLINE);
    }
    out.extend_from_slice(setting.key.as_bytes());
    out.push(EQUALS);
    out.extend_from_slice(&setting.value);
    out.push(NEWLINE);
}

impl SettingsTable<'_> {
    /// The first setting of `section` named `key`, ignoring ASCII case.
    fn find(&self, section: &[u8], key: &[u8]) -> Option<usize> {
        self.settings.iter().position(|setting| {
            eqi(section, setting.section.as_bytes()) && eqi(key, setting.key.as_bytes())
        })
    }

    /// Whether the line of `key` in `section`, its value `value`, is to be rewritten.
    fn changes(&self, index: usize, value: &[u8]) -> bool {
        self.settings
            .get(index)
            .is_some_and(|setting| !setting.keep && value != setting.value.as_slice())
    }

    /// A fresh file (`settings_write_full`): the header, then each section once, in the
    /// order the table first names it, with every setting of it in the table's order.
    #[must_use]
    pub fn fresh_file(&self) -> IniFile {
        let mut out = FRESH_FILE_HEADER.as_bytes().to_vec();
        let mut done = vec![false; self.settings.len()];
        for (first, setting) in self.settings.iter().enumerate() {
            if done.get(first).copied().unwrap_or(true) {
                continue;
            }
            let section = setting.section.as_bytes();
            let intro = self
                .intros
                .iter()
                .find(|intro| eqi(intro.section.as_bytes(), section))
                .map_or("", |intro| intro.intro);
            out.push(NEWLINE);
            out.push(SECTION_OPEN);
            out.extend_from_slice(section);
            out.push(SECTION_CLOSE);
            out.extend_from_slice(intro.as_bytes());
            out.push(NEWLINE);
            for (index, other) in self.settings.iter().enumerate().skip(first) {
                if let Some(written) = done.get_mut(index)
                    && !*written
                    && eqi(other.section.as_bytes(), section)
                {
                    *written = true;
                    write_setting(&mut out, other);
                }
            }
        }
        IniFile::new(out)
    }

    /// Saves over `existing` (`None`: there is no file yet) as the game does
    /// (`Settings_Save`). `None` when the file is to be left as it is.
    #[must_use]
    pub fn save(&self, existing: Option<&IniFile>) -> Option<IniFile> {
        let Some(existing) = existing else {
            return Some(self.fresh_file());
        };
        let lines = save_lines(existing.as_bytes());
        self.needs_writing(&lines)
            .then(|| IniFile::new(self.rewrite(&lines)))
    }

    /// Whether a value changes, a key comes twice or a key is missing (the save's first
    /// pass).
    fn needs_writing(&self, lines: &[&[u8]]) -> bool {
        let mut seen = vec![false; self.settings.len()];
        let mut section: &[u8] = &[];
        let mut dirty = false;
        for &line in lines {
            match SavedLine::of(line) {
                SavedLine::Section(Some(name)) => section = name,
                SavedLine::Section(None) | SavedLine::Other => {}
                SavedLine::Pair { key, equals } => {
                    if let Some(index) = self.find(section, key)
                        && let Some(was_seen) = seen.get_mut(index)
                    {
                        let value = file_value(line.get(equals + 1..).unwrap_or_default());
                        dirty |= *was_seen || self.changes(index, value);
                        *was_seen = true;
                    }
                }
            }
        }
        dirty || seen.contains(&false)
    }

    /// Writes the settings of `section` still missing, marking them seen
    /// (`flush_section_missing`).
    fn flush_missing(&self, out: &mut Vec<u8>, section: &[u8], seen: &mut [bool]) {
        for (setting, was_seen) in self.settings.iter().zip(seen.iter_mut()) {
            if !*was_seen && eqi(setting.section.as_bytes(), section) {
                write_setting(out, setting);
                *was_seen = true;
            }
        }
    }

    /// The file rewritten (the save's second pass).
    fn rewrite(&self, lines: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut seen = vec![false; self.settings.len()];
        let mut section: &[u8] = &[];
        for &line in lines {
            match SavedLine::of(line) {
                SavedLine::Section(name) => {
                    // Leaving a section: its missing keys end its (first) block.
                    if !section.is_empty() {
                        self.flush_missing(&mut out, section, &mut seen);
                    }
                    if let Some(name) = name {
                        section = name;
                    }
                    out.extend_from_slice(line);
                }
                SavedLine::Other => out.extend_from_slice(line),
                SavedLine::Pair { key, equals } => {
                    let found = self.find(section, key).and_then(|index| {
                        Some((index, self.settings.get(index)?, seen.get_mut(index)?))
                    });
                    match found {
                        None => out.extend_from_slice(line),
                        Some((_, _, was_seen)) if *was_seen => {}
                        Some((index, setting, was_seen)) => {
                            *was_seen = true;
                            let (head, tail) =
                                line.split_at_checked(equals + 1).unwrap_or((line, &[]));
                            if self.changes(index, file_value(tail)) {
                                out.extend_from_slice(head);
                                out.extend_from_slice(&setting.value);
                                out.push(NEWLINE);
                            } else {
                                out.extend_from_slice(line);
                            }
                        }
                    }
                }
            }
        }
        if !section.is_empty() {
            self.flush_missing(&mut out, section, &mut seen);
        }
        if seen.contains(&false) {
            out.extend_from_slice(ADDED_SECTIONS_COMMENT.as_bytes());
            let mut current: Option<&str> = None;
            for (setting, _) in self.settings.iter().zip(&seen).filter(|(_, seen)| !**seen) {
                if current
                    .is_none_or(|current| !eqi(current.as_bytes(), setting.section.as_bytes()))
                {
                    current = Some(&setting.section);
                    out.push(SECTION_OPEN);
                    out.extend_from_slice(setting.section.as_bytes());
                    out.push(SECTION_CLOSE);
                    out.push(NEWLINE);
                }
                write_setting(&mut out, setting);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
