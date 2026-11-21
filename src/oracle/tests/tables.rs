//! The 96/97 tables `world_extract` reads (`openswos_oracle::tables`), against what upstream's
//! source states: the countries' table, the team files' first global numbers, the season
//! end's countries.

#![cfg(not(oracle_unavailable))]

use openswos_oracle::tables::{CountryEntry, country, season_countries, team_file_bases};

/// The team files of the game: one a number.
const TEAM_FILES: usize = 256;

/// The countries of the season end (src/competition_data.c `_careerCompetitionIdRemapTable`).
const SEASON_COUNTRIES: usize = 65;

#[test]
fn a_country_has_its_continent_name_and_adjective() {
    let albania = country(0).expect("built").expect("Albania");
    assert_eq!(
        albania,
        CountryEntry {
            continent: 0,
            name: b"ALBANIA".to_vec(),
            adjective: b"ALBANIAN".to_vec()
        }
    );
    // `"\005ALGERIA\000ALGERIAN"`: Africa.
    let algeria = country(42).expect("built").expect("Algeria");
    assert_eq!(
        (algeria.continent, algeria.name.as_slice()),
        (5, &b"ALGERIA"[..])
    );
    // `"\002ARGENTINA\000ARGENTINIAN"`: South America.
    assert_eq!(country(43).expect("built").expect("Argentina").continent, 2);
}

#[test]
fn a_number_without_a_country_has_no_entry() {
    assert_eq!(country(47).expect("built"), None);
    assert_eq!(country(200).expect("built"), None);
}

#[test]
fn every_team_file_has_its_first_global_number() {
    let bases = team_file_bases().expect("built");
    assert_eq!(bases.len(), TEAM_FILES);
    assert_eq!(&bases[..3], &[0, 16, 26]);
    assert!(
        bases.windows(2).all(|pair| pair[0] <= pair[1]),
        "never down"
    );
}

#[test]
fn the_season_end_takes_65_countries_in_its_order() {
    let countries = season_countries().expect("built");
    assert_eq!(countries.len(), SEASON_COUNTRIES);
    assert_eq!(&countries[..4], &[0, 1, 76, 2]);
    assert_eq!(countries.last(), Some(&62));
}
