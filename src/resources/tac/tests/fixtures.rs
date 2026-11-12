//! The committed tactics files (`assets/TAC/`, from SWOS 2020): they round-trip, and decode
//! as their sidecar or the assertions below say. Printed as OpenSWOS's JSON tactics, they
//! read back as their records.

use openswos_common::Tactics;
use openswos_tac_resource::{JsonTacFormat, OUTFIELD_PLAYERS, TacFile, TacFileFormat, TeamTactics};
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

/// Bytes of the name field, and where the positions start.
const NAME_LEN: usize = 9;

/// Bytes of a player's positions.
const POSITIONS_LEN: usize = 35;

/// The record `bytes` as upstream's `write_tactics_buf` prints it: the name up to its NUL
/// (`"` and `\` escaped, other bytes as they are), the positions, the pairs, the slot.
fn json_text(bytes: &[u8]) -> Vec<u8> {
    let (name, rest) = bytes.split_at(NAME_LEN);
    let (positions, rest) = rest.split_at(OUTFIELD_PLAYERS * POSITIONS_LEN);
    let (pairs, rest) = rest.split_at(OUTFIELD_PLAYERS);
    let ball_out_of_play = rest.first().expect("the slot");
    let numbers = |bytes: &[u8]| {
        let numbers: Vec<String> = bytes.iter().map(u8::to_string).collect();
        format!("[{}]", numbers.join(","))
    };
    let positions: Vec<String> = positions.chunks(POSITIONS_LEN).map(numbers).collect();
    let mut text =
        b"{\n  \"schema_version\":3,\n  \"file_kind\":\"tactics\",\n  \"name\":\"".to_vec();
    for &byte in name.iter().take_while(|&&byte| byte != 0) {
        if byte == b'"' || byte == b'\\' {
            text.push(b'\\');
        }
        text.push(byte);
    }
    text.extend_from_slice(
        format!(
            "\",\n  \"player_positions\":[{}],\n  \"pair_ids\":{},\n  \"ball_out_of_play_tactics\":{ball_out_of_play}\n}}\n",
            positions.join(","),
            numbers(pairs),
        )
        .as_bytes(),
    );
    text
}

#[test]
fn every_fixture_written_as_openswos_json_reads_as_its_record() {
    for name in ["01_442DOS.TAC", "06_433.TAC", "04_5-3-2.tac"] {
        let path = assets_path().join("TAC").join(name);
        let bytes = std::fs::read(&path).expect("readable fixture");
        let tac = read(name);
        let text = json_text(&bytes);
        let tactics = assert_round_trip::<JsonTacFormat>(&text);
        // The writer writes the name up to its NUL: the rest of the record is the same.
        assert_eq!(tactics.name.text(), tac.tactics.name.text(), "{name}");
        assert!(tactics.name.is_canonical());
        let TeamTactics {
            name: _,
            players,
            pairs,
            ball_out_of_play,
        } = tac.tactics;
        assert_eq!(
            (tactics.players, tactics.pairs, tactics.ball_out_of_play),
            (players, pairs, ball_out_of_play),
            "{name}"
        );
    }
}
