//! `competitions/stadiums.json`: a world's stadium names (upstream src/world_stadium.c
//! `WorldStadium_Name`; written by `scripts/world_import.py` `stadiums` from the SWOS 96/97
//! Mod's stadium lists).
//!
//! ```json
//! { "clubs": [ [0, 0, "STADIUMI LORO BORICI"], … ],
//!   "venues": [ { "competition": "UEFA CHAMPIONS LEAGUE", "round": "FINALE", "name": "METROPOLITANO", "season": 1 } ] }
//! ```
//!
//! A match's ground is a neutral final's venue (by the competition's and the round's names as
//! the game spells them, and the career's season), else its home club's (by team file and
//! ordinal). Values are kept as written.

use serde::{Deserialize, Serialize};

use crate::json_file::JsonFile;

/// The longest stadium name the game takes, in letters (upstream `WORLD_STADIUM_NAME_MAX`: 48
/// small-font letters fit the screen); a longer one is not read.
pub const STADIUM_NAME_MAX: usize = 48;

/// A venue's season meaning every season (upstream: `ses == 0`).
pub const EVERY_SEASON: i32 = 0;

/// `stadiums.json` with its layout.
pub type StadiumsDocument = JsonFile<Stadiums>;

/// A world's stadium names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stadiums {
    /// The clubs' grounds, sorted by team file and ordinal.
    pub clubs: Vec<ClubStadium>,
    /// The neutral finals' venues.
    pub venues: Vec<Venue>,
}

/// A club's ground. Written `[file, ordinal, name]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(u8, u8, String)", into = "(u8, u8, String)")]
pub struct ClubStadium {
    /// The club's team file.
    pub file: u8,
    /// Its place in that file.
    pub ordinal: u8,
    /// The stadium's name, in the game's letters.
    pub name: String,
}

impl From<(u8, u8, String)> for ClubStadium {
    fn from((file, ordinal, name): (u8, u8, String)) -> Self {
        Self {
            file,
            ordinal,
            name,
        }
    }
}

impl From<ClubStadium> for (u8, u8, String) {
    fn from(
        ClubStadium {
            file,
            ordinal,
            name,
        }: ClubStadium,
    ) -> Self {
        (file, ordinal, name)
    }
}

/// A neutral final's venue.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Venue {
    /// The competition, as the game names it (English).
    pub competition: String,
    /// The round, as the game names it.
    pub round: String,
    /// The venue's name, in the game's letters.
    pub name: String,
    /// The career's season it is for (1, 2, …); [`EVERY_SEASON`] for every one.
    pub season: i32,
}

impl Venue {
    /// Whether the venue is the ground of `season`'s final.
    #[must_use]
    pub fn is_for(&self, season: i32) -> bool {
        self.season == EVERY_SEASON || self.season == season
    }
}

impl Stadiums {
    /// The ground of the club of team file `file` and ordinal `ordinal`, if named, and if the
    /// game takes the name (at most [`STADIUM_NAME_MAX`] letters).
    #[must_use]
    pub fn club(&self, file: u8, ordinal: u8) -> Option<&str> {
        self.clubs
            .iter()
            .find(|club| club.file == file && club.ordinal == ordinal)
            .map(|club| club.name.as_str())
            .filter(|name| name.chars().count() <= STADIUM_NAME_MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// World 2026-27's first club and first venue, as its package writes them.
    const TEXT: &str = "{\n  \"clubs\": [\n    [\n      0,\n      0,\n      \"STADIUMI LORO BORICI\"\n    ]\n  ],\n  \"venues\": [\n    {\n      \"competition\": \"UEFA CHAMPIONS LEAGUE\",\n      \"round\": \"FINALE\",\n      \"name\": \"METROPOLITANO\",\n      \"season\": 1\n    }\n  ]\n}\n";

    #[test]
    fn reads_and_writes_the_packages_file() {
        let document = StadiumsDocument::read(TEXT.as_bytes()).expect("a file");
        let stadiums = &document.content;
        assert_eq!(stadiums.club(0, 0), Some("STADIUMI LORO BORICI"));
        assert_eq!(stadiums.club(0, 1), None);
        let [venue] = stadiums.venues.as_slice() else {
            panic!("one venue");
        };
        assert!(venue.is_for(1));
        assert!(!venue.is_for(2));
        assert_eq!(document.to_bytes().expect("writes"), TEXT.as_bytes());
    }

    #[test]
    fn a_venue_of_season_0_is_every_seasons_and_long_names_are_not_taken() {
        let venue = Venue {
            competition: "C".to_owned(),
            round: String::new(),
            name: "V".to_owned(),
            season: EVERY_SEASON,
        };
        assert!(venue.is_for(1) && venue.is_for(40));
        let stadiums = Stadiums {
            clubs: vec![
                ClubStadium::from((1, 2, "X".repeat(STADIUM_NAME_MAX))),
                ClubStadium::from((1, 3, "X".repeat(STADIUM_NAME_MAX + 1))),
            ],
            venues: Vec::new(),
        };
        assert!(stadiums.club(1, 2).is_some());
        assert_eq!(stadiums.club(1, 3), None);
    }

    #[test]
    fn a_club_is_three_values() {
        assert!(serde_json::from_str::<ClubStadium>(r#"[1, 2]"#).is_err());
        assert!(
            serde_json::from_str::<Stadiums>(r#"{"clubs": [], "venues": [], "x": 0}"#).is_err()
        );
    }
}
