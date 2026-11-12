//! Your own copy of the PC game (`just test_real_assets <game_dir>`): the transfer pool
//! round-trips, and every pool record is a copy of a squad player of the team files: the
//! player at its ordinal in the team its id names. Skipped without `OPENSWOS_GAME_DIR`.

use std::collections::BTreeMap;
use std::path::Path;

use openswos_common::Importer;
use openswos_poolplyr_resource::PoolFileFormat;
use openswos_team_resource::{TeamFile, TeamFileFormat};
use openswos_test_utils::data::{find_ignoring_case, game_dir};
use openswos_test_utils::fixtures::assert_round_trip;
use openswos_test_utils::require;

#[test]
fn the_pool_round_trips() {
    let path = require!(game_dir().and_then(|dir| find_ignoring_case(&dir, "DATA/POOLPLYR.DAT")));
    let bytes = std::fs::read(&path).expect("readable pool");
    assert_round_trip::<PoolFileFormat>(&bytes);
}

#[test]
fn every_pool_record_copies_its_squad_player() {
    let data = require!(game_dir().and_then(|dir| find_ignoring_case(&dir, "DATA")));
    let pool = PoolFileFormat::import(&read(&data, "POOLPLYR.DAT")).expect("a valid pool");
    let mut team_files: BTreeMap<u8, TeamFile> = BTreeMap::new();
    for entry in &pool.players {
        let file_number = entry.team.file.0;
        let file = team_files.entry(file_number).or_insert_with(|| {
            TeamFileFormat::import(&read(&data, &format!("TEAM.{file_number:03}")))
                .expect("a valid team file")
        });
        let team = file
            .teams
            .iter()
            .find(|team| team.id == entry.team)
            .unwrap_or_else(|| panic!("{}: no team {:?}", entry.player.name, entry.team));
        assert_eq!(team.name, entry.team_name, "{}", entry.player.name);
        let squad_player = team.players.get(usize::from(entry.player_ordinal));
        assert_eq!(
            squad_player,
            Some(&entry.player),
            "{} of {}",
            entry.player.name,
            team.name
        );
    }
}

fn read(data: &Path, name: &str) -> Vec<u8> {
    let path = find_ignoring_case(data, name).unwrap_or_else(|skip| panic!("{skip}"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}
