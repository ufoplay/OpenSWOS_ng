//! `cups.json`, `tournaments.json` and `playoffs.json`: the knockout cups (upstream
//! `CompetitionCup`), the staged tournaments (`CompetitionTournament`) and the play-offs
//! (`Playoffs`).

use openswos_common::Nibble;
use openswos_common::json::JsonByte;
use serde::{Deserialize, Serialize};

use crate::shared::{CompetitionHeader, Names, RoundRules, SymbolId, TeamSeed};

/// The record kind of a cup: always `"cup"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CupKind {
    /// `"cup"`.
    #[serde(rename = "cup")]
    Cup,
}

/// A knockout cup (and the qualifying groups, friendlies and tours authored as cups).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cup {
    /// Its id.
    pub id: SymbolId,
    /// Always `"cup"`.
    #[serde(rename = "type")]
    pub kind: CupKind,
    /// Its header.
    pub comp: CompetitionHeader,
    /// Nonzero: the number of rounds is not the one its teams give (upstream
    /// `numRoundsOverride`).
    pub num_rounds_override: JsonByte,
    /// Its teams.
    pub teams: JsonByte,
    /// Upstream `awayGoalsMode`.
    pub away_goals_mode: JsonByte,
    /// The substitutes' bench: the high nibble of upstream's `subsAndPointsForWin`.
    pub substitutes: Nibble,
    /// A win's points: its low nibble.
    pub points_for_win: Nibble,
    /// The base of its round captions (upstream `roundStringBase`).
    pub round_string_base: JsonByte,
    /// Each round's rules.
    pub round_rule_flags: Vec<RoundRules>,
    /// Its names.
    pub name: Names,
    /// The teams its draw starts from.
    pub team_seed_list: Vec<TeamSeed>,
}

/// The record kind of a tournament: always `"tournament"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TournamentKind {
    /// `"tournament"`.
    #[serde(rename = "tournament")]
    Tournament,
}

/// A staged tournament: groups, then knockout.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tournament {
    /// Its id.
    pub id: SymbolId,
    /// Always `"tournament"`.
    #[serde(rename = "type")]
    pub kind: TournamentKind,
    /// Its header.
    pub comp: CompetitionHeader,
    /// Copied to every stage's seeding type (upstream `seedingType`).
    pub seeding_type: JsonByte,
    /// Copied to the running competition's menu state (upstream `menuState`).
    pub menu_state: JsonByte,
    /// How many times each team of a group meets each other (upstream `playEachTeam`).
    pub play_each_team: JsonByte,
    /// Upstream `awayGoalsMode`.
    pub away_goals_mode: JsonByte,
    /// The substitutes' bench: the high nibble of upstream's `subsAndPointsForWin`.
    pub substitutes: Nibble,
    /// A win's points: its low nibble.
    pub points_for_win: Nibble,
    /// The base of its round captions (upstream `roundStringBase`).
    pub round_string_base: JsonByte,
    /// Its stages.
    pub rounds: Vec<Round>,
    /// Upstream's header byte 33: never read.
    pub header_reserved: JsonByte,
    /// Each round's rules.
    pub round_rule_flags: Vec<RoundRules>,
    /// Its names.
    pub name: Names,
    /// The teams its draw starts from.
    pub team_seed_list: Vec<TeamSeed>,
}

/// A stage of a tournament (upstream `Round`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Round {
    /// Its teams.
    pub team_count: JsonByte,
    /// Its groups.
    pub group_count: JsonByte,
    /// The teams of each group.
    pub team_group_count: JsonByte,
}

/// Promotion or relegation play-offs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Playoff {
    /// Its id.
    pub id: SymbolId,
    /// Its header.
    pub comp: CompetitionHeader,
    /// Nonzero: the number of rounds is not the one its teams give.
    pub num_rounds_override: JsonByte,
    /// Its teams.
    pub teams: JsonByte,
    /// Upstream `awayGoalsMode`.
    pub away_goals_mode: JsonByte,
    /// The substitutes' bench: the high nibble of upstream's `subsAndMaxSubs`.
    pub substitutes: Nibble,
    /// The substitutions allowed: its low nibble.
    pub max_subs: Nibble,
    /// The base of its round captions.
    pub round_string_base: JsonByte,
    /// Each round's rules.
    pub round_rule_flags: Vec<RoundRules>,
    /// Its names.
    pub name: Names,
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = r#""comp":{"comp_id":1,"type":1,"team_number":-1,"begin_month":64,"end_month":32,"name_table_offset":15,"reserved":0,"team_list_offset":21,"seed_reorder_flag":1}"#;

    #[test]
    fn a_cup_round_trips() {
        let text = format!(
            r#"{{"id":"_cupWinnersCup","type":"cup",{HEADER},"num_rounds_override":0,"teams":32,"away_goals_mode":1,"substitutes":3,"points_for_win":5,"round_string_base":1,"round_rule_flags":[{{"legs":2,"extra_time":1,"penalties":1}}],"name":["_aCupWinnersCup",null],"team_seed_list":[{{"country":"CN_SCOTLAND","ordinal":33,"team":"RANGERS"}}]}}"#
        );
        let cup: Cup = serde_json::from_str(&text).expect("a cup");
        assert_eq!(cup.points_for_win.value(), 5);
        assert_eq!(serde_json::to_string(&cup).ok(), Some(text));
    }

    #[test]
    fn a_tournament_round_trips() {
        let text = format!(
            r#"{{"id":"_euroCup","type":"tournament",{HEADER},"seeding_type":1,"menu_state":1,"play_each_team":2,"away_goals_mode":3,"substitutes":3,"points_for_win":5,"round_string_base":4,"rounds":[{{"team_count":16,"group_count":4,"team_group_count":4}}],"header_reserved":0,"round_rule_flags":[],"name":["_aEuropeanChampi","_aEuropeanCup"],"team_seed_list":[]}}"#
        );
        let tournament: Tournament = serde_json::from_str(&text).expect("a tournament");
        assert_eq!(serde_json::to_string(&tournament).ok(), Some(text));
    }

    #[test]
    fn a_playoff_round_trips() {
        let text = format!(
            r#"{{"id":"_div1Playoffs",{HEADER},"num_rounds_override":0,"teams":4,"away_goals_mode":1,"substitutes":3,"max_subs":5,"round_string_base":1,"round_rule_flags":[{{"legs":2,"extra_time":1,"penalties":1}},{{"legs":0,"extra_time":1,"penalties":1}}],"name":["_aPlayoffs","_aDiv1Playoffs"]}}"#
        );
        let playoff: Playoff = serde_json::from_str(&text).expect("a play-off");
        assert_eq!(playoff.max_subs.value(), 5);
        assert_eq!(serde_json::to_string(&playoff).ok(), Some(text));
    }

    #[test]
    fn the_record_kinds_are_fixed() {
        let cup = format!(
            r#"{{"id":"_x","type":"tournament",{HEADER},"num_rounds_override":0,"teams":2,"away_goals_mode":0,"substitutes":3,"points_for_win":3,"round_string_base":1,"round_rule_flags":[],"name":[],"team_seed_list":[]}}"#
        );
        assert!(serde_json::from_str::<Cup>(&cup).is_err());
    }
}
