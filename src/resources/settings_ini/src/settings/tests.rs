//! The typed settings: read as the game reads them, written as upstream's table writes them.

use super::*;
use crate::writer::FRESH_FILE_HEADER;

/// The bytes inih reads of a line: its 200-byte buffer, less the NUL.
const LINE_BYTES: usize = 199;

/// What `language`'s 16-byte buffer keeps.
const LANGUAGE_BYTES: usize = 15;

fn file(text: &str) -> IniFile {
    IniFile::new(text.as_bytes().to_vec())
}

/// The value upstream's table writes for `key` of `section`.
fn table_value(settings: &Settings, section: &str, key: &str) -> String {
    let setting = settings
        .table()
        .into_iter()
        .find(|setting| setting.section == section && setting.key == key)
        .expect("a setting of the table");
    String::from_utf8(setting.value).expect("text")
}

#[test]
fn the_table_is_upstreams_order_and_defaults() {
    let table = Settings::default().table();
    assert_eq!(table.len(), table::DEFINITIONS.len());
    let first_keys: Vec<(&str, &str)> = table
        .iter()
        .take(2)
        .map(|setting| (setting.section.as_str(), setting.key.as_str()))
        .collect();
    assert_eq!(
        first_keys,
        [("gameplay", "fire_hold_lockout"), ("gameplay", "style")]
    );
    let defaults = Settings::default();
    for (section, key, value) in [
        ("gameplay", "style", "amiga"),
        ("replays", "frame_overlay", "true"),
        ("replays", "speed", "normal"),
        ("general", "language", "en"),
        ("general", "currency", "pound"),
        ("general", "world", "9697"),
        ("general", "file_sort", "name"),
        ("tweaks", "cpu_cards", "false"),
        ("tweaks", "five_substitutes", "true"),
        ("options", "game_length", "3"),
        ("options", "pitch_type", "seasonal"),
        ("options", "auto_replays", "true"),
        ("options", "all_teams_equal", "false"),
    ] {
        assert_eq!(
            table_value(&defaults, section, key),
            value,
            "{section}.{key}"
        );
    }
}

#[test]
fn a_fresh_file_reads_back_as_its_settings() {
    let fresh = Settings::default().fresh_file();
    let text = String::from_utf8(fresh.as_bytes().to_vec()).expect("text");
    assert!(text.starts_with(FRESH_FILE_HEADER));
    let sections: Vec<&str> = text.lines().filter(|line| line.starts_with('[')).collect();
    assert_eq!(
        sections,
        [
            "[gameplay]         ; deliberate departures from the original Amiga game",
            "[replays]",
            "[testing]",
            "[general]",
            "[mods]",
            "[tweaks]",
            "[options]"
        ]
    );
    assert_eq!(fresh.read().first_error, None);
    let read = Settings::read(&fresh);
    assert_eq!(read.table(), Settings::default().table());
    // Read back, the options are set: the table wrote them.
    assert_eq!(read.options.game_length, Some(GameLength::Three));
    assert_eq!(Settings::default().save(Some(&fresh), None), None);
}

/// Upstream's self-tests (`Settings_GameOptionsSelfTest`, `Settings_LauncherSelfTest`)
/// for the game's sections, and every other value a setting takes: written alone in a
/// file, read and saved, it comes back as written.
#[test]
fn every_value_comes_back() {
    let mut cases: Vec<(&str, &str, String)> = Vec::new();
    for style in GameplayStyle::ALL {
        cases.push(("gameplay", "style", style.spelling().to_owned()));
    }
    for tactics in CpuTactics::ALL {
        cases.push(("gameplay", "cpu_tactics", tactics.spelling().to_owned()));
    }
    for speed in ReplaySpeed::ALL {
        cases.push(("replays", "speed", speed.spelling().to_owned()));
    }
    for length in GameLength::ALL {
        cases.push(("options", "game_length", length.spelling().to_owned()));
    }
    for pitch in PitchType::ALL {
        cases.push(("options", "pitch_type", pitch.spelling().to_owned()));
    }
    for world in ["9697", "2627"] {
        cases.push(("general", "world", world.to_owned()));
    }
    for language in ["en", "de", "fr", "es", "it", "nl"] {
        cases.push(("general", "language", language.to_owned()));
    }
    for definition in &table::DEFINITIONS {
        if Settings::default().table().iter().any(|setting| {
            setting.key == definition.key && (setting.value == b"true" || setting.value == b"false")
        }) {
            for value in ["true", "false"] {
                cases.push((definition.section, definition.key, value.to_owned()));
            }
        }
    }
    for (section, key, value) in cases {
        let written = file(&format!("[{section}]\n{key}={value}\n"));
        let settings = Settings::read(&written);
        assert_eq!(
            table_value(&settings, section, key),
            value,
            "{section}.{key}"
        );
        let saved = settings.save(None, None).expect("a fresh file");
        assert_eq!(
            table_value(&Settings::read(&saved), section, key),
            value,
            "{section}.{key}"
        );
    }
}

#[test]
fn values_the_game_does_not_take_are_ignored() {
    let settings = Settings::read(&file(
        "[gameplay]\nstyle=pc\nfire_hold_lockout=maybe\n[general]\nlanguage=\nworld=\nfile_sort=size\n[options]\ngame_length=4\n",
    ));
    assert_eq!(settings, Settings::default());
}

#[test]
fn the_game_reads_sections_and_keys_in_any_case_but_the_tweaks() {
    let settings = Settings::read(&file(
        "[GamePlay]\nSTYLE=Dos\n[TWEAKS]\ncpu_cards=yes\nMore_Fouls=on\n",
    ));
    assert_eq!(settings.gameplay.style, GameplayStyle::Dos);
    assert!(settings.tweaks.contains(Tweaks::CPU_CARDS));
    // Faithful to the original: `Tweaks_Bit` compares the key exactly.
    assert!(!settings.tweaks.contains(Tweaks::MORE_FOULS));
}

#[test]
fn the_last_value_taken_counts() {
    let settings = Settings::read(&file(
        "[gameplay]\nstyle=dos\nstyle=xbox\nstyle=bogus\n[tweaks]\nthree_points_for_a_win=off\n",
    ));
    assert_eq!(settings.gameplay.style, GameplayStyle::Xbox);
    assert!(!settings.tweaks.contains(Tweaks::THREE_POINTS));
    assert!(settings.tweaks.contains(Tweaks::FIVE_SUBSTITUTES));
}

#[test]
fn texts_are_cut_to_the_games_buffers() {
    let long = "x".repeat(2 * LINE_BYTES);
    let settings = Settings::read(&file(&format!(
        "[general]\nlanguage={long}\nworld=20262027\nfile_sort=DATE\n[mods]\nenabled={long}\n"
    )));
    assert_eq!(settings.general.language.len(), LANGUAGE_BYTES);
    assert_eq!(settings.general.world, b"2026202");
    // The spelling is kept.
    assert_eq!(settings.general.file_sort, b"DATE");
    // The line's 199 bytes come first: `enabled=` leaves 191 of the 255 the buffer keeps.
    assert_eq!(settings.mods.enabled.len(), LINE_BYTES - "enabled=".len());
}

#[test]
fn any_word_sets_the_tactics_and_the_speed() {
    let settings = Settings::read(&file(
        "[gameplay]\ncpu_tactics=expert\ncpu_tactics=whatever\n[replays]\nspeed=half\n",
    ));
    assert_eq!(settings.gameplay.cpu_tactics, CpuTactics::Normal);
    assert_eq!(settings.replays.speed, ReplaySpeed::Half);
}

#[test]
fn a_setting_the_player_left_keeps_the_files_value() {
    let fresh =
        String::from_utf8(Settings::default().fresh_file().as_bytes().to_vec()).expect("text");
    let file = IniFile::new(fresh.replace("style=amiga", "style=bogus").into_bytes());
    let read = Settings::read(&file);
    // `bogus` is ignored: the setting is `amiga`, and the player leaves it.
    assert_eq!(read.gameplay.style, GameplayStyle::Amiga);
    assert_eq!(read.save(Some(&file), Some(&read)), None);
    // Without the baseline the table's value replaces it.
    let saved = read.save(Some(&file), None).expect("rewritten");
    assert_eq!(Settings::read(&saved), read);
    assert!(!String::from_utf8_lossy(saved.as_bytes()).contains("bogus"));
    // Changed by the player, the setting is written.
    let mut changed = read.clone();
    changed.gameplay.style = GameplayStyle::Dos;
    let saved = changed.save(Some(&file), Some(&read)).expect("rewritten");
    assert_eq!(Settings::read(&saved).gameplay.style, GameplayStyle::Dos);
    // A key twice is rewritten whatever the baseline.
    let twice = IniFile::new([fresh.as_bytes(), b"[gameplay]\nstyle=dos\n"].concat());
    let read = Settings::read(&twice);
    assert_eq!(read.gameplay.style, GameplayStyle::Dos);
    assert!(read.save(Some(&twice), Some(&read)).is_some());
}
