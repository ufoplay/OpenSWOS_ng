//! A world's competitions over World 1996-97's, as upstream lays them at start-up
//! (docs/WORLD_INVENTORY.md §9):
//!
//! - its leagues replace 96/97's by id, keeping 96/97's play-off links (src/world_data.c
//!   `install_league`, save_format_json.c `dj_inplace_load`);
//! - its new leagues come in with their countries and their tables (`World_InstallBaked`:
//!   `_countriesTable`, `_competitionsTable`, `Comp_Register`).

use std::collections::BTreeMap;

use openswos_competitions_json_resource::{
    CompetitionsDocument, CompetitionsFile, CountriesFile, Country, CountryCompetitions,
    CountryCompetitionsFile, League, LeaguesFile, SymbolId,
};
use openswos_team_json_resource::TeamJsonDocument;
use openswos_world_package_resource::TEAMS_FOLDER;

use crate::error::ExtractError;

/// The leagues' file.
const LEAGUES: &str = "leagues.json";

/// The countries' file.
const COUNTRIES: &str = "countries.json";

/// The country tables' file.
const COUNTRY_TABLES: &str = "country_competitions.json";

/// What starts a country's `CN_*` name.
const DEFINE_PREFIX: &str = "CN_";

/// What a country table's name is its league's id followed by (upstream's generated tables
/// are named so: `kWorld2627_worldAndLeagueTable` for `_worldAndLeague`).
const TABLE_SUFFIX: &str = "Table";

/// What stands in a `CN_*` name for a character that is not a letter or a digit.
const DEFINE_FILLER: char = '_';

/// Lays `world_leagues` over the competition files of `base`, adding the countries its new
/// leagues bring (each with its roster among `world_teams`) and their tables.
pub(super) fn merge(
    base: &mut BTreeMap<String, CompetitionsDocument>,
    world_leagues: &LeaguesFile,
    world_teams: &BTreeMap<String, TeamJsonDocument>,
) -> Result<(), ExtractError> {
    let new_leagues = merge_leagues(leagues_mut(base)?, world_leagues);
    let mut countries = Vec::new();
    let mut tables = Vec::new();
    for league in &new_leagues {
        let Some(new) = &league.new else {
            continue;
        };
        let roster = world_teams
            .iter()
            .find(|(_, document)| document.file.country == Some(new.country.0))
            .map(|(file, _)| file)
            .ok_or(ExtractError::NoTeamFile(new.country.0))?;
        countries.push(Country {
            number: new.country,
            name: new.name.clone(),
            define: define(&new.name),
            team_file: Some(format!("{TEAMS_FOLDER}/{roster}")),
        });
        tables.push(CountryCompetitions {
            number: new.country,
            name: new.name.clone(),
            table: SymbolId(format!("{}{TABLE_SUFFIX}", league.id.as_str())),
            leagues: Some(vec![league.id.clone()]),
            cups: None,
        });
    }
    let countries_file = countries_mut(base)?;
    countries_file.countries.extend(countries);
    countries_file.count = countries_file.countries.len();
    let tables_file = tables_mut(base)?;
    tables_file.countries.extend(tables);
    tables_file.count = tables_file.countries.len();
    Ok(())
}

/// Replaces each of `base`'s leagues `world` has, keeping its play-off links, and appends the
/// others; returns the appended ones.
fn merge_leagues(base: &mut LeaguesFile, world: &LeaguesFile) -> Vec<League> {
    let mut new_leagues = Vec::new();
    for league in &world.leagues {
        match base.leagues.iter_mut().find(|found| found.id == league.id) {
            Some(found) => {
                let links = found.playoff_links.take();
                *found = league.clone();
                found.playoff_links = links;
            }
            None => new_leagues.push(league.clone()),
        }
    }
    base.leagues.extend(new_leagues.iter().cloned());
    base.count = base.leagues.len();
    new_leagues
}

/// A new country's `CN_*` name: `CN_` and its name, each character not a letter or a digit
/// an underscore.
fn define(name: &str) -> String {
    let body: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                DEFINE_FILLER
            }
        })
        .collect();
    format!("{DEFINE_PREFIX}{body}")
}

/// The leagues' file of `base`.
fn leagues_mut(
    base: &mut BTreeMap<String, CompetitionsDocument>,
) -> Result<&mut LeaguesFile, ExtractError> {
    let Some(CompetitionsFile::Leagues(file)) =
        base.get_mut(LEAGUES).map(|document| &mut document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(LEAGUES));
    };
    Ok(file)
}

/// The countries' file of `base`.
fn countries_mut(
    base: &mut BTreeMap<String, CompetitionsDocument>,
) -> Result<&mut CountriesFile, ExtractError> {
    let Some(CompetitionsFile::Countries(file)) =
        base.get_mut(COUNTRIES).map(|document| &mut document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(COUNTRIES));
    };
    Ok(file)
}

/// The country tables' file of `base`.
pub(super) fn tables_mut(
    base: &mut BTreeMap<String, CompetitionsDocument>,
) -> Result<&mut CountryCompetitionsFile, ExtractError> {
    let Some(CompetitionsFile::CountryCompetitions(file)) = base
        .get_mut(COUNTRY_TABLES)
        .map(|document| &mut document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(COUNTRY_TABLES));
    };
    Ok(file)
}

#[cfg(test)]
mod tests {
    use openswos_common::{Importer, TeamFileNumber};
    use openswos_competitions_json_resource::CompetitionsJsonFormat;
    use openswos_team_json_resource::TeamJsonFormat;
    use openswos_test_utils::world_1996_97_path;

    use super::*;
    use crate::assemble::committed;

    /// A league as upstream writes it, `id`, `comp_id` and `tail` its own.
    fn league(id: &str, comp_id: u8, tail: &str) -> String {
        format!(
            r#"{{"id":"{id}","comp":{{"comp_id":{comp_id},"type":0,"team_number":0,"begin_month":64,"end_month":32,"name_table_offset":0,"reserved":0,"team_list_offset":0,"seed_reorder_flag":0}},"divisions_count":1,"rounds":2,"points_for_win":3,"substitutes":3,"max_subs":5,"divisions":[{{"teams":20,"promoted":0,"promoted_po":0,"relegated":0,"relegated_po":0,"qualifier_link_offset":0}}],"reserved1":0,"name":[null,"_aDivision1"]{tail}}}"#
        )
    }

    fn leagues(records: &[String]) -> LeaguesFile {
        let text = format!(
            r#"{{"type":"competition_leagues","count":{},"leagues":[{}]}}"#,
            records.len(),
            records.join(",")
        );
        let CompetitionsFile::Leagues(file) = CompetitionsJsonFormat::import(text.as_bytes())
            .expect("leagues")
            .file
        else {
            panic!("a leagues file");
        };
        file
    }

    /// A roster of country `country`.
    fn roster(country: u8) -> TeamJsonDocument {
        let text = format!(r#"{{"type":"team","country":{country},"count":0,"teams":[]}}"#);
        TeamJsonFormat::import(text.as_bytes()).expect("a roster")
    }

    #[test]
    fn a_world_league_replaces_its_id_keeping_the_play_off_links() {
        let mut base = leagues(&[
            league(
                "_englishLeague",
                0,
                r#","playoff_links":["_div1Playoffs",null]"#,
            ),
            league("_customsLeague", 36, ""),
        ]);
        let world = leagues(&[
            league("_englishLeague", 0, ""),
            league(
                "_worldAndLeague",
                170,
                r#","new":{"country":47,"name":"ANDORRA","byte":0}"#,
            ),
        ]);
        let added = merge_leagues(&mut base, &world);
        assert_eq!(added.len(), 1);
        assert_eq!(base.count, 3);
        let english = &base.leagues[0];
        assert_eq!(english.divisions[0].teams.byte(), 20);
        assert_eq!(
            english.playoff_links,
            Some(vec![Some(SymbolId("_div1Playoffs".to_owned())), None])
        );
        assert_eq!(
            base.leagues[1].id.as_str(),
            "_customsLeague",
            "96/97's own kept"
        );
        assert_eq!(base.leagues[2].id.as_str(), "_worldAndLeague");
    }

    #[test]
    fn a_new_league_brings_its_country_and_its_table() {
        let mut base = committed(&world_1996_97_path())
            .expect("the committed package")
            .competitions;
        let world = leagues(&[league(
            "_worldAndLeague",
            170,
            r#","new":{"country":47,"name":"BOSNIA-HERZ.","byte":0}"#,
        )]);
        let teams = BTreeMap::from([("BOSNIA.json".to_owned(), roster(47))]);
        merge(&mut base, &world, &teams).expect("merged");
        let Some(CompetitionsFile::Countries(countries)) = base.get(COUNTRIES).map(|d| &d.file)
        else {
            panic!("countries");
        };
        let added = countries.countries.last().expect("a country");
        assert_eq!(added.number, TeamFileNumber(47));
        assert_eq!(added.define, "CN_BOSNIA_HERZ_");
        assert_eq!(added.team_file.as_deref(), Some("teams/BOSNIA.json"));
        assert_eq!(countries.count, countries.countries.len());
        let Some(CompetitionsFile::CountryCompetitions(tables)) =
            base.get(COUNTRY_TABLES).map(|d| &d.file)
        else {
            panic!("tables");
        };
        let table = tables.countries.last().expect("a table");
        assert_eq!(table.table.as_str(), "_worldAndLeagueTable");
        assert_eq!(
            table.leagues,
            Some(vec![SymbolId("_worldAndLeague".to_owned())])
        );
    }

    #[test]
    fn a_new_country_without_its_roster_is_named() {
        let mut base = committed(&world_1996_97_path())
            .expect("the committed package")
            .competitions;
        let world = leagues(&[league(
            "_worldAndLeague",
            170,
            r#","new":{"country":47,"name":"ANDORRA","byte":0}"#,
        )]);
        let error = merge(&mut base, &world, &BTreeMap::new()).expect_err("no roster");
        assert!(error.to_string().contains("country 47"), "{error}");
    }
}
