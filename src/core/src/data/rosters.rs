//! The rosters: every team of the world's team files, typed, with the ids its JSON states.

use std::collections::BTreeMap;

use openswos_common::TeamId;
use openswos_team_json_resource::{EntryIds, TeamJsonDocument, team_checksum};
use openswos_team_resource::{PLAYERS_PER_TEAM, Team};
use openswos_world_package_resource::TEAMS_FOLDER;
use uuid::Uuid;

use crate::validation::{Field, Location, Problem, Problems};

/// A team of the rosters, by its roster and its place in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TeamRef {
    /// Its roster's index.
    roster: usize,
    /// Its index in the roster.
    team: usize,
}

/// A roster: one team file of the package (`teams/<NAME>.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster {
    /// Its file name in `teams/` (`BRAZIL.json`).
    pub file: String,
    /// Its teams, in the file's order.
    pub teams: Vec<RosterTeam>,
}

/// A team of a roster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterTeam {
    /// The team.
    pub team: Team,
    /// Its id.
    pub uuid: Uuid,
    /// Its players' ids, in squad order.
    pub player_uuids: [Uuid; PLAYERS_PER_TEAM],
    /// Whether its stated checksum is the one its players make. A team edited with its id
    /// kept is not canonical: upstream does not offer it as what its id stands for, to a
    /// custom team or online (docs/TEAM_IDS.md §4). It plays all the same.
    pub canonical: bool,
}

/// The rosters, and every team by its identity and by its id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rosters {
    /// The rosters, by file name.
    rosters: Vec<Roster>,
    /// The rosters' indices by file name.
    by_file: BTreeMap<String, usize>,
    /// The teams by identity.
    by_id: BTreeMap<TeamId, TeamRef>,
    /// The teams by id.
    by_uuid: BTreeMap<Uuid, TeamRef>,
}

impl Rosters {
    /// The roster in `teams/<file>`.
    #[must_use]
    pub fn roster(&self, file: &str) -> Option<&Roster> {
        self.by_file
            .get(file)
            .and_then(|index| self.rosters.get(*index))
    }

    /// The rosters, by file name.
    #[must_use]
    pub fn rosters(&self) -> &[Roster] {
        &self.rosters
    }

    /// The team `team`, which must be these rosters'.
    #[must_use]
    pub fn get(&self, team: TeamRef) -> &RosterTeam {
        &self.rosters[team.roster].teams[team.team]
    }

    /// The team whose identity is `id`.
    #[must_use]
    pub fn by_id(&self, id: TeamId) -> Option<TeamRef> {
        self.by_id.get(&id).copied()
    }

    /// The team whose id is `uuid`.
    #[must_use]
    pub fn by_uuid(&self, uuid: &Uuid) -> Option<TeamRef> {
        self.by_uuid.get(uuid).copied()
    }

    /// The teams of `files`. A team that does not convert is reported and left out; so is
    /// an identity or an id held again (no lookup could tell the two apart).
    pub(crate) fn build(
        files: &BTreeMap<String, TeamJsonDocument>,
        problems: &mut Problems,
    ) -> Self {
        let mut rosters = Self::default();
        let mut team_at: BTreeMap<TeamId, Location> = BTreeMap::new();
        let mut uuid_at: BTreeMap<Uuid, Location> = BTreeMap::new();
        for (file, document) in files {
            let path = roster_path(file);
            let roster_index = rosters.rosters.len();
            let mut roster = Roster {
                file: file.clone(),
                teams: Vec::new(),
            };
            for (index, entry) in document.file.teams.iter().enumerate() {
                let field = Field::key("teams").index(index);
                let team = match entry.to_team() {
                    Ok(team) => team,
                    Err(source) => {
                        problems.push(Location::field(&path, &field), Problem::Team(source));
                        continue;
                    }
                };
                let location = Location::field(&path, &field);
                if let Some(first) = team_at.get(&team.id) {
                    problems.push(
                        location,
                        Problem::DuplicateTeam {
                            team: team.id,
                            first: first.clone(),
                        },
                    );
                    continue;
                }
                let ids = EntryIds::of(entry);
                let player_fields = (0..PLAYERS_PER_TEAM)
                    .map(|player| field.then("players").index(player).then("uuid"));
                let claims = std::iter::once((ids.team, field.then("uuid")))
                    .chain(ids.players.iter().copied().zip(player_fields));
                let mut unique = true;
                for (uuid, claim) in claims {
                    let at = Location::field(&path, &claim);
                    if let Some(first) = uuid_at.get(&uuid) {
                        problems.push(
                            at,
                            Problem::DuplicateUuid {
                                uuid,
                                first: first.clone(),
                            },
                        );
                        unique = false;
                    } else {
                        uuid_at.insert(uuid, at);
                    }
                }
                if !unique {
                    continue;
                }
                team_at.insert(team.id, location);
                let team_ref = TeamRef {
                    roster: roster_index,
                    team: roster.teams.len(),
                };
                rosters.by_id.insert(team.id, team_ref);
                rosters.by_uuid.insert(ids.team, team_ref);
                roster.teams.push(RosterTeam {
                    canonical: team_checksum(&team) == ids.checksum,
                    team,
                    uuid: ids.team,
                    player_uuids: ids.players,
                });
            }
            rosters.by_file.insert(file.clone(), roster_index);
            rosters.rosters.push(roster);
        }
        rosters
    }
}

/// The path of the roster `file` in the package.
pub(crate) fn roster_path(file: &str) -> String {
    format!("{TEAMS_FOLDER}/{file}")
}
