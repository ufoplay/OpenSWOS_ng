//! Upstream's competition files (`assets/COMPETITIONS_JSON/`) are a valid world: World
//! 1996-97's competitions, countries and tables, every reference resolved.

mod common;

use openswos_common::{TeamFileNumber, TeamId, TeamOrdinal};
use openswos_competitions_json_resource::SymbolId;
use openswos_core::World;
use openswos_core::world::{CompetitionRef, GroupEntry, LeagueLink, TableContent};

fn id(text: &str) -> SymbolId {
    SymbolId(text.to_owned())
}

fn world() -> World {
    World::from_package(&common::package()).unwrap_or_else(|invalid| panic!("{invalid}"))
}

#[test]
fn upstreams_files_are_a_valid_world() {
    let world = world();
    assert_eq!(world.name(), "World 1996-97");
    assert_eq!(
        (
            world.leagues().len(),
            world.cups().len(),
            world.tournaments().len(),
            world.playoffs().len(),
        ),
        (66, 76, 14, 7)
    );
    assert_eq!(
        (
            world.qualifying_groups().len(),
            world.league_blocks().len(),
            world.career().len(),
            world.country_tables().len(),
        ),
        (10, 7, 35, 76)
    );
}

#[test]
fn brazil_is_listed_twice() {
    let world = world();
    let countries = world.countries();
    assert_eq!(countries.iter().count(), 71);
    let listed: Vec<&str> = countries
        .listed()
        .map(|country| country.name.as_str())
        .collect();
    assert_eq!(listed.len(), 72);
    assert_eq!(listed.iter().filter(|name| **name == "BRAZIL").count(), 2);
    assert_eq!(listed.first(), Some(&"BRAZIL"));
}

#[test]
fn englands_league_links_its_play_offs_and_its_block() {
    let world = world();
    let Some(CompetitionRef::League(english)) = world.competition(&id("_englishLeague")) else {
        panic!("England's league is a league");
    };
    let links = world.league(english).playoff_links.as_ref().expect("links");
    let [
        Some(LeagueLink::Playoff(playoffs)),
        Some(LeagueLink::Block(block)),
    ] = links.as_slice()
    else {
        panic!("a play-off and a block: {links:?}");
    };
    assert_eq!(world.playoff(*playoffs).record.id, id("_div1Playoffs"));
    let block = world.league_block(*block);
    assert_eq!(block.id, id("_englishLeagues1"));
    let next = block.next.map(|next| &world.league_block(next).id);
    assert_eq!(next, Some(&id("_englishLeagues2")));
}

#[test]
fn the_tables_are_the_countries_and_the_regions() {
    let world = world();
    let england = world
        .country_tables()
        .iter()
        .find(|table| table.number == TeamFileNumber(8))
        .expect("England's table");
    let TableContent::Own { leagues, cups, .. } = &england.content else {
        panic!("England's own table");
    };
    assert_eq!(leagues.len(), 1);
    let cups: Vec<&SymbolId> = cups.iter().map(|cup| &world.cup(*cup).record.id).collect();
    assert_eq!(cups, [&id("_englishCup"), &id("_englishLeagueCup")]);
    let regions = world
        .country_tables()
        .iter()
        .filter(|table| matches!(table.content, TableContent::Region(_)))
        .count();
    assert_eq!(regions, 11);
}

#[test]
fn europes_group_holds_the_european_cups_and_countries() {
    let world = world();
    let europe = world
        .qualifying_groups()
        .iter()
        .find(|group| group.id == id("_europeTable"))
        .expect("Europe's group");
    let competitions: Vec<&SymbolId> = europe
        .entries
        .iter()
        .filter_map(|entry| match entry {
            GroupEntry::Competition(competition) => Some(world.competition_id(*competition)),
            GroupEntry::Marker(_) => None,
        })
        .collect();
    assert_eq!(
        competitions,
        [
            &id("_europeanChampionships"),
            &id("_euroCup"),
            &id("_cupWinnersCup"),
            &id("_uefaCup")
        ]
    );
    assert_eq!(europe.countries.len(), 43);
}

#[test]
fn the_cup_winners_cup_starts_from_32_teams() {
    let world = world();
    let Some(CompetitionRef::Cup(cup)) = world.competition(&id("_cupWinnersCup")) else {
        panic!("a cup");
    };
    let seeds = &world.cup(cup).seeds;
    assert_eq!(seeds.len(), 32);
    // `{"country": "CN_GREECE", "ordinal": 0, "team": "AEK ATHENS"}`.
    assert_eq!(
        seeds.first(),
        Some(&TeamId {
            file: TeamFileNumber(15),
            ordinal: TeamOrdinal(0)
        })
    );
}
