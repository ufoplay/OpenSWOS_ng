//! Your own copy of the PC game (`just test_real_assets <game_dir>`): World 1996-97's game
//! data is valid, its rosters made from the game's `DATA/TEAM.nnn` as `world_extract` makes
//! them (src/tools/world_extract), with the game's pool and the built-in tactics. Every team a draw starts
//! from and every pool player's team is in the rosters. Skipped without
//! `OPENSWOS_GAME_DIR`.

mod common;

use std::collections::BTreeMap;

use openswos_common::Importer;
use openswos_core::{GameData, World};
use openswos_poolplyr_resource::PoolFileFormat;
use openswos_tactics_json_resource::{PresetFile, TacticsJsonFormat};
use openswos_team_json_resource::{
    EntryIds, TeamJsonDocument, TeamJsonFile, TrailingNewline, WORLD_1996_97_TAG,
    WORLD_PACKAGE_INDENT, WORLD_PACKAGE_NON_ASCII,
};
use openswos_team_resource::TeamFileFormat;
use openswos_test_utils::data::{find_ignoring_case, game_dir};
use openswos_test_utils::require;
use openswos_test_utils::tactics_path;

#[test]
fn world_1996_97_plays_with_the_pc_games_teams() {
    let data = require!(game_dir().and_then(|dir| find_ignoring_case(&dir, "DATA")));
    let world =
        World::from_package(&common::package()).unwrap_or_else(|invalid| panic!("{invalid}"));
    let mut teams = BTreeMap::new();
    for country in world.countries().iter() {
        let Some(roster) = &country.roster else {
            continue;
        };
        let name = format!("TEAM.{:03}", country.number.0);
        let path =
            find_ignoring_case(&data, &name).unwrap_or_else(|_| panic!("the game has {name}"));
        let file =
            TeamFileFormat::import(&std::fs::read(&path).expect("readable")).expect("a team file");
        let json = TeamJsonFile::from_team_file(&file, Some(&country.name), |team| {
            EntryIds::derived(WORLD_1996_97_TAG, team)
        });
        teams.insert(
            roster.clone(),
            TeamJsonDocument {
                file: json,
                indent: WORLD_PACKAGE_INDENT,
                non_ascii: WORLD_PACKAGE_NON_ASCII,
                trailing_newline: TrailingNewline::Present,
            },
        );
    }
    let pool_path = find_ignoring_case(&data, "POOLPLYR.DAT").expect("the game's pool");
    let pool =
        PoolFileFormat::import(&std::fs::read(&pool_path).expect("readable")).expect("a pool");
    let tactics = tactics();
    let data =
        GameData::new(world, &teams, pool, tactics).unwrap_or_else(|invalid| panic!("{invalid}"));
    let rosters = data.rosters();
    assert_eq!(rosters.rosters().len(), 71);
    let all: Vec<_> = rosters
        .rosters()
        .iter()
        .flat_map(|roster| &roster.teams)
        .collect();
    assert!(all.len() > 1_500, "{} teams", all.len());
    assert!(
        all.iter().all(|team| team.canonical),
        "derived checksums are the teams'"
    );
    assert_eq!(data.pool().len(), 900);
}

/// The built-in tactics (`data/tactics/`).
fn tactics() -> BTreeMap<PresetFile, openswos_tac_resource::TeamTactics> {
    std::fs::read_dir(tactics_path())
        .expect("the built-in tactics")
        .map(|entry| {
            let path = entry.expect("an entry").path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a UTF-8 name")
                .to_owned();
            let file = PresetFile::parse(&name).unwrap_or_else(|| panic!("{name}: a preset"));
            let json = TacticsJsonFormat::import(&std::fs::read(&path).expect("readable"))
                .expect("tactics");
            (file, json.tactics)
        })
        .collect()
}
