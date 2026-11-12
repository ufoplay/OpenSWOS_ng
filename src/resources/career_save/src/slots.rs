//! The competition slots: each competition in play as a `DIYCompetitionEx`, written through
//! its typed fields (upstream `emit_comp_slot`, src/save_format_json.c; the structs of
//! include/game_types.h). A slot's header is common; what follows depends on its type: a
//! league's progress, a cup's bracket and scores, a tournament's stage slots.
//!
//! What the numbers mean is the competitions' (M5); they are kept as written, in their
//! fields' C widths. Lists keep the length written.

use openswos_save_json_resource::{Compact, SaveString, SaveValue, to_save_bytes};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

/// The type of a league.
const LEAGUE: i16 = 0;

/// The type of a cup.
const CUP: i16 = 1;

/// The type of a tournament (a national team's World Cup or European Championship).
const TOURNAMENT: i16 = 2;

/// A slot's header (upstream `DIYCompetition`, and `DIYCompetitionEx`'s two words after
/// it).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotHeader {
    /// The competition's type: 0 a league, 1 a cup, 2 a tournament.
    #[serde(rename = "type")]
    pub competition_type: i16,
    /// In use (a cup), or over.
    pub is_active: u16,
    /// The slot's cache key byte (upstream `compSlot`).
    pub comp_slot: u8,
    /// The slot's index for cache matching.
    pub slot_id: u8,
    /// The team configuration byte (upstream `field_2E`).
    pub field_2e: u8,
    /// The month or phase for scheduling.
    pub view_phase: u8,
    /// The teams' average skill, or a trophy bonus.
    pub avg_skill: u8,
    /// The strength bracket.
    pub str_tier: u8,
    /// The teams.
    pub num_teams: i16,
    /// The pitch type or the season.
    pub pitch_type_or_season: i16,
    /// The pitch type.
    pub pitch_type: i16,
    /// The first month.
    pub start_month: i8,
    /// The last month.
    pub end_month: i8,
    /// The result code, or the winner.
    pub result_code: i16,
    /// The substitutes, or the squad view's parameter.
    pub subs_count: i16,
    /// Slot state A.
    pub slot_state_a: i16,
    /// Slot state B.
    pub slot_state_b: i16,
    /// Slot state C.
    pub slot_state_c: i16,
    /// 1 once a match has begun.
    pub match_started: i16,
    /// 1 when the career club takes part.
    pub career_team_involved: i16,
    /// 0 no groups, 1 groups, 2 groups and a final.
    pub has_groups: i16,
    /// The cup menu's state.
    pub menu_state: i16,
    /// The round iterator, or finished.
    pub round_iterator: i16,
    /// The teams in the current stage.
    pub num_teams_in_stage: i16,
    /// The match pairs a round.
    pub match_pairs_per_round: i16,
    /// The matches of the round.
    pub total_matches_in_round: i16,
    /// The round is complete, or the lowest division.
    pub round_complete: i16,
    /// The competition is finished, or the highest division's offset.
    pub competition_finished: i16,
    /// The round's index, or the promotions.
    pub current_round_index: i16,
    /// The rounds, or the relegations.
    pub num_rounds: i16,
    /// The sub-round counter.
    pub sub_round_counter: i16,
    /// Each team plays the others this many times.
    pub play_each_team: i16,
    /// The fewest substitutes.
    pub min_subs: u16,
    /// The points for a win.
    pub points_for_win: u16,
    /// The slot's cache key and index as the word the game reads (upstream
    /// `*(uint16_t *)&compSlot`).
    pub comp_slot_word: u16,
    /// The competition's name.
    pub name: SaveString,
    /// A league's or cup's teams, `{country, ordinal}` byte pairs; empty for a tournament.
    pub team_ids: Vec<u8>,
}

/// A league's fields (upstream `DIYCompetitionEx_LeagueView` and the league progress).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeagueSlot {
    /// The match data's 234 bytes, as laid out.
    pub matchdata_raw: Vec<u8>,
    /// The standings-sorted positions, a team each.
    pub team_entry_offsets: Vec<u16>,
    /// Matches played.
    pub matches_played_count: u16,
    /// The current slot.
    pub league_current_slot: u16,
    /// The match progress.
    pub league_match_progress: u16,
    /// The matches of the season.
    pub league_total_matches: u16,
    /// The section's half groups.
    pub section_half_group_count: u16,
    /// The section's matches a half.
    pub section_matches_per_half: u16,
    /// The half groups counted.
    pub section_half_groups_counter: u16,
    /// The section's matches played.
    pub section_matches_progress: u16,
    /// Each phase's first month (128), or a split league's days.
    pub phase_start_months: Vec<u8>,
    /// The fewest substitutes.
    pub league_min_substitutes: u16,
    /// The most substitutes.
    pub league_max_substitutes: u16,
    /// Groups, in a DIY league.
    pub league_has_groups_diy: u16,
    /// The matches of a section.
    pub league_matches_per_section: u16,
    /// The matches of a section left.
    pub league_matches_per_section_live: u16,
}

/// A cup's fields (upstream `DIYCompetitionEx_CupView` and the bracket's scratch).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CupSlot {
    /// The match data's 234 bytes, as laid out.
    pub matchdata_raw: Vec<u8>,
    /// Each team's identity (64): -1 the career club, otherwise its index among the
    /// selected teams.
    pub team_offset_table: Vec<i32>,
    /// The teams' grid pairings (64).
    pub team_grid_pairings: Vec<u8>,
    /// The knockout scores (18).
    pub cup_score_tab: Vec<i16>,
    /// The two-leg aggregates (18).
    pub cup_score_agg: Vec<i16>,
    /// The penalties (18).
    pub cup_score_pen: Vec<i16>,
    /// The current match.
    pub cup_current_match_index: u16,
    /// The competition's format.
    pub cup_competition_format: u16,
    /// The round's format.
    pub cup_round_format: u16,
    /// The second-leg counter.
    pub cup_second_leg_counter: u16,
    /// The group section flag.
    pub cup_group_section_flag: u16,
    /// The matches counted.
    pub cup_match_count: u16,
    /// The matches.
    pub cup_total_matches: u16,
    /// The current month's index.
    pub cup_cur_month_idx: u16,
    /// Each round's legs (6).
    pub cup_round_legs: Vec<u16>,
    /// Each round's extra time (6).
    pub cup_round_extra_time: Vec<u16>,
    /// Each round's penalties (6).
    pub cup_round_penalties: Vec<u16>,
}

/// A tournament stage's slot (upstream `DIYTournamentSlot`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TournamentStage {
    /// The cup bracket's doubled count.
    pub double_count: u16,
    /// Nonzero: a league or groups; 0: a cup.
    pub is_league: u16,
    /// The seeding.
    pub seeding: u16,
    /// The teams.
    pub num_teams: u16,
    /// The next slot, or the division factor.
    pub division_factor: u16,
    /// Each team plays the others this many times.
    pub play_each_team: u16,
    /// Initialised to 2 (upstream `initConst168`).
    pub init_168: u16,
    /// The groups.
    pub groups: u16,
    /// The teams a group.
    pub teams_per_group: u16,
    /// 1 normal, 2 skip (upstream `formatFlag16E`).
    pub format_16e: u16,
    /// Initialised to 1 (upstream `initConst170`).
    pub init_170: u16,
    /// 1 halve (upstream `formatFlag172`).
    pub format_172: u16,
    /// The matches a team in this slot.
    pub matches_per_slot: u16,
}

/// A tournament's fields: its stages' slots. Its match data rides the raw state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TournamentSlot {
    /// The stages' slots (6).
    pub tournament_slots: Vec<TournamentStage>,
}

/// Another type's fields: the match data only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OtherSlot {
    /// The match data's 234 bytes, as laid out.
    pub matchdata_raw: Vec<u8>,
}

/// What follows a slot's header, by its type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
pub enum SlotData {
    /// A league (type 0).
    League(LeagueSlot),
    /// A cup (type 1).
    Cup(CupSlot),
    /// A tournament (type 2).
    Tournament(TournamentSlot),
    /// Any other type.
    Other(OtherSlot),
}

/// A competition slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CompetitionSlot {
    /// Its header.
    #[serde(flatten)]
    pub header: SlotHeader,
    /// What follows, by its type.
    #[serde(flatten)]
    pub data: SlotData,
}

impl<'de> Deserialize<'de> for CompetitionSlot {
    /// Reads the header, then the fields its type has, and requires the slot to be
    /// written back exactly as read: an unknown, missing or misplaced member does not
    /// read.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let written = SaveValue::deserialize(deserializer)?;
        let value = serde_json::to_value(&written).map_err(D::Error::custom)?;
        let header: SlotHeader = serde_json::from_value(value.clone()).map_err(D::Error::custom)?;
        let data = match header.competition_type {
            LEAGUE => serde_json::from_value(value).map(SlotData::League),
            CUP => serde_json::from_value(value).map(SlotData::Cup),
            TOURNAMENT => serde_json::from_value(value).map(SlotData::Tournament),
            _ => serde_json::from_value(value).map(SlotData::Other),
        }
        .map_err(D::Error::custom)?;
        let slot = Self { header, data };
        let read = to_save_bytes(&slot, &Compact).map_err(D::Error::custom)?;
        if read == to_save_bytes(&written, &Compact).map_err(D::Error::custom)? {
            Ok(slot)
        } else {
            Err(D::Error::custom(format!(
                "a competition slot of type {}: its members are not the writer's (unknown, missing or out of place)",
                slot.header.competition_type
            )))
        }
    }
}

/// The five competition slots (upstream `comp_slots`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompSlots {
    /// Always 1: the slots are present.
    pub comp_slots_present: u8,
    /// The primary competition (upstream `competitionFileBuffer`).
    pub primary: CompetitionSlot,
    /// The secondary one.
    pub secondary: CompetitionSlot,
    /// The tertiary one.
    pub tertiary: CompetitionSlot,
    /// The DIY one.
    pub diy: CompetitionSlot,
    /// The European cups'.
    pub euro: CompetitionSlot,
}

/// A row of a tournament stage's standings (upstream `DIYTeamStats` without the team: the
/// primary's teams are in order).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRow {
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

/// A row of another slot's stage standings, its team by identity (upstream
/// `teamSelOffset`: -1 the career club, otherwise an index among the selected teams).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageStandingsRow {
    /// The team.
    pub so: i32,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};

    const HEADER: &str = r#""is_active":1,"comp_slot":0,"slot_id":0,"field_2e":0,"view_phase":3,"avg_skill":9,"str_tier":1,"num_teams":2,"pitch_type_or_season":0,"pitch_type":0,"start_month":-1,"end_month":5,"result_code":0,"subs_count":2,"slot_state_a":3,"slot_state_b":2,"slot_state_c":8,"match_started":0,"career_team_involved":1,"has_groups":0,"menu_state":0,"round_iterator":0,"num_teams_in_stage":0,"match_pairs_per_round":0,"total_matches_in_round":0,"round_complete":0,"competition_finished":0,"current_round_index":0,"num_rounds":0,"sub_round_counter":0,"play_each_team":2,"min_subs":0,"points_for_win":3,"comp_slot_word":0,"name":"SERIE D GIRONE F","team_ids":[20,191,20,192]"#;

    fn bytes(count: usize) -> String {
        vec!["0"; count].join(",")
    }

    fn league() -> String {
        format!(
            r#"{{"type":0,{HEADER},"matchdata_raw":[{}],"team_entry_offsets":[0,18],"matches_played_count":4,"league_current_slot":0,"league_match_progress":1,"league_total_matches":34,"section_half_group_count":0,"section_matches_per_half":0,"section_half_groups_counter":0,"section_matches_progress":0,"phase_start_months":[{}],"league_min_substitutes":0,"league_max_substitutes":5,"league_has_groups_diy":0,"league_matches_per_section":0,"league_matches_per_section_live":0}}"#,
            bytes(234),
            bytes(128)
        )
    }

    fn cup() -> String {
        format!(
            r#"{{"type":1,{HEADER},"matchdata_raw":[{}],"team_offset_table":[-1,0],"team_grid_pairings":[1,0],"cup_score_tab":[-1],"cup_score_agg":[0],"cup_score_pen":[0],"cup_current_match_index":0,"cup_competition_format":1,"cup_round_format":0,"cup_second_leg_counter":0,"cup_group_section_flag":0,"cup_match_count":0,"cup_total_matches":1,"cup_cur_month_idx":0,"cup_round_legs":[1,1,1,1,1,2],"cup_round_extra_time":[0,0,0,0,0,1],"cup_round_penalties":[1,1,1,1,1,1]}}"#,
            bytes(234)
        )
    }

    fn tournament() -> String {
        format!(
            r#"{{"type":2,{HEADER},"tournament_slots":[{{"double_count":0,"is_league":1,"seeding":0,"num_teams":4,"division_factor":0,"play_each_team":1,"init_168":2,"groups":8,"teams_per_group":4,"format_16e":1,"init_170":1,"format_172":0,"matches_per_slot":3}}]}}"#
        )
    }

    #[test]
    fn each_type_reads_its_fields() {
        let slot: CompetitionSlot = round_trip(&league());
        assert_eq!(slot.header.name, SaveString::from("SERIE D GIRONE F"));
        assert_eq!(slot.header.start_month, -1);
        let SlotData::League(league) = &slot.data else {
            panic!("{:?}", slot.data);
        };
        assert_eq!(league.league_total_matches, 34);
        let slot: CompetitionSlot = round_trip(&cup());
        assert!(matches!(&slot.data, SlotData::Cup(cup) if cup.team_offset_table == vec![-1, 0]));
        let slot: CompetitionSlot = round_trip(&tournament());
        assert!(matches!(&slot.data, SlotData::Tournament(t) if t.tournament_slots.len() == 1));
        let other = format!(r#"{{"type":4,{HEADER},"matchdata_raw":[1,2]}}"#);
        let slot: CompetitionSlot = round_trip(&other);
        assert!(matches!(&slot.data, SlotData::Other(o) if o.matchdata_raw == vec![1, 2]));
    }

    #[test]
    fn a_slot_must_be_the_writers() {
        for text in [
            // A cup's fields in a league.
            cup().replace(r#"{"type":1,"#, r#"{"type":0,"#),
            // An unknown member.
            league().replace(
                r#""matches_played_count":4"#,
                r#""matches_played_count":4,"x":1"#,
            ),
            // Two members swapped.
            league().replace(
                r#""league_current_slot":0,"league_match_progress":1"#,
                r#""league_match_progress":1,"league_current_slot":0"#,
            ),
            // A missing member.
            tournament().replace(
                r#""tournament_slots":[{"double_count":0,"#,
                r#""tournament_slots":[{"#,
            ),
            // A number past its width.
            league().replace(r#""start_month":-1"#, r#""start_month":128"#),
        ] {
            assert!(rejects::<CompetitionSlot>(&text), "{text}");
        }
    }

    #[test]
    fn stage_rows_round_trip() {
        let row: StageRow = round_trip(r#"{"gp":3,"w":2,"d":1,"l":0,"gf":5,"ga":1,"pts":7}"#);
        assert_eq!(row.pts, 7);
        let row: StageStandingsRow =
            round_trip(r#"{"so":-1,"gp":3,"w":2,"d":1,"l":0,"gf":5,"ga":1,"pts":7}"#);
        assert_eq!(row.so, -1);
    }
}
