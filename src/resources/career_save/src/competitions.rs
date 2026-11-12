//! What the save carries besides the competitions' raw state: the copies' dynamic bytes
//! (upstream `emit_comp_copy_dynamic`), the league tables re-laid by team identity
//! (`emit_competition_overrides`, `emit_ctx_league_standings`) and their rows
//! (`DIYTeamStats`, include/game_types.h), a world league's split (`LeagueSplitState`).
//!
//! What the numbers mean is the competitions' (M5); they are kept as written, in their
//! fields' C widths.

use serde::{Deserialize, Serialize};

/// The two bytes of the competition copies the career changes (upstream
/// `emit_comp_copy_dynamic`): the rest of each copy is its template's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompCopyDynamic {
    /// The World Cup qualifying group 8's teams (byte 13 of its copy).
    pub wc_group8_teams: u8,
    /// Oceania's World Cup qualification's teams (byte 13 of its copy).
    pub oceania_qual_teams: u8,
}

/// A league table's row (upstream `DIYTeamStats`), its team by identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeagueRow {
    /// The team: -1 the career club, otherwise its index among the selected teams
    /// (upstream `teamSelOffset`).
    pub id: i32,
    /// Games played.
    pub gp: u16,
    /// Won.
    pub w: u16,
    /// Drawn.
    pub d: u16,
    /// Lost.
    pub l: u16,
    /// Goals for.
    pub gf: u16,
    /// Goals against.
    pub ga: u16,
    /// Points.
    pub pts: u16,
}

/// A world league's split (upstream `LeagueSplitState`): its table cut into groups part
/// way through.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeagueSplit {
    /// Its table: the world's splits' index + 1.
    pub entry: u8,
    /// The stage (+ 1) whose groups are set; 0 before the split.
    pub frozen: u8,
    /// Its clubs.
    pub n: u8,
    /// Its league's competition.
    pub comp: u8,
    /// The table at the split: place to row (32).
    pub place_row: Vec<u8>,
    /// Each row's group (32).
    pub group: Vec<u8>,
    /// Each row's points before rounding, a tie-break (32).
    pub pre: Vec<u8>,
}

/// The primary competition's league table, laid out by team identity rather than by the
/// raw state's layout (upstream `emit_competition_overrides`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompetitionOverrides {
    /// The competition's type (0 a league); only a league has the rest.
    #[serde(rename = "type")]
    pub competition_type: i16,
    /// The league's rows (64).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub league_rows: Option<Vec<LeagueRow>>,
    /// The league's split, when a world's league is split.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub league_split: Option<LeagueSplit>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};

    const ROW: &str = r#"{"id":-1,"gp":6,"w":2,"d":1,"l":3,"gf":8,"ga":9,"pts":7}"#;

    #[test]
    fn a_league_row_round_trips() {
        let row: LeagueRow = round_trip(ROW);
        assert_eq!(row.id, -1);
        assert_eq!(row.pts, 7);
        assert!(rejects::<LeagueRow>(&ROW.replace("\"gp\":6", "\"gp\":-6")));
    }

    #[test]
    fn overrides_round_trip_for_a_league_a_split_and_a_cup() {
        let league: CompetitionOverrides =
            round_trip(&format!(r#"{{"type":0,"league_rows":[{ROW},{ROW}]}}"#));
        assert_eq!(league.league_rows.map(|rows| rows.len()), Some(2));
        let numbers = vec!["1"; 32].join(",");
        let split: CompetitionOverrides = round_trip(&format!(
            r#"{{"type":0,"league_rows":[{ROW}],"league_split":{{"entry":2,"frozen":0,"n":20,"comp":5,"place_row":[{numbers}],"group":[{numbers}],"pre":[{numbers}]}}}}"#
        ));
        assert_eq!(split.league_split.map(|split| split.n), Some(20));
        let cup: CompetitionOverrides = round_trip(r#"{"type":1}"#);
        assert_eq!(cup.competition_type, 1);
        assert_eq!(cup.league_rows, None);
    }

    #[test]
    fn the_copies_dynamic_bytes_round_trip() {
        let dynamic: CompCopyDynamic =
            round_trip(r#"{"wc_group8_teams":6,"oceania_qual_teams":4}"#);
        assert_eq!(dynamic.wc_group8_teams, 6);
    }
}
