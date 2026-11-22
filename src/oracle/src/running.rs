//! A competition under way (upstream `DIYCompetitionEx`, include/game_types.h) as the C reads
//! it: an image written into `_diyCompetitionStart`, every field read back through bindgen's
//! declaration of the struct, so a port's layout is checked against the compiler's.

use crate::Oracle;
use crate::ffi;

pub use crate::ffi::running::IMAGE_LEN;

/// Every field of upstream's competition header (`DIYCompetition`), as its C reads it: a
/// competition under way and a tournament's state begin with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderFields {
    /// `competitionType`.
    pub competition_type: i16,
    /// `isActive`.
    pub is_active: u16,
    /// `competitionName`.
    pub name: [u8; 36],
    /// `compSlot`.
    pub comp_slot: u8,
    /// `slotId`.
    pub slot_id: u8,
    /// `field_2E`.
    pub comp_id: u8,
    /// `viewRandomPhase`.
    pub view_random_phase: u8,
    /// `avgSkill`.
    pub avg_skill: u8,
    /// `strengthTier`.
    pub strength_tier: u8,
    /// `numTeams`.
    pub num_teams: i16,
    /// `pitchTypeOrSeason`.
    pub pitch_type_or_season: i16,
    /// `pitchType`.
    pub pitch_type: i16,
    /// `startMonth`.
    pub start_month: i8,
    /// `endMonth`.
    pub end_month: i8,
    /// `resultCode`.
    pub result_code: i16,
    /// `winnerTeam`, the same bytes.
    pub winner_team: u16,
    /// `substitutesCount`.
    pub substitutes_count: i16,
    /// `slotStateA`, `slotStateB`, `slotStateC`.
    pub slot_states: [i16; 3],
    /// `matchStarted`.
    pub match_started: i16,
    /// `careerTeamInvolved`.
    pub career_team_involved: i16,
    /// `hasGroups`.
    pub has_groups: i16,
    /// `menuState`.
    pub menu_state: i16,
    /// `roundIterator`.
    pub round_iterator: i16,
    /// `numTeamsInStage`.
    pub num_teams_in_stage: i16,
    /// `matchPairsPerRound`.
    pub match_pairs_per_round: i16,
    /// `totalMatchesInRound`.
    pub total_matches_in_round: i16,
    /// `roundComplete`.
    pub round_complete: i16,
    /// `competitionFinished`.
    pub competition_finished: i16,
    /// `currentRoundIndex`.
    pub current_round_index: i16,
    /// `numRounds`.
    pub num_rounds: i16,
    /// `subRoundCounter`.
    pub sub_round_counter: i16,
    /// `playEachTeam`.
    pub play_each_team: i16,
}

/// Every field of a competition under way, as upstream's C reads it (team ids as the native
/// word their two bytes make).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompetitionFields {
    /// The header (`base`).
    pub header: HeaderFields,
    /// `minSubstitutes`.
    pub min_substitutes: u16,
    /// `pointsForAWin`.
    pub points_for_win: u16,
    /// League view: `leagueMinSubstitutes`, `leagueMaxSubstitutes`, `hasGroupsDiy`,
    /// `totalMatchesPerSection`, `totalMatchesPerSectionLive`.
    pub league_words: [u16; 5],
    /// League view: `teamEntryOffsets`.
    pub team_entry_offsets: [u16; 64],
    /// League view: `shuffleAndPad`.
    pub shuffle_and_pad: [u8; 64],
    /// League view: `leagueTeamIds`.
    pub league_team_ids: [u16; 14],
    /// `competitionFormat`.
    pub competition_format: u16,
    /// `roundFormat`.
    pub round_format: u16,
    /// Cup view: `currentMatchIndex`.
    pub cup_current_match_index: u16,
    /// Cup view: `roundLegs`, `roundExtraTime`, `roundPenalties`.
    pub cup_round_rules: [[u16; 6]; 3],
    /// Cup view: `cupRoundResults`.
    pub cup_round_results: [u8; 100],
    /// `cupAggregateScores`.
    pub cup_aggregate_scores: [u16; 48],
    /// `qualifiedTeamsByDivision`.
    pub qualified_teams_by_division: [u16; 8],
    /// `matchesPlayedCount`, `leagueCurrentSlot`, `leagueMatchProgress`,
    /// `leagueTotalMatches`.
    pub league_counters: [u16; 4],
    /// The section: `halfGroupCount`, `matchesPerHalf`, `halfGroupsCounter`,
    /// `sectionMatchesProgress`.
    pub section_words: [u16; 4],
    /// The section: `phaseStartMonths`.
    pub phase_start_months: [u8; 128],
    /// `cupTeamIds`.
    pub cup_team_ids: [u16; 64],
    /// `teamGridPairings`.
    pub team_grid_pairings: [u8; 64],
    /// `teamShuffleOrder`.
    pub team_shuffle_order: Vec<u8>,
    /// Bracket scratch: `secondLegCounter`, `groupSectionFlag`, `matchCount`,
    /// `totalMatches`, `curMonthIdx`.
    pub scratch_words: [u16; 5],
    /// Bracket scratch: `progTable`.
    pub prog_table: [u8; 12],
    /// Bracket scratch: `teamOffsetTable`.
    pub team_offset_table: [i32; 64],
    /// Bracket scratch: `scratchTail`.
    pub scratch_tail: Vec<u8>,
    /// `leagueStandings`: each row's `teamSelOffset`, then its seven words.
    pub standings: Vec<(i32, [u16; 7])>,
}

impl Oracle {
    /// `image` written into `_diyCompetitionStart`, every field read back through the C
    /// struct.
    #[must_use]
    pub fn competition_fields(&mut self, image: &[u8; IMAGE_LEN]) -> CompetitionFields {
        ffi::running::fields(image)
    }
}
