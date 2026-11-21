//! `rules/fixtures.json`: the world's round robins, the order a league or a tournament's group
//! plays its matches in (upstream `_fixtureSchedulePtrTable`, src/fixture_data.c; a world's
//! odd sizes rewritten as real rounds, src/fixture_rounds.c), the competitions that play their
//! own, and whether a tournament's groups may be odd-sized.

use openswos_competitions_json_resource::SymbolId;
use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/fixtures.json`. Every package states its round robins: the engine has none of its
/// own, so no world's order is the engine's.
///
/// ```json
/// {
///   "odd_tournament_groups": false,
///   "schedules": [
///     { "teams": 2, "rounds": true, "pairs": [[0, 1]] },
///     …
///   ],
///   "competition_schedules": [
///     {
///       "competitions": ["_europeanChampionships2", "_europeanChampionships"],
///       "schedule": { "teams": 4, "rounds": true, "pairs": [[1, 0], [3, 2], …] }
///     }
///   ]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixtures {
    /// Whether a tournament's groups may hold an odd number of clubs (upstream
    /// diy_competition.c `InitDIYLeagueFromRecord` refuses them, panic 0x1313, in the world
    /// that does not allow them).
    pub odd_tournament_groups: bool,
    /// The round robin of each number of clubs, a size once.
    pub schedules: Vec<Schedule>,
    /// The competitions that play a round robin of their own instead of their size's
    /// (upstream diy_competition.c `getCurrentFixtureTeamPtrs`: the European Championships'
    /// groups play `_euroChampFixtureData`).
    pub competition_schedules: Vec<CompetitionSchedule>,
}

/// A round robin: the pairs of one cycle, in the order they are played. A second cycle plays
/// them again, each the other way round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    /// Its clubs.
    pub teams: u8,
    /// Whether it plays in real rounds, every club at most once a match day of `teams / 2`
    /// pairs (upstream `FixtureRounds_Real`): a career league of this size can be played. The
    /// original's odd tables are not (its career league refuses them, panic 0x1313).
    pub rounds: bool,
    /// Its pairs, each `[home, away]` as places among its clubs.
    pub pairs: Vec<[u8; 2]>,
}

/// A round robin some competitions play instead of their size's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompetitionSchedule {
    /// The competitions, by id.
    pub competitions: Vec<SymbolId>,
    /// Their round robin.
    pub schedule: Schedule,
}

impl RulesFile for Fixtures {
    const FILE_NAME: &'static str = "fixtures.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    /// A file with one schedule and one competition's, as this crate writes it.
    const TEXT: &str = "{\n  \"odd_tournament_groups\": true,\n  \"schedules\": [\n    {\n      \
        \"teams\": 2,\n      \"rounds\": true,\n      \"pairs\": [\n        [\n          0,\n          \
        1\n        ]\n      ]\n    }\n  ],\n  \"competition_schedules\": [\n    {\n      \
        \"competitions\": [\n        \"_cup\"\n      ],\n      \"schedule\": {\n        \
        \"teams\": 2,\n        \"rounds\": false,\n        \"pairs\": [\n          [\n            \
        1,\n            0\n          ]\n        ]\n      }\n    }\n  ]\n}\n";

    #[test]
    fn it_reads_and_writes_back() {
        let fixtures: Fixtures = read(TEXT.as_bytes()).expect("fixtures");
        assert_eq!(
            fixtures,
            Fixtures {
                odd_tournament_groups: true,
                schedules: vec![Schedule {
                    teams: 2,
                    rounds: true,
                    pairs: vec![[0, 1]],
                }],
                competition_schedules: vec![CompetitionSchedule {
                    competitions: vec![SymbolId("_cup".to_owned())],
                    schedule: Schedule {
                        teams: 2,
                        rounds: false,
                        pairs: vec![[1, 0]],
                    },
                }],
            }
        );
        assert_eq!(write(&fixtures), TEXT.as_bytes());
    }

    #[test]
    fn a_pair_is_two_places() {
        assert!(
            read::<Fixtures>(
                br#"{"odd_tournament_groups": false, "schedules": [{"teams": 3, "rounds": false,
                    "pairs": [[0, 1, 2]]}], "competition_schedules": []}"#,
            )
            .is_err(),
            "three places"
        );
        let error = read::<Fixtures>(br#"{"odd_tournament_groups": false, "schedules": []}"#)
            .expect_err("no competition schedules");
        assert!(
            error.to_string().contains("competition_schedules"),
            "{error}"
        );
    }
}
