//! The table is upstream's: the game's sections' rows of `settings_build`
//! (src/swos_settings.c), in its order, each comment verbatim, and `section_intro`'s intros. Read
//! from upstream's checkout in the cache (`just upstream_fetch`); skipped without it.

use openswos_settings_ini_resource::{SECTION_INTROS, Settings};
use openswos_test_utils::skip::{Skip, report};

/// What starts a row of the table.
const ROW: &str = "SETTING(n) = (IniEntry){";

/// The sections typed here.
const TYPED_SECTIONS: [&str; 8] = [
    "gameplay", "replays", "career", "testing", "general", "mods", "tweaks", "options",
];

/// How upstream's source names the setup in a comment (`PROJECT_SETUP_NAME`).
const SETUP_NAME: &str = "OpenSWOS Setup";

/// Upstream's src/swos_settings.c, from its checkout.
fn upstream_source() -> Option<String> {
    let path = openswos_test_utils::cache_path()
        .join("upstream")
        .join("OpenSWOS")
        .join("src")
        .join("swos_settings.c");
    let Ok(bytes) = std::fs::read(&path) else {
        report(&Skip::new(format!(
            "no {} (just upstream_fetch)",
            path.display()
        )));
        return None;
    };
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// `settings_build`'s body.
fn settings_build(source: &str) -> &str {
    let (_, body) = source
        .split_once("static int settings_build(")
        .expect("settings_build");
    let (body, _) = body
        .split_once("static void settings_write_full(")
        .expect("its end");
    body
}

/// The `(section, key)` of each row, in order: the first two string literals after `{`.
fn rows(table: &str) -> Vec<(String, String)> {
    table
        .split(ROW)
        .skip(1)
        .filter_map(|row| {
            let mut literals = row.split('"').skip(1).step_by(2);
            Some((literals.next()?.to_owned(), literals.next()?.to_owned()))
        })
        .collect()
}

/// A line of a comment as upstream's source spells it: escaped, the setup's name a macro.
fn as_source(line: &str) -> String {
    line.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace(SETUP_NAME, "\" PROJECT_SETUP_NAME \"")
}

#[test]
fn the_table_is_upstreams_rows_in_order() {
    let Some(source) = upstream_source() else {
        return;
    };
    let upstream: Vec<(String, String)> = rows(settings_build(&source))
        .into_iter()
        .filter(|(section, _)| TYPED_SECTIONS.contains(&section.as_str()))
        .collect();
    let ours: Vec<(String, String)> = Settings::default()
        .table()
        .into_iter()
        .map(|setting| (setting.section, setting.key))
        .collect();
    assert_eq!(ours, upstream);
}

#[test]
fn every_comment_and_intro_is_upstreams() {
    let Some(source) = upstream_source() else {
        return;
    };
    let table = settings_build(&source);
    for setting in Settings::default().table() {
        let Some(comment) = setting.comment else {
            continue;
        };
        for line in comment.split_inclusive('\n') {
            assert!(
                table.contains(&as_source(line)),
                "[{}] {}: {line:?}",
                setting.section,
                setting.key
            );
        }
    }
    for intro in SECTION_INTROS {
        assert!(
            source.contains(&format!("\"{}\"", intro.intro)),
            "{}",
            intro.section
        );
    }
}
