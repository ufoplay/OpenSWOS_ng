//! Real highlights (`OPENSWOS_SAVES_DIR`, `just test_saves`): every `.HIL` in the folder
//! reads and writes back byte for byte, OpenSWOS's JSON ones and the DOS game's. Skipped
//! without the folder; highlights are never committed (they hold the game's data).

use openswos_hil_resource::{DosHilFormat, JsonHilFormat};
use openswos_test_utils::data::saves_dir;
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;

/// The extension of highlights.
const HIL_EXTENSION: &str = ".HIL";

/// What a JSON file starts with, after any whitespace.
const JSON_OPEN: u8 = b'{';

#[test]
fn every_highlights_file_round_trips() {
    let dir = require!(saves_dir());
    let files = list_fixtures(&dir, |name| {
        name.to_ascii_uppercase().ends_with(HIL_EXTENSION)
    });
    for path in files {
        let bytes = std::fs::read(&path).expect("a readable highlights file");
        let is_json = bytes
            .iter()
            .find(|byte| !byte.is_ascii_whitespace())
            .is_some_and(|&byte| byte == JSON_OPEN);
        if is_json {
            let file = assert_round_trip::<JsonHilFormat>(&bytes);
            assert_eq!(
                usize::from(file.num_goals),
                file.scenes.len(),
                "{}",
                path.display()
            );
        } else {
            assert_round_trip::<DosHilFormat>(&bytes);
        }
    }
}
