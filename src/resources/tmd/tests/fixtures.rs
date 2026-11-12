//! The committed `assets/TMD/EUROCUP.TMD` (the PC game's): it round-trips and holds the
//! 1996-97 Champions' Cup teams.

use openswos_common::{TeamFileNumber, TeamId, TeamOrdinal};
use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::assert_round_trip;
use openswos_tmd_resource::TmdFileFormat;

/// The pool's teams: name, team file, ordinal in it.
const CHAMPIONS_CUP: [(&str, u8, u8); 16] = [
    ("AUXERRE", 13, 1),
    ("AJAX", 25, 0),
    ("GRASSHOPPERS", 37, 5),
    ("RANGERS", 33, 33),
    ("ATLETICO MADRID", 35, 6),
    ("STEAUA BUCHAREST", 30, 18),
    ("BOR. DORTMUND", 14, 4),
    ("WIDZEW LODZ", 28, 23),
    ("RAPID VIENNA", 1, 5),
    ("FENERBAHCE", 38, 9),
    ("JUVENTUS", 20, 22),
    ("AC MILAN", 20, 0),
    ("MANCHESTER UTD", 8, 20),
    ("IFK GOTHENBURG", 36, 8),
    ("ROSENBORG", 27, 11),
    ("FC PORTO", 29, 12),
];

#[test]
fn the_champions_cup_pool_round_trips_and_decodes() {
    let path = assets_path().join("TMD").join("EUROCUP.TMD");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let pool = assert_round_trip::<TmdFileFormat>(&bytes);
    let teams: Vec<(String, TeamId)> = pool
        .teams
        .iter()
        .map(|team| (team.name.text(), team.id))
        .collect();
    let expected: Vec<(String, TeamId)> = CHAMPIONS_CUP
        .iter()
        .map(|&(name, file, ordinal)| {
            (
                name.to_owned(),
                TeamId {
                    file: TeamFileNumber(file),
                    ordinal: TeamOrdinal(ordinal),
                },
            )
        })
        .collect();
    assert_eq!(teams, expected);
}
