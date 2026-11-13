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
mod tests {
    //! `Settings_Save` and `settings_write_full`, each rule on a file built for it.

    use super::*;

    /// The intro of `[gameplay]` in a fresh file (upstream `section_intro`).
    const GAMEPLAY_INTRO: &str = "         ; deliberate departures from the original Amiga game";

    fn setting(section: &str, key: &str, comment: Option<&str>, value: &str) -> Setting {
        Setting {
            section: section.to_owned(),
            key: key.to_owned(),
            comment: comment.map(str::to_owned),
            value: value.as_bytes().to_vec(),
            keep: false,
        }
    }

    /// A small table: two video keys (the second described by the first's comment), a
    /// gameplay key.
    fn settings() -> Vec<Setting> {
        vec![
            setting("video", "vsync", Some("; vsync: true | false"), "false"),
            setting("video", "scale", None, "3"),
            setting("gameplay", "style", Some("; style: amiga | dos"), "amiga"),
        ]
    }

    /// The intros of the table's sections.
    const INTROS: &[SectionIntro] = &[SectionIntro {
        section: "gameplay",
        intro: GAMEPLAY_INTRO,
    }];

    fn save(settings: &[Setting], text: &str) -> Option<String> {
        let table = SettingsTable {
            settings,
            intros: INTROS,
        };
        table
            .save(Some(&IniFile::new(text.as_bytes().to_vec())))
            .map(|file| String::from_utf8(file.as_bytes().to_vec()).expect("text"))
    }

    #[test]
    fn a_fresh_file_has_its_header_and_each_section_once() {
        let mut settings = settings();
        // A key added later in another section's place joins its own section.
        settings.push(setting("video", "show_fps", None, "false"));
        let table = SettingsTable {
            settings: &settings,
            intros: INTROS,
        };
        let expected = format!(
            "{FRESH_FILE_HEADER}\n[video]\n; vsync: true | false\nvsync=false\nscale=3\nshow_fps=false\n\n[gameplay]{GAMEPLAY_INTRO}\n; style: amiga | dos\nstyle=amiga\n"
        );
        assert_eq!(table.fresh_file().as_bytes(), expected.as_bytes());
        assert_eq!(table.save(None), Some(table.fresh_file()));
    }

    #[test]
    fn a_file_holding_the_table_is_left_alone() {
        let text = "; mine\n[Video]\n  VSync = false \nscale=3\n\n[gameplay]\nstyle=amiga\r\n[mine]\nx=1\n";
        assert_eq!(save(&settings(), text), None);
    }

    #[test]
    fn an_inline_comment_is_part_of_the_value_to_the_save() {
        // Faithful to the original: `false ; off` is not `false`, so the line is rewritten and
        // the comment goes.
        let text = "[video]\nvsync=false ; off\nscale=3\n[gameplay]\nstyle=amiga\n";
        assert_eq!(
            save(&settings(), text).as_deref(),
            Some("[video]\nvsync=false\nscale=3\n[gameplay]\nstyle=amiga\n")
        );
    }

    #[test]
    fn a_changed_value_replaces_the_old_after_its_key() {
        let mut settings = settings();
        if let Some(scale) = settings.get_mut(1) {
            scale.value = b"4".to_vec();
        }
        let text = "[video]\nvsync=false\n  scale = 3 ; three\r\n[gameplay]\nstyle=amiga\n";
        // Faithful to the original: what precedes the `=` stays, the blanks after it go.
        assert_eq!(
            save(&settings, text).as_deref(),
            Some("[video]\nvsync=false\n  scale =4\n[gameplay]\nstyle=amiga\n")
        );
    }

    #[test]
    fn a_kept_setting_keeps_the_files_value() {
        let mut settings = settings();
        if let Some(scale) = settings.get_mut(1) {
            scale.value = b"4".to_vec();
            scale.keep = true;
        }
        let text = "[video]\nvsync=false\nscale=2\n[gameplay]\nstyle=amiga\n";
        assert_eq!(save(&settings, text), None);
        if let Some(vsync) = settings.first_mut() {
            vsync.value = b"true".to_vec();
        }
        assert_eq!(
            save(&settings, text).as_deref(),
            Some("[video]\nvsync=true\nscale=2\n[gameplay]\nstyle=amiga\n")
        );
    }

    #[test]
    fn a_key_twice_loses_its_second_line() {
        let text = "[video]\nvsync=false\nscale=3\nVSYNC=true\n[gameplay]\nstyle=amiga\n";
        assert_eq!(
            save(&settings(), text).as_deref(),
            Some("[video]\nvsync=false\nscale=3\n[gameplay]\nstyle=amiga\n")
        );
    }

    #[test]
    fn a_missing_key_ends_the_first_block_of_its_section() {
        let text = "[video]\nvsync=false\n\n[gameplay]\nstyle=amiga\n[video]\nmine=1\n";
        assert_eq!(
            save(&settings(), text).as_deref(),
            Some("[video]\nvsync=false\n\nscale=3\n[gameplay]\nstyle=amiga\n[video]\nmine=1\n")
        );
        // At the end of the file too.
        let text = "[gameplay]\nstyle=amiga\n[video]\nvsync=false\n";
        assert_eq!(
            save(&settings(), text).as_deref(),
            Some("[gameplay]\nstyle=amiga\n[video]\nvsync=false\nscale=3\n")
        );
    }

    #[test]
    fn a_missing_section_ends_the_file_after_a_comment() {
        let text = "[video]\nvsync=false\nscale=3\n";
        let expected = format!(
            "[video]\nvsync=false\nscale=3\n{ADDED_SECTIONS_COMMENT}[gameplay]\n; style: amiga | dos\nstyle=amiga\n"
        );
        assert_eq!(save(&settings(), text), Some(expected));
    }

    #[test]
    fn the_save_reads_lines_by_its_own_rules() {
        // `:` is no separator to the save, so the key is missing (inih reads it).
        let text = "[video]\nvsync:false\nscale=3\n[gameplay]\nstyle=amiga\n";
        let saved = save(&settings(), text).expect("vsync is missing");
        assert!(
            saved
                .starts_with("[video]\nvsync:false\nscale=3\n; vsync: true | false\nvsync=false\n")
        );
        // A section name past 31 bytes is cut.
        let long = format!("[{}]\nk=1\n", "s".repeat(SAVE_SECTION_LEN + 3));
        let long_section = [setting(&"s".repeat(SAVE_SECTION_LEN - 1), "k", None, "1")];
        assert_eq!(save(&long_section, &long), None);
    }

    #[test]
    fn lines_past_the_four_hundredth_are_lost() {
        let mut text = String::from("[video]\nvsync=true\nscale=3\n[gameplay]\nstyle=amiga\n");
        for line in 0..SAVE_MAX_LINES {
            text.push_str(&format!("; {line}\n"));
        }
        let saved = save(&settings(), &text).expect("vsync changed");
        assert_eq!(saved.lines().count(), SAVE_MAX_LINES);
        assert!(saved.starts_with("[video]\nvsync=false\n"));
    }

    #[test]
    fn a_long_line_is_two_to_the_save() {
        // The comment fills a read (255 bytes); its tail is a line of its own: `scale=9`, the
        // first `scale`, rewritten; `scale=3` is the second, dropped.
        let padding = "x".repeat(SAVE_LINE_LEN - 1 - "; ".len());
        let text =
            format!("[video]\n; {padding}scale=9\nvsync=false\nscale=3\n[gameplay]\nstyle=amiga\n");
        assert_eq!(
            save(&settings(), &text),
            Some(format!(
                "[video]\n; {padding}scale=3\nvsync=false\n[gameplay]\nstyle=amiga\n"
            ))
        );
    }
}
