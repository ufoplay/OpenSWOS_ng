//! World 2026-27's rosters, from its package (downloaded into `target/cache/`, D8): every
//! `teams/*.json` round-trips byte for byte, converts to typed teams and back, and states
//! the checksums its data gives. Skipped when the package can be neither found in the cache
//! nor downloaded.

use std::collections::BTreeSet;
use std::io::Read;

use openswos_common::{Exporter, Importer};
use openswos_team_json_resource::{EntryIds, TeamEntry, TeamJsonFormat, team_checksum};
use openswos_test_utils::cache::{WORLD_2627_PACKAGE, fetch};

/// The folder of the rosters in the package.
const TEAMS_DIR: &str = "/teams/";

/// The rosters of the package: (file name, bytes).
fn rosters() -> Option<Vec<(String, Vec<u8>)>> {
    let path = match fetch(&WORLD_2627_PACKAGE) {
        Ok(path) => path,
        Err(skip) => {
            openswos_test_utils::skip::report(&skip);
            return None;
        }
    };
    let file = std::fs::File::open(&path).expect("the cached package");
    let mut archive = zip::ZipArchive::new(file).expect("a zip");
    let mut rosters = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("a zip entry");
        let name = entry.name().expect("a readable entry name").into_owned();
        if name.contains(TEAMS_DIR) && name.ends_with(".json") {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("an entry's bytes");
            rosters.push((name, bytes));
        }
    }
    assert!(!rosters.is_empty(), "no roster in {}", path.display());
    Some(rosters)
}

#[test]
fn every_roster_round_trips_byte_for_byte() {
    let Some(rosters) = rosters() else { return };
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        let exported =
            TeamJsonFormat::export(&document).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            exported == *bytes,
            "{name}: the export differs from the file"
        );
    }
}

#[test]
fn every_team_converts_to_typed_and_back() {
    let Some(rosters) = rosters() else { return };
    let mut teams = 0;
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        for entry in &document.file.teams {
            let team = entry
                .to_team()
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            let back = TeamEntry::from_team(&team, EntryIds::of(entry))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(&back, entry, "{name}: {}", team.name);
            teams += 1;
        }
    }
    eprintln!(
        "{} rosters, {teams} teams: all convert both ways",
        rosters.len()
    );
}

#[test]
fn every_team_states_the_checksum_its_data_gives() {
    let Some(rosters) = rosters() else { return };
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        for entry in &document.file.teams {
            let team = entry
                .to_team()
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(
                team_checksum(&team),
                entry.checksum,
                "{name}: {}",
                team.name
            );
        }
    }
}

#[test]
/// World 2026-27's ids are `uuid5(namespace, "2627/team/<club id>")` with the SWOS 96/97 Mod's
/// own club ids (upstream `scripts/world_import.py`), which the JSON does not carry: they can
/// only be read, never recomputed. Every one names a single team or player.
fn every_id_is_unique() {
    let Some(rosters) = rosters() else { return };
    let mut ids = BTreeSet::new();
    let mut count = 0;
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        for entry in &document.file.teams {
            let entry_ids = EntryIds::of(entry);
            for id in std::iter::once(entry_ids.team).chain(entry_ids.players) {
                count += 1;
                assert!(ids.insert(id), "{name}: {id} names two teams or players");
            }
        }
    }
    eprintln!("{count} ids, all distinct");
}
