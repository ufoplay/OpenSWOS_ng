//! A small valid world and its game data, for the tests: three countries (one listed twice),
//! two leagues linked to a play-off and a block, cups with seeds, a tournament, two
//! qualifying groups, two blocks, a career index, four competition tables and a tier of
//! UEFA Cup berths. The teams, the pool and the tactics come from `assets/`.

use std::collections::BTreeMap;

use openswos_common::{Importer, TeamFileNumber, TeamId, TeamOrdinal};
use openswos_competitions_json_resource::CompetitionsJsonFormat;
use openswos_poolplyr_resource::{PoolFile, PoolFileFormat};
use openswos_tac_resource::TeamTactics;
use openswos_tactics_json_resource::{PresetFile, TacticsJsonFormat};
use openswos_team_json_resource::{
    EntryIds, TeamJsonDocument, TeamJsonFile, TrailingNewline, WORLD_1996_97_TAG,
};
use openswos_team_resource::{TeamFile, TeamFileFormat};
use openswos_test_utils::assets_path;
use openswos_world_package_resource::{WorldIdentity, WorldPackage, WorldTag};

/// Albania's number.
pub(crate) const ALBANIA: TeamFileNumber = TeamFileNumber(0);

/// England's number.
pub(crate) const ENGLAND: TeamFileNumber = TeamFileNumber(8);

/// The competition files, by name.
const FILES: [(&str, &str); 10] = [
    (
        "countries.json",
        r#"{"type": "countries", "count": 4, "countries": [
            {"number": 8, "name": "ENGLAND", "define": "CN_ENGLAND", "team_file": "teams/ENGLAND.json"},
            {"number": 0, "name": "ALBANIA", "define": "CN_ALBANIA", "team_file": "teams/ALBANIA.json"},
            {"number": 8, "name": "ENGLAND", "define": "CN_ENGLAND", "team_file": "teams/ENGLAND.json"},
            {"number": 80, "name": "EUROPE", "define": "CN_EUROPE", "team_file": null}]}"#,
    ),
    (
        "leagues.json",
        r#"{"type": "competition_leagues", "count": 2, "leagues": [
            {"id": "_albanianLeague", "comp": {"comp_id": 37, "type": 0, "team_number": 0, "begin_month": 64, "end_month": 32, "name_table_offset": 0, "reserved": 0, "team_list_offset": 0, "seed_reorder_flag": 0},
             "divisions_count": 1, "rounds": 2, "points_for_win": 3, "substitutes": 3, "max_subs": 5,
             "divisions": [{"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0}],
             "reserved1": 0, "name": []},
            {"id": "_englishLeague", "comp": {"comp_id": 44, "type": 0, "team_number": 8, "begin_month": 64, "end_month": 32, "name_table_offset": 0, "reserved": 0, "team_list_offset": 0, "seed_reorder_flag": 0},
             "divisions_count": 1, "rounds": 2, "points_for_win": 3, "substitutes": 3, "max_subs": 5,
             "divisions": [{"teams": 1, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0}],
             "reserved1": 0, "name": [], "playoff_links": ["_div1Playoffs", "_englishLeagues1", null]}]}"#,
    ),
    (
        "cups.json",
        r#"{"type": "competition_cups", "count": 3, "competitions": [
            {"id": "_albanianCup", "type": "cup", "comp": {"comp_id": 109, "type": 1, "team_number": 0, "begin_month": 88, "end_month": 32, "name_table_offset": 0, "reserved": 0, "team_list_offset": 0, "seed_reorder_flag": 0},
             "num_rounds_override": 0, "teams": 2, "away_goals_mode": 1, "substitutes": 3, "points_for_win": 5, "round_string_base": 1,
             "round_rule_flags": [{"legs": 1, "extra_time": 1, "penalties": 1}], "name": [], "team_seed_list": []},
            {"id": "_cupWinnersCup", "type": "cup", "comp": {"comp_id": 1, "type": 1, "team_number": -1, "begin_month": 64, "end_month": 32, "name_table_offset": 15, "reserved": 0, "team_list_offset": 21, "seed_reorder_flag": 1},
             "num_rounds_override": 0, "teams": 2, "away_goals_mode": 1, "substitutes": 3, "points_for_win": 5, "round_string_base": 1,
             "round_rule_flags": [{"legs": 2, "extra_time": 1, "penalties": 1}], "name": ["_aEuropeanCupWin", "_aEuroCwCup"],
             "team_seed_list": [{"country": "CN_ENGLAND", "ordinal": 0, "team": null}, {"country": 0, "ordinal": 1, "team": null}]},
            {"id": "_friendly", "type": "cup", "comp": {"comp_id": 5, "type": 0, "team_number": -1, "begin_month": 0, "end_month": 0, "name_table_offset": 15, "reserved": 0, "team_list_offset": 21, "seed_reorder_flag": 0},
             "num_rounds_override": 0, "teams": 2, "away_goals_mode": 1, "substitutes": 3, "points_for_win": 3, "round_string_base": 0,
             "round_rule_flags": [{"legs": 0, "extra_time": 0, "penalties": 0}], "name": [], "team_seed_list": []}]}"#,
    ),
    (
        "tournaments.json",
        r#"{"type": "competition_tournaments", "count": 1, "competitions": [
            {"id": "_euroCup", "type": "tournament", "comp": {"comp_id": 0, "type": 2, "team_number": -1, "begin_month": 64, "end_month": 32, "name_table_offset": 35, "reserved": 0, "team_list_offset": 41, "seed_reorder_flag": 1},
             "seeding_type": 1, "menu_state": 1, "play_each_team": 2, "away_goals_mode": 3, "substitutes": 3, "points_for_win": 5, "round_string_base": 4,
             "rounds": [{"team_count": 2, "group_count": 0, "team_group_count": 2}], "header_reserved": 0,
             "round_rule_flags": [{"legs": 2, "extra_time": 1, "penalties": 1}], "name": ["_aEuropeanChampi_1", "_aEuropeanCup"],
             "team_seed_list": [{"country": "CN_ALBANIA", "ordinal": 0, "team": "TIRANA"}]}]}"#,
    ),
    (
        "playoffs.json",
        r#"{"type": "playoffs", "count": 1, "playoffs": [
            {"id": "_div1Playoffs", "comp": {"comp_id": 47, "type": 1, "team_number": 8, "begin_month": -128, "end_month": -124, "name_table_offset": 11, "reserved": 1, "team_list_offset": 17, "seed_reorder_flag": 1},
             "num_rounds_override": 0, "teams": 4, "away_goals_mode": 1, "substitutes": 3, "max_subs": 3, "round_string_base": 1,
             "round_rule_flags": [{"legs": 2, "extra_time": 1, "penalties": 1}], "name": ["_aDiv1Playoffs", "_aPlayoffs"]}]}"#,
    ),
    (
        "qualifying.json",
        r#"{"type": "qualifying_groups", "count": 2, "groups": [
            {"id": "_europeTable", "comp_cup": [252, "_euroCup", "_cupWinnersCup", -1], "countries": [0, 8, 255, 99]},
            {"id": "_customsTable", "comp_cup": [-2, -1], "countries": []}]}"#,
    ),
    (
        "league_blocks.json",
        r#"{"type": "league_blocks", "count": 2, "groups": [
            {"id": "_englishLeagues1", "leagues": [{"league": "_englishLeague", "link_params": [0, 0, 0, 1]}, {"league": null, "link_params": []}],
             "playoffs": "_div1Playoffs", "next_group": "_englishLeagues2", "qualifier_index_pairs": [0, 4, -1]},
            {"id": "_englishLeagues2", "leagues": [{"league": "_englishLeague", "link_params": [0]}], "next_group": null}]}"#,
    ),
    (
        "career.json",
        r#"{"type": "career_index", "count": 3, "contests": [
            {"competition": "_euroCup", "competition_copy": "_euroCupCopy"},
            {"competition": "_friendly", "competition_copy": "_friendlyCopy"},
            {"competition": null, "competition_copy": null}]}"#,
    ),
    (
        "country_competitions.json",
        r#"{"type": "country_competitions", "count": 4, "countries": [
            {"number": 0, "name": "ALBANIA", "table": "_albaniaTable", "leagues": ["_albanianLeague"], "cups": ["_albanianCup"]},
            {"number": 8, "name": "ENGLAND", "table": "_englandTable", "leagues": ["_englishLeague"]},
            {"number": 80, "name": "EUROPE", "table": "_europeTable"},
            {"number": 72, "name": "CUSTOM", "table": "_customsTable"}]}"#,
    ),
    (
        "seed_buckets.json",
        r#"{"type": "seed_buckets", "_comment": "test", "euro_cup_qual_seed_buckets": [
            {"tier": "bucket2", "size": 2, "allocation": [{"country": "ENGLAND", "number": 8, "berths": 1}, {"country": "ALBANIA", "number": 0, "berths": 1}]}]}"#,
    ),
];

/// The small world's package, without rosters.
pub(crate) fn package() -> WorldPackage {
    package_with(&[])
}

/// The small world's package with `changes` made to its files' text: each `(file, from,
/// to)` replaces the first `from` in `file`, which must hold it.
pub(crate) fn package_with(changes: &[(&str, &str, &str)]) -> WorldPackage {
    let competitions = FILES
        .iter()
        .map(|(name, text)| {
            let text = changes.iter().filter(|(file, _, _)| file == name).fold(
                (*text).to_owned(),
                |text, (_, from, to)| {
                    assert!(text.contains(from), "{name} has no {from}");
                    text.replacen(from, to, 1)
                },
            );
            let document = CompetitionsJsonFormat::import(text.as_bytes())
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            ((*name).to_owned(), document)
        })
        .collect();
    WorldPackage {
        identity: WorldIdentity {
            comments: Vec::new(),
            tag: WorldTag("9697".to_owned()),
            name: "World 1996-97".to_owned(),
            other: Vec::new(),
        },
        manifest: None,
        teams: BTreeMap::new(),
        competitions,
    }
}

/// `count` teams of the PC game's `TEAM.085` (four), renumbered as `country`'s.
fn teams_of(country: TeamFileNumber, count: usize) -> TeamFile {
    let bytes = std::fs::read(assets_path().join("TEAM/TEAM.085")).expect("the fixture");
    let mut file = TeamFileFormat::import(&bytes).expect("a team file");
    file.teams.truncate(count);
    for (ordinal, team) in (0u8..).zip(&mut file.teams) {
        team.id = TeamId {
            file: country,
            ordinal: TeamOrdinal(ordinal),
        };
    }
    file
}

/// A roster as World 1996-97's are made: derived ids and checksums.
pub(crate) fn roster(file: &TeamFile, name: &str) -> TeamJsonDocument {
    TeamJsonDocument {
        file: TeamJsonFile::from_team_file(file, Some(name), |team| {
            EntryIds::derived(WORLD_1996_97_TAG, team)
        }),
        trailing_newline: TrailingNewline::Present,
    }
}

/// The small world's rosters: Albania's two teams, England's one, Europe none.
pub(crate) fn teams() -> BTreeMap<String, TeamJsonDocument> {
    BTreeMap::from([
        (
            "ALBANIA.json".to_owned(),
            roster(&teams_of(ALBANIA, 2), "ALBANIA"),
        ),
        (
            "ENGLAND.json".to_owned(),
            roster(&teams_of(ENGLAND, 1), "ENGLAND"),
        ),
    ])
}

/// A pool of one player: the fourth of Albania's second team.
pub(crate) fn pool() -> PoolFile {
    let bytes = std::fs::read(assets_path().join("POOLPLYR/POOLPLYR.DAT")).expect("the fixture");
    let mut pool = PoolFileFormat::import(&bytes).expect("a pool");
    pool.players.truncate(1);
    for player in &mut pool.players {
        player.team = TeamId {
            file: ALBANIA,
            ordinal: TeamOrdinal(1),
        };
        player.player_ordinal = 3;
    }
    pool
}

/// Upstream's built-in tactics (`assets/TACTICS_JSON/`).
pub(crate) fn tactics() -> BTreeMap<PresetFile, TeamTactics> {
    let dir = assets_path().join("TACTICS_JSON");
    std::fs::read_dir(&dir)
        .expect("the fixtures")
        .map(|entry| {
            let path = entry.expect("an entry").path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a UTF-8 name")
                .to_owned();
            let file = PresetFile::parse(&name).unwrap_or_else(|| panic!("{name}: a preset"));
            let bytes = std::fs::read(&path).expect("readable");
            let json = TacticsJsonFormat::import(&bytes).expect("tactics");
            (file, json.tactics)
        })
        .collect()
}
