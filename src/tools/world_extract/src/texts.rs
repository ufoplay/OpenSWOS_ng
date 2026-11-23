//! A world's texts and classes of competition (docs/PLAN.md M5 #4b2, #4c): `rules/strings.json`,
//! the compiled strings its records' names name and the names a league, a cup and a tournament
//! fall back on, read
//! from upstream's compiled strings through the oracle; `rules/competition_classes.json`, the
//! national teams' competitions and the continental suspensions' contests, from upstream's
//! lists of records and the ids its code names.

use std::collections::BTreeMap;

use openswos_competitions_json_resource::{
    CareerFile, CompetitionsDocument, CompetitionsFile, Names, SymbolId,
};
use openswos_oracle::tables::{CompetitionLists, CompiledString, DefaultNames, ListedRecord};
use openswos_world_rules_resource::{CompetitionClasses, Strings};

use crate::error::ExtractError;

/// The contests upstream's `continental_record` (diy_competition.c) names by id besides its
/// list: the Copa Libertadores (include/comp_identity.h `COMPID_copaLibertadores`), the UEFA
/// qualifying ties (`COMPID_uefaTieChampionsLeague` to `COMPID_uefaTieConferenceLeague`), the
/// other confederations' tie and main round (`COMPID_contiTie`, `COMPID_contiMain`). A world's
/// records of them join its class. A Super Cup's tie (`COMPID_worldSuperCup`) joins it only
/// while the UEFA Super Cup is played (`SuperCup_UefaTie`): World 2026-27's component (M5 #9).
const CONTINENTAL_COMP_IDS: [u8; 6] = [162, 239, 240, 241, 242, 243];

/// The career index's file.
const CAREER: &str = "career.json";

/// A name's text: ASCII, as upstream's compiled strings are.
///
/// # Errors
///
/// [`ExtractError::NotAscii`] for bytes beyond ASCII.
pub(crate) fn text(bytes: &[u8]) -> Result<String, ExtractError> {
    if bytes.is_ascii() {
        Ok(bytes.iter().copied().map(char::from).collect())
    } else {
        Err(ExtractError::NotAscii(bytes.to_vec()))
    }
}

/// A record of `competitions`: its competition id, its id and its names.
struct Record<'a> {
    /// Its `compId`.
    comp_id: u8,
    /// Its id.
    id: &'a SymbolId,
    /// Its names.
    names: &'a Names,
}

/// Every league, cup, tournament and play-off of `competitions`.
fn records(competitions: &BTreeMap<String, CompetitionsDocument>) -> Vec<Record<'_>> {
    let mut records = Vec::new();
    for document in competitions.values() {
        match &document.file {
            CompetitionsFile::Leagues(file) => {
                records.extend(file.leagues.iter().map(|record| Record {
                    comp_id: record.comp.comp_id.byte(),
                    id: &record.id,
                    names: &record.name,
                }));
            }
            CompetitionsFile::Cups(file) => {
                records.extend(file.competitions.iter().map(|record| Record {
                    comp_id: record.comp.comp_id.byte(),
                    id: &record.id,
                    names: &record.name,
                }));
            }
            CompetitionsFile::Tournaments(file) => {
                records.extend(file.competitions.iter().map(|record| Record {
                    comp_id: record.comp.comp_id.byte(),
                    id: &record.id,
                    names: &record.name,
                }));
            }
            CompetitionsFile::Playoffs(file) => {
                records.extend(file.playoffs.iter().map(|record| Record {
                    comp_id: record.comp.comp_id.byte(),
                    id: &record.id,
                    names: &record.name,
                }));
            }
            CompetitionsFile::Countries(_)
            | CompetitionsFile::Qualifying(_)
            | CompetitionsFile::LeagueBlocks(_)
            | CompetitionsFile::Career(_)
            | CompetitionsFile::CountryCompetitions(_)
            | CompetitionsFile::SeedBuckets(_) => {}
        }
    }
    records
}

/// `rules/strings.json`: the text of every compiled string of `symbols` (upstream's, by name)
/// a name of `competitions` names, and the names a league, a cup and a tournament fall back on
/// (`defaults`),
/// each by its symbol when it has one, else as its text.
///
/// # Errors
///
/// [`ExtractError::NotAscii`] for a text beyond ASCII.
pub(crate) fn strings(
    competitions: &BTreeMap<String, CompetitionsDocument>,
    symbols: &[(String, Vec<u8>)],
    defaults: &DefaultNames,
) -> Result<Strings, ExtractError> {
    let compiled: BTreeMap<&str, &[u8]> = symbols
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_slice()))
        .collect();
    let mut used = BTreeMap::new();
    let records = records(competitions);
    let names = records
        .iter()
        .flat_map(|record| record.names.iter().flatten());
    for name in names {
        if let Some(bytes) = compiled.get(name.as_str()) {
            used.insert(name.clone(), text(bytes)?);
        }
    }
    let mut default = |string: &CompiledString| match &string.symbol {
        Some(symbol) => {
            used.insert(symbol.clone(), text(&string.text)?);
            Ok(symbol.clone())
        }
        None => text(&string.text),
    };
    let league_reset_name = default(&defaults.league_reset)?;
    let cup_reset_name = default(&defaults.cup_reset)?;
    let division_names = defaults
        .divisions
        .iter()
        .map(&mut default)
        .collect::<Result<_, ExtractError>>()?;
    let cup_name = default(&defaults.cup)?;
    let tournament_reset_name = default(&defaults.tournament_reset)?;
    let tournament_name = default(&defaults.tournament)?;
    let cup_round_names = defaults
        .cup_rounds
        .iter()
        .map(&mut default)
        .collect::<Result<_, ExtractError>>()?;
    Ok(Strings {
        symbols: used,
        league_reset_name,
        cup_reset_name,
        tournament_reset_name,
        division_names,
        cup_name,
        tournament_name,
        cup_round_names,
    })
}

/// `rules/competition_classes.json`: upstream's `lists`, each record by its id in
/// `competitions` and each career copy by the name `career.json` gives it, the records of
/// `continental_record`'s ids after the continental list's.
///
/// # Errors
///
/// When a listed record's id is no record's or several's, a listed copy is of no career
/// contest, or `competitions` has no career index.
pub(crate) fn classes(
    competitions: &BTreeMap<String, CompetitionsDocument>,
    lists: &CompetitionLists,
) -> Result<CompetitionClasses, ExtractError> {
    let Some(CompetitionsFile::Career(career)) =
        competitions.get(CAREER).map(|document| &document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(CAREER));
    };
    let mut by_comp_id = BTreeMap::new();
    for record in records(competitions) {
        if by_comp_id.insert(record.comp_id, record.id).is_some() {
            return Err(ExtractError::RepeatedCompId(record.comp_id));
        }
    }
    let named = |listed: &[ListedRecord]| {
        listed
            .iter()
            .map(|&record| listed_id(record, &by_comp_id, career))
            .collect::<Result<Vec<_>, ExtractError>>()
    };
    let national_teams = named(&lists.national_teams)?;
    let mut continental = named(&lists.continental)?;
    continental.extend(continental_records(competitions));
    Ok(CompetitionClasses {
        national_teams,
        continental,
    })
}

/// The records of `competitions` with an id `continental_record` names, in the ids' order.
pub(crate) fn continental_records(
    competitions: &BTreeMap<String, CompetitionsDocument>,
) -> Vec<SymbolId> {
    let records = records(competitions);
    CONTINENTAL_COMP_IDS
        .iter()
        .flat_map(|&comp_id| {
            records
                .iter()
                .filter(move |record| record.comp_id == comp_id)
                .map(|record| record.id.clone())
        })
        .collect()
}

/// The id of a listed record: its record's, or the name of the copy `career` makes of it.
fn listed_id(
    record: ListedRecord,
    by_comp_id: &BTreeMap<u8, &SymbolId>,
    career: &CareerFile,
) -> Result<SymbolId, ExtractError> {
    let template = *by_comp_id
        .get(&record.comp_id)
        .ok_or(ExtractError::NoRecordOfId(record.comp_id))?;
    if !record.copy {
        return Ok(template.clone());
    }
    career
        .contests
        .iter()
        .find(|contest| contest.competition.as_ref() == Some(template))
        .and_then(|contest| contest.competition_copy.clone())
        .ok_or_else(|| ExtractError::NoCareerCopy(template.as_str().to_owned()))
}

#[cfg(test)]
mod tests {
    use openswos_common::Importer;
    use openswos_competitions_json_resource::CompetitionsJsonFormat;

    use super::*;

    /// A package's competitions of three records and a career index of one contest.
    fn competitions() -> BTreeMap<String, CompetitionsDocument> {
        let files = [
            (
                "leagues.json",
                r#"{"type": "competition_leagues", "count": 1, "leagues": [
                    {"id": "_albanianLeague", "comp": {"comp_id": 37, "type": 0, "team_number": 0, "begin_month": 64, "end_month": 32, "name_table_offset": 0, "reserved": 0, "team_list_offset": 0, "seed_reorder_flag": 0},
                     "divisions_count": 1, "rounds": 2, "points_for_win": 3, "substitutes": 3, "max_subs": 5,
                     "divisions": [{"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0}],
                     "reserved1": 0, "name": ["_aSuperliga", "LITERAL"]}]}"#,
            ),
            (
                "cups.json",
                r#"{"type": "competition_cups", "count": 1, "competitions": [
                    {"id": "_friendly", "type": "cup", "comp": {"comp_id": 20, "type": 0, "team_number": -1, "begin_month": 0, "end_month": 0, "name_table_offset": 15, "reserved": 0, "team_list_offset": 21, "seed_reorder_flag": 0},
                     "num_rounds_override": 0, "teams": 2, "away_goals_mode": 1, "substitutes": 3, "points_for_win": 3, "round_string_base": 0,
                     "round_rule_flags": [], "name": [null, "_aFriendly"], "team_seed_list": []}]}"#,
            ),
            (
                "tournaments.json",
                r#"{"type": "competition_tournaments", "count": 1, "competitions": [
                    {"id": "_copaLibertadores", "type": "tournament", "comp": {"comp_id": 162, "type": 2, "team_number": -1, "begin_month": 64, "end_month": 32, "name_table_offset": 35, "reserved": 0, "team_list_offset": 41, "seed_reorder_flag": 1},
                     "seeding_type": 1, "menu_state": 1, "play_each_team": 2, "away_goals_mode": 3, "substitutes": 3, "points_for_win": 5, "round_string_base": 4,
                     "rounds": [], "header_reserved": 0, "round_rule_flags": [], "name": [], "team_seed_list": []}]}"#,
            ),
            (
                "career.json",
                r#"{"type": "career_index", "count": 1, "contests": [
                    {"competition": "_friendly", "competition_copy": "_friendlyCopy"}]}"#,
            ),
        ];
        files
            .into_iter()
            .map(|(name, text)| {
                let document = CompetitionsJsonFormat::import(text.as_bytes())
                    .unwrap_or_else(|error| panic!("{name}: {error}"));
                (name.to_owned(), document)
            })
            .collect()
    }

    fn compiled(symbol: Option<&str>, text: &[u8]) -> CompiledString {
        CompiledString {
            symbol: symbol.map(str::to_owned),
            text: text.to_vec(),
        }
    }

    #[test]
    fn the_strings_are_the_symbols_the_names_name_and_the_defaults() {
        let symbols = [
            ("_aSuperliga", &b"SUPERLIGA"[..]),
            ("_aFriendly", b"FRIENDLY"),
            ("_aUnused", b"UNUSED"),
            ("_aDiyLeague", b"DIY LEAGUE"),
            ("_aDivision1_0", b"DIVISION 1"),
            ("_aCup_0", b"CUP"),
        ]
        .map(|(name, text)| (name.to_owned(), text.to_vec()));
        let defaults = DefaultNames {
            league_reset: compiled(Some("_aDiyLeague"), b"DIY LEAGUE"),
            cup_reset: compiled(None, b"DIY CUP"),
            divisions: vec![
                compiled(Some("_aDivision1_0"), b"DIVISION 1"),
                compiled(None, b"NOT LISTED"),
            ],
            cup: compiled(Some("_aCup_0"), b"CUP"),
            tournament_reset: compiled(None, b"DIY TOURNAMENT"),
            tournament: compiled(None, b"LEAGUE"),
            cup_rounds: vec![compiled(None, b"FINAL")],
        };
        let strings = strings(&competitions(), &symbols, &defaults).expect("strings");
        let listed: Vec<(&str, &str)> = strings
            .symbols
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str()))
            .collect();
        assert_eq!(
            listed,
            [
                ("_aCup_0", "CUP"),
                ("_aDivision1_0", "DIVISION 1"),
                ("_aDiyLeague", "DIY LEAGUE"),
                ("_aFriendly", "FRIENDLY"),
                ("_aSuperliga", "SUPERLIGA"),
            ]
        );
        assert_eq!(strings.league_reset_name, "_aDiyLeague");
        assert_eq!(strings.cup_reset_name, "DIY CUP", "not listed: its text");
        assert_eq!(strings.division_names, ["_aDivision1_0", "NOT LISTED"]);
        assert_eq!(strings.cup_name, "_aCup_0");
        assert_eq!(strings.tournament_reset_name, "DIY TOURNAMENT");
        assert_eq!(strings.tournament_name, "LEAGUE");
        assert_eq!(strings.cup_round_names, ["FINAL"]);
    }

    #[test]
    fn a_compiled_text_beyond_ascii_is_refused() {
        let symbols = [("_aSuperliga".to_owned(), vec![0xC9])];
        let defaults = DefaultNames {
            league_reset: compiled(None, b""),
            cup_reset: compiled(None, b""),
            divisions: Vec::new(),
            cup: compiled(None, b""),
            tournament_reset: compiled(None, b""),
            tournament: compiled(None, b""),
            cup_rounds: Vec::new(),
        };
        let error = strings(&competitions(), &symbols, &defaults).expect_err("not ASCII");
        assert!(matches!(error, ExtractError::NotAscii(bytes) if bytes == [0xC9]));
    }

    #[test]
    fn the_classes_name_records_and_career_copies_by_id() {
        let record = |comp_id, copy| ListedRecord { comp_id, copy };
        let lists = CompetitionLists {
            national_teams: vec![record(20, true)],
            continental: vec![record(20, false), record(37, false)],
        };
        let classes = classes(&competitions(), &lists).expect("classes");
        fn ids(ids: &[SymbolId]) -> Vec<&str> {
            ids.iter().map(SymbolId::as_str).collect()
        }
        assert_eq!(ids(&classes.national_teams), ["_friendlyCopy"]);
        // The Copa after the list: `continental_record` names it by id.
        assert_eq!(
            ids(&classes.continental),
            ["_friendly", "_albanianLeague", "_copaLibertadores"]
        );
    }

    #[test]
    fn a_listed_record_the_package_lacks_is_refused() {
        let record = |comp_id, copy| ListedRecord { comp_id, copy };
        let lists = |listed| CompetitionLists {
            national_teams: vec![listed],
            continental: Vec::new(),
        };
        let error = classes(&competitions(), &lists(record(99, false))).expect_err("no record");
        assert!(matches!(error, ExtractError::NoRecordOfId(99)), "{error}");
        let error = classes(&competitions(), &lists(record(37, true))).expect_err("no copy");
        assert!(
            matches!(&error, ExtractError::NoCareerCopy(id) if id == "_albanianLeague"),
            "{error}"
        );
    }
}
