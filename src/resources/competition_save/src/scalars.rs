//! `comp_scalars`: the scalars a competition save carries, the fields of upstream's
//! career table tagged for it (`s_career_scalars`' `FXC`/`FAC`/`FXS`/`FAS` entries;
//! `emit_scalar_table_comp`, src/save_format_json.c): the primary slot's header and raw
//! state, the team id lists, the save's version, the match settings and the user's
//! tactics. In the table's order, typed as the career's (`CareerScalars`); byte buffers
//! are written whole.

use openswos_career_save_resource::SaveTactics;
use openswos_save_json_resource::SaveString;
use serde::{Deserialize, Serialize};

/// A competition save's scalars.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompetitionScalars {
    /// Upstream: the word at `_careerFileBuffer` (the transfer market's teams in a
    /// career; a competition save holds none).
    pub tmd_team_count: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.competitionType`.
    pub primary_slot_type: i16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.competitionOver`.
    pub competition_over: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.competitionName`: a 36-byte name, up
    /// to its NUL.
    pub diy_preset_name_of_competition: SaveString,
    /// Upstream `_primaryCompetitionBuf.tourn.header.trophyBonusValue`.
    pub major_trophy_bonus_value: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.numTeams`.
    pub num_diy_teams: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.winnerTeam`.
    pub primary_comp_winner_team: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.squadViewParam`.
    pub primary_comp_squad_view_param: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.slotStateA`.
    pub primary_comp_slot_state_a: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.slotStateB`.
    pub primary_comp_slot_state_b: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.slotStateC`.
    pub primary_comp_slot_state_c: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.minDivisionIndex`.
    pub diy_team_min_division_index: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.maxDivisionOffset`.
    pub diy_team_max_division_offset: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.promotionCount`.
    pub league_promotion_count: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.relegationCount`.
    pub league_relegation_count: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.leagueWarningsEnabled`.
    pub league_warnings_enabled: u16,
    /// Upstream `_gameLength`.
    pub game_length: u16,
    /// Upstream `_autoReplays`.
    pub auto_replays: u16,
    /// Upstream `_menuMusic`.
    pub menu_music: u16,
    /// Upstream `_autoSaveHighlights`.
    pub auto_save_highlights: u16,
    /// Upstream `_allPlayerTeamsEqual`.
    pub all_player_teams_equal: u16,
    /// Upstream `_pitchType`.
    pub pitch_type: u16,
    /// Upstream `_saveFileVersion`.
    pub save_file_version: u16,
    /// Upstream `_saveFileAmigaType`.
    pub save_file_amiga_type: u32,
    /// Upstream `_numSelectedTeams`.
    pub num_selected_teams_inner: u16,
    /// Upstream: the primary slot's league team ids (`matchData.league.leagueTeamIds` of
    /// `_primaryCompetitionBuf`, 288 bytes).
    pub diy_team_id_list_type_0: Vec<u8>,
    /// Upstream: the primary slot's cup team ids (`cupTeamIds` of `_primaryCompetitionBuf`,
    /// 2310 bytes).
    pub diy_team_id_list_type_1: Vec<u8>,
    /// Upstream `_primaryCompetitionBuf.tourn.teamIds` (148 bytes).
    pub diy_team_id_list_type_2: Vec<u8>,
    /// Upstream `_primaryCompetitionBuf`: the slot's raw state, as laid out in memory on
    /// x86_64 (`CompetitionSlotBuffer`); its name pointers written as 1 when set, 0 when
    /// not.
    pub primary_competition_context: Vec<u8>,
    /// Upstream `_teamTacticsUSERA`: a user's tactic (370 bytes).
    pub team_tactics_user_a: SaveTactics,
    /// Upstream `_teamTacticsUSERB`: a user's tactic (370 bytes).
    pub team_tactics_user_b: SaveTactics,
    /// Upstream `_teamTacticsUSERC`: a user's tactic (370 bytes).
    pub team_tactics_user_c: SaveTactics,
    /// Upstream `_teamTacticsUSERD`: a user's tactic (370 bytes).
    pub team_tactics_user_d: SaveTactics,
    /// Upstream `_teamTacticsUSERE`: a user's tactic (370 bytes).
    pub team_tactics_user_e: SaveTactics,
    /// Upstream `_teamTacticsUSERF`: a user's tactic (370 bytes).
    pub team_tactics_user_f: SaveTactics,
}
