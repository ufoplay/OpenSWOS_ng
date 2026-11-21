//! `rules/fixtures.json`: which fixture behaviours the world plays (docs/WORLD_INVENTORY.md
//! §5, `OddRoundRobin`).

use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/fixtures.json`. Every package names its behaviours: there is no default, so no
/// world's behaviour is the engine's.
///
/// ```json
/// {
///   "odd_round_robin": "original_pair_tables",
///   "odd_tournament_groups": false
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixtures {
    /// How a league with an odd number of clubs gets its fixtures.
    pub odd_round_robin: OddRoundRobin,
    /// Whether a tournament's groups may hold an odd number of clubs (upstream
    /// diy_competition.c `InitDIYLeagueFromRecord` refuses them, panic 0x1313, in the world
    /// that does not allow them).
    pub odd_tournament_groups: bool,
}

/// How a league with an odd number of clubs gets its fixtures: a behaviour component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OddRoundRobin {
    /// The original's pair tables (upstream `_fixtureSchedulePtrTable`). Faithful to the
    /// original: they are not real rounds, and a career league refuses them (panic 0x1313).
    OriginalPairTables,
    /// Real rounds, a club resting each day (upstream fixture_rounds.c,
    /// `FixtureRounds_Install`).
    RoundsWithRest,
}

impl RulesFile for Fixtures {
    const FILE_NAME: &'static str = "fixtures.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    #[test]
    fn it_reads_and_writes_back() {
        let text = "{\n  \"odd_round_robin\": \"rounds_with_rest\",\n  \"odd_tournament_groups\": \
                    true\n}\n";
        let fixtures: Fixtures = read(text.as_bytes()).expect("fixtures");
        assert_eq!(
            fixtures,
            Fixtures {
                odd_round_robin: OddRoundRobin::RoundsWithRest,
                odd_tournament_groups: true
            }
        );
        assert_eq!(write(&fixtures), text.as_bytes());
    }

    #[test]
    fn an_unknown_behaviour_does_not_read() {
        let error =
            read::<Fixtures>(br#"{"odd_round_robin": "shuffled", "odd_tournament_groups": false}"#)
                .expect_err("no such behaviour");
        assert!(error.to_string().contains("shuffled"), "{error}");
    }
}
