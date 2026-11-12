//! Your own copy of the PC game (`just test_real_assets <game_dir>`): every team database
//! in its `DATA/` folder round-trips. Skipped without `OPENSWOS_GAME_DIR`.

use openswos_common::Importer;
use openswos_team_resource::{TeamFileError, TeamFileFormat};
use openswos_test_utils::data::{find_ignoring_case, game_dir};
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;

/// `DATA/TEAM.248` of the GOG release: not a team database (another layout, a stray
/// file the game does not read; upstream's file list ends at `TEAM.085`).
const NOT_A_TEAM_FILE: &str = "TEAM.248";

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
