//! The 96/97 tables `world_extract` reads (`openswos_oracle::tables`), against what upstream's
//! source states: the countries' table, the team files' first global numbers, the season
//! end's countries, the round robins.

#![cfg(not(oracle_unavailable))]

use std::collections::BTreeSet;

use openswos_oracle::tables::{
    CountryEntry, country, euro_championship_fixtures, fixture_schedules, season_countries,
    team_file_bases,
};

/// The team files of the game: one a number.
const TEAM_FILES: usize = 256;

/// The countries of the season end (src/competition_data.c `_careerCompetitionIdRemapTable`).
const SEASON_COUNTRIES: usize = 65;

/// The sizes upstream has a round robin for: 2 to 24 clubs.
const SCHEDULE_SIZES: usize = 23;

/// The European Championships' groups: 4 clubs, 6 pairs (`_euroChampFixtureData`).
const EURO_CHAMPIONSHIP_PAIR_BYTES: usize = 12;

#[test]
fn each_size_has_its_round_robin() {
    let schedules = fixture_schedules().expect("built");
    assert_eq!(schedules.len(), SCHEDULE_SIZES);
    for (teams, schedule) in (2u8..).zip(&schedules) {
        assert_eq!(schedule.teams, teams);
        // Every pair of clubs once, each a club against another.
        let pairs: BTreeSet<(u8, u8)> = schedule
            .pairs
            .iter()
            .map(|&(home, away)| (home.min(away), home.max(away)))
            .collect();
        let all = usize::from(teams) * usize::from(teams - 1) / 2;
        assert_eq!((schedule.pairs.len(), pairs.len()), (all, all), "{teams}");
        assert!(
            schedule
                .pairs
                .iter()
                .all(|&(home, away)| home != away && home.max(away) < teams)
        );
        // `FixtureRounds_Real` before a world installs rounds: the even sizes only.
        assert_eq!(schedule.rounds, teams % 2 == 0, "{teams}");
    }
    // `_fixtureSchedule4Teams`: { 0, 1, 2, 3, 0, 2, 1, 3, 3, 0, 1, 2 }.
    assert_eq!(
        schedules[2].pairs,
        [(0, 1), (2, 3), (0, 2), (1, 3), (3, 0), (1, 2)]
    );
}

#[test]
fn the_european_championships_have_their_own_order() {
    let bytes = euro_championship_fixtures().expect("built");
    assert_eq!(
        bytes.get(..EURO_CHAMPIONSHIP_PAIR_BYTES),
        Some(&[1, 0, 3, 2, 2, 1, 0, 3, 1, 3, 0, 2][..])
    );
}

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
