//! The committed original team files (`assets/TEAM/`): they round-trip, and decode as
//! their sidecars or the assertions below say.

use openswos_common::{Nibble, Position, TeamFileNumber, TeamId, TeamOrdinal};
use openswos_team_resource::{
    Division, Face, Form, MatchFlags, Nationality, PLAYERS_PER_TEAM, PlayerValues, ShirtType,
    TeamControls, TeamFileFormat, TeamRuntime,
};
use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::{assert_round_trip, assert_sidecar, fixtures};

fn read(name: &str) -> Vec<u8> {
    let path = assets_path().join("TEAM").join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

#[test]
fn every_fixture_round_trips() {
    let files = fixtures("TEAM", |name| {
        name.starts_with("TEAM.") && !name.ends_with(".json")
    });
    assert_eq!(files.len(), 3, "{files:?}");
    for path in files {
        let bytes = std::fs::read(&path).expect("readable fixture");
        assert_round_trip::<TeamFileFormat>(&bytes);
    }
}

#[test]
fn small_files_match_their_sidecars() {
    for name in ["TEAM.057", "TEAM.085"] {
        let model = assert_round_trip::<TeamFileFormat>(&read(name));
        assert_sidecar(&assets_path().join("TEAM").join(name), &model);
    }
}

#[test]
fn england_decodes() {
    let england = assert_round_trip::<TeamFileFormat>(&read("TEAM.008"));
    assert_eq!(england.teams.len(), 93);

    let arsenal = england.teams.first().expect("a first team");
    assert_eq!(arsenal.name.text(), "ARSENAL");
    assert_eq!(
        arsenal.id,
        TeamId {
            file: TeamFileNumber(8),
            ordinal: TeamOrdinal(0)
        }
    );
    assert_eq!(arsenal.division, Division(0));

    for (index, team) in england.teams.iter().enumerate() {
        assert_eq!(usize::from(team.id.ordinal.0), index, "{}", team.name);
        assert_eq!(team.id.file, TeamFileNumber(8), "{}", team.name);
        let mut order = team.player_order;
        order.sort_unstable();
        let expected: Vec<u8> = (0..).take(PLAYERS_PER_TEAM).collect();
        assert_eq!(
            order.to_vec(),
            expected,
            "{}: the order is a permutation",
            team.name
        );
        // Clubs keep their real squad order, so the shirts are 1..=16 in any order.
        let mut shirts: Vec<u8> = team
            .players
            .iter()
            .map(|player| player.shirt_number)
            .collect();
        shirts.sort_unstable();
        assert_eq!(shirts, (1..=16).collect::<Vec<u8>>(), "{}", team.name);
        assert!(
            team.players
                .iter()
                .any(|player| player.position == Position::Goalkeeper),
            "{}",
            team.name
        );
    }
}

#[test]
fn runtime_fields_are_zero_in_the_original_files() {
    for name in ["TEAM.008", "TEAM.057", "TEAM.085"] {
        for team in assert_round_trip::<TeamFileFormat>(&read(name)).teams {
            assert_eq!(
                team.runtime,
                TeamRuntime {
                    controls: TeamControls::NotSelected,
                    own_goals: 0,
                    match_flags: MatchFlags::empty(),
                    player_values: PlayerValues::Clean,
                },
                "{}",
                team.name
            );
            for player in team.players {
                assert!(player.runtime.flags.is_empty(), "{}", player.name);
                assert_eq!(player.runtime.form, Form::from(0), "{}", player.name);
                assert_eq!(player.skills.passing_high, Nibble::ZERO, "{}", player.name);
            }
        }
    }
}

#[test]
fn south_korea_is_one_national_team() {
    let korea = assert_round_trip::<TeamFileFormat>(&read("TEAM.057"));
    let [team] = korea.teams.as_slice() else {
        panic!("one team expected, got {}", korea.teams.len());
    };
    assert_eq!(team.name.text(), "SOUTH KOREA");
    // Nationalities have a numbering of their own: Korea's team file is 57, its players'
    // nationality 127.
    assert!(
        team.players
            .iter()
            .all(|player| player.nationality == Nationality(127))
    );
    assert!(ShirtType::ALL.contains(&team.primary_kit.shirt_type));
    assert!(
        team.players
            .iter()
            .all(|player| Face::ALL.contains(&player.face))
    );
}
