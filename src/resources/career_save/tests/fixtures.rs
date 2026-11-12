//! The committed team files' teams (`assets/TEAM/`) as a career save writes them: each
//! reads back as the team it was. Faithful to the original: a name's bytes after its end
//! are not written, so only teams whose names have none come back identical.

use openswos_career_save_resource::SaveTeam;
use openswos_common::Importer;
use openswos_save_json_resource::{Compact, from_save_bytes, to_save_bytes};
use openswos_team_resource::{Team, TeamFileFormat};
use openswos_test_utils::assets_path;

/// The committed team files.
const TEAM_FILES: [&str; 3] = ["TEAM.008", "TEAM.057", "TEAM.085"];

fn names_are_canonical(team: &Team) -> bool {
    team.name.is_canonical()
        && team.coach.is_canonical()
        && team.players.iter().all(|player| player.name.is_canonical())
}

#[test]
fn every_team_reads_back_as_written() {
    let mut teams = 0;
    for file in TEAM_FILES {
        let bytes = std::fs::read(assets_path().join("TEAM").join(file)).expect("a fixture");
        let team_file = TeamFileFormat::import(&bytes).expect("a team file");
        for team in team_file.teams {
            assert!(names_are_canonical(&team), "{file}: {}", team.name.text());
            let text = to_save_bytes(&SaveTeam(team.clone()), &Compact).expect("writes");
            let read: SaveTeam = from_save_bytes(&text).expect("reads back");
            assert_eq!(read.0, team, "{file}: {}", team.name.text());
            teams += 1;
        }
    }
    assert!(teams > 0);
}
