//! The competition scalars are upstream's: `CompetitionScalars` writes exactly the fields
//! of `s_career_scalars` (src/save_format_json.c) tagged for the competition saves, in its
//! order. Read from upstream's checkout in the cache (`just upstream_fetch`); skipped
//! without it.

use openswos_competition_save_resource::CompetitionScalars;
use openswos_save_json_resource::{Compact, SaveValue, from_save_bytes, to_save_bytes};
use openswos_test_utils::skip::{Skip, report};

/// The table's macros of the entries in the competition saves.
const COMPETITION_MACROS: [&str; 4] = ["FXC(\"", "FAC(\"", "FXS(\"", "FAS(\""];

#[test]
fn the_scalars_are_upstreams_tagged_fields() {
    let path = openswos_test_utils::cache_path()
        .join("upstream")
        .join("OpenSWOS")
        .join("src")
        .join("save_format_json.c");
    let Ok(bytes) = std::fs::read(&path) else {
        report(&Skip::new(format!(
            "no {} (just upstream_fetch)",
            path.display()
        )));
        return;
    };
    let text = String::from_utf8_lossy(&bytes);
    let table = text
        .split_once("static const FieldDesc s_career_scalars[] = {")
        .and_then(|(_, rest)| rest.split_once("\n};"))
        .map(|(table, _)| table)
        .expect("upstream's scalar table");
    let tagged: Vec<String> = table
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            COMPETITION_MACROS
                .iter()
                .find_map(|prefix| Some(line.strip_prefix(prefix)?.split('"').next()?.to_owned()))
        })
        .collect();
    let bytes = to_save_bytes(&CompetitionScalars::default(), &Compact).expect("writes");
    let SaveValue::Object(members) = from_save_bytes::<SaveValue>(&bytes).expect("reads") else {
        panic!("not an object");
    };
    let keys: Vec<String> = members.into_iter().map(|(key, _)| key).collect();
    assert_eq!(keys, tagged);
}
