//! The header both of upstream's competition states begin with (`DIYCompetition`), checked
//! field by field: `assert_header!(ours, c)` within a proptest, `ours` the port's state,
//! `c` the C's `HeaderFields`.

/// Every header field of `$ours` is `$c`'s.
macro_rules! assert_header {
    ($ours:expr, $c:expr) => {{
        let ours = &$ours;
        let c = &$c;
        prop_assert_eq!(ours.competition_type(), c.competition_type);
        prop_assert_eq!(ours.is_active(), c.is_active);
        prop_assert_eq!(ours.name(), c.name);
        prop_assert_eq!(
            (
                ours.comp_slot(),
                ours.slot_id(),
                ours.comp_id(),
                ours.view_random_phase()
            ),
            (c.comp_slot, c.slot_id, c.comp_id, c.view_random_phase)
        );
        prop_assert_eq!(
            (ours.avg_skill(), ours.strength_tier()),
            (c.avg_skill, c.strength_tier)
        );
        prop_assert_eq!(
            (
                ours.num_teams(),
                ours.pitch_type_or_season(),
                ours.pitch_type()
            ),
            (c.num_teams, c.pitch_type_or_season, c.pitch_type)
        );
        prop_assert_eq!(
            (ours.start_month(), ours.end_month()),
            (c.start_month, c.end_month)
        );
        prop_assert_eq!(
            (ours.result_code(), ours.winner_team()),
            (c.result_code, c.winner_team)
        );
        prop_assert_eq!(ours.substitutes_count(), c.substitutes_count);
        prop_assert_eq!(
            [
                ours.slot_state_a(),
                ours.slot_state_b(),
                ours.slot_state_c()
            ],
            c.slot_states
        );
        prop_assert_eq!(
            (
                ours.match_started(),
                ours.career_team_involved(),
                ours.has_groups(),
                ours.menu_state()
            ),
            (
                c.match_started,
                c.career_team_involved,
                c.has_groups,
                c.menu_state
            )
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
            (
                ours.round_complete(),
                ours.competition_finished(),
                ours.current_round_index()
            ),
            (
                c.round_complete,
                c.competition_finished,
                c.current_round_index
            )
        );
        prop_assert_eq!(
            (
                ours.num_rounds(),
                ours.sub_round_counter(),
                ours.play_each_team()
            ),
            (c.num_rounds, c.sub_round_counter, c.play_each_team)
        );
    }};
}
