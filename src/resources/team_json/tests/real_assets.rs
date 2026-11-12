//! Your own copy of the PC game (`just test_real_assets <game_dir>`): every team file
//! survives this JSON, and BRAZIL's checksum is upstream's documented one. Skipped without
//! `OPENSWOS_GAME_DIR`.

use openswos_common::{Exporter, Importer, TeamFileNumber, TeamId, TeamOrdinal};
use openswos_team_json_resource::{
    EntryIds, TeamJsonDocument, TeamJsonFile, TeamJsonFormat, TrailingNewline, WORLD_1996_97_TAG,
    team_checksum,
};
use openswos_team_resource::TeamFileFormat;
use openswos_test_utils::data::{find_ignoring_case, game_dir, game_file};
use openswos_test_utils::fixtures::list_fixtures;
use openswos_test_utils::require;

/// `DATA/TEAM.248`: not a team database (see `openswos_team_resource`'s tests).
const NOT_A_TEAM_FILE: &str = "TEAM.248";

#[test]
fn every_team_file_survives_the_json() {
    let data = require!(game_dir().and_then(|dir| find_ignoring_case(&dir, "DATA")));
    let files = list_fixtures(&data, |name| {
        name.to_ascii_uppercase().starts_with("TEAM.")
            && !name.eq_ignore_ascii_case(NOT_A_TEAM_FILE)
    });
    assert!(!files.is_empty(), "no TEAM.* file in {}", data.display());
    for path in &files {
        let binary = TeamFileFormat::import(&std::fs::read(path).expect("readable"))
            .expect("a valid team file");
        let file = TeamJsonFile::from_team_file(&binary, None, |team| {
            EntryIds::derived(WORLD_1996_97_TAG, team)
        })
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let text = TeamJsonFormat::export(&TeamJsonDocument {
            file,
            trailing_newline: TrailingNewline::Present,
        })
        .expect("exports");
        let back = TeamJsonFormat::import(&text).expect("imports");
        assert_eq!(
            back.file.to_team_file().expect("converts"),
            binary,
            "{}",
            path.display()
        );
    }
}

/// docs/TEAM_IDS.md §2's example: BRAZIL (file 82, ordinal 2).
#[test]
fn brazils_checksum_is_upstreams() {
    let path = require!(game_file("DATA/TEAM.082"));
    let file = TeamFileFormat::import(&std::fs::read(&path).expect("readable"))
        .expect("a valid team file");
    let brazil_id = TeamId {
        file: TeamFileNumber(82),
        ordinal: TeamOrdinal(2),
    };
    let brazil = file
        .teams
        .iter()
        .find(|team| team.id == brazil_id)
        .expect("BRAZIL");
    assert_eq!(brazil.name.text(), "BRAZIL");
    assert_eq!(team_checksum(brazil).as_str(), "f2f9140ad5127ab1");
}
