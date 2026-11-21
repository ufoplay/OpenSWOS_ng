//! `assets/TEAM_JSON/OCEANIA.json`: World 1996-97's team JSON for the PC game's `TEAM.085`,
//! as this port writes it into a world package: derived 96/97 ids, and the World 2026-27
//! package's style (unsigned numbers, a trailing newline). It round-trips byte for byte and
//! holds exactly the teams of the binary `assets/TEAM/TEAM.085`. With `OPENSWOS_BLESS=1`,
//! the fixture is (re)written from the binary file.

use openswos_common::{Exporter, Importer};
use openswos_team_json_resource::{
    EntryIds, TeamJsonDocument, TeamJsonFile, TeamJsonFormat, WORLD_1996_97_TAG,
    WORLD_PACKAGE_INDENT, WORLD_PACKAGE_NON_ASCII, WORLD_PACKAGE_TRAILING_NEWLINE,
};
use openswos_team_resource::TeamFileFormat;
use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::{SidecarMode, assert_round_trip};

/// Setup's name of team file 85 (its `kCountries` table).
const COUNTRY_NAME: &str = "OCEANIA";

fn read(folder: &str, name: &str) -> Vec<u8> {
    let path = assets_path().join(folder).join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// The fixture's expected bytes, made from the binary team file.
fn from_binary() -> Vec<u8> {
    let binary = TeamFileFormat::import(&read("TEAM", "TEAM.085")).expect("a valid team file");
    let file = TeamJsonFile::from_team_file(&binary, Some(COUNTRY_NAME), |team| {
        EntryIds::derived(WORLD_1996_97_TAG, team)
    });
    TeamJsonFormat::export(&TeamJsonDocument {
        file,
        indent: WORLD_PACKAGE_INDENT,
        non_ascii: WORLD_PACKAGE_NON_ASCII,
        trailing_newline: WORLD_PACKAGE_TRAILING_NEWLINE,
    })
    .expect("exports")
}

#[test]
fn the_fixture_is_world_1996_97s_json_for_team_085() {
    let expected = from_binary();
    let path = assets_path().join("TEAM_JSON").join("OCEANIA.json");
    match SidecarMode::from_env() {
        SidecarMode::Bless => std::fs::write(&path, &expected).expect("fixture written"),
        SidecarMode::Compare => assert!(
            read("TEAM_JSON", "OCEANIA.json") == expected,
            "{} is stale",
            path.display()
        ),
    }
}

#[test]
fn the_fixture_round_trips_and_holds_the_binary_teams() {
    let document = assert_round_trip::<TeamJsonFormat>(&read("TEAM_JSON", "OCEANIA.json"));
    assert_eq!(document.trailing_newline, WORLD_PACKAGE_TRAILING_NEWLINE);
    let solomon_islands = document.file.teams.last().expect("a last team");
    assert!(
        solomon_islands
            .players
            .iter()
            .all(|player| player.nationality.written() == 151),
        "nationalities are written unsigned"
    );
    assert_eq!(document.file.country, Some(85));
    assert_eq!(
        document.file.country_name,
        Some(Some(COUNTRY_NAME.to_owned()))
    );
    let binary = TeamFileFormat::import(&read("TEAM", "TEAM.085")).expect("a valid team file");
    assert_eq!(document.file.to_team_file().expect("converts"), binary);
    let names: Vec<&str> = document
        .file
        .teams
        .iter()
        .map(|entry| entry.team.team_name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["AUSTRALIA", "FIJI", "NEW ZEALAND", "SOLOMON ISLANDS"]
    );
}

#[test]
fn world_1996_97_ids_are_generated_from_the_original_files() {
    let document = TeamJsonFormat::import(&read("TEAM_JSON", "OCEANIA.json")).expect("valid");
    for entry in &document.file.teams {
        let team = entry.to_team().expect("converts");
        assert_eq!(
            EntryIds::of(entry),
            EntryIds::derived(WORLD_1996_97_TAG, &team),
            "{}",
            team.name
        );
    }
}

#[test]
fn every_team_and_player_must_state_its_ids() {
    let original: serde_json::Value =
        serde_json::from_slice(&read("TEAM_JSON", "OCEANIA.json")).expect("JSON");
    let without = |path: &[&str], key: &str| {
        let mut value = original.clone();
        let object = path
            .iter()
            .fold(&mut value, |node, step| match step.parse::<usize>() {
                Ok(index) => &mut node[index],
                Err(_) => &mut node[*step],
            })
            .as_object_mut()
            .expect("an object");
        assert!(object.remove(key).is_some(), "{key} was there");
        serde_json::to_vec(&value).expect("serializes")
    };
    for (what, bytes) in [
        ("a team uuid", without(&["teams", "0"], "uuid")),
        ("a checksum", without(&["teams", "1"], "checksum")),
        (
            "a player uuid",
            without(&["teams", "2", "players", "7"], "uuid"),
        ),
    ] {
        assert!(
            TeamJsonFormat::import(&bytes).is_err(),
            "a file without {what} imported"
        );
    }
}
