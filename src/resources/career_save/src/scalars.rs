//! `career_scalars`: the career's state outside the other sections, from upstream's
//! field table (`s_career_scalars`, src/save_format_json.c; `emit_scalar_table`). Its 249
//! fields come in the table's order, typed by its field types: numbers in their C widths,
//! names, byte buffers, word arrays.
//!
//! - **Raw state.** The five competition slots' state (`*_competition_context`,
//!   `CompetitionSlotBuffer`) and a few other buffers are written as they lie in memory,
//!   in the x86_64 layout upstream's golden careers are made with. They are kept as
//!   bytes; what they hold is the competitions' (M5, M7).
//! - **The user's tactics** are 370-byte tactics records (`SaveTactics`).
//! - **Lengths.** Byte buffers and arrays keep the length written: upstream writes three
//!   grown lists at the original's length unless the entries past it are in use.
//!
//! What the numbers mean is the career's (M7).

use openswos_common::{ByteReader, ByteWriter};
use openswos_save_json_resource::SaveString;
use openswos_tac_resource::{TACTICS_RECORD_LEN, TacticsError, TeamTactics};
use serde::{Deserialize, Serialize};

/// A user's tactic, which the save writes as the record's 370 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "Vec<u8>", into = "Vec<u8>")]
pub struct SaveTactics(pub TeamTactics);

impl TryFrom<Vec<u8>> for SaveTactics {
    type Error = TacticsError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut reader = ByteReader::new(&bytes);
        let tactics = TeamTactics::read(&mut reader)?;
        reader.finish()?;
        Ok(Self(tactics))
    }
}

impl Default for SaveTactics {
    /// The zeroed record: a new career's user tactics before any is edited.
    fn default() -> Self {
        Self::try_from(vec![0; TACTICS_RECORD_LEN]).expect("a zeroed record is a tactic")
    }
}

impl From<SaveTactics> for Vec<u8> {
    fn from(tactics: SaveTactics) -> Self {
        let mut writer = ByteWriter::with_capacity(TACTICS_RECORD_LEN);
        tactics.0.write(&mut writer);
        writer.into_bytes()
    }
}

/// The career's scalar table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CareerScalars {
    /// The transfer market's teams: `tmd_teams`' length (upstream: the word at
    /// `_careerFileBuffer`).
    pub tmd_team_count: u16,
    /// Upstream `_primaryCompetitionBuf.tourn.header.competitionType`.
    pub primary_slot_type: i16,
    /// Upstream `_competitionContextBuf.tourn.header.competitionType`.
    pub secondary_slot_type: i16,
    /// Upstream `_tertiaryCompetitionContextBuf.tourn.header.competitionType`.
    pub tertiary_slot_type: i16,
    /// Upstream `_euroCupsCompetitionContextBuf.tourn.header.competitionType`.
    pub quaternary_slot_type: i16,
    /// Upstream `_diyCompetitionContextBuf.tourn.header.competitionType`.
    pub diy_slot_type: i16,
    /// Upstream `_currentSeasonShowing`.
    pub current_season_showing: u16,
    /// Upstream `_stayingInOldClub`.
    pub staying_in_old_club: u16,
    /// Upstream `_overdraftStreak`.
    pub overdraft_streak: u16,
    /// Upstream `_overdraftWarningState`.
    pub overdraft_warning_state: u16,
    /// Upstream `_performanceWarningState`.
    pub performance_warning_state: u16,
    /// Upstream `_overdraftWarningCooldown`.
    pub overdraft_warning_cooldown: u16,
    /// Upstream `_performanceWarningCooldown`.
    pub performance_warning_cooldown: u16,
    /// Upstream `_youreSacked`.
    pub youre_sacked: u16,
    /// Upstream `_retired`.
    pub retired: u16,
    /// Upstream `_competitionSlotUpdatedFlag`.
    pub competition_slot_updated_flag: u16,
    /// Upstream `_careerTeamLostMatch`.
    pub career_team_lost_match: u16,
    /// Upstream `_clubBusinessActive`.
    pub club_business_active: u16,
    /// Upstream `_newBalance`.
    pub new_balance: i32,
    /// Upstream `_old_balance`.
    pub old_balance: i32,
    /// Upstream `_playerPurchases`.
    pub player_purchases: i32,
    /// Upstream `_playerSales`.
    pub player_sales: i32,
    /// Upstream `_generalRunningCosts`.
    pub general_running_costs: i32,
    /// Upstream `_playerWagesBill`.
    pub player_wages_bill: i32,
    /// Upstream `_gateReceipts`.
    pub gate_receipts: i32,
    /// Upstream `_startingBalance`.
    pub starting_balance: i32,
    /// Upstream `_startingBalanceCopy`.
    pub starting_balance_copy: i32,
    /// Upstream `_competitionBonuses`.
    pub competition_bonuses: i32,
    /// Upstream `_sensisoftPlcSponsorship`.
    pub sensisoft_plc_sponsorship: i32,
    /// Upstream `_financeTotalsNeedResetFlag`.
    pub finance_totals_need_reset_flag: u16,
    /// Upstream `_weeklyRunningCosts`.
    pub weekly_running_costs: u32,
    /// Upstream `_weeklyPlayerWages`.
    pub weekly_player_wages: u32,
    /// Upstream `_totalTeamValue`.
    pub total_team_value: i32,
    /// Upstream `_weeklyWageByPlayerIndex` (signed long words of 128 bytes).
    pub weekly_wage_by_player_index: Vec<i32>,
    /// Upstream `_playerFormModifiers` (32 bytes).
    pub player_form_modifiers: Vec<u8>,
    /// Upstream `_sensisoftPlcSponsorshipFactor`.
    pub sensisoft_plc_sponsorship_factor: u16,
    /// Upstream `_additionalInvestments`.
    pub additional_investments: u16,
    /// Upstream `_teamValueDivisor`.
    pub team_value_divisor: u16,
    /// Upstream `_expectedLeagueRankIndex`.
    pub expected_league_rank_index: u16,
    /// Upstream `_boardConfidenceHistory` (words of 8 bytes).
    pub board_confidence_history: Vec<u16>,
    /// Upstream `_boardConfidenceScore`.
    pub board_confidence_score: u16,
    /// Upstream `_previousBoardConfidence`.
    pub previous_board_confidence: u16,
    /// Upstream `_playWinCupAnimation`.
    pub play_win_cup_animation: u16,
    /// Upstream `_minorTrophyBonusActive`.
    pub minor_trophy_bonus_active: u16,
    /// Upstream `_leagueTitleBonusActive`.
    pub league_title_bonus_active: u16,
    /// Upstream `_majorTrophyBonusActive`.
    pub major_trophy_bonus_active: u16,
    /// Upstream `_cupBonus1Active`.
    pub cup_bonus_1_active: u16,
    /// Upstream `_cupBonus2Active`.
    pub cup_bonus_2_active: u16,
    /// Upstream `_wonWorldCup`.
    pub won_world_cup: u16,
    /// Upstream `_wonContinentalCup`.
    pub won_continental_cup: u16,
    /// Upstream `_wonInvitationTournament`.
    pub won_invitation_tournament: u16,
    /// Upstream `_qualifyingStage1PendingFlag0`.
    pub qualifying_stage_1_pending_flag0: u16,
    /// Upstream `_qualifyingStage1PendingFlag1`.
    pub qualifying_stage_1_pending_flag1: u16,
    /// Upstream `_qualifyingStage2PendingFlag0`.
    pub qualifying_stage_2_pending_flag0: u16,
    /// Upstream `_qualifyingStage2PendingFlag1`.
    pub qualifying_stage_2_pending_flag1: u16,
    /// Upstream `_isCareer`.
    pub is_career: u16,
    /// Upstream `_careerMatchSimulatedFlag`.
    pub career_match_simulated_flag: u16,
    /// Upstream `_gamesPlayed`.
    pub games_played: u16,
    /// Upstream `_numWins`.
    pub num_wins: u16,
    /// Upstream `_numDrawn`.
    pub num_drawn: u16,
    /// Upstream `_numLost`.
    pub num_lost: u16,
    /// Upstream `_goalsScored`.
    pub goals_scored: u16,
    /// Upstream `_goalsTaken`.
    pub goals_taken: u16,
    /// Upstream `_boardConfidenceMod0`.
    pub board_confidence_mod_0: u16,
    /// Upstream `_boardConfidenceMod1`.
    pub board_confidence_mod_1: u16,
    /// Upstream `_boardConfidenceMod2`.
    pub board_confidence_mod_2: u16,
    /// Upstream `_boardConfidenceMod3`.
    pub board_confidence_mod_3: u16,
    /// Upstream `_boardConfidenceValue0`.
    pub board_confidence_value_0: u16,
    /// Upstream `_boardConfidenceValue1`.
    pub board_confidence_value_1: u16,
    /// Upstream `_boardConfidenceMod4`.
    pub board_confidence_mod_4: u16,
    /// Upstream `_boardConfidenceMod5`.
    pub board_confidence_mod_5: u16,
    /// Upstream `_boardConfidenceMod6`.
    pub board_confidence_mod_6: u16,
    /// Upstream `_boardConfidenceMod7`.
    pub board_confidence_mod_7: u16,
    /// Upstream `_boardConfidenceMod8`.
    pub board_confidence_mod_8: u16,
    /// Upstream `_boardConfidenceMod9`.
    pub board_confidence_mod_9: u16,
    /// Upstream `_contractNegotiationActive`.
    pub contract_negotiation_active: u16,
    /// Upstream `_wageScalingDivisor`.
    pub wage_scaling_divisor: u16,
    /// Upstream `_teamNoOfferAccepted`.
    pub team_no_offer_accepted: u16,
    /// Upstream `_teamFundsAccepted`.
    pub team_funds_accepted: i32,
    /// Upstream `_isNationalSelector`.
    pub is_national_selector: u16,
    /// Upstream `_someTeamFlags`.
    pub some_team_flags: u8,
    /// Upstream `_lastNationalityCall`.
    pub last_nationality_call: u16,
    /// Upstream `_nationalTeamNationality`.
    pub national_team_nationality: u8,
    /// Upstream `_nationalTeamFileId`.
    pub national_team_file_id: u8,
    /// Upstream `_sameNationalCallFlag`.
    pub same_national_call_flag: u16,
    /// Upstream `_sackedReputationScore`.
    pub sacked_reputation_score: u16,
    /// Upstream `_nationalSelectorReputationPoints`.
    pub national_selector_reputation_points: u16,
    /// Upstream `_nationalTeamFileCode`.
    pub national_team_file_code: u8,
    /// Upstream `_seasonReview`.
    pub season_review: u16,
    /// Upstream `_worldCupHostTeam`.
    pub world_cup_host_team: u16,
    /// Upstream `_worldCupQualifyingTeam`.
    pub world_cup_qualifying_team: u16,
    /// Upstream `_euroChampionshipQualTeam`.
    pub euro_championship_qual_team: u16,
    /// Upstream `_selTeamNumber`.
    pub sel_team_number: u16,
    /// Upstream `_coachOrPlayer`.
    pub coach_or_player: u16,
    /// Upstream `_careerTeamLeague`.
    pub career_team_league: u16,
    /// Upstream `_currentDivisionIndex`.
    pub current_division_index: u16,
    /// Upstream `_leagueFirstRoundNumber`.
    pub league_first_round_number: u16,
    /// Upstream `_playPromotedAnimation`.
    pub play_promoted_animation: u16,
    /// Upstream `_playRelegatedAnimation`.
    pub play_relegated_animation: u16,
    /// Upstream `_mrOrMs`.
    pub mr_or_ms: u16,
    /// Upstream `_managerSexBuffer`: a 4-byte name, up to its NUL.
    pub manager_sex_buffer: SaveString,
    /// Upstream `_managerNameBuffer`: a 9-byte name, up to its NUL.
    pub manager_name_buffer: SaveString,
    /// Upstream `_managerSurnameBuffer`: a 13-byte name, up to its NUL.
    pub manager_surname_buffer: SaveString,
    /// Upstream `_plNationality`.
    pub pl_nationality: u8,
    /// Upstream `_winCupBonusCount`.
    pub win_cup_bonus_count: u16,
    /// Upstream `_minorTrophyBonusCount`.
    pub minor_trophy_bonus_count: u16,
    /// Upstream `_majorTrophyBonusCount`.
    pub major_trophy_bonus_count: u16,
    /// Upstream `_careerReputationPoints`.
    pub career_reputation_points: i16,
    /// Upstream `_removedOfferPlayersFromSelectedTeamCount`.
    pub removed_offer_players_from_selected_team_count: u16,
    /// Upstream `_removedOfferPlayersFromOtherTeamsCount`.
    pub removed_offer_players_from_other_teams_count: u16,
    /// Upstream `_homeMarketUpdateBacklog`.
    pub home_market_update_backlog: u16,
    /// Upstream `_foreignMarketUpdateBacklog`.
    pub foreign_market_update_backlog: u16,
    /// Upstream `_offerExpiryIterationsRemaining`.
    pub offer_expiry_iterations_remaining: u16,
    /// Upstream `_numForeignMarketPlayers`.
    pub num_foreign_market_players: u16,
    /// Upstream `_numHomeMarketPlayers`.
    pub num_home_market_players: u16,
    /// Upstream `_numOffers`.
    pub num_offers: u16,
    /// Upstream `_careerSeasonsCompleted`.
    pub career_seasons_completed: u16,
    /// Upstream `_numForeignMarketPlayers2`.
    pub num_foreign_market_players_2: u16,
    /// Upstream `_pendingForeignPlayerCount`.
    pub pending_foreign_player_count: u16,
    /// Upstream `_seasonTransferQuota`.
    pub season_transfer_quota: u16,
    /// Upstream `_transferPoolSlotLimit`.
    pub transfer_pool_slot_limit: u16,
    /// Upstream `_offerSwapSlotLimit`.
    pub offer_swap_slot_limit: u16,
    /// Upstream `_transferPoolSlotsUsed`.
    pub transfer_pool_slots_used: u16,
    /// Upstream `_offerSwapSlotsUsed`.
    pub offer_swap_slots_used: u16,
    /// Upstream `_numNewJobOffers`.
    pub num_new_job_offers: u16,
    /// Upstream `_numJobOffers`.
    pub num_job_offers: u16,
    /// Upstream `_timeToNegotiate`.
    pub time_to_negotiate: u16,
    /// Upstream `_numPlayersOnTransferMarket`.
    pub num_players_on_transfer_market: u16,
    /// Upstream `_lastOfferPrimaryPlayerOrdinal`.
    pub last_offer_primary_player_ordinal: u16,
    /// Upstream `_lastOfferExtraPlayersWanted`.
    pub last_offer_extra_players_wanted: u16,
    /// Upstream `_lastOfferSwapPlayerOrdinal`.
    pub last_offer_swap_player_ordinal: u16,
    /// Upstream `_lastOfferExtraPlayerOrdinal`.
    pub last_offer_extra_player_ordinal: u16,
    /// Upstream `_offerNegotiationAttemptCount`.
    pub offer_negotiation_attempt_count: u16,
    /// Upstream `_transferMarketRandOffset0`.
    pub transfer_market_rand_offset_0: u16,
    /// Upstream `_transferMarketRandOffset1`.
    pub transfer_market_rand_offset_1: u16,
    /// Upstream `_transferMarketRandOffset2`.
    pub transfer_market_rand_offset_2: u16,
    /// Upstream `_transferMarketPhase`.
    pub transfer_market_phase: u16,
    /// Upstream `_careerNumPlayers`.
    pub career_num_players: u16,
    /// Upstream `_selTeamSelectedPlayerPosition`.
    pub sel_team_selected_player_position: u16,
    /// Upstream `_lastFinancialWeekIndex`.
    pub last_financial_week_index: u8,
    /// Upstream `_numPlayers`.
    pub num_players: u16,
    /// Upstream `_currentTeamNumber`.
    pub current_team_number: u16,
    /// Upstream `_currentTeamSelectedPlayerPosition`.
    pub current_team_selected_player_position: u16,
    /// Upstream `_trainingMatchSavedPlayerPos`.
    pub training_match_saved_player_pos: u16,
    /// Upstream `_lastMatchSavedPlayerPos`.
    pub last_match_saved_player_pos: u16,
    /// Upstream `_lastMatchSavedCoachOrPlayer`.
    pub last_match_saved_coach_or_player: u16,
    /// Upstream `_currentTactics`.
    pub current_tactics: u8,
    /// Upstream `_lastMatchSavedTeamControls`.
    pub last_match_saved_team_controls: u8,
    /// Upstream `_seasonFinished`.
    pub season_finished: u16,
    /// Upstream `_careerFinished`.
    pub career_finished: u16,
    /// Upstream `_euroCupSlotIndex`.
    pub euro_cup_slot_index: u16,
    /// Upstream `_seasonNumberOfSquads`.
    pub season_number_of_squads: u16,
    /// Upstream `_teamSelectionKeyCacheIndex`.
    pub team_selection_key_cache_index: u16,
    /// Upstream `_teamSelectionKeyCacheOffset`.
    pub team_selection_key_cache_offset: u16,
    /// Upstream `_teamSelectionKeyCacheCount`.
    pub team_selection_key_cache_count: u16,
    /// Upstream `_careerSlotProcessingInProgress`.
    pub career_slot_processing_in_progress: u16,
    /// Upstream `_teamSelectionKeyCacheEnd`.
    pub team_selection_key_cache_end: u16,
    /// Upstream `_careerJobOfferPhaseBias`.
    pub career_job_offer_phase_bias: u8,
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
    /// Upstream `_viewRandomPhase`.
    pub view_random_phase: u8,
    /// Upstream `_seasonPlaying`.
    pub season_playing: u8,
    /// Upstream `_inFriendlyMenu`.
    pub in_friendly_menu: u16,
    /// Upstream `_isNationalTeam`.
    pub is_national_team: u16,
    /// Upstream `_gameType`.
    pub game_type: u16,
    /// Upstream `_contestStarted`.
    pub contest_started: u16,
    /// Upstream `_careerLoadedFlag`.
    pub career_loaded_flag: u16,
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
    /// Upstream `_seed2`.
    pub seed_2: u8,
    /// Upstream `_randXorIndex2`.
    pub rand_xor_index_2: u8,
    /// Upstream `_randXorKey2`.
    pub rand_xor_key_2: u8,
    /// Upstream `_newRandomSeedCopy`.
    pub new_random_seed_copy: u8,
    /// Upstream `_randomSeed`.
    pub random_seed: u16,
    /// Upstream `_saveFileVersion`.
    pub save_file_version: u16,
    /// Upstream `_saveFileAmigaType`.
    pub save_file_amiga_type: u32,
    /// Upstream `_numSelectedTeams`.
    pub num_selected_teams_inner: u16,
    /// Upstream `_competitionContextBuf.tourn.header.competitionName`: a 36-byte name, up
    /// to its NUL.
    pub secondary_competition_name: SaveString,
    /// Upstream `_competitionContextBuf.tourn.header.trophyBonusValue`.
    pub minor_trophy_bonus_value: u16,
    /// Upstream `_competitionContextBuf.tourn.header.winnerTeam`.
    pub secondary_comp_winner_team: u16,
    /// Upstream `_competitionContextBuf.tourn.header.slotStateA`.
    pub secondary_comp_slot_state_a: u16,
    /// Upstream `_competitionContextBuf.tourn.header.slotStateB`.
    pub secondary_comp_slot_state_b: u16,
    /// Upstream `_competitionContextBuf.tourn.header.slotStateC`.
    pub secondary_comp_slot_state_c: u16,
    /// Upstream `_competitionContextBuf.tourn.header.compFinished`.
    pub secondary_comp_finished: u16,
    /// Upstream `_tertiaryCompetitionContextBuf.tourn.header.competitionName`: a 36-byte
    /// name, up to its NUL.
    pub tertiary_competition_name: SaveString,
    /// Upstream `_tertiaryCompetitionContextBuf.tourn.header.trophyBonusValue`.
    pub league_title_bonus_value: u16,
    /// Upstream `_tertiaryCompetitionContextBuf.tourn.header.winnerTeam`.
    pub tertiary_comp_winner_team: u16,
    /// Upstream `_tertiaryCompetitionContextBuf.tourn.header.slotStateA`.
    pub tertiary_comp_slot_state_a: u16,
    /// Upstream `_tertiaryCompetitionContextBuf.tourn.header.slotStateB`.
    pub tertiary_comp_slot_state_b: u16,
    /// Upstream `_tertiaryCompetitionContextBuf.tourn.header.slotStateC`.
    pub tertiary_comp_slot_state_c: u16,
    /// Upstream `_tertiaryCompetitionContextBuf.tourn.header.compFinished`.
    pub tertiary_comp_finished: u16,
    /// Upstream `_diyCompetitionContextBuf.tourn.header.slotStateA`.
    pub diy_comp_slot_state_a: u16,
    /// Upstream `_diyCompetitionContextBuf.tourn.header.slotStateB`.
    pub diy_comp_slot_state_b: u16,
    /// Upstream `_diyCompetitionContextBuf.tourn.header.slotStateC`.
    pub diy_comp_slot_state_c: u16,
    /// Upstream `_euroCupsCompetitionContextBuf.tourn.header.competitionName`: a 36-byte
    /// name, up to its NUL.
    pub quaternary_competition_name: SaveString,
    /// Upstream `_euroCupsCompetitionContextBuf.tourn.header.trophyBonusValue`.
    pub major_trophy_or_cup_bonus_value: u16,
    /// Upstream `_euroCupsCompetitionContextBuf.tourn.header.winnerTeam`.
    pub quaternary_comp_winner_team: u16,
    /// Upstream `_euroCupsCompetitionContextBuf.tourn.header.slotStateA`.
    pub quaternary_comp_slot_state_a: u16,
    /// Upstream `_euroCupsCompetitionContextBuf.tourn.header.slotStateB`.
    pub quaternary_comp_slot_state_b: u16,
    /// Upstream `_euroCupsCompetitionContextBuf.tourn.header.slotStateC`.
    pub quaternary_comp_slot_state_c: u16,
    /// Upstream `_removedPlayerStatAccum` (words of 2 bytes).
    pub removed_player_stat_accum: Vec<u16>,
    /// Upstream `_careerUnavailablePlayerList` (16 bytes).
    pub career_unavailable_player_list: Vec<u8>,
    /// Upstream `_currentMatchPlayers` (32 bytes).
    pub current_match_players: Vec<u8>,
    /// Upstream `_seasonSquadTeamNumbers` (words of 48 bytes).
    pub season_squad_team_numbers: Vec<u16>,
    /// Upstream `_playerPositions` (32 bytes).
    pub player_positions: Vec<u8>,
    /// Upstream `_trainingMatchSavedPlayers` (32 bytes).
    pub training_match_saved_players: Vec<u8>,
    /// Upstream `_trainingMatchSavedShirtNums` (32 bytes).
    pub training_match_saved_shirt_nums: Vec<u8>,
    /// Upstream `_trainingMatchSavedTactics` (2 bytes).
    pub training_match_saved_tactics: Vec<u8>,
    /// Upstream `_lastMatchPlayers` (32 bytes).
    pub last_match_players: Vec<u8>,
    /// Upstream `_lastMatchSavedShirtNums` (32 bytes).
    pub last_match_saved_shirt_nums: Vec<u8>,
    /// Upstream `_careerLastMatchIndexByCompetitionSlot` (256 bytes).
    pub career_last_match_index_by_competition_slot: Vec<u8>,
    /// Upstream `_teamBudgetRankTable` (128 bytes).
    pub team_budget_rank_table: Vec<u8>,
    /// Upstream `_playersOnTransferMarket`, as saves made before 2026-11-05 write it: its
    /// raw bytes (10), in the saving host's byte order. Upstream's loader ignores it (the
    /// list is lost); it is kept as written. A save has this or
    /// `players_on_transfer_market_ids`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub players_on_transfer_market: Option<Vec<u8>>,
    /// Upstream `_playersOnTransferMarket`: the manager's players on the transfer list, as
    /// words (5, the original's list, unless the entries past it are in use; upstream
    /// commit 5769efd, 2026-11-05).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub players_on_transfer_market_ids: Option<Vec<u16>>,
    /// Upstream `_foreignMarketPlayers` (CAREER_MAX_FOREIGN_LISTING *
    /// CAREER_MARKET_ENTRY_SIZE bytes). Upstream writes the original's length unless the
    /// entries past it are in use.
    pub foreign_market_players: Vec<u8>,
    /// Upstream `_homeMarketPlayers` (100 bytes).
    pub home_market_players: Vec<u8>,
    /// Upstream `_diyLeagueSeasonStateTable` (GLOBAL_TEAM_NUMBERS bytes). Upstream writes
    /// the original's length unless the entries past it are in use.
    pub diy_league_season_state_table: Vec<u8>,
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
    /// Upstream `_competitionContextBuf`: the slot's raw state, as laid out in memory on
    /// x86_64 (`CompetitionSlotBuffer`); its name pointers written as 1 when set, 0 when
    /// not.
    pub competition_context: Vec<u8>,
    /// Upstream `_tertiaryCompetitionContextBuf`: the slot's raw state, as laid out in
    /// memory on x86_64 (`CompetitionSlotBuffer`); its name pointers written as 1 when set,
    /// 0 when not.
    pub tertiary_competition_context: Vec<u8>,
    /// Upstream `_diyCompetitionContextBuf`: the slot's raw state, as laid out in memory on
    /// x86_64 (`CompetitionSlotBuffer`); its name pointers written as 1 when set, 0 when
    /// not.
    pub diy_competition_context: Vec<u8>,
    /// Upstream `_euroCupsCompetitionContextBuf`: the slot's raw state, as laid out in
    /// memory on x86_64 (`CompetitionSlotBuffer`); its name pointers written as 1 when set,
    /// 0 when not.
    pub euro_cups_competition_context: Vec<u8>,
    /// Upstream `_teamSelectionKeyCache` (1870 bytes).
    pub team_selection_key_cache: Vec<u8>,
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
    /// Upstream `_careerUnavailablePlayerLookupTable` (80 bytes).
    pub career_unavailable_player_lookup_table: Vec<u8>,
    /// Upstream `_careerUnavailablePlayerNames` (340 bytes).
    pub career_unavailable_player_names: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};
    use openswos_save_json_resource::{Compact, to_save_bytes};

    #[test]
    fn a_users_tactic_is_its_record() {
        let text = format!("[{}]", vec!["0"; TACTICS_RECORD_LEN].join(","));
        let tactics: SaveTactics = round_trip(&text);
        assert_eq!(tactics, SaveTactics::default());
        assert!(rejects::<SaveTactics>("[0,0]"));
        let last_slot = format!("[{},18]", vec!["0"; TACTICS_RECORD_LEN - 1].join(","));
        assert!(rejects::<SaveTactics>(&last_slot), "18 is no tactic slot");
    }

    #[test]
    fn the_table_writes_every_field_in_order() {
        let scalars = CareerScalars {
            players_on_transfer_market_ids: Some(vec![6, 0, 0, 0, 0]),
            ..CareerScalars::default()
        };
        let bytes = to_save_bytes(&scalars, &Compact).expect("writes");
        let text = String::from_utf8(bytes).expect("ASCII");
        assert!(
            text.starts_with(r#"{"tmd_team_count":0,"primary_slot_type":0,"#),
            "{text}"
        );
        assert!(text.contains(r#","manager_name_buffer":"","#));
        assert!(text.contains(r#","players_on_transfer_market_ids":[6,0,0,0,0],"#));
        assert!(!text.contains(r#""players_on_transfer_market":"#));
        assert!(text.ends_with(r#","career_unavailable_player_names":[]}"#));
        let read: CareerScalars = round_trip(&text);
        assert_eq!(read, scalars);
    }
}
