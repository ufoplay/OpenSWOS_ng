//! World 2026-27 completed (`just world_complete_2627`): its download over World 1996-97, its
//! rules, its European cups and its national cups from its baked install, its pool unpacked: a
//! valid world with valid game data. A file of its own: the baked install writes the
//! oracle's 96/97 tables for good, so it runs in a process of its own. Skipped without the
//! oracle or the network.

use openswos_common::{TeamFileNumber, TeamId, TeamOrdinal};
use openswos_competitions_json_resource::{Continent, SymbolId};
use openswos_core::competition::schedule::Pair;
use openswos_core::world::{CompetitionRef, LeagueLink, TableContent};
use openswos_test_utils::tactics_path;
use openswos_world_extract::check::{builtin_tactics, game_data};
use openswos_world_package_resource::WorldPackage;

mod common;
use common::completed;

/// Its countries: 96/97's 71 and the 26 its leagues bring.
const COUNTRIES: usize = 97;

/// Its leagues: its 91, and 96/97's customs league, which it does not replace.
const LEAGUES: usize = 92;

/// Its rosters: 101 countries' clubs, the six continents' national teams.
const ROSTERS: usize = 107;

/// The clubs of Italy's team file (`resize_country(20, 256)`).
const ITALY_CLUBS: u16 = 256;

/// The national teams of its Europe file (`fit_world_national_files`: upstream logs "55
/// national teams in file 80").
const EUROPE_NATIONAL_TEAMS: u16 = 55;

/// The countries of its season end.
const SEASON_COUNTRIES: usize = 91;

/// The players of its foreign pool (`teams/POOLPLYR.DAT`, unpacked).
const POOL_PLAYERS: usize = 900;

/// The clubs of a league phase.
const LEAGUE_PHASE_CLUBS: usize = 36;

/// Its cups: 96/97's 76, and the 50 of its 101 national and league cups that are not 96/97's
/// (`kWorld2627Cups`; the other 51 replace 96/97's of their ids).
const CUPS: usize = 126;

/// The clubs of its English cup's real bracket (`kWorld2627_englishCupExt`).
const ENGLISH_CUP_CLUBS: usize = 64;

/// The European Championships' groups' clubs.
const EURO_GROUP: u8 = 4;

#[test]
fn world_2627_completes_into_a_valid_world() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = dir.path().join("2026-27");
    let Some(completed) = completed(&out) else {
        return;
    };
    assert_eq!(
        (completed.countries, completed.leagues, completed.rosters),
        (COUNTRIES, LEAGUES, ROSTERS)
    );
    assert_eq!(completed.pool, POOL_PLAYERS);

    let opened = WorldPackage::open_dir(&out).expect("the package opens");
    assert!(opened.fingerprint.is_match(), "it states its own files'");
    assert!(opened.ignored.is_empty(), "{:?}", opened.ignored);
    assert_eq!(
        opened.package.pool.as_ref().map(|pool| pool.players.len()),
        Some(POOL_PLAYERS),
        "written unpacked, and read back"
    );
    let tactics = builtin_tactics(&tactics_path()).expect("the tactics");
    let data = game_data(&opened.package, tactics).unwrap_or_else(|error| panic!("{error}"));
    let world = data.world();
    assert_eq!(world.name(), "World 2026-27");
    assert_eq!(world.first_year(), 2026);
    // Andorra and Canada: new countries, their continents from the baked countries' table.
    assert_eq!(world.continent(TeamFileNumber(47)), Some(Continent::Europe));
    assert_eq!(
        world.continent(TeamFileNumber(56)),
        Some(Continent::NorthAmerica)
    );
    let andorra = world.competition(&SymbolId("_worldAndLeague".to_owned()));
    assert!(matches!(andorra, Some(CompetitionRef::League(_))));

    let numbering = world.team_numbering();
    let first = |file: u8| {
        numbering.global_number(TeamId {
            file: TeamFileNumber(file),
            ordinal: TeamOrdinal(0),
        })
    };
    assert_eq!(first(21) - first(20), ITALY_CLUBS);
    assert_eq!(first(81) - first(80), EUROPE_NATIONAL_TEAMS);
    assert!(numbering.squad_numbers);
    assert_eq!(world.season_end().countries.len(), SEASON_COUNTRIES);
    // Its odd round robins are real rounds (`FixtureRounds_Install`), 3 clubs' the original's;
    // the European Championships play their own, as in 96/97.
    let schedules = world.schedules();
    let rounds = |teams| schedules.of_size(teams).map(|schedule| schedule.rounds());
    assert_eq!(
        (rounds(3), rounds(5), rounds(23), rounds(24)),
        (Some(false), Some(true), Some(true), Some(true))
    );
    let euro = world
        .competition(&SymbolId("_europeanChampionships".to_owned()))
        .expect("the European Championships");
    assert_eq!(
        schedules
            .of(euro, EURO_GROUP)
            .and_then(|schedule| schedule.pairs().first().copied()),
        Some(Pair { home: 1, away: 0 })
    );
    assert!(world.odd_tournament_groups());

    // England's league is the world's, its play-off links 96/97's (`install_league`).
    let Some(CompetitionRef::League(english)) =
        world.competition(&SymbolId("_englishLeague".to_owned()))
    else {
        panic!("England's league");
    };
    let links = world
        .league(english)
        .playoff_links
        .as_ref()
        .expect("its links");
    assert!(matches!(links.first(), Some(Some(LeagueLink::Playoff(_)))));

    // The league phases take the European cups' places; 96/97's stay, unplayed.
    let career: Vec<_> = world
        .career()
        .iter()
        .take(3)
        .filter_map(|contest| contest.competition)
        .map(|competition| world.competition_id(competition).as_str().to_owned())
        .collect();
    assert_eq!(
        career,
        [
            "_uefaChampionsLeague",
            "_uefaEuropaLeague",
            "_uefaConferenceLeague"
        ]
    );
    let Some(champions @ CompetitionRef::Tournament(tournament)) =
        world.competition(&SymbolId("_uefaChampionsLeague".to_owned()))
    else {
        panic!("a tournament");
    };
    assert!(world.is_played(champions));
    assert_eq!(world.tournament(tournament).seeds.len(), LEAGUE_PHASE_CLUBS);
    let euro_cup = world
        .competition(&SymbolId("_euroCup".to_owned()))
        .expect("96/97's record, a template");
    assert!(!world.is_played(euro_cup));

    // Each national and league cup in its country's table, in its slot (`WorldCups_Install`).
    assert_eq!(world.cups().len(), CUPS);
    let cups_of = |number: u8| -> Vec<&str> {
        world
            .country_tables()
            .iter()
            .filter(|table| table.number == TeamFileNumber(number))
            .flat_map(|table| match &table.content {
                TableContent::Own { cups, .. } => cups.clone(),
                TableContent::Region(_) => Vec::new(),
            })
            .map(|cup| world.competition_id(CompetitionRef::Cup(cup)).as_str())
            .collect()
    };
    assert_eq!(cups_of(20), ["_italianCup", "_italianLeagueCup"]);
    assert_eq!(cups_of(40), ["_welshCup", "_worldWalLeagueCup"], "grown");
    assert_eq!(cups_of(47), ["_worldAndCup"], "a new country's");
    let Some(english @ CompetitionRef::Cup(cup)) =
        world.competition(&SymbolId("_englishCup".to_owned()))
    else {
        panic!("England's cup");
    };
    assert!(world.is_played(english));
    assert_eq!(world.cup(cup).seeds.len(), ENGLISH_CUP_CLUBS);
}
