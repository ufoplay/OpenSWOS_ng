//! Real saves (`OPENSWOS_SAVES_DIR`, `just test_saves`): every JSON save in the folder
//! round-trips byte for byte. Skipped without the folder. Saves are never committed: they
//! hold the game's data.

use openswos_common::Exporter;
use openswos_save_json_resource::{SCHEMA_VERSION, SaveJsonFormat};
use openswos_test_utils::data::saves_dir;
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;

/// The extensions of upstream's JSON saves.
const SAVE_EXTENSIONS: [&str; 6] = [".CAR", ".SEA", ".PRE", ".DIY", ".HIL", ".TAC"];

#[test]
fn every_json_save_round_trips() {
    let dir = require!(saves_dir());
    let saves = list_fixtures(&dir, |name| {
        let upper = name.to_ascii_uppercase();
        SAVE_EXTENSIONS
            .iter()
            .any(|extension| upper.ends_with(extension))
    });
    let mut read = 0;
    for path in saves {
        let bytes = std::fs::read(&path).expect("a readable save");
        // `.HIL` and `.TAC` may still be binary: only JSON ones are this crate's.
        if bytes.first() != Some(&b'{') {
            continue;
        }
        let document = assert_round_trip::<SaveJsonFormat>(&bytes);
        assert!(
            SaveJsonFormat::export(&document).is_ok(),
            "{}",
            path.display()
        );
        read += 1;
    }
    assert!(
        read > 0,
        "no JSON save of schema {SCHEMA_VERSION} in {}",
        dir.display()
    );
}
