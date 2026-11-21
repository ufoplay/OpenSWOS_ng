//! A season of the manager's record (upstream `SeasonInformations`, include/game_types.h;
//! `emit_season_informations`, src/save_format_json.c): one per season of the career, the
//! seasons not played zero. A save writes 35 (as before CAREER YEARS), or as many as a
//! longer career's years, up to 40 (`save_si_rows`, `CAREER_MAX_SEASONS`).
//!
//! What the numbers mean (trophies, results, the manager's status) is the career's (M7);
//! they are kept as written, in their fields' C widths. Names are the bytes written:
//! upstream's loader keeps 17 bytes of the club's name and 22 of a scorer's (one less than
//! each slot).

use openswos_save_json_resource::SaveString;
use serde::{Deserialize, Serialize};

/// A season of the manager's record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonInformation {
    /// The club managed (upstream `clubNo`).
    pub club_no: u16,
    /// Its name.
    pub club_name: SaveString,
    /// The trophies won.
    pub trophy_flags: u16,
    /// The league's name; empty without one.
    #[serde(rename = "league_str")]
    pub league: SaveString,
    /// The first cup's name; empty without one.
    #[serde(rename = "str_1")]
    pub cup_1: SaveString,
    /// The second cup's name; empty without one.
    #[serde(rename = "str_2")]
    pub cup_2: SaveString,
    /// The third cup's name; empty without one.
    #[serde(rename = "str_3")]
    pub cup_3: SaveString,
    /// The best scorer's goals.
    pub best_scorer_goals: u16,
    /// The best scorer's name.
    pub best_scorer_name: SaveString,
    /// The second best scorer's name.
    pub second_scorer_name: SaveString,
    /// The result in the first competition.
    pub comp_1_result: i8,
    /// The result in the second.
    pub comp_2_result: i8,
    /// The result in the third.
    pub comp_3_result: i8,
    /// The result in the fourth.
    pub comp_4_result: i8,
    /// Goals scored.
    pub total_goals_scored: u16,
    /// Goals conceded.
    pub total_goals_taken: u16,
    /// Games played.
    pub total_games: u8,
    /// Games won.
    pub total_wins: u8,
    /// Games drawn.
    pub total_drawn: u8,
    /// Games lost.
    pub total_lost: u8,
    /// Friendlies won.
    pub friendly_wins: u8,
    /// Friendlies drawn.
    pub friendly_draws: u8,
    /// Friendlies lost.
    pub friendly_lost: u8,
    /// Tour games won.
    pub tour_wins: u8,
    /// Tour games drawn.
    pub tour_draws: u8,
    /// Tour games lost.
    pub tour_lost: u8,
    /// The tournament's status.
    pub tournament_status: u8,
    /// The manager's status.
    pub manager_status: i8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use openswos_save_json_resource::{Compact, from_save_bytes, to_save_bytes};

    const SEASON: &str = r#"{"club_no":5311,"club_name":"L'AQUILA","trophy_flags":0,"league_str":"SERIE D GIRONE F","str_1":"","str_2":"COPPA ITALIA SERIE D","str_3":"","best_scorer_goals":0,"best_scorer_name":"","second_scorer_name":"","comp_1_result":-1,"comp_2_result":0,"comp_3_result":0,"comp_4_result":0,"total_goals_scored":0,"total_goals_taken":0,"total_games":0,"total_wins":0,"total_drawn":0,"total_lost":0,"friendly_wins":0,"friendly_draws":0,"friendly_lost":0,"tour_wins":0,"tour_draws":0,"tour_lost":0,"tournament_status":0,"manager_status":0}"#;

    #[test]
    fn a_season_round_trips_in_the_writers_order() {
        let season: SeasonInformation = from_save_bytes(SEASON.as_bytes()).expect("a season");
        assert_eq!(season.club_no, 5311);
        assert_eq!(season.cup_2, SaveString::from("COPPA ITALIA SERIE D"));
        assert_eq!(season.comp_1_result, -1);
        let bytes = to_save_bytes(&season, &Compact).expect("writes");
        assert_eq!(bytes, SEASON.as_bytes().to_vec());
    }

    #[test]
    fn numbers_keep_their_widths() {
        for (from, to) in [
            ("\"total_games\":0", "\"total_games\":256"),
            ("\"comp_1_result\":-1", "\"comp_1_result\":128"),
            ("\"club_no\":5311", "\"club_no\":-1"),
        ] {
            let text = SEASON.replace(from, to);
            assert!(
                from_save_bytes::<SeasonInformation>(text.as_bytes()).is_err(),
                "{to}"
            );
        }
    }
}
