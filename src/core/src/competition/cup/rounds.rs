//! A cup's rounds' names (docs/PLAN.md M5 #6): which of the world's round names
//! (`rules/strings.json`'s `cup_round_names`, upstream `_cupRoundStringTable`) a round takes
//! (upstream diy_competition.c `GetNextCupRoundStringIndex`, `GetCupRoundStringIndex`). A
//! world's own names for its national cups' and play-offs' rounds (`WorldCups_RoundStringIndex`,
//! `WorldPlayoff_RoundName`) join with World 2026-27's components (M5 #9).

/// The final's name (`_aFinal_2`).
const FINAL: u16 = 8;

/// The semi-final's (`_aSemiFinal`).
const SEMI_FINAL: u16 = 7;

/// The quarter-final's (`_aQuarterFinal`).
const QUARTER_FINAL: u16 = 6;

/// The final group's (`_aFinalGroup`).
const FINAL_GROUP: u16 = 9;

/// The third place play-off's (`_aThirdPlacePlay`).
const THIRD_PLACE: u16 = 10;

/// The final groups' (`_aFinalGroups`).
const FINAL_GROUPS: u16 = 11;

/// The teams of a semi-final round.
const SEMI_FINAL_TEAMS: u16 = 4;

/// The teams of a quarter-final round.
const QUARTER_FINAL_TEAMS: u16 = 8;

/// The name of round `round` (from 0) of a cup of `rounds`, its rounds' names from `base`
/// (the record's `round_string_base`: 1 from the first round's) (upstream
/// `GetNextCupRoundStringIndex`): the last three the quarter-final, the semi-final and the
/// final, the others the base's on. Faithful to the original: the counts are words that wrap.
#[must_use]
pub fn round_name(round: u16, rounds: u16, base: u16) -> u16 {
    if round == rounds.wrapping_sub(1) {
        FINAL
    } else if round == rounds.wrapping_sub(2) {
        SEMI_FINAL
    } else if round == rounds.wrapping_sub(3) {
        QUARTER_FINAL
    } else {
        round.wrapping_add(base).wrapping_sub(1)
    }
}

/// The name of a tournament's stage `stage` (from 0) of `stages` (upstream
/// `GetCupRoundStringIndex`): the last a final group (`groups` 1), final groups (more) or the
/// final (a knockout); a knockout's semi-final and quarter-final by its teams (`teams`), the
/// third place play-off before the final when the tournament plays one (`third_place`); else
/// the stage's own index.
#[must_use]
pub fn stage_name(stage: u16, groups: u16, stages: u16, teams: u16, third_place: bool) -> u16 {
    let mut last = stages.wrapping_sub(1);
    if groups != 0 {
        return if stage != last {
            stage
        } else if groups == 1 {
            FINAL_GROUP
        } else {
            FINAL_GROUPS
        };
    }
    if stage == last {
        return FINAL;
    }
    last = last.wrapping_sub(1);
    if third_place {
        if stage == last {
            return THIRD_PLACE;
        }
        last = last.wrapping_sub(1);
    }
    if stage == last {
        return if teams == SEMI_FINAL_TEAMS {
            SEMI_FINAL
        } else {
            stage
        };
    }
    last = last.wrapping_sub(1);
    if stage == last && teams == QUARTER_FINAL_TEAMS {
        return QUARTER_FINAL;
    }
    stage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cup_s_last_rounds_are_its_final_and_the_two_before() {
        // Six rounds from the first round's name.
        let names: Vec<u16> = (0..6).map(|round| round_name(round, 6, 1)).collect();
        assert_eq!(names, [0, 1, 2, QUARTER_FINAL, SEMI_FINAL, FINAL]);
        assert_eq!(round_name(0, 1, 1), FINAL, "a cup of one round");
        assert_eq!(round_name(0, 6, 3), 2, "from the third round's name");
    }

    #[test]
    fn a_stage_is_named_by_its_place_and_its_teams() {
        assert_eq!(stage_name(4, 0, 5, 2, false), FINAL);
        assert_eq!(stage_name(3, 0, 5, 4, false), SEMI_FINAL);
        assert_eq!(stage_name(3, 0, 5, 6, false), 3, "not of four teams");
        assert_eq!(stage_name(2, 0, 5, 8, false), QUARTER_FINAL);
        assert_eq!(stage_name(4, 0, 6, 2, true), THIRD_PLACE);
        assert_eq!(stage_name(3, 0, 6, 4, true), SEMI_FINAL);
        assert_eq!(stage_name(0, 6, 5, 24, false), 0, "a group stage");
        assert_eq!(stage_name(4, 1, 5, 4, false), FINAL_GROUP);
        assert_eq!(stage_name(4, 2, 5, 8, false), FINAL_GROUPS);
    }
}
