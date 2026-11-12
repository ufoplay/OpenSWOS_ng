//! World 2026-27's competitions, from its package (downloaded into `target/cache/`, D8):
//! its `competitions/leagues.json` round-trips byte for byte. Skipped when the package can
//! be neither found in the cache nor downloaded.

use std::io::Read;

use openswos_common::json::TrailingNewline;
use openswos_competitions_json_resource::{
    CompetitionsFile, CompetitionsJsonFormat, Continent, LeaguesFile,
};
use openswos_test_utils::cache::{WORLD_2627_PACKAGE, fetch};
use openswos_test_utils::fixtures::assert_round_trip;

/// The leagues of the package.
const LEAGUES: &str = "/competitions/leagues.json";

/// The package's `competitions/leagues.json`.
fn leagues() -> Option<LeaguesFile> {
    let path = match fetch(&WORLD_2627_PACKAGE) {
        Ok(path) => path,
        Err(skip) => {
            openswos_test_utils::skip::report(&skip);
            return None;
        }
    };
    let file = std::fs::File::open(&path).expect("the cached package");
    let mut archive = zip::ZipArchive::new(file).expect("a zip");
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("a zip entry");
        let name = entry.name().expect("a readable entry name").into_owned();
        if name.ends_with(LEAGUES) {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("the entry's bytes");
            let document = assert_round_trip::<CompetitionsJsonFormat>(&bytes);
            assert_eq!(document.trailing_newline, TrailingNewline::Present);
            let CompetitionsFile::Leagues(leagues) = document.file else {
                panic!("{name} is not a leagues file");
            };
            return Some(leagues);
        }
    }
    panic!("no {LEAGUES} in {}", path.display());
}

#[test]
fn the_leagues_round_trip_byte_for_byte() {
    let Some(leagues) = leagues() else { return };
    assert_eq!(leagues.leagues.len(), 91);
    let italy = leagues
        .leagues
        .iter()
        .find(|league| league.id.as_str() == "_italianLeague")
        .expect("the Italian league");
    assert_eq!(italy.divisions.len(), 14, "Serie A to the Serie D groups");
}

#[test]
fn new_countries_name_their_continent() {
    let Some(leagues) = leagues() else { return };
    let new: Vec<_> = leagues
        .leagues
        .iter()
        .filter_map(|league| league.new.as_ref())
        .collect();
    assert_eq!(new.len(), 26);
    let andorra = new
        .iter()
        .find(|country| country.name == "ANDORRA")
        .expect("Andorra");
    assert_eq!(andorra.continent, Continent::Europe);
    let canada = new
        .iter()
        .find(|country| country.name == "CANADA")
        .expect("Canada");
    assert_eq!(canada.continent, Continent::NorthAmerica);
}
