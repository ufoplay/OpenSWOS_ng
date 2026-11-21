//! `competitions/bteams.json`: a world's B teams and the call-up rules of their countries
//! (upstream src/bteam.c `load`, the B-team call-ups of a world's career; written by
//! `scripts/world_import.py` `bteams` from the SWOS 96/97 Mod's `DATA\BAGE.nnn`).
//!
//! ```json
//! { "rules": [ { "over_age_cap": 4, "u23": 23, "gk_any_age": true, "windows": [[48, 64], [96, 104]] } ],
//!   "teams": [ { "b": 5164, "parent": 5125, "rules": 0 } ] }
//! ```
//!
//! Values are kept as written; what the game makes of them (the rules' defaults, the lists'
//! limits) is given by accessors. Rules are the career's (M7).

use openswos_common::TeamId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::json_file::JsonFile;

/// The over-age players a match may call up when a country's rules say nothing: no cap
/// (upstream `load`: `0xFF`).
pub const NO_OVER_AGE_CAP: u8 = 0xFF;

/// The age from which a player is over age when a country's rules say nothing (upstream
/// `load`: 23).
pub const DEFAULT_UNDER_AGE: u8 = 23;

/// The most rules the game reads (upstream `MAX_RULES`); later ones are ignored.
pub const MAX_RULES: usize = 16;

/// The most windows of a country the game reads (upstream `MAX_WINDOWS`).
pub const MAX_WINDOWS: usize = 4;

/// The most B teams the game reads (upstream `MAX_TEAMS`).
pub const MAX_TEAMS: usize = 64;

/// `bteams.json` with its layout.
pub type BTeamsDocument = JsonFile<BTeams>;

/// A world's B teams and their countries' call-up rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BTeams {
    /// The countries' rules, which the teams name by index.
    pub rules: Vec<CallUpRules>,
    /// The B teams.
    pub teams: Vec<BTeam>,
}

impl BTeams {
    /// The rules of `team`, if its index names some the game reads.
    #[must_use]
    pub fn rules_of(&self, team: &BTeam) -> Option<&CallUpRules> {
        self.rules
            .get(usize::from(team.rules))
            .filter(|_| usize::from(team.rules) < MAX_RULES)
    }
}

/// A country's call-up rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallUpRules {
    /// The most over-age players a match may call up; `None`: the key is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub over_age_cap: Option<u8>,
    /// The age from which a player is over age (by year of birth); `None`: absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u23: Option<u8>,
    /// Whether a goalkeeper of any age may play; `None`: absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gk_any_age: Option<bool>,
    /// The transfer windows a season-long call-up is made in; `None`: absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<Vec<CallUpWindow>>,
}

impl CallUpRules {
    /// The over-age cap the game plays with ([`NO_OVER_AGE_CAP`] when absent; 255 is none).
    #[must_use]
    pub fn effective_over_age_cap(&self) -> u8 {
        self.over_age_cap.unwrap_or(NO_OVER_AGE_CAP)
    }

    /// The age from which a player is over age, as the game plays it ([`DEFAULT_UNDER_AGE`]
    /// when absent).
    #[must_use]
    pub fn effective_under_age(&self) -> u8 {
        self.u23.unwrap_or(DEFAULT_UNDER_AGE)
    }

    /// Whether a goalkeeper of any age may play, as the game plays it (no when absent).
    #[must_use]
    pub fn effective_gk_any_age(&self) -> bool {
        self.gk_any_age.unwrap_or_default()
    }

    /// The windows the game reads: the first [`MAX_WINDOWS`].
    #[must_use]
    pub fn effective_windows(&self) -> &[CallUpWindow] {
        let windows = self.windows.as_deref().unwrap_or_default();
        windows.get(..MAX_WINDOWS).unwrap_or(windows)
    }
}

/// A transfer window: its first and last week of the career's calendar (eight a month from
/// January), both in it. Written `[first, last]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "[u8; 2]", into = "[u8; 2]")]
pub struct CallUpWindow {
    /// Its first week.
    pub first: u8,
    /// Its last week.
    pub last: u8,
}

impl From<[u8; 2]> for CallUpWindow {
    fn from([first, last]: [u8; 2]) -> Self {
        Self { first, last }
    }
}

impl From<CallUpWindow> for [u8; 2] {
    fn from(CallUpWindow { first, last }: CallUpWindow) -> Self {
        [first, last]
    }
}

/// A B team and its first team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BTeam {
    /// The B team.
    #[serde(with = "team_word")]
    pub b: TeamId,
    /// Its first team, whose manager calls its players up.
    #[serde(with = "team_word")]
    pub parent: TeamId,
    /// Its country's rules: an index into [`BTeams::rules`].
    pub rules: u8,
}

/// A team as the game packs it in a word: its file in the high byte, its ordinal in the low.
mod team_word {
    use super::{Deserialize, Deserializer, Serializer, TeamId};

    pub(super) fn serialize<S: Serializer>(
        team: &TeamId,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u16(u16::from_be_bytes(team.to_bytes()))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<TeamId, D::Error> {
        u16::deserialize(deserializer).map(|word| TeamId::from_bytes(word.to_be_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openswos_common::{TeamFileNumber, TeamOrdinal};

    /// World 2026-27's first rules and B team, as its package writes them.
    const TEXT: &str = "{\n  \"rules\": [\n    {\n      \"over_age_cap\": 4,\n      \"u23\": 23,\n      \"gk_any_age\": true,\n      \"windows\": [\n        [\n          48,\n          64\n        ],\n        [\n          96,\n          104\n        ]\n      ]\n    }\n  ],\n  \"teams\": [\n    {\n      \"b\": 5164,\n      \"parent\": 5125,\n      \"rules\": 0\n    }\n  ]\n}\n";

    #[test]
    fn reads_and_writes_the_packages_file() {
        let document = BTeamsDocument::read(TEXT.as_bytes()).expect("a file");
        let teams = &document.content;
        let [team] = teams.teams.as_slice() else {
            panic!("one B team");
        };
        // 5164 = 0x142C: file 20 (Italy), ordinal 44.
        assert_eq!(
            team.b,
            TeamId {
                file: TeamFileNumber(20),
                ordinal: TeamOrdinal(44)
            }
        );
        let rules = teams.rules_of(team).expect("its rules");
        assert_eq!(rules.effective_over_age_cap(), 4);
        assert_eq!(
            rules.effective_windows(),
            [
                CallUpWindow {
                    first: 48,
                    last: 64
                },
                CallUpWindow {
                    first: 96,
                    last: 104
                }
            ]
        );
        assert_eq!(document.to_bytes().expect("writes"), TEXT.as_bytes());
    }

    #[test]
    fn absent_rules_are_the_games_defaults() {
        let rules: CallUpRules = serde_json::from_str("{}").expect("rules");
        assert_eq!(rules.effective_over_age_cap(), NO_OVER_AGE_CAP);
        assert_eq!(rules.effective_under_age(), DEFAULT_UNDER_AGE);
        assert!(!rules.effective_gk_any_age());
        assert!(rules.effective_windows().is_empty());
        assert_eq!(serde_json::to_string(&rules).expect("writes"), "{}");
    }

    #[test]
    fn the_game_reads_four_windows_and_16_rules() {
        let window = CallUpWindow { first: 1, last: 2 };
        let rules = CallUpRules {
            over_age_cap: None,
            u23: None,
            gk_any_age: None,
            windows: Some(vec![window; MAX_WINDOWS + 1]),
        };
        assert_eq!(rules.effective_windows().len(), MAX_WINDOWS);
        let team = |rules| BTeam {
            b: TeamId::from_bytes([1, 2]),
            parent: TeamId::from_bytes([1, 0]),
            rules,
        };
        let teams = BTeams {
            rules: vec![rules; MAX_RULES + 1],
            teams: Vec::new(),
        };
        assert!(teams.rules_of(&team(15)).is_some());
        assert!(teams.rules_of(&team(16)).is_none(), "past the game's 16");
        assert!(teams.rules_of(&team(17)).is_none(), "no such rules");
    }

    #[test]
    fn other_keys_do_not_read() {
        assert!(serde_json::from_str::<BTeams>(r#"{"rules": [], "teams": [], "x": 1}"#).is_err());
        assert!(serde_json::from_str::<CallUpWindow>("[1]").is_err());
    }
}
