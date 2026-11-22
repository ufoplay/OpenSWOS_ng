//! `rules/strings.json`: the texts the world's names refer to (upstream's compiled strings by
//! their symbols' names, src/string_symbols.c), and the names a league, a cup and a tournament
//! fall back on.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/strings.json`. A name of the world (a record's `name`, a default here) is the text
/// of the symbol it names, or itself when it names none (upstream save_format_json.c
/// `dj_intern_names`). A text is one character a byte (Latin-1).
///
/// ```json
/// {
///   "symbols": {
///     "_aDivision1": "DIVISION 1",
///     "_aDiyLeague": "DIY LEAGUE"
///   },
///   "league_reset_name": "_aDiyLeague",
///   "cup_reset_name": "_aDiyCup",
///   "tournament_reset_name": "_aDiyTournament",
///   "division_names": ["_aPremierDivisio", "_aDivision1_0", "_aDivision2_0", "_aDivision3_0"],
///   "cup_name": "_aCup_0",
///   "tournament_name": "_aLeague_1"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strings {
    /// Each symbol's text, by the symbol's name.
    pub symbols: BTreeMap<String, String>,
    /// The name a league's reset writes first (upstream diy_competition.c
    /// `DIY_InitCompetitionDefaults`: `_aDiyLeague`); what a shorter name leaves of it stays.
    pub league_reset_name: String,
    /// The name a cup's reset writes first (upstream `InitDIYCup`: `_aDiyCup`), the same way.
    pub cup_reset_name: String,
    /// The name a tournament's reset writes first (upstream `InitDIYTournament`:
    /// `_aDiyTournament`), the same way.
    pub tournament_reset_name: String,
    /// Each division's name, by index, for a league whose record names none (upstream
    /// `_divisionStringTable`).
    pub division_names: Vec<String>,
    /// A cup's name when its record names none (upstream `InitDIYCupFromRecord`: `_aCup_0`).
    pub cup_name: String,
    /// A tournament's name when its record names none (upstream `InitDIYLeagueFromRecord`:
    /// `_aLeague_1`).
    pub tournament_name: String,
}

impl RulesFile for Strings {
    const FILE_NAME: &'static str = "strings.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    /// A file of two symbols, as this crate writes it.
    const TEXT: &str = "{\n  \"symbols\": {\n    \"_aDivision1\": \"DIVISION 1\",\n    \
        \"_aDiyLeague\": \"DIY LEAGUE\"\n  },\n  \"league_reset_name\": \"_aDiyLeague\",\n  \
        \"cup_reset_name\": \"DIY CUP\",\n  \"tournament_reset_name\": \"DIY TOURNAMENT\",\n  \
        \"division_names\": [\n    \"_aDivision1\",\n    \"DIVISION 2\"\n  ],\n  \
        \"cup_name\": \"CUP\",\n  \"tournament_name\": \"LEAGUE\"\n}\n";

    #[test]
    fn it_reads_and_writes_back() {
        let strings: Strings = read(TEXT.as_bytes()).expect("strings");
        assert_eq!(
            strings.symbols.get("_aDivision1").map(String::as_str),
            Some("DIVISION 1")
        );
        assert_eq!(strings.league_reset_name, "_aDiyLeague");
        assert_eq!(strings.cup_reset_name, "DIY CUP");
        assert_eq!(strings.division_names, ["_aDivision1", "DIVISION 2"]);
        assert_eq!(strings.cup_name, "CUP");
        assert_eq!(strings.tournament_reset_name, "DIY TOURNAMENT");
        assert_eq!(strings.tournament_name, "LEAGUE");
        assert_eq!(write(&strings), TEXT.as_bytes());
    }

    #[test]
    fn the_symbols_are_written_in_order() {
        let strings = Strings {
            symbols: [("_b", "B"), ("_a", "A")]
                .map(|(name, text)| (name.to_owned(), text.to_owned()))
                .into(),
            league_reset_name: String::new(),
            cup_reset_name: String::new(),
            tournament_reset_name: String::new(),
            division_names: Vec::new(),
            cup_name: String::new(),
            tournament_name: String::new(),
        };
        let text = String::from_utf8(write(&strings)).expect("ASCII");
        assert!(text.find("\"_a\"") < text.find("\"_b\""), "{text}");
    }

    #[test]
    fn a_missing_key_does_not_read() {
        let error = read::<Strings>(
            br#"{"symbols": {}, "league_reset_name": "", "cup_reset_name": "", "tournament_reset_name": "", "division_names": [], "cup_name": ""}"#,
        )
        .expect_err("no tournament name");
        assert!(error.to_string().contains("tournament_name"), "{error}");
    }
}
