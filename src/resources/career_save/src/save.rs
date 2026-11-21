//! The career save: its top-level members in the writer's order (`SaveJson_WriteCareer`,
//! src/save_format_json.c). Members the writer writes only in some careers are options.
//!
//! Every member is typed but the mods' sections, which belong to the mods and are kept as
//! written ([`SaveValue`]).

use std::fmt;

use openswos_common::{Exporter, Importer};
use openswos_save_json_resource::{
    FileKind, SaveString, SaveValue, SchemaVersion, UpstreamLayout, from_save_bytes, to_save_bytes,
};
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

use crate::competitions::{CompCopyDynamic, CompetitionOverrides, LeagueRow};
use crate::contests::{LeagueStage, Promedio, SeasonWinners, StageTitles, WorldPlayoff};
use crate::continuation::ExactContinuation;
use crate::offers::{JobOffer, TransferOffer};
use crate::player::{MarketPlayer, SavePlayer};
use crate::pools::{StatePools, TeamPool};
use crate::scalars::CareerScalars;
use crate::season::SeasonInformation;
use crate::slots::{CompSlots, CompetitionSlot, StageRow, StageStandingsRow};
use crate::supercups::{CupRecord, EuroTitleHolders, WorldSuperCup};
use crate::team::SaveTeam;
use crate::uefa::{UefaCoef, UefaQualifying};
use crate::world_career::{BTeamCalls, CpuReplacements, Ffp, SquadExtras, UefaPrizes, WorldRules};
use crate::world_cups::{ContiEdition, ContiHolders, WorldCupD, WorldCups};

/// The national teams' files whose sizes a world's save states: countries 80..=85, one per
/// continent (upstream `national_file_sizes`).
pub const NATIONAL_FILES: usize = 6;

/// The competitions of a national coach's season (upstream `_careerCupPickList[5]`).
pub const CUP_PICK_LIST_LEN: usize = 5;

/// The members the writer puts on the line of the member before: the stages' cup lists
/// and the cache's scores.
const INLINE_MEMBERS: &[&str] = &[
    "comp_slot_stage_cupfull",
    "secondary_comp_slot_stage_cupfull",
    "tertiary_comp_slot_stage_cupfull",
    "diy_comp_slot_stage_cupfull",
    "euro_comp_slot_stage_cupfull",
    "cache_scores",
];

/// The file kind: `career`, the only one a career save has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CareerKind;

impl Serialize for CareerKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(FileKind::Career.name())
    }
}

impl<'de> Deserialize<'de> for CareerKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(CareerKindVisitor)
    }
}

struct CareerKindVisitor;

impl Visitor<'_> for CareerKindVisitor {
    type Value = CareerKind;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "the file kind \"{}\"", FileKind::Career.name())
    }

    fn visit_str<E: de::Error>(self, kind: &str) -> Result<CareerKind, E> {
        if kind == FileKind::Career.name() {
            Ok(CareerKind)
        } else {
            Err(E::custom(format!("file kind {kind:?}: not a career save")))
        }
    }
}

/// A career save (`.CAR`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CareerSave {
    /// The schema version.
    pub schema_version: SchemaVersion,
    /// The file kind.
    pub file_kind: CareerKind,
    /// The world the career is played in (its tag, `9697`); absent in saves made before
    /// worlds, which are World 1996-97's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world: Option<SaveString>,
    /// The selected teams' count: `selected_teams`' length.
    pub num_selected_teams: u16,
    /// The teams in each national teams' file, countries 80..=85 (upstream
    /// `national_file_sizes`): every world's but World 1996-97's. A save without it was made
    /// with 96/97's (49 34 10 15 31 4); the game refuses one whose sizes are not its own.
    /// Written `"key": value`, as the header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub national_files: Option<[i32; NATIONAL_FILES]>,
    /// The manager's record, a season each.
    pub season_informations: Vec<SeasonInformation>,
    /// The country of the current competition (upstream: `teamNumber` of
    /// `currentCompetitionPtr`); 255 without one.
    pub primary_country_code: u8,
    /// CAREER YEARS: the career's own length, 30 or 40 (upstream `_careerParams.num_seasons`);
    /// absent for the original's 20.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub career_years: Option<u16>,
    /// The career's scalar table.
    pub career_scalars: CareerScalars,
    /// The primary competition's league table by team identity.
    pub competition_overrides: CompetitionOverrides,
    /// The competition copies' dynamic bytes.
    pub comp_copy_dynamic: CompCopyDynamic,
    /// Each career contest's copy's team list (upstream `emit_comp_copy_teamlists`), in
    /// the contests' order: `{country, ordinal}` byte pairs, as written; empty for a copy
    /// without one.
    pub comp_copy_teamlists: Vec<Vec<u8>>,
    /// The five competition slots.
    pub comp_slots: CompSlots,
    /// A tournament primary's current stage (a group league or a knockout cup): only when
    /// the primary is a tournament, with the members up to `stage_score_pen`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_slot_stage: Option<CompetitionSlot>,
    /// The stage's cup team ids, `{country, ordinal}` byte pairs, a team of the tournament
    /// each.
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
    /// The secondary slot's current stage: only when it holds a tournament.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_comp_slot_stage: Option<CompetitionSlot>,
    /// That stage's cup team ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_comp_slot_stage_cupfull: Option<Vec<u8>>,
    /// That stage's standings, when a group league.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_comp_slot_stage_standings: Option<Vec<StageStandingsRow>>,
    /// The tertiary slot's current stage: only when it holds a tournament.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tertiary_comp_slot_stage: Option<CompetitionSlot>,
    /// That stage's cup team ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tertiary_comp_slot_stage_cupfull: Option<Vec<u8>>,
    /// That stage's standings, when a group league.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tertiary_comp_slot_stage_standings: Option<Vec<StageStandingsRow>>,
    /// The diy slot's current stage: only when it holds a tournament.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diy_comp_slot_stage: Option<CompetitionSlot>,
    /// That stage's cup team ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diy_comp_slot_stage_cupfull: Option<Vec<u8>>,
    /// That stage's standings, when a group league.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diy_comp_slot_stage_standings: Option<Vec<StageStandingsRow>>,
    /// The euro slot's current stage: only when it holds a tournament.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub euro_comp_slot_stage: Option<CompetitionSlot>,
    /// That stage's cup team ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub euro_comp_slot_stage_cupfull: Option<Vec<u8>>,
    /// That stage's standings, when a group league.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub euro_comp_slot_stage_standings: Option<Vec<StageStandingsRow>>,
    /// The secondary slot's league table by team identity: only when it holds a league.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_league_rows: Option<Vec<LeagueRow>>,
    /// The tertiary slot's league table by team identity: only when it holds a league.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tertiary_league_rows: Option<Vec<LeagueRow>>,
    /// The diy slot's league table by team identity: only when it holds a league.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diy_league_rows: Option<Vec<LeagueRow>>,
    /// The euro slot's league table by team identity: only when it holds a league.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub euro_league_rows: Option<Vec<LeagueRow>>,
    /// The team selection key cache's entries' strides (upstream `emit_cache_strides`).
    pub cache_strides: Vec<u16>,
    /// Its games' scores, in walk order: packed score, aggregate, penalties.
    #[serde(with = "crate::flat")]
    pub cache_scores: Vec<[u16; 3]>,
    /// The club the manager runs.
    pub career_team: SaveTeam,
    /// The team being played or viewed.
    pub current_team: SaveTeam,
    /// The teams of the competitions in play.
    pub selected_teams: Vec<SaveTeam>,
    /// The selection parked at a season's hub (upstream `Career_ParkedTeams`): the
    /// club's league and its European cup's clubs. Only when something is parked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_teams_parked: Option<Vec<SaveTeam>>,
    /// The transfer market's teams (as many as `career_scalars`' `tmd_team_count`).
    pub tmd_teams: Vec<SaveTeam>,
    /// A world's pooled countries (team pools): only when one is in use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_pools: Option<Vec<TeamPool>>,
    /// The world's cups: in every world but World 1996-97.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_cups: Option<WorldCups>,
    /// The Coppa Italia Serie D: in a world that has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_cupd: Option<WorldCupD>,
    /// The Super Cups: in a world that has them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_supercup: Option<WorldSuperCup>,
    /// The cups' summary: once a European table or a confederations' edition is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cup_record: Option<CupRecord>,
    /// The play-off the career club plays: only while it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_playoff: Option<WorldPlayoff>,
    /// The stage titles decided mid-season: only once one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_titles: Option<StageTitles>,
    /// The European title holders: every world's but World 1996-97's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub euro_title_holders: Option<EuroTitleHolders>,
    /// The season winners' page's last season: only once written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_winners: Option<SeasonWinners>,
    /// The UEFA coefficients: in a world that keeps them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uefa_coef: Option<UefaCoef>,
    /// The UEFA prize money: every world's but World 1996-97's, with the next four.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uefa_prizes: Option<UefaPrizes>,
    /// The financial fair play.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ffp: Option<Ffp>,
    /// The CPU replacements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_replacements: Option<CpuReplacements>,
    /// The squads of 30: the players 17.. gone from their clubs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub squad_extras: Option<SquadExtras>,
    /// The B-team call-ups.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b_team: Option<BTeamCalls>,
    /// The match rules kept with the career: every world's but World 1996-97's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_rules: Option<WorldRules>,
    /// What the career needs to go on exactly: only with `[career] exact_continuation` on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exact_continuation: Option<ExactContinuation>,
    /// A national coach's season's competitions, as career contests' indices (-1 / -2 the
    /// table's ends, -3 none, -4 unknown): only for a national coach.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cup_pick_list: Option<[i32; CUP_PICK_LIST_LEN]>,
    /// The qualifying rounds: while the coefficients' world plays them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uefa_qualifying: Option<UefaQualifying>,
    /// The leagues' stage tables: only those in use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub league_stages: Option<Vec<LeagueStage>>,
    /// The state championships: in a world that has them, once started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_pools: Option<StatePools>,
    /// Argentina's promedio: in a world that keeps it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promedio: Option<Promedio>,
    /// The other confederations' cups' holders: with the next member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conti_holders: Option<ContiHolders>,
    /// The confederation cups' edition being played.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conti_edition: Option<ContiEdition>,
    /// The foreign transfer market's player pool: 432 entries at least (the original's),
    /// the unused ones zero.
    pub career_foreign_market_players: Vec<MarketPlayer>,
    /// Clubs' offers for the manager's players: 5.
    pub transfer_offer_table: Vec<TransferOffer>,
    /// The job offers not yet shown: 5 at least.
    pub new_job_offers: Vec<JobOffer>,
    /// The job offers: 5 at least.
    pub job_offers: Vec<JobOffer>,
    /// The career club's reserves: 16 players.
    pub career_team_reserves: Vec<SavePlayer>,
    /// The current team's reserves: 16 players.
    pub current_team_reserves: Vec<SavePlayer>,
    /// The career mods' sections, by mod (docs/MODS.md §27). Only when a mod wrote one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mods: Option<SaveValue>,
}

impl CareerSave {
    /// The writer's layout of this save: its header written `"key": value` (4 members,
    /// 3 without a world, and the national files' sizes after them), the inline members on
    /// the line of the one before.
    #[must_use]
    pub fn layout(&self) -> UpstreamLayout {
        let header = FileKind::Career.spaced_members() - usize::from(self.world.is_none());
        UpstreamLayout {
            spaced_members: header + usize::from(self.national_files.is_some()),
            inline: INLINE_MEMBERS,
        }
    }

    /// Fails when the counts the save states disagree with what it holds, or the
    /// transfer list is written under neither or both of its keys.
    fn check(&self) -> Result<(), CareerSaveError> {
        if usize::from(self.num_selected_teams) != self.selected_teams.len() {
            return Err(CareerSaveError::SelectedTeams {
                stated: self.num_selected_teams,
                teams: self.selected_teams.len(),
            });
        }
        let scalars = &self.career_scalars;
        if usize::from(scalars.tmd_team_count) != self.tmd_teams.len() {
            return Err(CareerSaveError::TmdTeams {
                stated: scalars.tmd_team_count,
                teams: self.tmd_teams.len(),
            });
        }
        if scalars.players_on_transfer_market.is_some()
            == scalars.players_on_transfer_market_ids.is_some()
        {
            return Err(CareerSaveError::TransferList);
        }
        Ok(())
    }
}

/// Why a career save does not read or write.
#[derive(Debug, Error)]
pub enum CareerSaveError {
    /// Not the saves' JSON, or not a career save.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// `tmd_team_count` disagrees with `tmd_teams`.
    #[error("tmd_team_count says {stated}, the save holds {teams} transfer-market teams")]
    TmdTeams {
        /// The count stated.
        stated: u16,
        /// The teams held.
        teams: usize,
    },
    /// The transfer list is under neither of its keys, or under both.
    #[error(
        "the transfer list must be under one key: players_on_transfer_market_ids, or (before 2026-11-05) players_on_transfer_market"
    )]
    TransferList,
    /// `num_selected_teams` disagrees with `selected_teams`.
    #[error("num_selected_teams says {stated}, the save holds {teams} selected teams")]
    SelectedTeams {
        /// The count stated.
        stated: u16,
        /// The teams held.
        teams: usize,
    },
}

/// The career save format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CareerSaveFormat;

impl Importer for CareerSaveFormat {
    type Model = CareerSave;
    type Error = CareerSaveError;

    fn import(bytes: &[u8]) -> Result<CareerSave, CareerSaveError> {
        let save: CareerSave = from_save_bytes(bytes)?;
        save.check()?;
        Ok(save)
    }
}

impl Exporter for CareerSaveFormat {
    type Model = CareerSave;
    type Error = CareerSaveError;

    fn export(model: &CareerSave) -> Result<Vec<u8>, CareerSaveError> {
        model.check()?;
        Ok(to_save_bytes(model, &model.layout())?)
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // lines the tests just wrote
mod tests {
    use super::*;
    use openswos_common::ByteReader;
    use openswos_team_resource::{PLAYER_RECORD_LEN, Player, TEAM_RECORD_LEN, Team};

    fn team() -> SaveTeam {
        SaveTeam(Team::read(&mut ByteReader::new(&[0; TEAM_RECORD_LEN])).expect("a team"))
    }

    fn player() -> SavePlayer {
        SavePlayer(Player::read(&mut ByteReader::new(&[0; PLAYER_RECORD_LEN])).expect("a player"))
    }

    /// A slot of a type with only its match data.
    fn slot() -> CompetitionSlot {
        openswos_save_json_resource::from_save_bytes(br#"{"type":4,"is_active":0,"comp_slot":0,"slot_id":0,"field_2e":0,"view_phase":0,"avg_skill":0,"str_tier":0,"num_teams":0,"pitch_type_or_season":0,"pitch_type":0,"start_month":0,"end_month":0,"result_code":0,"subs_count":0,"slot_state_a":0,"slot_state_b":0,"slot_state_c":0,"match_started":0,"career_team_involved":0,"has_groups":0,"menu_state":0,"round_iterator":0,"num_teams_in_stage":0,"match_pairs_per_round":0,"total_matches_in_round":0,"round_complete":0,"competition_finished":0,"current_round_index":0,"num_rounds":0,"sub_round_counter":0,"play_each_team":0,"min_subs":0,"points_for_win":0,"comp_slot_word":0,"name":"","team_ids":[],"matchdata_raw":[]}"#)
            .expect("a slot")
    }

    /// A small career save: one season, one selected team, no market.
    fn save() -> CareerSave {
        CareerSave {
            schema_version: SchemaVersion,
            file_kind: CareerKind,
            world: Some(SaveString::from("9697")),
            num_selected_teams: 1,
            national_files: None,
            season_informations: vec![SeasonInformation::default()],
            primary_country_code: 20,
            career_years: None,
            career_scalars: CareerScalars {
                players_on_transfer_market_ids: Some(Vec::new()),
                ..CareerScalars::default()
            },
            competition_overrides: CompetitionOverrides {
                competition_type: 1,
                league_rows: None,
                league_split: None,
            },
            comp_copy_dynamic: CompCopyDynamic::default(),
            comp_copy_teamlists: vec![vec![1, 0], Vec::new()],
            comp_slots: CompSlots {
                comp_slots_present: 1,
                primary: slot(),
                secondary: slot(),
                tertiary: slot(),
                diy: slot(),
                euro: slot(),
            },
            comp_slot_stage: None,
            comp_slot_stage_cupfull: None,
            stage_standings: None,
            stage_score_tab: None,
            stage_score_agg: None,
            stage_score_pen: None,
            secondary_comp_slot_stage: Some(slot()),
            secondary_comp_slot_stage_cupfull: Some(vec![1, 2]),
            secondary_comp_slot_stage_standings: None,
            tertiary_comp_slot_stage: None,
            tertiary_comp_slot_stage_cupfull: None,
            tertiary_comp_slot_stage_standings: None,
            diy_comp_slot_stage: None,
            diy_comp_slot_stage_cupfull: None,
            diy_comp_slot_stage_standings: None,
            euro_comp_slot_stage: None,
            euro_comp_slot_stage_cupfull: None,
            euro_comp_slot_stage_standings: None,
            secondary_league_rows: None,
            tertiary_league_rows: None,
            diy_league_rows: None,
            euro_league_rows: None,
            cache_strides: vec![22],
            cache_scores: vec![[768, 0, 0]],
            career_team: team(),
            current_team: team(),
            selected_teams: vec![team()],
            selected_teams_parked: None,
            tmd_teams: Vec::new(),
            team_pools: None,
            world_cups: None,
            world_cupd: None,
            world_supercup: None,
            cup_record: None,
            world_playoff: None,
            stage_titles: None,
            euro_title_holders: None,
            season_winners: None,
            uefa_coef: None,
            uefa_prizes: None,
            ffp: None,
            cpu_replacements: None,
            squad_extras: None,
            b_team: None,
            world_rules: None,
            exact_continuation: None,
            cup_pick_list: None,
            uefa_qualifying: None,
            league_stages: None,
            state_pools: None,
            promedio: None,
            conti_holders: None,
            conti_edition: None,
            career_foreign_market_players: Vec::new(),
            transfer_offer_table: vec![TransferOffer::default()],
            new_job_offers: Vec::new(),
            job_offers: vec![JobOffer::default()],
            career_team_reserves: vec![player()],
            current_team_reserves: Vec::new(),
            mods: None,
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
        let bytes = CareerSaveFormat::export(&save()).expect("writes");
        let lines = lines(&bytes);
        assert_eq!(lines[0], "{");
        assert_eq!(lines[1], "  \"schema_version\": 3,");
        assert_eq!(lines[2], "  \"file_kind\": \"career\",");
        assert_eq!(lines[3], "  \"world\": \"9697\",");
        assert_eq!(lines[4], "  \"num_selected_teams\": 1,");
        assert!(lines[5].starts_with("  \"season_informations\":[{\"club_no\":0,"));
        assert!(lines[12].starts_with("  \"secondary_comp_slot_stage\":{\"type\":4,"));
        assert!(lines[12].ends_with(",\"secondary_comp_slot_stage_cupfull\":[1,2],"));
        assert_eq!(
            lines[13],
            "  \"cache_strides\":[22],\"cache_scores\":[768,0,0],"
        );
        assert_eq!(lines.last().map(String::as_str), Some("}"));
        assert!(bytes.ends_with(b"\n}\n"));
        assert_eq!(
            CareerSaveFormat::import(&bytes).expect("reads back"),
            save()
        );
    }

    #[test]
    fn a_save_without_a_world_spaces_three_members() {
        let mut save = save();
        save.world = None;
        let bytes = CareerSaveFormat::export(&save).expect("writes");
        let lines = lines(&bytes);
        assert_eq!(lines[3], "  \"num_selected_teams\": 1,");
        assert!(lines[4].starts_with("  \"season_informations\":["));
        assert_eq!(CareerSaveFormat::import(&bytes).expect("reads back"), save);
    }

    /// A world's career: the national files' sizes spaced as the header, CAREER YEARS after
    /// the primary country, the world's members in the writer's order.
    #[test]
    fn a_worlds_save_writes_its_members_in_the_writers_order() {
        let mut save = save();
        save.world = Some(SaveString::from("2627"));
        save.national_files = Some([55, 53, 10, 35, 47, 11]);
        save.career_years = Some(40);
        save.euro_title_holders = Some(EuroTitleHolders {
            champions: 1,
            cup_winners: 2,
            uefa: 3,
        });
        save.world_rules = Some(WorldRules {
            cpu_red_cards: 1,
            cpu_injuries: 1,
            goalkeeper_injuries: 0,
            gk_seed: -2,
        });
        save.cup_pick_list = Some([-1, -2, -3, 4, 47]);
        let bytes = CareerSaveFormat::export(&save).expect("writes");
        let lines = lines(&bytes);
        assert_eq!(lines[4], "  \"num_selected_teams\": 1,");
        assert_eq!(lines[5], "  \"national_files\": [55,53,10,35,47,11],");
        assert!(lines[6].starts_with("  \"season_informations\":["));
        assert_eq!(lines[8], "  \"career_years\":40,");
        let keys: Vec<&str> = lines
            .iter()
            .filter_map(|line| line.strip_prefix("  \"")?.split('"').next())
            .collect();
        let position = |key: &str| keys.iter().position(|k| *k == key).expect(key);
        assert!(position("euro_title_holders") < position("world_rules"));
        assert!(position("world_rules") < position("cup_pick_list"));
        assert!(position("cup_pick_list") < position("career_foreign_market_players"));
        assert_eq!(CareerSaveFormat::import(&bytes).expect("reads back"), save);
    }

    #[test]
    fn the_selected_teams_count_must_agree() {
        let mut save = save();
        save.num_selected_teams = 2;
        assert!(matches!(
            CareerSaveFormat::export(&save),
            Err(CareerSaveError::SelectedTeams {
                stated: 2,
                teams: 1
            })
        ));
        let bytes = String::from_utf8(CareerSaveFormat::export(&self::save()).expect("writes"))
            .expect("ASCII")
            .replace("\"num_selected_teams\": 1", "\"num_selected_teams\": 0");
        assert!(matches!(
            CareerSaveFormat::import(bytes.as_bytes()),
            Err(CareerSaveError::SelectedTeams {
                stated: 0,
                teams: 1
            })
        ));
    }

    #[test]
    fn the_market_teams_count_and_the_transfer_list_key_must_agree() {
        let mut save = save();
        save.career_scalars.tmd_team_count = 1;
        assert!(matches!(
            CareerSaveFormat::export(&save),
            Err(CareerSaveError::TmdTeams {
                stated: 1,
                teams: 0
            })
        ));
        let mut save = self::save();
        save.career_scalars.players_on_transfer_market = Some(vec![0; 10]);
        assert!(matches!(
            CareerSaveFormat::export(&save),
            Err(CareerSaveError::TransferList)
        ));
        save.career_scalars.players_on_transfer_market_ids = None;
        let bytes = CareerSaveFormat::export(&save).expect("a save made before 2026-11-05");
        assert_eq!(CareerSaveFormat::import(&bytes).expect("reads back"), save);
    }

    #[test]
    fn other_versions_kinds_and_members_do_not_read() {
        let text =
            String::from_utf8(CareerSaveFormat::export(&save()).expect("writes")).expect("ASCII");
        for (from, to) in [
            ("\"schema_version\": 3", "\"schema_version\": 2"),
            ("\"file_kind\": \"career\"", "\"file_kind\": \"diy\""),
            (
                "\"primary_country_code\":20",
                "\"primary_country_code\":20,\"extra\":1",
            ),
        ] {
            let changed = text.replace(from, to);
            assert_ne!(changed, text);
            assert!(
                CareerSaveFormat::import(changed.as_bytes()).is_err(),
                "{to}"
            );
        }
    }
}
