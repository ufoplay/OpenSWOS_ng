//! A world's UEFA coefficients (upstream `UefaCoefState`, include/uefa_coef.h) and its
//! qualifying rounds (`uq_state`, include/uefa_access.h). Saved only by a career in a world
//! with them.
//!
//! What the numbers mean is the world's competitions' (M5); they are kept as written, in
//! their fields' C widths. Lists keep the length written.

use serde::{Deserialize, Serialize};

/// The associations in the coefficients' tables: one per country number.
pub const ASSOCIATIONS: usize = 256;

/// The clubs of a qualifying list.
pub const QUALIFYING_LIST_LEN: usize = 36;

/// The UEFA coefficients (upstream `UefaCoefState`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UefaCoef {
    /// The ring's current season (of 5).
    pub ring: u8,
    /// Whether this season's points are closed.
    pub season_closed: u8,
    /// Per association, its clubs this season (256).
    pub association_clubs: Vec<u8>,
    /// Per association, its points this season, in thousandths (256).
    pub association_points: Vec<u32>,
    /// Per ring season (5), per association, its points in thousandths.
    #[serde(with = "crate::flat")]
    pub association_seasons: Vec<[u32; ASSOCIATIONS]>,
    /// Per club: team number, points this season, then its ring's 5 seasons.
    #[serde(with = "crate::flat")]
    pub clubs: Vec<[u16; 7]>,
    /// Each cup's previous knockout round (3).
    pub previous_counts: Vec<u8>,
    /// Each cup's previous round's clubs (3 × 16).
    #[serde(with = "crate::flat")]
    pub previous: Vec<[u16; 16]>,
    /// Each cup's final: its two clubs (3).
    #[serde(with = "crate::flat")]
    pub finals: Vec<[u16; 2]>,
    /// Whether each cup's phase ended (3).
    pub phase_ended: Vec<u8>,
}

/// The qualifying rounds' state (upstream `uq_state`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UefaQualifying {
    /// `'UQ'` while valid.
    pub mark: u16,
    /// The career club's team word.
    pub manager_club: u16,
    /// The stand-in holding its league-phase place; 0 none.
    pub stand_in: u16,
    /// The stand-in's list: 0 Champions League, 1 Europa League, 2 Conference League.
    pub stand_in_cup: u8,
    /// The career club's ties played.
    pub ties_played: u8,
    /// Their results: bit j, tie j won.
    pub results: u8,
    /// Each list's clubs (3).
    pub list_counts: Vec<u8>,
    /// The lists written at the season end (3 × 36).
    #[serde(with = "crate::flat")]
    pub lists: Vec<[u16; QUALIFYING_LIST_LEN]>,
    /// Every tie in playing order: round, whether the first club went through, the two
    /// clubs (team words).
    #[serde(with = "crate::flat")]
    pub bracket: Vec<[u16; 4]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};

    fn numbers(value: u32, count: usize) -> String {
        vec![value.to_string(); count].join(",")
    }

    #[test]
    fn the_coefficients_round_trip() {
        let text = format!(
            r#"{{"ring":2,"season_closed":0,"association_clubs":[4],"association_points":[12500],"association_seasons":[{}],"clubs":[575,1000,1,2,3,4,5],"previous_counts":[0,0,0],"previous":[],"finals":[0,0,0,0,0,0],"phase_ended":[0,1,0]}}"#,
            numbers(7, ASSOCIATIONS)
        );
        let coef: UefaCoef = round_trip(&text);
        assert_eq!(coef.association_seasons.len(), 1);
        assert_eq!(coef.clubs, vec![[575, 1000, 1, 2, 3, 4, 5]]);
        assert!(rejects::<UefaCoef>(
            &text.replace(r#"[575,1000,1,2,3,4,5]"#, "[575]")
        ));
    }

    #[test]
    fn the_qualifying_rounds_round_trip() {
        let text = format!(
            r#"{{"mark":20821,"manager_club":5123,"stand_in":0,"stand_in_cup":0,"ties_played":1,"results":1,"list_counts":[36,0,0],"lists":[{}],"bracket":[1,1,5123,7000]}}"#,
            numbers(9, QUALIFYING_LIST_LEN)
        );
        let qualifying: UefaQualifying = round_trip(&text);
        assert_eq!(qualifying.bracket, vec![[1, 1, 5123, 7000]]);
        assert_eq!(qualifying.lists, vec![[9; QUALIFYING_LIST_LEN]]);
    }
}
