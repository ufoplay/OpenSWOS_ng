//! Your settings file (`SWOS.ini` beside `OPENSWOS_SAVES_DIR`, `just test_saves`): it writes
//! back byte for byte, the game reads it without an error, a save of the values it holds
//! leaves it alone, a changed value changes its line only, a missing key joins its section;
//! read as the game's typed settings, a save adds only the keys it lacks and a changed setting
//! changes its line. A file longer than the save reads (a fresh file of a newer build can be)
//! is rewritten by every save, as the game does: then only that is checked. Skipped without
//! the file; never committed (it holds your settings).

use openswos_settings_ini_resource::{
    IniFile, IniFileFormat, SAVE_MAX_LINES, Setting, Settings, SettingsTable,
};
use openswos_test_utils::data::settings_file;
use openswos_test_utils::fixtures::assert_round_trip;
use openswos_test_utils::require;
use openswos_test_utils::skip::{Skip, report};

/// The most bytes a read of the save takes (`fgets` into 256 bytes, its NUL included).
const SAVE_READ_BYTES: usize = 255;

/// A key no build writes, added by the test.
const TEST_KEY: &str = "openswos_ng_test_key";

/// The test key's value.
const TEST_VALUE: &str = "1";

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The lines of `bytes`, each without its newline.
fn lines(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.split(|&byte| byte == b'\n').collect()
}

/// A table of the file's own settings: the first value of each key.
fn own_settings(file: &IniFile) -> Vec<Setting> {
    let mut settings: Vec<Setting> = Vec::new();
    for value in file.read().values {
        if !settings
            .iter()
            .any(|setting| value.is(&setting.section, &setting.key))
        {
            settings.push(Setting {
                section: text(&value.section),
                key: text(&value.name),
                comment: None,
                value: value.value.clone(),
                keep: false,
            });
        }
    }
    settings
}

/// Whether the save reads all of `bytes`: at most [`SAVE_MAX_LINES`] reads, a line longer than a
/// read taking more than one.
fn fits_the_save(bytes: &[u8]) -> bool {
    let reads: usize = bytes
        .split_inclusive(|&byte| byte == b'\n')
        .map(|line| line.len().div_ceil(SAVE_READ_BYTES))
        .sum();
    reads <= SAVE_MAX_LINES
}

/// Faithful to the original: a file longer than the save reads loses its last lines and the
/// keys there come back as missing, so every save rewrites it. Reported as a skip of the rest.
fn rewritten_past_the_saves_lines(saved: Option<&IniFile>) {
    assert!(saved.is_some(), "a save rewrites a file past its lines");
    report(&Skip::new(format!(
        "your settings file is longer than the save's {SAVE_MAX_LINES} lines, which every save \
         rewrites: only that is checked"
    )));
}

fn save(settings: &[Setting], file: &IniFile) -> Option<IniFile> {
    SettingsTable {
        settings,
        intros: &[],
    }
    .save(Some(file))
}

#[test]
fn your_settings_file_reads_and_saves_as_the_game_does() {
    let path = require!(settings_file());
    let bytes = std::fs::read(&path).expect("a readable settings file");
    let file = assert_round_trip::<IniFileFormat>(&bytes);
    assert_eq!(file.read().first_error, None, "{}", path.display());
    let settings = own_settings(&file);
    assert!(!settings.is_empty());
    if !fits_the_save(&bytes) {
        rewritten_past_the_saves_lines(save(&settings, &file).as_ref());
        return;
    }
    assert_eq!(save(&settings, &file), None, "a save of its own values");

    let changed = {
        let mut changed = settings.clone();
        changed.first_mut().expect("a setting").value.push(b'X');
        changed
    };
    let first = changed.first().expect("a setting");
    let saved = save(&changed, &file).expect("a changed value is written");
    let (before, after) = (lines(&bytes), lines(saved.as_bytes()));
    assert_eq!(before.len(), after.len());
    let differ = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    assert_eq!(differ, 1);
    assert!(
        saved
            .read()
            .values
            .iter()
            .any(|value| value.is(&first.section, &first.key) && value.value == first.value)
    );

    let mut added = settings.clone();
    let section = added.first().expect("a setting").section.clone();
    added.push(Setting {
        section: section.clone(),
        key: TEST_KEY.to_owned(),
        comment: None,
        value: TEST_VALUE.as_bytes().to_vec(),
        keep: false,
    });
    let saved = save(&added, &file).expect("a missing key is written");
    assert_eq!(lines(saved.as_bytes()).len(), before.len() + 1);
    assert!(
        saved
            .read()
            .values
            .iter()
            .any(|value| value.is(&section, TEST_KEY) && value.value == TEST_VALUE.as_bytes())
    );
}

#[test]
fn your_settings_save_as_the_game_saves_them() {
    let path = require!(settings_file());
    let file = IniFile::new(std::fs::read(&path).expect("a readable settings file"));
    let settings = Settings::read(&file);
    if !fits_the_save(file.as_bytes()) {
        rewritten_past_the_saves_lines(settings.save(Some(&file), Some(&settings)).as_ref());
        return;
    }
    // Left as read, a save adds only the keys the file lacks (a file of an older build):
    // every line of it stays, in order, and it reads back as the same settings.
    let left = settings
        .save(Some(&file), Some(&settings))
        .unwrap_or_else(|| file.clone());
    assert_eq!(Settings::read(&left), settings);
    let mut saved_lines = lines(left.as_bytes()).into_iter();
    assert!(
        lines(file.as_bytes())
            .iter()
            .all(|line| saved_lines.any(|saved| saved == *line))
    );
    // Saved again, it is left alone.
    assert_eq!(settings.save(Some(&left), Some(&settings)), None);

    // A setting changed by the player changes its line.
    let mut changed = settings.clone();
    changed.replays.auto_save = !settings.replays.auto_save;
    let saved = changed
        .save(Some(&file), Some(&settings))
        .expect("a changed setting is written");
    assert_eq!(Settings::read(&saved), changed);
    let (before, after) = (lines(left.as_bytes()), lines(saved.as_bytes()));
    assert_eq!(before.len(), after.len());
    assert_eq!(before.iter().zip(&after).filter(|(a, b)| a != b).count(), 1);
}
