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
    let text =
        "; mine\n[Video]\n  VSync = false \nscale=3\n\n[gameplay]\nstyle=amiga\r\n[mine]\nx=1\n";
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
        saved.starts_with("[video]\nvsync:false\nscale=3\n; vsync: true | false\nvsync=false\n")
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
