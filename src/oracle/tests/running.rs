//! `openswos_core::competition::running` against upstream's `DIYCompetitionEx`
//! (include/game_types.h): any image, every field the port reads is the field the C reads,
//! both views of each union included, so every offset is the compiler's. Without upstream's
//! checkout the oracle is not built and these tests are not either (`tests/available.rs`
//! says so).
#![cfg(not(oracle_unavailable))]

use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition, STANDINGS_ROWS};
use openswos_oracle::Oracle;
use proptest::collection::vec;
use proptest::prelude::*;

/// The places of the league's order (`teamEntryOffsets[64]`).
const PLACES: usize = 64;

/// The league's team slots (`leagueTeamIds[14]`).
const LEAGUE_TEAMS: usize = 14;

/// A cup's rounds (`roundLegs[6]` and its two siblings).
const CUP_ROUNDS: usize = 6;

/// A cup's round results (`cupRoundResults[100]`).
const CUP_RESULTS: usize = 100;

/// The cup's aggregates (`cupAggregateScores[48]`).
const AGGREGATES: usize = 48;

/// The divisions' qualifiers (`qualifiedTeamsByDivision[8]`).
const DIVISIONS: usize = 8;

/// The phases' months (`phaseStartMonths[128]`).
const PHASES: usize = 128;

/// The shuffle order's bytes (`teamShuffleOrder[698]`).
const SHUFFLE: usize = 698;

/// The month progression's bytes (`progTable[12]`).
const PROG: usize = 12;

/// The bracket scratch's tail (`scratchTail[0x184]`).
const TAIL: usize = 388;

/// The port's team bytes as the native word the C reads.
fn word(team: [u8; 2]) -> u16 {
    u16::from_ne_bytes(team)
}

proptest! {
    #[test]
    fn every_field_is_where_upstream_s_compiler_puts_it(bytes in vec(any::<u8>(), IMAGE_LEN)) {
        let image: [u8; IMAGE_LEN] = bytes.try_into().expect("the image's bytes");
        let c = Oracle::lock().competition_fields(&image);
        let ours = RunningCompetition::from_image(&image);
        prop_assert_eq!(ours.image(), &image);
        // The header.
        prop_assert_eq!(ours.competition_type(), c.competition_type);
        prop_assert_eq!(ours.is_active(), c.is_active);
        prop_assert_eq!(ours.name(), c.name);
        prop_assert_eq!(
            (ours.comp_slot(), ours.slot_id(), ours.comp_id(), ours.view_random_phase()),
            (c.comp_slot, c.slot_id, c.comp_id, c.view_random_phase)
        );
        prop_assert_eq!((ours.avg_skill(), ours.strength_tier()), (c.avg_skill, c.strength_tier));
        prop_assert_eq!(
            (ours.num_teams(), ours.pitch_type_or_season(), ours.pitch_type()),
            (c.num_teams, c.pitch_type_or_season, c.pitch_type)
        );
        prop_assert_eq!((ours.start_month(), ours.end_month()), (c.start_month, c.end_month));
        prop_assert_eq!((ours.result_code(), ours.winner_team()), (c.result_code, c.winner_team));
        prop_assert_eq!(ours.substitutes_count(), c.substitutes_count);
        prop_assert_eq!(
            [ours.slot_state_a(), ours.slot_state_b(), ours.slot_state_c()],
            c.slot_states
        );
        prop_assert_eq!(
            (ours.match_started(), ours.career_team_involved(), ours.has_groups(), ours.menu_state()),
            (c.match_started, c.career_team_involved, c.has_groups, c.menu_state)
        );
        prop_assert_eq!(
            (ours.round_iterator(), ours.num_teams_in_stage()),
            (c.round_iterator, c.num_teams_in_stage)
        );
        prop_assert_eq!(
            (ours.match_pairs_per_round(), ours.total_matches_in_round()),
            (c.match_pairs_per_round, c.total_matches_in_round)
        );
        prop_assert_eq!(
            (ours.round_complete(), ours.competition_finished(), ours.current_round_index()),
            (c.round_complete, c.competition_finished, c.current_round_index)
        );
        prop_assert_eq!(
            (ours.num_rounds(), ours.sub_round_counter(), ours.play_each_team()),
            (c.num_rounds, c.sub_round_counter, c.play_each_team)
        );
        prop_assert_eq!((ours.min_substitutes(), ours.points_for_win()), (c.min_substitutes, c.points_for_win));
        // The match data, both views.
        prop_assert_eq!(
            [
                ours.league_min_substitutes(),
                ours.league_max_substitutes(),
                ours.has_groups_diy(),
                ours.total_matches_per_section(),
                ours.total_matches_per_section_live(),
            ],
            c.league_words
        );
        for place in 0..PLACES {
            prop_assert_eq!(ours.team_entry_offset(place), c.team_entry_offsets[place]);
            prop_assert_eq!(ours.shuffle_and_pad(place), c.shuffle_and_pad[place]);
        }
        for team in 0..LEAGUE_TEAMS {
            prop_assert_eq!(word(ours.league_team(team)), c.league_team_ids[team]);
        }
        prop_assert_eq!((ours.competition_format(), ours.round_format()), (c.competition_format, c.round_format));
        prop_assert_eq!(ours.cup_current_match_index(), c.cup_current_match_index);
        for round in 0..CUP_ROUNDS {
            prop_assert_eq!(
                [ours.cup_round_legs(round), ours.cup_round_extra_time(round), ours.cup_round_penalties(round)],
                [c.cup_round_rules[0][round], c.cup_round_rules[1][round], c.cup_round_rules[2][round]]
            );
        }
        for result in 0..CUP_RESULTS {
            prop_assert_eq!(ours.cup_round_result(result), c.cup_round_results[result]);
        }
        // The rest.
        for score in 0..AGGREGATES {
            prop_assert_eq!(ours.cup_aggregate_score(score), c.cup_aggregate_scores[score]);
        }
        for division in 0..DIVISIONS {
            prop_assert_eq!(ours.qualified_team_by_division(division), c.qualified_teams_by_division[division]);
        }
        prop_assert_eq!(
            [ours.matches_played_count(), ours.league_current_slot(), ours.league_match_progress(), ours.league_total_matches()],
            c.league_counters
        );
        prop_assert_eq!(
            [ours.half_group_count(), ours.matches_per_half(), ours.half_groups_counter(), ours.section_matches_progress()],
            c.section_words
        );
        for phase in 0..PHASES {
            prop_assert_eq!(ours.phase_start_month(phase), c.phase_start_months[phase]);
        }
        for team in 0..PLACES {
            prop_assert_eq!(word(ours.cup_team(team)), c.cup_team_ids[team]);
            prop_assert_eq!(ours.grid_pairing(team), c.team_grid_pairings[team]);
            prop_assert_eq!(ours.team_offset(team), c.team_offset_table[team]);
        }
        for place in 0..SHUFFLE {
            prop_assert_eq!(ours.shuffle_order(place), c.team_shuffle_order[place]);
        }
        prop_assert_eq!(
            [ours.second_leg_counter(), ours.group_section_flag(), ours.match_count(), ours.total_matches(), ours.cur_month_idx()],
            c.scratch_words
        );
        for byte in 0..PROG {
            prop_assert_eq!(ours.prog_table(byte), c.prog_table[byte]);
        }
        for byte in 0..TAIL {
            prop_assert_eq!(ours.scratch_tail(byte), c.scratch_tail[byte]);
        }
        for (index, &(team, words)) in c.standings.iter().enumerate().take(STANDINGS_ROWS) {
            let row = ours.row(index);
            let line = row.line;
            prop_assert_eq!(row.team, team);
            prop_assert_eq!(
                [line.played, line.won, line.drawn, line.lost, line.goals_for, line.goals_against, line.points],
                words
            );
        }
    }
}
