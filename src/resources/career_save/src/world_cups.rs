//! A world's cups: its national and league cups (upstream `WorldCupsState`,
//! include/world_cups.h), Italy's Coppa Italia Serie D (`WorldCupDState`,
//! include/world_cupd.h), and the other confederations' cups (`ct_state`, `ct_ed`,
//! include/world_conti.h). Saved only by a career in a world with them.
//!
//! What the numbers mean is the world's competitions' (M5); they are kept as written, in
//! their fields' C widths. Lists keep the length written.

use serde::{Deserialize, Serialize};

/// The career's seasons in the world, and its cups' state (upstream `WorldCupsState`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCups {
    /// The seasons ended (255: and more).
    pub seasons: u8,
    /// The league cup's winner: `{country, ordinal}`; 0 not decided yet.
    pub league_cup_winner: u16,
    /// The season its winner is of.
    pub league_cup_season: u8,
    /// The season the places are of.
    pub places_season: u8,
    /// The places' country; 0 none kept.
    pub places_country: u8,
    /// Per slot of the country's team files, its final place (1..31) | its division's
    /// level << 5. The writer leaves out the zeros past the first 256.
    pub places: Vec<u8>,
}

/// The Coppa Italia Serie D's state this season (upstream `WorldCupDState`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCupD {
    /// The season (the seasons ended + 1; 0 not started).
    pub season: u8,
    /// The round to come.
    pub round: u8,
    /// The round (+ 1) whose pairs are drawn.
    pub drawn: u8,
    /// The drawn round's ties.
    pub drawn_ties: [u8; 4],
    /// Each tie's winner: 1 a, 2 b, 0 not decided (168).
    pub results: Vec<u8>,
    /// Each tie's host: 0 a, 1 b (168).
    pub hosts: Vec<u8>,
    /// Whether the career club's tie is in slot 4.
    pub built: u8,
    /// Which tie it is.
    pub tie: u8,
    /// Its first leg's week.
    pub week1: u8,
    /// Its second leg's week.
    pub week2: u8,
    /// Its legs.
    pub legs: u8,
    /// Its home club: `{country, ordinal}`.
    pub team_a: u16,
    /// Its away club.
    pub team_b: u16,
    /// Whether this season's first rounds were generated (not the table's).
    pub generated: u8,
    /// The generated preliminary round's ties.
    pub preliminary_ties: u8,
    /// Each generated tie's two sides (210).
    pub sides: Vec<u16>,
    /// The round from which the rest is in slot 2 (+ 1; 0 not).
    pub slot2: u8,
}

/// The other confederations' cups' holders (upstream `ct_state`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContiHolders {
    /// `'CT'` (0x5443) while valid.
    pub mark: u16,
    /// The season.
    pub season: u16,
    /// Each cup's holder (16): team word, association, strength, ranking points.
    #[serde(with = "crate::flat")]
    pub holders: Vec<[u32; 4]>,
}

/// The confederation cups' edition being played (upstream `ct_ed`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContiEdition {
    /// `'CE'` (0x4543) while valid.
    pub mark: u16,
    /// The edition's random seed.
    pub seed: u32,
    /// The career club's team word; 0 none.
    pub manager_club: u16,
    /// The career club's results so far.
    pub results_count: u8,
    /// The cup whose main round is in slot 3 (upstream `g_contiMainCup`).
    pub main_cup: u8,
    /// The edition's flags (upstream `pad`: `CT_ED_G2`, the Mod 4.0.6's goals model;
    /// `CT_ED_S1`); absent in a save made before them (0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u8>,
    /// The entrants: team word, cup << 4 | round, strength / 16 | slot << 12.
    #[serde(with = "crate::flat")]
    pub entrants: Vec<[u16; 3]>,
    /// The career club's results (16): cup, round, value.
    #[serde(with = "crate::flat")]
    pub results: Vec<[u16; 3]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};

    #[test]
    fn the_world_cups_round_trip() {
        let cups: WorldCups = round_trip(
            r#"{"seasons":0,"league_cup_winner":5123,"league_cup_season":0,"places_season":0,"places_country":0,"places":[1,34,0]}"#,
        );
        assert_eq!(cups.league_cup_winner, 5123);
        assert_eq!(cups.places, vec![1, 34, 0]);
    }

    #[test]
    fn the_serie_d_cup_round_trips() {
        let cup: WorldCupD = round_trip(
            r#"{"season":1,"round":3,"drawn":0,"drawn_ties":[0,0,0,0],"results":[1,2],"hosts":[0,1],"built":1,"tie":4,"week1":9,"week2":11,"legs":2,"team_a":5311,"team_b":5312,"generated":1,"preliminary_ties":41,"sides":[7,8],"slot2":0}"#,
        );
        assert_eq!(cup.team_a, 5311);
        assert!(rejects::<WorldCupD>(
            r#"{"season":1,"round":3,"drawn":0,"drawn_ties":[0,0,0],"results":[],"hosts":[],"built":1,"tie":4,"week1":9,"week2":11,"legs":2,"team_a":1,"team_b":2,"generated":1,"preliminary_ties":41,"sides":[],"slot2":0}"#
        ));
    }

    #[test]
    fn the_confederation_cups_round_trip() {
        let holders: ContiHolders =
            round_trip(r#"{"mark":21571,"season":2,"holders":[300,4,4000000000,12]}"#);
        assert_eq!(holders.holders, vec![[300, 4, 4_000_000_000, 12]]);
        let edition: ContiEdition = round_trip(
            r#"{"mark":17731,"seed":4294967295,"manager_club":0,"results_count":0,"main_cup":3,"entrants":[1,2,3],"results":[0,0,0,1,1,1]}"#,
        );
        assert_eq!(edition.seed, u32::MAX);
        assert_eq!(edition.flags, None);
        let edition: ContiEdition = round_trip(
            r#"{"mark":17731,"seed":1,"manager_club":0,"results_count":0,"main_cup":0,"flags":1,"entrants":[],"results":[]}"#,
        );
        assert_eq!(edition.flags, Some(1));
        assert!(rejects::<ContiHolders>(
            r#"{"mark":21571,"season":2,"holders":[300]}"#
        ));
    }
}
