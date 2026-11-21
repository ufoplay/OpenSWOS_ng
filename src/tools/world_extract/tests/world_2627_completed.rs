//! World 2026-27 completed (`just world_complete_2627`): its download over World 1996-97, its
//! rules and its European cups from its baked install, its pool unpacked: a valid world with
//! valid game data. A file of its own: the baked install writes the
//! oracle's 96/97 tables for good, so it runs in a process of its own. Skipped without the
//! oracle or the network.

use openswos_common::{TeamFileNumber, TeamId, TeamOrdinal};
use openswos_competitions_json_resource::{Continent, SymbolId};
use openswos_core::world::{CompetitionRef, LeagueLink};
use openswos_oracle::tables::InstallError;
use openswos_test_utils::skip::{Skip, report};
use openswos_test_utils::tactics_path;
use openswos_world_extract::check::{builtin_tactics, game_data};
use openswos_world_extract::commands::{Completed, complete_2627};
use openswos_world_extract::error::ExtractError;
use openswos_world_package_resource::WorldPackage;
use openswos_world_rules_resource::OddRoundRobin;

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

/// The completed world written into a new folder, or `None` (the skip reported) without the
/// oracle or the network.
fn completed(dir: &std::path::Path) -> Option<Completed> {
    match complete_2627(Some(dir)) {
        Ok(completed) => Some(completed),
        Err(ExtractError::Oracle(unavailable)) => {
            report(&Skip::new(unavailable.to_string()));
            None
        }
        Err(ExtractError::Install(InstallError::Unavailable(unavailable))) => {
            report(&Skip::new(unavailable.to_string()));
            None
        }
        Err(ExtractError::Download(reason)) => {
            report(&Skip::new(reason));
            None
        }
        Err(error) => panic!("{error}"),
    }
}

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
    assert_eq!(
        world.fixtures().odd_round_robin,
        OddRoundRobin::RoundsWithRest
    );
    assert!(world.fixtures().odd_tournament_groups);

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
}
