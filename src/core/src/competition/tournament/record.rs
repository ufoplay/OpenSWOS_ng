//! What the engine reads of a tournament's record (docs/PLAN.md M5 #4d2): upstream's
//! `CompetitionTournament` (`CompTournamentHeader`, its round rules, its seeds), its bytes as
//! `InitDIYLeagueFromRecord` reads them.

use openswos_competitions_json_resource::{CompetitionType, Tournament};
use thiserror::Error;

use crate::competition::running::TeamBytes;
use crate::competition::start::NIBBLE_SHIFT;

/// The stages a record states (`rounds[6]`).
const RECORD_STAGES: usize = 6;

/// The bytes of a stage in the record (`Round`: its teams, its groups, its teams a group).
const STAGE_BYTES: usize = 3;

/// The stages' bytes, and the record's reserved byte after them (`rounds[6]`, `reserved`).
pub const STAGES_BYTES: usize = RECORD_STAGES * STAGE_BYTES + 1;

/// The round rule bytes a record holds (`roundRuleFlags[6]`).
pub const ROUND_RULES: usize = 6;

/// The seeds a record holds (`teamSeedList[32]`).
pub const SEEDS: usize = 32;

/// What the engine reads of a tournament's record, in upstream's layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TournamentRecord {
    /// Its competition id (`compId`).
    pub comp_id: u8,
    /// Its first month (`beginMonth`).
    pub begin_month: i8,
    /// Its last month (`endMonth`).
    pub end_month: i8,
    /// Nonzero when its seeds order its first stage (`seedReorderFlag`).
    pub seed_reorder: i8,
    /// Every stage's seeding (`seedingType`): 0 a shuffle, otherwise seeded by value.
    pub seeding_type: i8,
    /// The menu's state (`menuState`): a knockout stage's away goals.
    pub menu_state: i8,
    /// How many times the teams of a group play each other (`playEachTeam`).
    pub play_each_team: i8,
    /// A group stage's points for a win (`awayGoalsMode`, which upstream reads as them).
    pub away_goals_mode: i8,
    /// The substitutes: the fewest in the high nibble, the most in the low one
    /// (`subsAndPointsForWin`).
    pub substitutes: u8,
    /// Its stages (`roundStringBase`, which upstream reads as their count).
    pub stage_count: i8,
    /// Its stages' bytes: each stage's teams, groups (a knockout's 0; a byte below 0 doubles
    /// its bracket) and teams a group, then the reserved byte, which a sixth stage reads as the
    /// teams that go through (`rounds[6]`, `reserved`).
    pub stages: [u8; STAGES_BYTES],
    /// Each stage's rules (legs, extra time and penalties in bits 7-6, 5-4 and 3-2).
    pub round_rules: [u8; ROUND_RULES],
    /// The teams its first stage is ordered by, those its file leaves out `[0, 0]`
    /// (`teamSeedList[32]`).
    pub seeds: [TeamBytes; SEEDS],
}

/// A record no tournament is started from.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecordError {
    /// A competition that is no tournament.
    #[error("{0} is no tournament")]
    NotATournament(String),
    /// More of something than upstream's record holds: stages, round rules or seeds.
    #[error("{id} states {count} {what}, more than the {room} its record holds")]
    PastRecord {
        /// The record's id.
        id: String,
        /// What it states too many of.
        what: &'static str,
        /// How many it states.
        count: usize,
        /// How many upstream's record holds.
        room: usize,
    },
}

impl TournamentRecord {
    /// The record of `tournament` (`tournaments.json`), its seeds `seeds` (the world's
    /// identities of its `team_seed_list`).
    ///
    /// # Errors
    ///
    /// [`RecordError`] when `tournament` is not of a tournament's type, or states more stages,
    /// round rules or seeds than its record holds.
    pub fn from_tournament(
        tournament: &Tournament,
        seeds: &[TeamBytes],
    ) -> Result<Self, RecordError> {
        let id = tournament.id.as_str();
        match tournament.comp.kind {
            CompetitionType::Tournament => {}
            CompetitionType::League | CompetitionType::Cup => {
                return Err(RecordError::NotATournament(id.to_owned()));
            }
        }
        let past = |what, count, room| RecordError::PastRecord {
            id: id.to_owned(),
            what,
            count,
            room,
        };
        if tournament.rounds.len() > RECORD_STAGES {
            return Err(past("stages", tournament.rounds.len(), RECORD_STAGES));
        }
        if tournament.round_rule_flags.len() > ROUND_RULES {
            return Err(past(
                "round rules",
                tournament.round_rule_flags.len(),
                ROUND_RULES,
            ));
        }
        if seeds.len() > SEEDS {
            return Err(past("seeds", seeds.len(), SEEDS));
        }
        let mut stages = [0; STAGES_BYTES];
        for (bytes, round) in stages
            .as_chunks_mut::<STAGE_BYTES>()
            .0
            .iter_mut()
            .zip(&tournament.rounds)
        {
            *bytes = [
                round.team_count.byte(),
                round.group_count.byte(),
                round.team_group_count.byte(),
            ];
        }
        stages[STAGES_BYTES - 1] = tournament.header_reserved.byte();
        let mut round_rules = [0; ROUND_RULES];
        for (byte, &rules) in round_rules.iter_mut().zip(&tournament.round_rule_flags) {
            *byte = u8::from(rules);
        }
        let mut seed_list = [[0; 2]; SEEDS];
        seed_list[..seeds.len()].copy_from_slice(seeds);
        Ok(Self {
            comp_id: tournament.comp.comp_id.byte(),
            begin_month: tournament.comp.begin_month.byte().cast_signed(),
            end_month: tournament.comp.end_month.byte().cast_signed(),
            seed_reorder: tournament.comp.seed_reorder_flag.byte().cast_signed(),
            seeding_type: tournament.seeding_type.byte().cast_signed(),
            menu_state: tournament.menu_state.byte().cast_signed(),
            play_each_team: tournament.play_each_team.byte().cast_signed(),
            away_goals_mode: tournament.away_goals_mode.byte().cast_signed(),
            substitutes: u8::from(tournament.substitutes) << NIBBLE_SHIFT
                | u8::from(tournament.points_for_win),
            stage_count: tournament.round_string_base.byte().cast_signed(),
            stages,
            round_rules,
            seeds: seed_list,
        })
    }

    /// Stage `stage`'s bytes: its teams, its groups, its teams a group, and the teams that go
    /// through (the next stage's teams; the reserved byte for the sixth).
    #[must_use]
    pub fn stage(&self, stage: usize) -> [u8; 4] {
        let at = |offset: usize| self.stages.get(stage * STAGE_BYTES + offset).copied();
        [
            at(0).unwrap_or_default(),
            at(1).unwrap_or_default(),
            at(2).unwrap_or_default(),
            at(STAGE_BYTES).unwrap_or_default(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use openswos_common::Importer;
    use openswos_competitions_json_resource::{CompetitionsFile, CompetitionsJsonFormat};

    use super::*;

    fn tournaments(text: &str) -> Vec<Tournament> {
        let CompetitionsFile::Tournaments(file) = CompetitionsJsonFormat::import(text.as_bytes())
            .expect("tournaments")
            .file
        else {
            panic!("tournaments");
        };
        file.competitions
    }

    /// The European Cup's record, as World 1996-97 holds it, its seeds two.
    const EURO_CUP: &str = r#"{"type": "competition_tournaments", "count": 1, "competitions": [
        {"id": "_euroCup", "type": "tournament", "comp": {"comp_id": 0, "type": 2, "team_number": -1, "begin_month": 64, "end_month": 32, "name_table_offset": 35, "reserved": 0, "team_list_offset": 41, "seed_reorder_flag": 1},
         "seeding_type": 1, "menu_state": 1, "play_each_team": 2, "away_goals_mode": 3, "substitutes": 3, "points_for_win": 5, "round_string_base": 4,
         "rounds": [{"team_count": 16, "group_count": 4, "team_group_count": 4}, {"team_count": 8, "group_count": 0, "team_group_count": 8},
                    {"team_count": 4, "group_count": 0, "team_group_count": 4}, {"team_count": 2, "group_count": 0, "team_group_count": 2},
                    {"team_count": 1, "group_count": 0, "team_group_count": 0}, {"team_count": 0, "group_count": 0, "team_group_count": 0}],
         "header_reserved": 7,
         "round_rule_flags": [{"legs": 0, "extra_time": 0, "penalties": 0}, {"legs": 2, "extra_time": 1, "penalties": 1}],
         "name": ["_aEuropeanChampi_1", "_aEuropeanCup"],
         "team_seed_list": [{"country": 15, "ordinal": 1, "team": "AUXERRE"}, {"country": 21, "ordinal": 0, "team": "AJAX"}]}]}"#;

    #[test]
    fn a_tournament_record_is_its_bytes() {
        let euro_cup = &tournaments(EURO_CUP)[0];
        let record =
            TournamentRecord::from_tournament(euro_cup, &[[15, 1], [21, 0]]).expect("a record");
        assert_eq!(
            (record.comp_id, record.seed_reorder, record.stage_count),
            (0, 1, 4)
        );
        assert_eq!(
            (
                record.seeding_type,
                record.menu_state,
                record.play_each_team,
                record.away_goals_mode
            ),
            (1, 1, 2, 3)
        );
        assert_eq!(record.substitutes, 0x35);
        assert_eq!(
            record.stage(0),
            [16, 4, 4, 8],
            "the next stage's teams go through"
        );
        assert_eq!(record.stage(4), [1, 0, 0, 0]);
        assert_eq!(
            record.stage(5),
            [0, 0, 0, 7],
            "the reserved byte goes through"
        );
        assert_eq!(record.round_rules, [0, 0x94, 0, 0, 0, 0]);
        assert_eq!(&record.seeds[..3], &[[15, 1], [21, 0], [0, 0]]);
    }

    #[test]
    fn a_record_past_upstream_s_is_refused() {
        let euro_cup = &tournaments(EURO_CUP)[0];
        let error =
            TournamentRecord::from_tournament(euro_cup, &[[0, 0]; 33]).expect_err("33 seeds");
        assert_eq!(
            error.to_string(),
            "_euroCup states 33 seeds, more than the 32 its record holds"
        );
        let mut seven = euro_cup.clone();
        seven.rounds.push(seven.rounds[0].clone());
        assert!(matches!(
            TournamentRecord::from_tournament(&seven, &[]),
            Err(RecordError::PastRecord {
                what: "stages",
                count: 7,
                ..
            })
        ));
        let mut cup = euro_cup.clone();
        cup.comp.kind = CompetitionType::Cup;
        assert_eq!(
            TournamentRecord::from_tournament(&cup, &[]),
            Err(RecordError::NotATournament("_euroCup".to_owned()))
        );
    }
}
