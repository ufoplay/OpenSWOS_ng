//! Your own copy of the PC game (`just test_real_assets <game_dir>`): every European-cup
//! pool round-trips, and each of its teams is the one its id names in the team files.
//! Skipped without `OPENSWOS_GAME_DIR`.

use std::path::Path;

use openswos_common::Importer;
use openswos_team_resource::TeamFileFormat;
use openswos_test_utils::data::{find_ignoring_case, game_dir};
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;
use openswos_tmd_resource::TmdFileFormat;

/// The pools of the PC game.
const POOLS: [&str; 3] = ["EUROCUP.TMD", "EUROCWC.TMD", "UEFACUP.TMD"];

#[test]
fn every_pool_round_trips() {
    let data = require!(game_dir().and_then(|dir| find_ignoring_case(&dir, "DATA")));
    let files = list_fixtures(&data, |name| name.to_ascii_uppercase().ends_with(".TMD"));
    let mut names: Vec<String> = files
        .iter()
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().to_ascii_uppercase())
        .collect();
    names.sort();
    assert_eq!(names, POOLS.to_vec());
    for path in &files {
        let bytes = std::fs::read(path).expect("readable pool");
        assert_round_trip::<TmdFileFormat>(&bytes);
    }
}

#[test]
fn every_pool_team_is_the_team_its_id_names() {
    let data = require!(game_dir().and_then(|dir| find_ignoring_case(&dir, "DATA")));
    for pool_name in POOLS {
        let pool = TmdFileFormat::import(&read(&data, pool_name)).expect("a valid pool");
        for team in &pool.teams {
            let team_file = format!("TEAM.{:03}", team.id.file.0);
            let file = TeamFileFormat::import(&read(&data, &team_file)).expect("a valid team file");
            let named = file
                .teams
                .iter()
                .find(|candidate| candidate.id == team.id)
                .unwrap_or_else(|| {
                    panic!(
                        "{pool_name}: {} has no team {:?} in {team_file}",
                        team.name, team.id
                    )
                });
            assert_eq!(named.name, team.name, "{pool_name}: {:?}", team.id);
        }
    }
}

fn read(data: &Path, name: &str) -> Vec<u8> {
    let path = find_ignoring_case(data, name).unwrap_or_else(|skip| panic!("{skip}"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}
