//! The competition save: a preset competition (`.SEA`, `.PRE`) or a DIY one (`.DIY`),
//! its members in the writer's order (`write_competition_buf`, src/save_format_json.c).
//! Its sections are the career save's: the primary slot, its league table by team
//! identity, a tournament's stage, the teams.

use std::fmt;

use openswos_career_save_resource::{CompetitionOverrides, CompetitionSlot, SaveTeam, StageRow};
use openswos_common::{Exporter, Importer};
use openswos_save_json_resource::{
    FileKind, SaveString, SchemaVersion, UpstreamLayout, from_save_bytes, to_save_bytes,
};
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

use crate::scalars::CompetitionScalars;

/// The members the writer puts on the line of the member before: the stage's cup list.
const INLINE_MEMBERS: &[&str] = &["comp_slot_stage_cupfull"];

/// Which competition a save holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompetitionKind {
    /// A preset competition (`.SEA`, `.PRE`): file kind `preset`.
    Preset,
    /// A DIY competition (`.DIY`): file kind `diy`.
    Diy,
}

impl CompetitionKind {
    fn file_kind(self) -> FileKind {
        match self {
            Self::Preset => FileKind::Preset,
            Self::Diy => FileKind::Diy,
        }
    }
}

impl Serialize for CompetitionKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.file_kind().name())
    }
}

impl<'de> Deserialize<'de> for CompetitionKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(KindVisitor)
    }
}

struct KindVisitor;

impl Visitor<'_> for KindVisitor {
    type Value = CompetitionKind;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the file kind \"preset\" or \"diy\"")
    }

    fn visit_str<E: de::Error>(self, kind: &str) -> Result<CompetitionKind, E> {
        [CompetitionKind::Preset, CompetitionKind::Diy]
            .into_iter()
            .find(|candidate| candidate.file_kind().name() == kind)
            .ok_or_else(|| E::custom(format!("file kind {kind:?}: not a competition save")))
    }
}

/// A competition save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompetitionSave {
    /// The schema version.
    pub schema_version: SchemaVersion,
    /// A preset or a DIY competition.
    pub file_kind: CompetitionKind,
    /// The world it is played in (its tag); absent in saves made before worlds, which are
    /// World 1996-97's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world: Option<SaveString>,
    /// The teams: `diy_teams`' length.
    pub num_diy_teams: u16,
    /// The scalars.
    pub comp_scalars: CompetitionScalars,
    /// The competition: the primary slot.
    pub comp_slot: CompetitionSlot,
    /// Its league table by team identity.
    pub competition_overrides: CompetitionOverrides,
    /// A tournament's current stage (a group league or a knockout cup): only when the
    /// competition is a tournament, with the members up to `stage_score_pen`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_slot_stage: Option<CompetitionSlot>,
    /// The stage's cup team ids, `{country, ordinal}` byte pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_slot_stage_cupfull: Option<Vec<u8>>,
    /// A group stage's standings, the tournament's teams in order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_standings: Option<Vec<StageRow>>,
    /// A knockout stage's scores (18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_score_tab: Option<Vec<u16>>,
    /// Its two-leg aggregates (18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_score_agg: Option<Vec<u16>>,
    /// Its penalties (18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_score_pen: Option<Vec<u16>>,
    /// Another installed world's countries its teams are from, at the numbers the save gave
    /// them (upstream include/world_packs.h): only when there are some.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_packs: Option<Vec<WorldPack>>,
    /// The competition's teams, as the team crate's records.
    pub diy_teams: Vec<SaveTeam>,
}

/// Another world's country a competition's teams are from (the Mod 4.1's "across worlds"):
/// the country number it plays under for the session, the world and its team file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPack {
    /// The country number its teams have in the competition.
    pub number: u8,
    /// The world's tag (`2627`).
    pub world: SaveString,
    /// Its team file (`ITALY.json`).
    pub file: SaveString,
}

impl CompetitionSave {
    /// The writer's layout: a member a line, none spaced, the stage's cup list on the line
    /// of the stage.
    #[must_use]
    pub fn layout(&self) -> UpstreamLayout {
        UpstreamLayout {
            spaced_members: self.file_kind.file_kind().spaced_members(),
            inline: INLINE_MEMBERS,
        }
    }

    /// Fails when the teams' count disagrees with the teams.
    fn check(&self) -> Result<(), CompetitionSaveError> {
        if usize::from(self.num_diy_teams) == self.diy_teams.len() {
            Ok(())
        } else {
            Err(CompetitionSaveError::Teams {
                stated: self.num_diy_teams,
                teams: self.diy_teams.len(),
            })
        }
    }
}

/// Why a competition save does not read or write.
#[derive(Debug, Error)]
pub enum CompetitionSaveError {
    /// Not the saves' JSON, or not a competition save.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// `num_diy_teams` disagrees with `diy_teams`.
    #[error("num_diy_teams says {stated}, the save holds {teams} teams")]
    Teams {
        /// The count stated.
        stated: u16,
        /// The teams held.
        teams: usize,
    },
}

/// The competition save format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetitionSaveFormat;

impl Importer for CompetitionSaveFormat {
    type Model = CompetitionSave;
    type Error = CompetitionSaveError;

    fn import(bytes: &[u8]) -> Result<CompetitionSave, CompetitionSaveError> {
        let save: CompetitionSave = from_save_bytes(bytes)?;
        save.check()?;
        Ok(save)
    }
}

impl Exporter for CompetitionSaveFormat {
    type Model = CompetitionSave;
    type Error = CompetitionSaveError;

    fn export(model: &CompetitionSave) -> Result<Vec<u8>, CompetitionSaveError> {
        model.check()?;
        Ok(to_save_bytes(model, &model.layout())?)
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // lines the tests just wrote
mod tests {
    use super::*;
    use openswos_career_save_resource::SaveTeam;
    use openswos_common::ByteReader;
    use openswos_team_resource::{TEAM_RECORD_LEN, Team};

    /// A slot's header, then what a cup adds.
    fn cup_slot(competition_type: i16) -> CompetitionSlot {
        let text = format!(
            r#"{{"type":{competition_type},"is_active":1,"comp_slot":0,"slot_id":0,"field_2e":0,"view_phase":0,"avg_skill":0,"str_tier":0,"num_teams":2,"pitch_type_or_season":0,"pitch_type":0,"start_month":0,"end_month":0,"result_code":0,"subs_count":0,"slot_state_a":0,"slot_state_b":0,"slot_state_c":0,"match_started":0,"career_team_involved":0,"has_groups":0,"menu_state":0,"round_iterator":0,"num_teams_in_stage":0,"match_pairs_per_round":0,"total_matches_in_round":0,"round_complete":0,"competition_finished":0,"current_round_index":0,"num_rounds":0,"sub_round_counter":0,"play_each_team":0,"min_subs":0,"points_for_win":0,"comp_slot_word":0,"name":"MY CUP","team_ids":[1,0,1,1],"matchdata_raw":[],"team_offset_table":[0,1],"team_grid_pairings":[],"cup_score_tab":[],"cup_score_agg":[],"cup_score_pen":[],"cup_current_match_index":0,"cup_competition_format":0,"cup_round_format":0,"cup_second_leg_counter":0,"cup_group_section_flag":0,"cup_match_count":0,"cup_total_matches":0,"cup_cur_month_idx":0,"cup_round_legs":[],"cup_round_extra_time":[],"cup_round_penalties":[]}}"#
        );
        from_save_bytes(text.as_bytes()).expect("a cup slot")
    }

    fn team() -> SaveTeam {
        SaveTeam(Team::read(&mut ByteReader::new(&[0; TEAM_RECORD_LEN])).expect("a team"))
    }

    /// A DIY cup of two teams.
    fn save() -> CompetitionSave {
        CompetitionSave {
            schema_version: SchemaVersion,
            file_kind: CompetitionKind::Diy,
            world: Some(SaveString::from("9697")),
            num_diy_teams: 2,
            comp_scalars: CompetitionScalars::default(),
            comp_slot: cup_slot(1),
            competition_overrides: CompetitionOverrides {
                competition_type: 1,
                league_rows: None,
                league_split: None,
            },
            comp_slot_stage: None,
            comp_slot_stage_cupfull: None,
            stage_standings: None,
            stage_score_tab: None,
            stage_score_agg: None,
            stage_score_pen: None,
            world_packs: None,
            diy_teams: vec![team(), team()],
        }
    }

    fn lines(bytes: &[u8]) -> Vec<String> {
        String::from_utf8(bytes.to_vec())
            .expect("ASCII")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn writes_the_writers_layout_and_reads_it_back() {
        let bytes = CompetitionSaveFormat::export(&save()).expect("writes");
        let lines = lines(&bytes);
        assert_eq!(lines[0], "{");
        assert_eq!(lines[1], "  \"schema_version\":3,");
        assert_eq!(lines[2], "  \"file_kind\":\"diy\",");
        assert_eq!(lines[3], "  \"world\":\"9697\",");
        assert_eq!(lines[4], "  \"num_diy_teams\":2,");
        assert!(lines[5].starts_with("  \"comp_scalars\":{\"tmd_team_count\":0,"));
        assert!(lines[6].starts_with("  \"comp_slot\":{\"type\":1,"));
        assert_eq!(lines[7], "  \"competition_overrides\":{\"type\":1},");
        assert!(lines[8].starts_with("  \"diy_teams\":[{\"team\":{"));
        assert_eq!(lines.len(), 10);
        assert!(bytes.ends_with(b"]\n}\n"));
        assert_eq!(
            CompetitionSaveFormat::import(&bytes).expect("reads back"),
            save()
        );
    }

    #[test]
    fn another_worlds_countries_come_before_the_teams() {
        let mut save = save();
        save.world_packs = Some(vec![WorldPack {
            number: 200,
            world: SaveString::from("2627"),
            file: SaveString::from("ITALY.json"),
        }]);
        let bytes = CompetitionSaveFormat::export(&save).expect("writes");
        let lines = lines(&bytes);
        assert_eq!(
            lines[8],
            "  \"world_packs\":[{\"number\":200,\"world\":\"2627\",\"file\":\"ITALY.json\"}],"
        );
        assert!(lines[9].starts_with("  \"diy_teams\":["));
        assert_eq!(
            CompetitionSaveFormat::import(&bytes).expect("reads back"),
            save
        );
    }

    #[test]
    fn a_tournament_carries_its_stage() {
        let mut save = save();
        save.file_kind = CompetitionKind::Preset;
        save.comp_slot_stage = Some(cup_slot(1));
        save.comp_slot_stage_cupfull = Some(vec![1, 0, 1, 1]);
        save.stage_score_tab = Some(vec![0; 18]);
        save.stage_score_agg = Some(vec![0; 18]);
        save.stage_score_pen = Some(vec![0; 18]);
        let bytes = CompetitionSaveFormat::export(&save).expect("writes");
        let lines = lines(&bytes);
        assert_eq!(lines[2], "  \"file_kind\":\"preset\",");
        assert!(lines[8].starts_with("  \"comp_slot_stage\":{\"type\":1,"));
        assert!(lines[8].ends_with(",\"comp_slot_stage_cupfull\":[1,0,1,1],"));
        assert!(lines[9].starts_with("  \"stage_score_tab\":[0,"));
        assert_eq!(
            CompetitionSaveFormat::import(&bytes).expect("reads back"),
            save
        );
    }

    #[test]
    fn the_teams_count_must_agree() {
        let mut save = save();
        save.num_diy_teams = 3;
        assert!(matches!(
            CompetitionSaveFormat::export(&save),
            Err(CompetitionSaveError::Teams {
                stated: 3,
                teams: 2
            })
        ));
    }

    #[test]
    fn other_kinds_and_members_do_not_read() {
        let text = String::from_utf8(CompetitionSaveFormat::export(&save()).expect("writes"))
            .expect("ASCII");
        for (from, to) in [
            ("\"file_kind\":\"diy\"", "\"file_kind\":\"career\""),
            ("\"schema_version\":3", "\"schema_version\":2"),
            ("\"num_diy_teams\":2,", "\"num_diy_teams\":2,\"extra\":1,"),
        ] {
            let changed = text.replace(from, to);
            assert_ne!(changed, text);
            assert!(
                CompetitionSaveFormat::import(changed.as_bytes()).is_err(),
                "{to}"
            );
        }
    }
}
