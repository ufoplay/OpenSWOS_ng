//! SWOS 2020's DIY competitions (`assets/DIY/`): they round-trip byte for byte and decode
//! as the files say.

use openswos_common::{Importer, Tactics};
use openswos_competition_save_resource::{DosCompetitionType, DosDiyFile, DosDiyFormat};
use openswos_team_resource::TeamControls;
use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::{assert_round_trip, fixtures};

/// The user's tactics' names in a new competition.
const USER_TACTICS_NAMES: [&str; 6] = ["USER_A", "USER_B", "USER_C", "USER_D", "USER_E", "USER_F"];

fn read(name: &str) -> DosDiyFile {
    let bytes = std::fs::read(assets_path().join("DIY").join(name)).expect("a fixture");
    assert_round_trip::<DosDiyFormat>(&bytes)
}

fn team_names(file: &DosDiyFile) -> Vec<String> {
    file.teams.iter().map(|team| team.name.text()).collect()
}

#[test]
fn every_fixture_round_trips() {
    let files = fixtures("DIY", |name| name.to_ascii_uppercase().ends_with(".DIY"));
    assert_eq!(files.len(), 2);
    for path in files {
        let bytes = std::fs::read(&path).expect("a fixture");
        let file = DosDiyFormat::import(&bytes).expect("a DOS .DIY");
        assert_eq!(
            usize::try_from(file.header.num_teams).ok(),
            Some(file.teams.len())
        );
        let names: Vec<String> = file
            .user_tactics
            .iter()
            .map(|tactics| tactics.name.text())
            .collect();
        assert_eq!(names, USER_TACTICS_NAMES);
        assert!(
            file.user_tactics
                .iter()
                .all(|tactics| tactics.ball_out_of_play == Tactics::FourFourTwo)
        );
    }
}

#[test]
fn the_champions_league_final_is_a_cup_of_two() {
    let file = read("2005CLFINAL.DIY");
    assert_eq!(file.header.competition_type, DosCompetitionType::Cup);
    assert_eq!(file.header.name.text(), "2005 CHAMPIONS LEAGUE");
    assert_eq!(file.header.name_ptr, 0);
    assert_eq!(team_names(&file), ["AC MILAN", "LIVERPOOL"]);
    assert_eq!(file.options.pitch_type, u16::MAX);
    // Faithful to the original: the global team numbers are little-endian in memory.
    let milan = file.teams.first().expect("AC Milan");
    assert_eq!(milan.global_number.0.swap_bytes(), 424);
}

#[test]
fn euro_1964_is_a_cup_of_four_national_teams() {
    let file = read("EC1964.diy");
    assert_eq!(file.header.competition_type, DosCompetitionType::Cup);
    assert_eq!(file.header.name.text(), "UEFA EURO 1964");
    assert_eq!(
        team_names(&file),
        ["DENMARK", "HUNGARY", "SPAIN", "SOVIET UNION"]
    );
    assert!(
        file.teams
            .iter()
            .all(|team| team.runtime.controls == TeamControls::Coach)
    );
    assert!(file.teams.iter().all(|team| team.id.file.0 == 72));
}
