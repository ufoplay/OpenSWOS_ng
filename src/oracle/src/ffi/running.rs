//! `DIYCompetitionEx` read through bindgen's declaration of it.

use super::bindings::diy_competition_c::{self as diy, DIYCompetitionEx};
use crate::running::CompetitionFields;

/// The bytes of the struct in upstream's 64-bit build.
pub const IMAGE_LEN: usize = 2640;

/// `image` written into `_diyCompetitionStart`, every field read back.
pub(crate) fn fields(image: &[u8; IMAGE_LEN]) -> CompetitionFields {
    assert_eq!(
        size_of::<DIYCompetitionEx>(),
        IMAGE_LEN,
        "the 64-bit build's struct"
    );
    // SAFETY: the image is the struct's size, written over the global the lock's holder owns
    // (its name pointer is never followed); every field is then read by value, unaligned
    // where the struct is packed, its unions as the views upstream declares.
    unsafe {
        let competition = &raw mut diy::_diyCompetitionStart;
        competition
            .cast::<u8>()
            .copy_from_nonoverlapping(image.as_ptr(), IMAGE_LEN);
        let c = competition.read_unaligned();
        let base = c.base;
        let league = c.matchData.league;
        let cup = c.matchData.cup;
        let section = c.leagueSectionState;
        let scratch = c.__bindgen_anon_1.bracketScratch;
        CompetitionFields {
            competition_type: base.competitionType,
            is_active: base.__bindgen_anon_1.isActive,
            name: base.competitionName.map(i8::cast_unsigned),
            comp_slot: base.compSlot,
            slot_id: base.slotId,
            comp_id: base.field_2E,
            view_random_phase: base.viewRandomPhase,
            avg_skill: base.__bindgen_anon_2.avgSkill,
            strength_tier: base.strengthTier,
            num_teams: base.numTeams,
            pitch_type_or_season: base.pitchTypeOrSeason,
            pitch_type: base.pitchType,
            start_month: base.startMonth,
            end_month: base.endMonth,
            result_code: base.__bindgen_anon_3.resultCode,
            winner_team: base.__bindgen_anon_3.winnerTeam,
            substitutes_count: base.__bindgen_anon_4.substitutesCount,
            slot_states: [base.slotStateA, base.slotStateB, base.slotStateC],
            match_started: base.matchStarted,
            career_team_involved: base.careerTeamInvolved,
            has_groups: base.hasGroups,
            menu_state: base.menuState,
            round_iterator: base.__bindgen_anon_5.roundIterator,
            num_teams_in_stage: base.numTeamsInStage,
            match_pairs_per_round: base.matchPairsPerRound,
            total_matches_in_round: base.totalMatchesInRound,
            round_complete: base.__bindgen_anon_6.roundComplete,
            competition_finished: base.__bindgen_anon_7.competitionFinished,
            current_round_index: base.__bindgen_anon_8.currentRoundIndex,
            num_rounds: base.__bindgen_anon_9.numRounds,
            sub_round_counter: base.subRoundCounter,
            play_each_team: base.playEachTeam,
            min_substitutes: c.minSubstitutes,
            points_for_win: c.pointsForAWin,
            league_words: [
                league.leagueMinSubstitutes,
                league.leagueMaxSubstitutes,
                league.hasGroupsDiy,
                league.totalMatchesPerSection,
                league.totalMatchesPerSectionLive,
            ],
            team_entry_offsets: league.teamEntryOffsets,
            shuffle_and_pad: league.shuffleAndPad,
            league_team_ids: league.leagueTeamIds,
            competition_format: league.competitionFormat,
            round_format: league.roundFormat,
            cup_current_match_index: cup.currentMatchIndex,
            cup_round_rules: [cup.roundLegs, cup.roundExtraTime, cup.roundPenalties],
            cup_round_results: cup.cupRoundResults,
            cup_aggregate_scores: c.cupAggregateScores,
            qualified_teams_by_division: c.qualifiedTeamsByDivision,
            league_counters: [
                c.matchesPlayedCount,
                c.leagueCurrentSlot,
                c.leagueMatchProgress,
                c.leagueTotalMatches,
            ],
            section_words: [
                section.halfGroupCount,
                section.matchesPerHalf,
                section.halfGroupsCounter,
                section.sectionMatchesProgress,
            ],
            phase_start_months: section.__bindgen_anon_1.phaseStartMonths,
            cup_team_ids: c.cupTeamIds,
            team_grid_pairings: c.teamGridPairings,
            team_shuffle_order: c.__bindgen_anon_1.teamShuffleOrder.to_vec(),
            scratch_words: [
                scratch.secondLegCounter,
                scratch.groupSectionFlag,
                scratch.matchCount,
                scratch.totalMatches,
                scratch.curMonthIdx,
            ],
            prog_table: scratch.progTable,
            team_offset_table: scratch.teamOffsetTable,
            scratch_tail: scratch.scratchTail.to_vec(),
            standings: { c.__bindgen_anon_2.leagueStandings }
                .into_iter()
                .map(|row| {
                    (
                        row.teamSelOffset,
                        [
                            row.gamesPlayed,
                            row.wins,
                            row.draws,
                            row.losses,
                            row.goalsFor,
                            row.goalsAgainst,
                            row.points,
                        ],
                    )
                })
                .collect(),
        }
    }
}
