//! Real tactics (`OPENSWOS_SAVES_DIR`, `just test_saves`): every `.TAC` in the folder reads
//! and writes back byte for byte, OpenSWOS's JSON ones and the binary ones. Skipped
//! without the folder; saves are never committed.

use openswos_tac_resource::{JsonTacFormat, TacFileFormat};
use openswos_test_utils::data::saves_dir;
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;

/// The extension of tactics files.
const TAC_EXTENSION: &str = ".TAC";

/// What a JSON file starts with, after any whitespace.
const JSON_OPEN: u8 = b'{';

#[test]
fn every_tactics_file_round_trips() {
    let dir = require!(saves_dir());
    let files = list_fixtures(&dir, |name| {
        name.to_ascii_uppercase().ends_with(TAC_EXTENSION)
    });
    for path in files {
        let bytes = std::fs::read(&path).expect("a readable tactics file");
        let is_json = bytes
            .iter()
            .find(|byte| !byte.is_ascii_whitespace())
            .is_some_and(|&byte| byte == JSON_OPEN);
        if is_json {
            assert_round_trip::<JsonTacFormat>(&bytes);
        } else {
            assert_round_trip::<TacFileFormat>(&bytes);
        }
    }
}
