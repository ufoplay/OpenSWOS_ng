//! SWOS 2020's replays (`assets/HIL/`): a one-goal highlights file and the first 100
//! frames of a whole-match replay. They round-trip byte for byte and decode as they are.

use openswos_common::Importer;
use openswos_hil_resource::{DosHilFormat, DosRplFormat, ReplayFrame};
use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::assert_round_trip;

/// SWOS 2020's highlight slots: a widescreen scene takes 61000 bytes.
const SWOS_2020_SLOT: usize = 61_000;

/// The frames kept of the replay.
const RPL_FRAMES: usize = 100;

/// The sprites SWOS 2020 records in a frame.
const SPRITES_A_FRAME: usize = 27;

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(assets_path().join("HIL").join(name)).expect("a fixture")
}

fn goals(frame: &ReplayFrame) -> (u16, u16) {
    (frame.team1_goals, frame.team2_goals)
}

#[test]
fn a_highlight_of_one_goal() {
    let bytes = bytes("26-1111-1654-1-0.HIL");
    let file = assert_round_trip::<DosHilFormat>(&bytes);
    let header = &file.header;
    assert_eq!(header.num_scenes, 1);
    assert_eq!((header.team1_goals, header.team2_goals), (1, 0));
    assert_eq!(file.scenes.len(), 1);
    let scene = file.scenes.first().expect("a scene");
    let encoded = scene.encode().expect("encodes");
    assert_eq!(encoded.len(), SWOS_2020_SLOT);
    assert!(
        scene
            .frames
            .iter()
            .all(|frame| frame.sprites.len() == SPRITES_A_FRAME)
    );
    let first = scene.frames.first().expect("a frame");
    let last = scene.frames.last().expect("a frame");
    assert_eq!(goals(first), (0, 0), "before the goal");
    assert_eq!(goals(last), (1, 0), "after it");
    assert!(!header.team1.game.name.text().is_empty());
}

#[test]
fn the_start_of_a_whole_match_replay() {
    let bytes = bytes("LATEST-100.RPL");
    let file = assert_round_trip::<DosRplFormat>(&bytes);
    assert_eq!(file.header.team1.game.name.text(), "LAZIO");
    assert_eq!(file.header.team2.game.name.text(), "ADVANCED");
    assert_eq!(file.header.game_name.text(), "TEAM TRAINING");
    assert_eq!(file.scene.frames.len(), RPL_FRAMES);
    assert!(file.scene.leftover.is_empty());
    let first = file.scene.frames.first().expect("a frame");
    assert_eq!(goals(first), (0, 0));
}

#[test]
fn a_highlight_is_not_a_replay_and_back() {
    // The two share their header: a .HIL's slots read as one long scene only up to the
    // first goal's end, and the leftover then holds the rest.
    let bytes = bytes("26-1111-1654-1-0.HIL");
    let as_replay = assert_round_trip::<DosRplFormat>(&bytes);
    assert!(!as_replay.scene.leftover.is_empty());
    assert!(DosHilFormat::import(&bytes).is_ok());
}
