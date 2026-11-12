//! The committed tactics files (`assets/TAC/`, from SWOS 2020): they round-trip, and decode
//! as their sidecar or the assertions below say.

use openswos_common::Tactics;
use openswos_tac_resource::{OUTFIELD_PLAYERS, TacFile, TacFileFormat};
use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::{assert_round_trip, assert_sidecar, fixtures};

/// The extension of tactics files.
const TAC_EXTENSION: &str = ".TAC";

/// The bytes SWOS 2020's CPU tactics end with.
const SWOS_2020_TRAILER: [u8; 6] = [0; 6];

fn read(name: &str) -> TacFile {
    let path = assets_path().join("TAC").join(name);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    assert_round_trip::<TacFileFormat>(&bytes)
}

#[test]
fn every_fixture_round_trips() {
    let files = fixtures("TAC", |name| {
        name.to_ascii_uppercase().ends_with(TAC_EXTENSION)
    });
    assert_eq!(files.len(), 3, "{files:?}");
    for path in files {
        let bytes = std::fs::read(&path).expect("readable fixture");
        assert_round_trip::<TacFileFormat>(&bytes);
    }
}

#[test]
fn the_dos_4_4_2_matches_its_sidecar() {
    let name = "01_442DOS.TAC";
    let tac = read(name);
    assert_eq!(tac.tactics.name.text(), "4-4-2");
    assert_eq!(tac.tactics.ball_out_of_play, Tactics::FourFourTwo);
    assert!(tac.trailer.is_empty());
    assert_sidecar(&assets_path().join("TAC").join(name), &tac);
}

#[test]
fn a_name_keeps_the_bytes_after_its_end() {
    let tac = read("06_433.TAC");
    assert_eq!(tac.tactics.name.text(), "4-3-3");
    assert!(!tac.tactics.name.is_canonical());
    assert_eq!(tac.tactics.ball_out_of_play, Tactics::FourThreeThree);
}

#[test]
fn swos_2020_cpu_tactics_keep_their_trailer() {
    let tac = read("04_5-3-2.tac");
    assert_eq!(tac.tactics.name.text(), "532");
    assert_eq!(tac.tactics.ball_out_of_play, Tactics::FiveThreeTwo);
    assert_eq!(tac.trailer, SWOS_2020_TRAILER.to_vec());
}

#[test]
fn every_outfield_player_has_a_pair_or_none() {
    for name in ["01_442DOS.TAC", "06_433.TAC", "04_5-3-2.tac"] {
        let pairs = read(name).tactics.pairs;
        assert_eq!(pairs.len(), OUTFIELD_PLAYERS);
        // A pair links two players or more: no id stands alone.
        for pair in pairs.iter().flatten() {
            let members = pairs.iter().filter(|other| **other == Some(*pair)).count();
            assert!(members >= 2, "{name}: pair {} has one player", pair.value());
        }
    }
}
