//! Real career saves (`OPENSWOS_SAVES_DIR`, `just test_saves`): every `.CAR` in the folder
//! reads as a typed career and writes back byte for byte. Skipped without the folder.
//! Saves are never committed: they hold the game's data.

use openswos_career_save_resource::CareerSaveFormat;
use openswos_test_utils::data::saves_dir;
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;

/// The extension of career saves.
const CAREER_EXTENSION: &str = ".CAR";

/// The seasons of the manager's record in a save (upstream `SAVE_SI_ENTRY_COUNT`).
const SEASONS: usize = 35;

/// The players of a reserves list.
const RESERVES: usize = 16;

/// The foreign market's pool in a save, at least (upstream `CAREER_ORIG_FOREIGN_POOL`).
const MARKET_POOL: usize = 432;

/// The transfer offers of a save.
const TRANSFER_OFFERS: usize = 5;

/// The job offers of each queue, at least (upstream `CAREER_ORIG_JOB_OFFERS`).
const JOB_OFFERS: usize = 5;

/// A league table's rows in the overrides (upstream `DIY_OVR_LEAGUE_ROWS`).
const LEAGUE_ROWS: usize = 64;

/// The competition type of a league.
const LEAGUE: i16 = 0;

#[test]
fn every_career_save_round_trips() {
    let dir = require!(saves_dir());
    let saves = list_fixtures(&dir, |name| {
        name.to_ascii_uppercase().ends_with(CAREER_EXTENSION)
    });
    let mut read = 0;
    for path in saves {
        let bytes = std::fs::read(&path).expect("a readable save");
        if bytes.first() != Some(&b'{') {
            continue;
        }
        let save = assert_round_trip::<CareerSaveFormat>(&bytes);
        assert_eq!(
            save.season_informations.len(),
            SEASONS,
            "{}",
            path.display()
        );
        assert_eq!(save.career_team_reserves.len(), RESERVES);
        assert_eq!(save.current_team_reserves.len(), RESERVES);
        assert!(save.career_foreign_market_players.len() >= MARKET_POOL);
        assert_eq!(save.transfer_offer_table.len(), TRANSFER_OFFERS);
        assert!(save.new_job_offers.len() >= JOB_OFFERS);
        assert!(save.job_offers.len() >= JOB_OFFERS);
        let overrides = &save.competition_overrides;
        assert_eq!(
            overrides.league_rows.as_ref().map(Vec::len),
            (overrides.competition_type == LEAGUE).then_some(LEAGUE_ROWS)
        );
        read += 1;
    }
    assert!(read > 0, "no JSON career save in {}", dir.display());
}
