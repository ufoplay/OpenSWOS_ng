//! `assets/COMPETITIONS_JSON/`: upstream's ten competition files (`bin/data_json/
//! competitions/`). Each one round-trips byte for byte, is the file its name says, and
//! names only competitions the files define.

use std::collections::BTreeSet;

use openswos_common::json::TrailingNewline;
use openswos_competitions_json_resource::{
    CompCupEntry, CompetitionType, CompetitionsDocument, CompetitionsFile, CompetitionsJsonFormat,
    SymbolId,
};
use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::{assert_round_trip, fixtures};

/// The ten files and the `type` each one has.
const FILES: [(&str, &str); 10] = [
    ("career.json", "career_index"),
    ("countries.json", "countries"),
    ("country_competitions.json", "country_competitions"),
    ("cups.json", "competition_cups"),
    ("league_blocks.json", "league_blocks"),
    ("leagues.json", "competition_leagues"),
    ("playoffs.json", "playoffs"),
    ("qualifying.json", "qualifying_groups"),
    ("seed_buckets.json", "seed_buckets"),
    ("tournaments.json", "competition_tournaments"),
];

fn read(name: &str) -> CompetitionsFile {
    let path = assets_path().join("COMPETITIONS_JSON").join(name);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let document: CompetitionsDocument = assert_round_trip::<CompetitionsJsonFormat>(&bytes);
    assert_eq!(document.trailing_newline, TrailingNewline::Absent, "{name}");
    document.file
}

/// The `type` key of a file.
fn kind(file: &CompetitionsFile) -> String {
    serde_json::to_value(file)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|kind| kind.as_str().map(str::to_owned))
        })
        .expect("a type key")
}

#[test]
fn every_file_round_trips_and_is_what_its_name_says() {
    let found = fixtures("COMPETITIONS_JSON", |name| name.ends_with(".json"));
    assert_eq!(found.len(), FILES.len(), "{found:?}");
    for (name, expected) in FILES {
        assert_eq!(kind(&read(name)), expected, "{name}");
    }
}

/// Every id the record files define.
fn defined_ids() -> BTreeSet<SymbolId> {
    let mut ids = BTreeSet::new();
    for (name, _) in FILES {
        match read(name) {
            CompetitionsFile::Leagues(file) => ids.extend(file.leagues.into_iter().map(|x| x.id)),
            CompetitionsFile::Cups(file) => {
                ids.extend(file.competitions.into_iter().map(|x| x.id));
            }
            CompetitionsFile::Tournaments(file) => {
                ids.extend(file.competitions.into_iter().map(|x| x.id));
            }
            CompetitionsFile::Playoffs(file) => ids.extend(file.playoffs.into_iter().map(|x| x.id)),
            CompetitionsFile::Qualifying(file) => ids.extend(file.groups.into_iter().map(|x| x.id)),
            CompetitionsFile::LeagueBlocks(file) => {
                ids.extend(file.groups.into_iter().map(|x| x.id));
            }
            CompetitionsFile::Career(_)
            | CompetitionsFile::CountryCompetitions(_)
            | CompetitionsFile::Countries(_)
            | CompetitionsFile::SeedBuckets(_) => {}
        }
    }
    ids
}

/// Every competition id the files name. Not the country tables (`_albaniaTable`), which
/// only upstream's C data holds, nor the career copies (`_euroCupCopy`), made on load.
fn referenced_ids() -> Vec<(String, SymbolId)> {
    let mut refs = Vec::new();
    let mut add = |what: &str, id: Option<SymbolId>| {
        if let Some(id) = id {
            refs.push((what.to_owned(), id));
        }
    };
    for (name, _) in FILES {
        match read(name) {
            CompetitionsFile::Leagues(file) => {
                for league in file.leagues {
                    for link in league.playoff_links.into_iter().flatten() {
                        add("a league's play-off link", link);
                    }
                }
            }
            CompetitionsFile::LeagueBlocks(file) => {
                for block in file.groups {
                    for league in block.leagues {
                        add("a block's league", league.league);
                    }
                    add("a block's play-offs", block.playoffs.flatten());
                    add("a block's next block", block.next_group.flatten());
                }
            }
            CompetitionsFile::Career(file) => {
                for contest in file.contests {
                    add("a career contest", contest.competition);
                }
            }
            CompetitionsFile::CountryCompetitions(file) => {
                for country in file.countries {
                    for id in country.leagues.into_iter().chain(country.cups).flatten() {
                        add("a country's competition", Some(id));
                    }
                }
            }
            CompetitionsFile::Qualifying(file) => {
                for group in file.groups {
                    for entry in group.comp_cup {
                        match entry {
                            CompCupEntry::Competition(id) => add("a qualifying group's", Some(id)),
                            CompCupEntry::Marker(_) => {}
                        }
                    }
                }
            }
            CompetitionsFile::Cups(_)
            | CompetitionsFile::Tournaments(_)
            | CompetitionsFile::Playoffs(_)
            | CompetitionsFile::Countries(_)
            | CompetitionsFile::SeedBuckets(_) => {}
        }
    }
    refs
}

#[test]
fn every_competition_named_is_defined_once() {
    let defined = defined_ids();
    assert_eq!(defined.len(), 180, "ids are unique across the files");
    let refs = referenced_ids();
    assert!(refs.len() > 200, "{} references", refs.len());
    for (what, id) in refs {
        assert!(
            defined.contains(&id),
            "{what} {} is not defined",
            id.as_str()
        );
    }
}

#[test]
fn records_decode_as_upstream_describes_them() {
    let CompetitionsFile::Tournaments(tournaments) = read("tournaments.json") else {
        panic!("tournaments.json is not the tournaments");
    };
    let euro_cup = tournaments
        .competitions
        .iter()
        .find(|tournament| tournament.id.as_str() == "_euroCup")
        .expect("the European Cup");
    assert_eq!(euro_cup.comp.kind, CompetitionType::Tournament);
    assert_eq!(euro_cup.rounds.len(), 6);
    assert_eq!(euro_cup.points_for_win.value(), 5);

    let CompetitionsFile::LeagueBlocks(blocks) = read("league_blocks.json") else {
        panic!("league_blocks.json is not the league blocks");
    };
    let english = blocks
        .groups
        .iter()
        .find(|block| block.id.as_str() == "_englishLeagues1")
        .expect("the English block");
    assert_eq!(
        english.next_group,
        Some(Some(SymbolId("_englishLeagues2".to_owned())))
    );

    let CompetitionsFile::Cups(cups) = read("cups.json") else {
        panic!("cups.json is not the cups");
    };
    let friendly = cups
        .competitions
        .iter()
        .find(|cup| cup.id.as_str() == "_friendly")
        .expect("the friendly");
    assert_eq!(
        friendly.comp.kind,
        CompetitionType::League,
        "authored as a league"
    );
    assert!(
        cups.competitions
            .iter()
            .flat_map(|cup| &cup.round_rule_flags)
            .any(|rules| rules.low.is_some()),
        "some rule bytes set bits 1-0"
    );

    // Upstream's leagues bring no new country; World 2026-27's do.
    let CompetitionsFile::Leagues(leagues) = read("leagues.json") else {
        panic!("leagues.json is not the leagues");
    };
    assert!(leagues.leagues.iter().all(|league| league.new.is_none()));
}
