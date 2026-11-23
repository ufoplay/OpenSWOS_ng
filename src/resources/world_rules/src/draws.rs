//! `rules/draws.json`: the draws a world states for its tournaments apart from the engine's
//! own (upstream diy_competition.c draws some tournaments' stages by its own tables, and ranks
//! their first stage's qualifiers its own way, picked by competition id).

use openswos_competitions_json_resource::SymbolId;
use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/draws.json`.
///
/// ```json
/// {
///   "own_draws": [
///     {
///       "competitions": ["_europeanChampionships"],
///       "ranking": "by_place",
///       "stages": [{"keep": 16}, {"order": [1, 4, 5, 0, 2, 7, 6, 3]}, {"keep": 2}]
///     }
///   ]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draws {
    /// The tournaments whose stages are drawn by their own tables, not the engine's draw
    /// (upstream `buildCompSpecificTeamList`, `buildCompSpecificTeamShuffle`).
    pub own_draws: Vec<OwnDraw>,
}

/// Tournaments drawn by the same own tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnDraw {
    /// The tournaments, by id.
    pub competitions: Vec<SymbolId>,
    /// How a stage of groups ranks the teams it sends on.
    pub ranking: Ranking,
    /// Each stage's draw, from the first: a stage past the list is drawn as its last.
    pub stages: Vec<StageDraw>,
}

/// How a stage of groups ranks the teams it sends on, the next stage's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ranking {
    /// The engine's: every stage of groups by place (the groups' winners first, in the groups'
    /// order), each place's teams by their record, a place that only some of its teams go
    /// through on by their record, then by their group (upstream
    /// `DIY_BuildSeededBracketAndUpdateScores`).
    ByPlace,
    /// The first stage only, group by group (a group's winner, its runner-up, then the next
    /// group's), then the best of the place some go through on by their record, the four
    /// ranked by their group; a later stage's tables kept as they are (upstream
    /// `DIY_AdjustWorldCupBracketScores`).
    ByGroup,
}

/// A stage's own draw: the order its teams take, from the order the stage before left (the
/// seeds', or the ranking's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageDraw {
    /// The first places kept in their order (upstream copies them as they are): the stage's
    /// teams are no more.
    Keep(u8),
    /// Each place the team of that place before (upstream's shuffle tables): the stage's
    /// teams are no more than the places.
    Order(Vec<u8>),
    /// The order set by the groups the best placed teams come from (upstream
    /// `_diyGroupPatternLookup`, `_diyLeagueTeamShuffleMap`).
    BestPlaced(BestPlaced),
}

/// A stage drawn by where the best placed teams of the stage of groups before come from: the
/// teams that went through on their place first, then the best placed, as the ranking leaves
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BestPlaced {
    /// The first place of the best placed: as many follow as a pattern orders.
    pub first: u8,
    /// The patterns, the first that matches taken: each a set of groups the best placed come
    /// from, and their order then.
    pub patterns: Vec<Pattern>,
    /// Each place of the stage: the team of a place before, or (`null`) the next best placed
    /// in the matched pattern's order.
    pub places: Vec<Option<u8>>,
}

/// The groups the best placed come from, and their order then.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pattern {
    /// The groups, from 0 (upstream's bits of a byte: 0 to 7).
    pub groups: Vec<u8>,
    /// The best placed in the order the stage's places take them, each by its rank among
    /// them.
    pub order: Vec<u8>,
}

impl RulesFile for Draws {
    const FILE_NAME: &'static str = "draws.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    /// A file of one own draw, as this crate writes it.
    const TEXT: &str = "{\n  \"own_draws\": [\n    {\n      \"competitions\": [\n        \
        \"_worldCup\"\n      ],\n      \"ranking\": \"by_group\",\n      \"stages\": [\n        \
        {\n          \"keep\": 24\n        },\n        {\n          \"best_placed\": {\n            \
        \"first\": 12,\n            \"patterns\": [\n              {\n                \
        \"groups\": [\n                  0,\n                  1\n                ],\n                \
        \"order\": [\n                  1,\n                  0\n                ]\n              \
        }\n            ],\n            \"places\": [\n              null,\n              3,\n              \
        null,\n              5\n            ]\n          }\n        },\n        {\n          \
        \"order\": [\n            1,\n            0\n          ]\n        }\n      ]\n    }\n  ]\n}\n";

    #[test]
    fn it_reads_and_writes_back() {
        let draws: Draws = read(TEXT.as_bytes()).expect("draws");
        assert_eq!(
            draws.own_draws,
            [OwnDraw {
                competitions: vec![SymbolId("_worldCup".to_owned())],
                ranking: Ranking::ByGroup,
                stages: vec![
                    StageDraw::Keep(24),
                    StageDraw::BestPlaced(BestPlaced {
                        first: 12,
                        patterns: vec![Pattern {
                            groups: vec![0, 1],
                            order: vec![1, 0],
                        }],
                        places: vec![None, Some(3), None, Some(5)],
                    }),
                    StageDraw::Order(vec![1, 0]),
                ],
            }]
        );
        assert_eq!(String::from_utf8(write(&draws)).expect("text"), TEXT);
    }

    #[test]
    fn a_draw_without_its_stages_or_of_an_unknown_ranking_does_not_read() {
        let error =
            read::<Draws>(br#"{"own_draws": [{"competitions": [], "ranking": "by_place"}]}"#)
                .expect_err("no stages");
        assert!(error.to_string().contains("stages"), "{error}");
        let error = read::<Draws>(
            br#"{"own_draws": [{"competitions": [], "ranking": "by_luck", "stages": []}]}"#,
        )
        .expect_err("no such ranking");
        assert!(error.to_string().contains("by_luck"), "{error}");
        let error = read::<Draws>(
            br#"{"own_draws": [{"competitions": [], "ranking": "by_place", "stages": [{"shuffle": 2}]}]}"#,
        )
        .expect_err("no such draw");
        assert!(error.to_string().contains("shuffle"), "{error}");
    }
}
