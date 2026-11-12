//! A world's league contests: the play-off the career club plays (upstream
//! `WorldPlayoffState`, include/world_playoff.h), the stage titles decided mid-season
//! (`WorldStageTitle`), the season winners' page (`sr_winners`), the stage tables kept for
//! the season end (`sr_stage_bank`) and Argentina's promedio (`sr_promedio`; the rules
//! library, third_party/swos-competition-engine/rules/swosrules.h). Saved only when in
//! use.
//!
//! What the numbers mean is the world's competitions' (M5); they are kept as written, in
//! their fields' C widths. Lists keep the length written.

use serde::{Deserialize, Serialize};

/// The play-off the career club plays (upstream `WorldPlayoffState`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPlayoff {
    /// Which kind of contest (upstream `WORLD_PLAYOFF_*`).
    pub kind: u8,
    /// A level tie decided by penalties, not by the better placed.
    pub etp: u8,
    /// A group tie's round flags (legs, extra time, penalties).
    pub rules: u8,
    /// Its last tie's winner; 0 not decided yet.
    pub winner: u16,
    /// The Coppa Italia Serie C winner's entrant.
    pub cup: u8,
    /// The round under way's real ties.
    pub ties: u8,
    /// The other groups simulated early.
    pub early: u8,
    /// A group's placement play-off: its final's flags.
    pub final_rules: u8,
    /// The clubs (`{country, ordinal}`), in entrant order.
    pub teams: Vec<u16>,
    /// Their priorities: a level tie goes to the lower.
    pub places: Vec<u8>,
    /// Their points.
    pub points: Vec<u16>,
    /// Their wins.
    pub won: Vec<u16>,
    /// Their goals scored.
    pub scored: Vec<u16>,
    /// Their table places.
    pub table_places: Vec<u8>,
    /// Their groups.
    pub groups: Vec<u8>,
    /// The round under way: each tie's first entrant.
    pub tie_a: Vec<u8>,
    /// Each tie's second entrant.
    pub tie_b: Vec<u8>,
    /// The divisions simulated early.
    pub early_divisions: Vec<u8>,
    /// Their clubs' counts.
    pub early_counts: Vec<u8>,
    /// Their tables' rows: team, played, won, drawn, lost, for, against, points.
    #[serde(with = "crate::flat")]
    pub early_rows: Vec<[u16; 8]>,
    /// A contest of the pack's spec: country, play-off, stage, season, week, first
    /// group, skip, groups' end, more. With the five lists after it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<[u8; 9]>,
    /// Each entrant's series wins (32).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_wins: Option<Vec<u8>>,
    /// Each tie's winner (32; 255 none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tie_winners: Option<Vec<u8>>,
    /// Each tie's loser (32; 255 none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tie_losers: Option<Vec<u8>>,
    /// The round under way's tie indices (16).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round_ties: Option<Vec<u8>>,
    /// The entrants' global numbers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_numbers: Option<Vec<u16>>,
}

/// The stage titles decided mid-season (upstream `WorldStageTitle` × 3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageTitles {
    /// The section's version: 0.
    pub version: u8,
    /// Per stage: season, decided, champion's and runner-up's teams, their global numbers
    /// (65535 none).
    #[serde(with = "crate::flat")]
    pub stages: Vec<[u16; 6]>,
    /// Per stage: the career club's result (the season record's code), its play-off + 1.
    #[serde(with = "crate::flat")]
    pub club_results: Vec<[u8; 2]>,
}

/// The season winners' page's last season (upstream `sr_winners`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonWinners {
    /// The section's version: 0.
    pub version: u8,
    /// The season.
    pub season: u8,
    /// Per title: its type, index and team word.
    #[serde(with = "crate::flat")]
    pub titles: Vec<[u16; 3]>,
}

/// A league's stage tables (upstream `sr_stage_bank`), kept for the season end's
/// stage-title and title play-offs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeagueStage {
    /// The league's country.
    pub country: u8,
    /// Bit k: table k is this season's.
    pub fresh: u8,
    /// The kept table's division.
    pub kept_division: u8,
    /// Each table's clubs (4).
    pub clubs: Vec<u8>,
    /// The tables' rows: team, global number, won, for, against, points.
    #[serde(with = "crate::flat")]
    pub tables: Vec<[u16; 6]>,
    /// A 'keep' stage's table, in the same rows.
    #[serde(with = "crate::flat")]
    pub kept: Vec<[u16; 6]>,
}

/// Argentina's promedio history (upstream `sr_promedio`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Promedio {
    /// The clubs.
    pub clubs_count: u8,
    /// Per club: global number, then points and matches of each of the two seasons before.
    #[serde(with = "crate::flat")]
    pub clubs: Vec<[u16; 5]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};

    const PLAYOFF: &str = r#"{"kind":1,"etp":0,"rules":2,"winner":0,"cup":0,"ties":1,"early":0,"final_rules":0,"teams":[5311,5312],"places":[1,2],"points":[3,1],"won":[1,0],"scored":[2,1],"table_places":[1,2],"groups":[0,0],"tie_a":[0],"tie_b":[1],"early_divisions":[],"early_counts":[],"early_rows":[]"#;

    #[test]
    fn a_play_off_round_trips_with_or_without_its_spec() {
        let playoff: WorldPlayoff = round_trip(&format!("{PLAYOFF}}}"));
        assert_eq!(playoff.teams, vec![5311, 5312]);
        assert_eq!(playoff.spec, None);
        let spec = format!(
            r#"{PLAYOFF},"spec":[20,1,255,2,0,0,0,0,0],"series_wins":[0],"tie_winners":[255],"tie_losers":[255],"round_ties":[0],"global_numbers":[575,576]}}"#
        );
        let playoff: WorldPlayoff = round_trip(&spec);
        assert_eq!(playoff.spec, Some([20, 1, 255, 2, 0, 0, 0, 0, 0]));
        let rows = PLAYOFF.replace(r#""early_rows":[]"#, r#""early_rows":[1,2,3,4,5,6,7,8]"#);
        let playoff: WorldPlayoff = round_trip(&format!("{rows}}}"));
        assert_eq!(playoff.early_rows, vec![[1, 2, 3, 4, 5, 6, 7, 8]]);
    }

    #[test]
    fn the_titles_and_winners_round_trip() {
        let titles: StageTitles =
            round_trip(r#"{"version":0,"stages":[2,1,5311,5312,575,576],"club_results":[3,1]}"#);
        assert_eq!(titles.stages, vec![[2, 1, 5311, 5312, 575, 576]]);
        let winners: SeasonWinners =
            round_trip(r#"{"version":0,"season":3,"titles":[128,0,5311,1,2,5312]}"#);
        assert_eq!(winners.titles.len(), 2);
        assert!(rejects::<SeasonWinners>(
            r#"{"version":0,"season":3,"titles":[1,2]}"#
        ));
    }

    #[test]
    fn the_stage_tables_and_promedio_round_trip() {
        let stage: LeagueStage = round_trip(
            r#"{"country":20,"fresh":1,"kept_division":0,"clubs":[1,0,0,0],"tables":[5311,575,3,9,2,10],"kept":[]}"#,
        );
        assert_eq!(stage.tables, vec![[5311, 575, 3, 9, 2, 10]]);
        let promedio: Promedio = round_trip(r#"{"clubs_count":1,"clubs":[575,50,38,47,38]}"#);
        assert_eq!(promedio.clubs, vec![[575, 50, 38, 47, 38]]);
    }
}
