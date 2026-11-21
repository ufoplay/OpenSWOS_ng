//! The game data: a world with its rosters, the transfer pool and the built-in tactics, each
//! checked against the others (docs/PLAN.md §4). What the match, the competitions and the
//! career play with.

mod rosters;
mod tactics;

use std::collections::BTreeMap;

use openswos_poolplyr_resource::{PoolFile, PoolPlayer};
use openswos_tac_resource::TeamTactics;
use openswos_tactics_json_resource::PresetFile;
use openswos_team_json_resource::TeamJsonDocument;
use openswos_team_resource::PLAYERS_PER_TEAM;

pub use rosters::{Roster, RosterTeam, Rosters, TeamRef};
pub use tactics::{PresetTactics, TACTICS_FOLDER};

use crate::validation::{Field, Invalid, Location, Problem, Problems};
use crate::world::{Slot, World};

/// Where the pool's problems are reported: the PC game's pool file, whose records the pool
/// holds (upstream `DATA/POOLPLYR.DAT`).
pub const POOL_FILE: &str = "POOLPLYR.DAT";

/// A world and what it plays with, valid together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameData {
    /// The world.
    world: World,
    /// Its rosters.
    rosters: Rosters,
    /// The transfer pool.
    pool: Vec<PoolPlayer>,
    /// The built-in tactics.
    tactics: PresetTactics,
}

impl GameData {
    /// The game data of `world`, its rosters (`teams`, the package's `teams/` by file
    /// name), the transfer pool and the built-in tactics.
    ///
    /// Checked: every team converts; no identity or id is held twice; every country's
    /// roster is in the package and holds only the country's teams; every team a draw starts
    /// from and every pool player's team is in the rosters, and the player in its squad;
    /// every preset has its shipped tactics. A team whose stated checksum is not its
    /// players' is not canonical, which is no problem ([`RosterTeam::canonical`]).
    ///
    /// # Errors
    ///
    /// [`Invalid`], with every problem found.
    pub fn new(
        world: World,
        teams: &BTreeMap<String, TeamJsonDocument>,
        pool: PoolFile,
        tactics: BTreeMap<PresetFile, TeamTactics>,
    ) -> Result<Self, Invalid> {
        let mut problems = Problems::default();
        let rosters = Rosters::build(teams, &mut problems);
        check_country_rosters(&world, &rosters, &mut problems);
        check_seeds(&world, &rosters, &mut problems);
        check_pool(&pool, &rosters, &mut problems);
        let tactics = PresetTactics::build(tactics, &mut problems);
        let data = tactics.map(|tactics| Self {
            world,
            rosters,
            pool: pool.players,
            tactics,
        });
        problems.finish(data)
    }

    /// The world.
    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Its rosters.
    #[must_use]
    pub fn rosters(&self) -> &Rosters {
        &self.rosters
    }

    /// The transfer pool.
    #[must_use]
    pub fn pool(&self) -> &[PoolPlayer] {
        &self.pool
    }

    /// The built-in tactics.
    #[must_use]
    pub fn tactics(&self) -> &PresetTactics {
        &self.tactics
    }
}

/// Every country's roster is in the package, and holds only the country's teams.
fn check_country_rosters(world: &World, rosters: &Rosters, problems: &mut Problems) {
    for country in world.countries().iter() {
        let Some(file) = &country.roster else {
            continue;
        };
        let Some(roster) = rosters.roster(file) else {
            problems.push(
                country.roster_location(),
                Problem::MissingRoster(rosters::roster_path(file)),
            );
            continue;
        };
        for (index, team) in roster.teams.iter().enumerate() {
            if team.team.id.file != country.number {
                problems.push(
                    Location::field(
                        &rosters::roster_path(file),
                        &Field::key("teams")
                            .index(index)
                            .then("team")
                            .then("country_number"),
                    ),
                    Problem::ForeignTeam {
                        roster: country.number,
                        found: team.team.id.file,
                    },
                );
            }
        }
    }
}

/// Every team a draw starts from is in the rosters.
fn check_seeds(world: &World, rosters: &Rosters, problems: &mut Problems) {
    let cups = world.cups().iter().map(|cup| (&cup.record.id, &cup.seeds));
    let tournaments = world
        .tournaments()
        .iter()
        .map(|tournament| (&tournament.record.id, &tournament.seeds));
    for (slot, competitions) in [
        (Slot::Cups, cups.collect::<Vec<_>>()),
        (Slot::Tournaments, tournaments.collect()),
    ] {
        for (index, (id, seeds)) in competitions.into_iter().enumerate() {
            for (seed, team) in seeds.iter().enumerate() {
                if rosters.by_id(*team).is_none() {
                    problems.push(
                        Location::field(
                            &slot.path(),
                            &Field::key("competitions")
                                .record(index, id)
                                .then("team_seed_list")
                                .index(seed),
                        ),
                        Problem::UnknownTeam(*team),
                    );
                }
            }
        }
    }
}

/// Every pool player's team is in the rosters, and the player in its squad.
fn check_pool(pool: &PoolFile, rosters: &Rosters, problems: &mut Problems) {
    for (index, player) in pool.players.iter().enumerate() {
        let field = Field::key("players").index(index);
        if rosters.by_id(player.team).is_none() {
            problems.push(
                Location::field(POOL_FILE, &field.then("team")),
                Problem::UnknownTeam(player.team),
            );
        }
        if usize::from(player.player_ordinal) >= PLAYERS_PER_TEAM {
            problems.push(
                Location::field(POOL_FILE, &field.then("player_ordinal")),
                Problem::UnknownPlayer(player.player_ordinal),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use openswos_common::{TeamFileNumber, TeamId, TeamOrdinal};
    use openswos_poolplyr_resource::PoolFile;
    use openswos_tac_resource::TeamTactics;
    use openswos_tactics_json_resource::{Preset, PresetFile, Release};
    use openswos_team_json_resource::{TeamChecksum, TeamJsonDocument};
    use openswos_team_resource::TeamFile;

    use super::*;
    use crate::fixture::{ALBANIA, ENGLAND, package, pool, roster, tactics, teams};

    /// The parts of the small world's game data, to change before checking them.
    struct Parts {
        teams: BTreeMap<String, TeamJsonDocument>,
        pool: PoolFile,
        tactics: BTreeMap<PresetFile, TeamTactics>,
    }

    fn parts() -> Parts {
        Parts {
            teams: teams(),
            pool: pool(),
            tactics: tactics(),
        }
    }

    fn check(parts: Parts) -> Result<GameData, Invalid> {
        let world = World::from_package(&package()).expect("a valid world");
        let Parts {
            teams,
            pool,
            tactics,
        } = parts;
        GameData::new(world, &teams, pool, tactics)
    }

    /// The problems of `parts`, a line each.
    fn problems(parts: Parts) -> Vec<String> {
        match check(parts) {
            Ok(_) => Vec::new(),
            Err(invalid) => invalid.0.iter().map(ToString::to_string).collect(),
        }
    }

    fn team(file: TeamFileNumber, ordinal: u8) -> TeamId {
        TeamId {
            file,
            ordinal: TeamOrdinal(ordinal),
        }
    }

    fn roster_mut<'a>(parts: &'a mut Parts, file: &str) -> &'a mut TeamJsonDocument {
        parts
            .teams
            .get_mut(file)
            .unwrap_or_else(|| panic!("the fixture has {file}"))
    }

    #[test]
    fn the_small_world_plays_with_its_rosters_pool_and_tactics() {
        let data = check(parts()).expect("valid game data");
        let rosters = data.rosters();
        assert_eq!(
            rosters
                .rosters()
                .iter()
                .map(|roster| (roster.file.as_str(), roster.teams.len()))
                .collect::<Vec<_>>(),
            [("ALBANIA.json", 2), ("ENGLAND.json", 1)]
        );
        let second = rosters
            .by_id(team(ALBANIA, 1))
            .expect("Albania's second team");
        let found = rosters.get(second);
        assert_eq!(found.team.id, team(ALBANIA, 1));
        assert!(found.canonical);
        assert_eq!(rosters.by_uuid(&found.uuid), Some(second));
        assert_eq!(rosters.by_id(team(ENGLAND, 1)), None);
        assert_eq!(rosters.roster("EUROPE.json"), None, "Europe has no roster");
        assert_eq!(data.pool().len(), 1);
        for preset in Preset::ALL {
            assert_eq!(
                Some(data.tactics().shipped(preset)),
                data.tactics().get(PresetFile {
                    preset,
                    release: Release::Shipped
                })
            );
        }
        assert_eq!(data.world().name(), "World 1996-97");
    }

    #[test]
    fn an_edited_team_is_not_canonical_but_plays() {
        let mut parts = parts();
        let entry = roster_mut(&mut parts, "ENGLAND.json")
            .file
            .teams
            .first_mut()
            .expect("a team");
        entry.checksum = TeamChecksum::try_from("0000000000000000".to_owned()).expect("a checksum");
        let data = check(parts).expect("valid game data");
        let english = data
            .rosters()
            .by_id(team(ENGLAND, 0))
            .expect("England's team");
        assert!(!data.rosters().get(english).canonical);
    }

    #[test]
    fn a_countrys_missing_roster_is_reported_with_what_needs_it() {
        let mut parts = parts();
        parts.teams.remove("ENGLAND.json");
        assert_eq!(
            problems(parts),
            [
                "competitions/countries.json: countries[0].team_file: the package has no roster \
             teams/ENGLAND.json",
                "competitions/cups.json: competitions[1] (_cupWinnersCup).team_seed_list[0]: the \
             rosters have no team (8, 0)",
            ]
        );
    }

    #[test]
    fn a_team_in_another_countrys_roster_is_reported() {
        let mut parts = parts();
        let entry = roster_mut(&mut parts, "ALBANIA.json")
            .file
            .teams
            .get_mut(1)
            .expect("a second team");
        entry.team.country_number = ENGLAND.0;
        assert_eq!(
            problems(parts),
            [
                "teams/ALBANIA.json: teams[1].team.country_number: the team is country 8's, the \
             roster country 0's",
                "competitions/cups.json: competitions[1] (_cupWinnersCup).team_seed_list[1]: the \
             rosters have no team (0, 1)",
                "POOLPLYR.DAT: players[0].team: the rosters have no team (0, 1)",
            ]
        );
    }

    #[test]
    fn an_identity_or_an_id_held_twice_is_reported() {
        let mut parts = parts();
        let albania = parts.teams["ALBANIA.json"]
            .file
            .to_team_file()
            .expect("teams");
        let first = albania.teams.first().cloned().expect("a team");
        // Another roster holding Albania's first team again: the same identity.
        parts.teams.insert(
            "SECOND.json".to_owned(),
            roster(
                &TeamFile {
                    teams: vec![first.clone()],
                },
                "SECOND",
            ),
        );
        // And one holding it under another identity, but with its id.
        let mut renumbered = first;
        renumbered.id = team(ALBANIA, 9);
        let mut copy = roster(
            &TeamFile {
                teams: vec![renumbered],
            },
            "COPY",
        );
        let albanian_uuid = parts.teams["ALBANIA.json"].file.teams[0].uuid;
        copy.file.teams[0].uuid = albanian_uuid;
        parts.teams.insert("COPY.json".to_owned(), copy);
        assert_eq!(
            problems(parts),
            [
                format!(
                    "teams/COPY.json: teams[0].uuid: {albanian_uuid} is used again (first at \
                 teams/ALBANIA.json: teams[0].uuid)"
                )
                .as_str(),
                "teams/SECOND.json: teams[0]: team (0, 0) is in the rosters again (first at \
             teams/ALBANIA.json: teams[0])",
            ]
        );
    }

    #[test]
    fn a_team_that_does_not_convert_is_reported() {
        let mut parts = parts();
        let entry = roster_mut(&mut parts, "ENGLAND.json")
            .file
            .teams
            .first_mut()
            .expect("a team");
        // A character no name field stores (a name too long is cut, as the game cuts it).
        entry.team.team_name = "ENGLAND \u{3a9}".to_owned();
        let found = problems(parts);
        let [converts, seed] = found.as_slice() else {
            panic!("two problems: {found:?}");
        };
        assert!(
            converts.starts_with("teams/ENGLAND.json: teams[0]: team_name: "),
            "{converts}"
        );
        assert!(seed.ends_with("the rosters have no team (8, 0)"), "{seed}");
    }

    #[test]
    fn a_pool_player_must_be_a_player_of_a_team_of_the_rosters() {
        let mut parts = parts();
        for player in &mut parts.pool.players {
            player.team = team(ENGLAND, 4);
            player.player_ordinal = 16;
        }
        assert_eq!(
            problems(parts),
            [
                "POOLPLYR.DAT: players[0].team: the rosters have no team (8, 4)",
                "POOLPLYR.DAT: players[0].player_ordinal: a team has no player 16",
            ]
        );
    }

    #[test]
    fn every_preset_needs_its_shipped_tactics() {
        let mut parts = parts();
        for release in [Release::Shipped, Release::Dos] {
            parts.tactics.remove(&PresetFile {
                preset: Preset::FourFourTwo,
                release,
            });
        }
        parts.tactics.remove(&PresetFile {
            preset: Preset::Defend,
            release: Release::Shipped,
        });
        assert_eq!(
            problems(parts),
            [
                "tactics/442.json: the file is missing",
                "tactics/Defend.json: the file is missing",
            ],
            "the other releases are optional"
        );
    }
}
