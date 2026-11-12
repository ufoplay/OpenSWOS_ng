//! Real competition saves (`OPENSWOS_SAVES_DIR`, `just test_saves`): every JSON `.SEA`,
//! `.PRE` and `.DIY` in the folder reads as a typed competition, and every DOS `.DIY` as
//! the DOS game's file, and each writes back byte for byte. Skipped without the folder.
//! Saves are never committed: they hold the game's data.

use openswos_competition_save_resource::{CompetitionSaveFormat, DosDiyFormat};
use openswos_test_utils::data::saves_dir;
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;

/// The extensions of competition saves.
const EXTENSIONS: [&str; 3] = [".SEA", ".PRE", ".DIY"];

#[test]
fn every_json_competition_save_round_trips() {
    let dir = require!(saves_dir());
    let saves = list_fixtures(&dir, |name| {
        let upper = name.to_ascii_uppercase();
        EXTENSIONS
            .iter()
            .any(|extension| upper.ends_with(extension))
    });
    for path in saves {
        let bytes = std::fs::read(&path).expect("a readable save");
        if bytes.first() == Some(&b'{') {
            let save = assert_round_trip::<CompetitionSaveFormat>(&bytes);
            assert_eq!(usize::from(save.num_diy_teams), save.diy_teams.len());
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("DIY"))
        {
            assert_round_trip::<DosDiyFormat>(&bytes);
        }
    }
}
