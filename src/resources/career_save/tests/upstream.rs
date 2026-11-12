//! The scalar table is upstream's: `CareerScalars` writes exactly the fields of
//! `s_career_scalars` (src/save_format_json.c), in its order. Read from upstream's
//! checkout in the cache (`just upstream_fetch`); skipped without it.

use openswos_career_save_resource::CareerScalars;
use openswos_save_json_resource::{Compact, SaveValue, from_save_bytes, to_save_bytes};
use openswos_test_utils::skip::{Skip, report};

/// The macros of the table's entries, and whether each is in the competition saves.
const ENTRY_MACROS: [(&str, bool); 6] = [
    ("FX(\"", false),
    ("FA(\"", false),
    ("FXC(\"", true),
    ("FAC(\"", true),
    ("FXS(\"", true),
    ("FAS(\"", true),
];

/// The table's field names in order, with whether each is in the competition saves.
fn upstream_table() -> Option<Vec<(String, bool)>> {
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
        return None;
    };
    let text = String::from_utf8_lossy(&bytes);
    let table = text
        .split_once("static const FieldDesc s_career_scalars[] = {")
        .and_then(|(_, rest)| rest.split_once("\n};"))
        .map(|(table, _)| table)
        .expect("upstream's scalar table");
    Some(
        table
            .lines()
            .filter_map(|line| {
                let line = line.trim_start();
                ENTRY_MACROS.iter().find_map(|(prefix, competition)| {
                    let name = line.strip_prefix(prefix)?.split('"').next()?;
                    Some((name.to_owned(), *competition))
                })
            })
            .collect(),
    )
}

/// The keys `value` writes, in order.
fn keys<T: serde::Serialize>(value: &T) -> Vec<String> {
    let bytes = to_save_bytes(value, &Compact).expect("writes");
    let value = from_save_bytes::<SaveValue>(&bytes).expect("reads");
    let SaveValue::Object(members) = value else {
        panic!("{value:?} is not an object");
    };
    members.into_iter().map(|(key, _)| key).collect()
}

#[test]
fn the_scalars_are_upstreams_table() {
    let Some(table) = upstream_table() else {
        return;
    };
    let scalars = CareerScalars {
        players_on_transfer_market_ids: Some(Vec::new()),
        ..CareerScalars::default()
    };
    let names: Vec<String> = table.into_iter().map(|(name, _)| name).collect();
    assert_eq!(keys(&scalars), names);
}
