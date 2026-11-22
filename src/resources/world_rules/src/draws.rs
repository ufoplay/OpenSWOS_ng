//! `rules/draws.json`: the draws a world states for its competitions apart from the engine's
//! own (upstream diy_competition.c draws some tournaments' stages by its own tables, picked by
//! competition id).

use openswos_competitions_json_resource::SymbolId;
use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/draws.json`.
///
/// ```json
/// {
///   "own_draws": [
///     {"competitions": ["_worldCup"], "first_stage": 24},
///     {"competitions": ["_europeanChampionships2", "_europeanChampionships"], "first_stage": 16}
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
    /// The places a first stage keeps in their order (upstream copies them as they are): its
    /// teams are no more. The later stages' tables come with their loads (M5 #7).
    pub first_stage: u8,
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
        \"_worldCup\"\n      ],\n      \"first_stage\": 24\n    }\n  ]\n}\n";

    #[test]
    fn it_reads_and_writes_back() {
        let draws: Draws = read(TEXT.as_bytes()).expect("draws");
        assert_eq!(
            draws.own_draws,
            [OwnDraw {
                competitions: vec![SymbolId("_worldCup".to_owned())],
                first_stage: 24,
            }]
        );
        assert_eq!(write(&draws), TEXT.as_bytes());
    }

    #[test]
    fn a_draw_without_its_first_stage_does_not_read() {
        let error =
            read::<Draws>(br#"{"own_draws": [{"competitions": []}]}"#).expect_err("no first stage");
        assert!(error.to_string().contains("first_stage"), "{error}");
    }
}
