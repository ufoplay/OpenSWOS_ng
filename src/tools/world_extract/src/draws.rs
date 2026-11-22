//! A world's own draws (docs/PLAN.md M5 #4d2): `rules/draws.json`, the tournaments upstream
//! draws by its own tables, picked by competition id (src/diy_competition.c
//! `DIY_BuildTeamGridAndIndexMaps`, `DIY_ShuffleTeamsAndBuildCompTeamList`), each the
//! world's tournament records of the ids.

use std::collections::BTreeMap;

use openswos_competitions_json_resource::{CompetitionsDocument, CompetitionsFile};
use openswos_world_rules_resource::{Draws, OwnDraw};

use crate::error::ExtractError;

/// The tournaments' file.
const TOURNAMENTS: &str = "tournaments.json";

/// The World Cup (include/comp_identity.h `COMPID_worldCup`): `buildCompSpecificTeamList`,
/// whose first stage copies its first 24 places.
const WORLD_CUP: (&[u8], u8) = (&[4], 24);

/// The European Championships (`COMPID_europeanChampionships2`,
/// `COMPID_europeanChampionships`): `buildCompSpecificTeamShuffle`, whose first stage copies
/// its first 16 places.
const EUROPEAN_CHAMPIONSHIPS: (&[u8], u8) = (&[21, 22], 16);

/// `rules/draws.json`: each own draw upstream's code names, its competitions the tournaments of
/// `competitions` with its ids, in the ids' order; a draw of no tournament left out.
///
/// # Errors
///
/// [`ExtractError::NoCompetitionFile`] when `competitions` has no tournaments.
pub(crate) fn draws(
    competitions: &BTreeMap<String, CompetitionsDocument>,
) -> Result<Draws, ExtractError> {
    let Some(CompetitionsFile::Tournaments(file)) =
        competitions.get(TOURNAMENTS).map(|document| &document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(TOURNAMENTS));
    };
    let own_draws = [WORLD_CUP, EUROPEAN_CHAMPIONSHIPS]
        .into_iter()
        .map(|(comp_ids, first_stage)| OwnDraw {
            competitions: comp_ids
                .iter()
                .flat_map(|&comp_id| {
                    file.competitions
                        .iter()
                        .filter(move |tournament| tournament.comp.comp_id.byte() == comp_id)
                        .map(|tournament| tournament.id.clone())
                })
                .collect(),
            first_stage,
        })
        .filter(|draw| !draw.competitions.is_empty())
        .collect();
    Ok(Draws { own_draws })
}

#[cfg(test)]
mod tests {
    use openswos_common::Importer;
    use openswos_competitions_json_resource::{CompetitionsJsonFormat, SymbolId};

    use super::*;

    /// Tournaments of the ids 22, 4 and 0.
    fn competitions() -> BTreeMap<String, CompetitionsDocument> {
        let record = |id: &str, comp_id: u8| {
            format!(
                r#"{{"id": "{id}", "type": "tournament", "comp": {{"comp_id": {comp_id}, "type": 2, "team_number": -1, "begin_month": 64, "end_month": 32, "name_table_offset": 35, "reserved": 0, "team_list_offset": 41, "seed_reorder_flag": 1}},
                 "seeding_type": 1, "menu_state": 0, "play_each_team": 1, "away_goals_mode": 3, "substitutes": 3, "points_for_win": 5, "round_string_base": 1,
                 "rounds": [], "header_reserved": 0, "round_rule_flags": [], "name": [], "team_seed_list": []}}"#
            )
        };
        let text = format!(
            r#"{{"type": "competition_tournaments", "count": 3, "competitions": [{}, {}, {}]}}"#,
            record("_europeanChampionships", 22),
            record("_worldCup", 4),
            record("_euroCup", 0)
        );
        let document = CompetitionsJsonFormat::import(text.as_bytes()).expect("tournaments");
        BTreeMap::from([(TOURNAMENTS.to_owned(), document)])
    }

    #[test]
    fn the_own_draws_are_the_tournaments_of_their_ids() {
        let draws = draws(&competitions()).expect("draws");
        let id = |text: &str| SymbolId(text.to_owned());
        assert_eq!(
            draws.own_draws,
            [
                OwnDraw {
                    competitions: vec![id("_worldCup")],
                    first_stage: 24,
                },
                OwnDraw {
                    competitions: vec![id("_europeanChampionships")],
                    first_stage: 16,
                },
            ]
        );
    }

    #[test]
    fn a_world_without_tournaments_is_refused() {
        let error = draws(&BTreeMap::new()).expect_err("no tournaments");
        assert!(matches!(
            error,
            ExtractError::NoCompetitionFile(TOURNAMENTS)
        ));
    }
}
