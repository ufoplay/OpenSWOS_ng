//! The committed `assets/POOLPLYR/POOLPLYR.DAT` (the PC game's): it round-trips and
//! decodes as expected.

use openswos_common::{TeamFileNumber, TeamId, TeamOrdinal};
use openswos_poolplyr_resource::PoolFileFormat;
use openswos_team_resource::PLAYERS_PER_TEAM;
use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::assert_round_trip;

/// Players in the PC game's pool.
const POOL_SIZE: usize = 900;

#[test]
fn the_pool_round_trips_and_decodes() {
    let path = assets_path().join("POOLPLYR").join("POOLPLYR.DAT");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let pool = assert_round_trip::<PoolFileFormat>(&bytes);
    assert_eq!(pool.players.len(), POOL_SIZE);

    let first = pool.players.first().expect("a first player");
    assert_eq!(first.player.name.text(), "ERIC DEFLANDRE");
    assert_eq!(first.team_name.text(), "CLUB BRUGES");
    assert_eq!(
        first.team,
        TeamId {
            file: TeamFileNumber(2),
            ordinal: TeamOrdinal(4)
        }
    );
    assert_eq!(first.player_ordinal, 2);

    let last = pool.players.last().expect("a last player");
    assert_eq!(last.player.name.text(), "DARKO MAVRAK");
    assert_eq!(last.team_name.text(), "IFK NORRKOPING");
    assert_eq!(
        last.team,
        TeamId {
            file: TeamFileNumber(36),
            ordinal: TeamOrdinal(9)
        }
    );

    for player in &pool.players {
        assert!(
            !player.team_name.text().is_empty(),
            "{}: no club name",
            player.player.name
        );
        assert!(
            !player.player.name.text().is_empty(),
            "a player of {} has no name",
            player.team_name
        );
        assert!(
            usize::from(player.player_ordinal) < PLAYERS_PER_TEAM,
            "{}: ordinal beyond the squad",
            player.player.name
        );
    }
}
