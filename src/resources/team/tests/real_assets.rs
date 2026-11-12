//! Your own copy of the PC game (`just test_real_assets <game_dir>`): every team database
//! in its `DATA/` folder and every edited-teams file (`*.EDT`) round-trips. Skipped without
//! `OPENSWOS_GAME_DIR`.

use std::collections::BTreeMap;

use openswos_common::Importer;
use openswos_team_resource::{TeamFileError, TeamFileFormat};
use openswos_test_utils::data::{find_ignoring_case, game_dir};
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;

/// `DATA/TEAM.248` of the GOG release: not a team database (another layout, a stray
/// file the game does not read; upstream's file list ends at `TEAM.085`).
const NOT_A_TEAM_FILE: &str = "TEAM.248";

/// The extension of the edited-teams files.
const EDT_EXTENSION: &str = ".EDT";

#[test]
fn every_team_database_round_trips() {
    let data = require!(game_dir().and_then(|dir| find_ignoring_case(&dir, "DATA")));
    let files = list_fixtures(&data, |name| {
        name.to_ascii_uppercase().starts_with("TEAM.")
            && !name.eq_ignore_ascii_case(NOT_A_TEAM_FILE)
    });
    assert!(!files.is_empty(), "no TEAM.* file in {}", data.display());
    let mut teams = 0;
    for path in &files {
        let bytes = std::fs::read(path).expect("readable team file");
        teams += assert_round_trip::<TeamFileFormat>(&bytes).teams.len();
    }
    eprintln!("{} team files, {teams} teams: all round-trip", files.len());
}

#[test]
fn the_stray_team_248_is_rejected() {
    let data = require!(game_dir().and_then(|dir| find_ignoring_case(&dir, "DATA")));
    let path = require!(find_ignoring_case(&data, NOT_A_TEAM_FILE));
    let bytes = std::fs::read(&path).expect("readable file");
    assert!(matches!(
        TeamFileFormat::import(&bytes),
        Err(TeamFileError::Read(_) | TeamFileError::Team { .. })
    ));
}

/// The game's `*.EDT` files (`CUSTOMS.EDT`, `SAFE.EDT`).
fn edt_files() -> Option<Vec<std::path::PathBuf>> {
    let dir = match game_dir() {
        Ok(dir) => dir,
        Err(skip) => {
            openswos_test_utils::skip::report(&skip);
            return None;
        }
    };
    let files = list_fixtures(&dir, |name| {
        name.to_ascii_uppercase().ends_with(EDT_EXTENSION)
    });
    Some(files)
}

#[test]
fn every_edt_file_round_trips() {
    let Some(files) = edt_files() else { return };
    for path in &files {
        let bytes = std::fs::read(path).expect("readable EDT file");
        let teams = assert_round_trip::<TeamFileFormat>(&bytes).teams.len();
        eprintln!("{}: {teams} teams round-trip", path.display());
    }
}

/// Faithful to the original: the `.EDT` files store the global team numbers little-endian.
/// Swapped, each is its team file's base plus the team's ordinal, as the game sets them.
#[test]
fn edt_global_numbers_are_stored_little_endian() {
    let Some(files) = edt_files() else { return };
    for path in &files {
        let bytes = std::fs::read(path).expect("readable EDT file");
        let file = TeamFileFormat::import(&bytes).expect("a valid EDT file");
        let mut bases: BTreeMap<u8, Vec<i32>> = BTreeMap::new();
        for team in &file.teams {
            let stored = u16::from_le_bytes(team.global_number.0.to_be_bytes());
            let base = i32::from(stored) - i32::from(team.id.ordinal.0);
            bases.entry(team.id.file.0).or_default().push(base);
        }
        for (team_file, found) in &bases {
            assert!(
                found.windows(2).all(|pair| pair.first() == pair.last()),
                "{}: team file {team_file} has bases {found:?}",
                path.display()
            );
        }
    }
}
