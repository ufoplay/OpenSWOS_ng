//! A world's Super Cups (upstream `WorldSuperCupState`, include/world_supercup.h), its cups'
//! summary (`CupRecordState`, include/cup_record.h) and its European title holders
//! (`_euroCupTitleHolderTeamId` and the others): what `SaveJson_WriteCareer` writes for them
//! in a world's career.
//!
//! What the numbers mean is the world's competitions' (M5, M7); they are kept as written, in
//! their fields' C widths. Lists keep the length written.

use serde::{Deserialize, Serialize};

/// The Super Cups' state (upstream `WorldSuperCupState`): per Super Cup what the last season
/// end left, the European holders, this season's national and UEFA Super Cups, the career
/// club's tie. Written only in a world with Super Cups.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSuperCup {
    /// The Super Cups (at most 16, upstream `WORLD_SUPERCUPS_MAX`).
    pub count: u8,
    /// Per Super Cup, the season end its table is of (0: none).
    pub table_seasons: Vec<u8>,
    /// Per Super Cup, the top division's clubs kept.
    pub table_counts: Vec<u8>,
    /// Per Super Cup, its top division's final order (24 team numbers `{country, ordinal}`
    /// each, upstream `WORLD_SUPERCUP_TABLE`).
    pub tables: Vec<u16>,
    /// Per Super Cup, the season end its cup's result is of.
    pub cup_seasons: Vec<u8>,
    /// Per Super Cup, the national cup's winner and finalist (`0xFFFF`: none).
    pub cups: Vec<u16>,
    /// The season end the European holders are of (0: none).
    pub holders_season: u8,
    /// The Champions, Europa and Conference League's holders (`0xFFFF`: none).
    pub holders: [u16; 3],
    /// The season handled (the seasons + 1; 0: none).
    pub season: u8,
    /// Its Super Cup's index.
    pub entry: u8,
    /// 1 the career club's semi-final built, 2 won (its final due), 3 done.
    pub round: u8,
    /// Whether the career club won it.
    pub won: u8,
    /// The other semi-final's winner.
    pub other: u16,
    /// This season's winner (0: not known).
    pub winner: u16,
    /// The UEFA Super Cup's season handled.
    pub uefa_season: u8,
    /// 1 the career club's tie built, 3 done or not in it.
    pub uefa_state: u8,
    /// Whether the career club won it.
    pub uefa_won: u8,
    /// Its winner.
    pub uefa_winner: u16,
    /// The tie in slot 4: 1 national, 2 the UEFA Super Cup's, 0 none (or decided).
    pub built: u8,
    /// Whether it is a final.
    #[serde(rename = "final")]
    pub is_final: u8,
    /// Its week.
    pub week: u8,
    /// Its home club.
    pub team_a: u16,
    /// Its away club.
    pub team_b: u16,
}

/// The world's cups' summary (upstream `CupRecordState`): the career club's European cup
/// this season (the league phase's final table and its knockout matches) and the other
/// confederations' cups' matches of this edition. Written once either is kept.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CupRecord {
    /// The season its European table was kept in (0: none).
    pub euro_season: u8,
    /// The European contest's id.
    pub euro_contest: u8,
    /// The confederations' edition it is of (0: none).
    pub conti_edition: u8,
    /// The league phase's final order: team numbers (at most 36, `CUP_RECORD_TABLE`).
    pub table: Vec<u16>,
    /// Their points.
    pub points: Vec<u8>,
    /// The European cup's matches (at most 96, `CUP_RECORD_GAMES`).
    pub euro_matches: Vec<CupMatch>,
    /// The confederations' cups' matches (at most 160, `CUP_RECORD_CONTI`).
    pub conti_matches: Vec<CupMatch>,
}

/// A match of the cups' summary (upstream `CupRecGame`). Written
/// `[round, flags, home goals, away goals, home penalties, away penalties, home, away]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "CupMatchText", into = "CupMatchText")]
pub struct CupMatch {
    /// European: its knockout stage (1 the first); a confederation's: cup << 4 | round.
    pub round: u8,
    /// Its flags.
    pub flags: u8,
    /// The home side's goals, extra time in.
    pub home_goals: u8,
    /// The away side's goals.
    pub away_goals: u8,
    /// The home side's penalties.
    pub home_penalties: u8,
    /// The away side's penalties.
    pub away_penalties: u8,
    /// The home side: team number `{country, ordinal}`.
    pub home: u16,
    /// The away side.
    pub away: u16,
}

/// A match as written.
type CupMatchText = (u8, u8, u8, u8, u8, u8, u16, u16);

impl From<CupMatchText> for CupMatch {
    fn from(
        (round, flags, home_goals, away_goals, home_penalties, away_penalties, home, away): CupMatchText,
    ) -> Self {
        Self {
            round,
            flags,
            home_goals,
            away_goals,
            home_penalties,
            away_penalties,
            home,
            away,
        }
    }
}

impl From<CupMatch> for CupMatchText {
    fn from(game: CupMatch) -> Self {
        let CupMatch {
            round,
            flags,
            home_goals,
            away_goals,
            home_penalties,
            away_penalties,
            home,
            away,
        } = game;
        (
            round,
            flags,
            home_goals,
            away_goals,
            home_penalties,
            away_penalties,
            home,
            away,
        )
    }
}

/// A world's European title holders, as team words (upstream `_euroCupTitleHolderTeamId`,
/// `_cupWinnersCupTitleHolderTeamId`, `_uefaCupTitleHolderTeamId`). Faithful to the original:
/// outside the career buffer, so a save before them had none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EuroTitleHolders {
    /// The Champions' Cup's holder.
    pub champions: u16,
    /// The Cup Winners' Cup's.
    pub cup_winners: u16,
    /// The UEFA Cup's.
    pub uefa: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};

    #[test]
    fn the_super_cups_round_trip() {
        let cups: WorldSuperCup = round_trip(
            r#"{"count":1,"table_seasons":[2],"table_counts":[1],"tables":[5123],"cup_seasons":[2],"cups":[5124,65535],"holders_season":2,"holders":[65535,1,2],"season":3,"entry":0,"round":1,"won":0,"other":0,"winner":0,"uefa_season":0,"uefa_state":3,"uefa_won":0,"uefa_winner":0,"built":1,"final":1,"week":5,"team_a":5123,"team_b":5124}"#,
        );
        assert_eq!(cups.is_final, 1);
        assert_eq!(cups.holders, [0xFFFF, 1, 2]);
        assert!(rejects::<WorldSuperCup>(r#"{"count":1}"#));
    }

    #[test]
    fn the_cups_summary_round_trips() {
        let record: CupRecord = round_trip(
            r#"{"euro_season":2,"euro_contest":9,"conti_edition":0,"table":[5123,300],"points":[18,3],"euro_matches":[[1,0,2,1,0,0,5123,300]],"conti_matches":[]}"#,
        );
        assert_eq!(
            record.euro_matches,
            vec![CupMatch {
                round: 1,
                flags: 0,
                home_goals: 2,
                away_goals: 1,
                home_penalties: 0,
                away_penalties: 0,
                home: 5123,
                away: 300
            }]
        );
        assert!(rejects::<CupMatch>("[1,0,2,1,0,0,5123]"));
    }

    #[test]
    fn the_title_holders_round_trip() {
        let holders: EuroTitleHolders =
            round_trip(r#"{"champions":5123,"cup_winners":0,"uefa":7}"#);
        assert_eq!(holders.champions, 5123);
    }
}
