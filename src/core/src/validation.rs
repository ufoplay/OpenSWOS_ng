//! What validation reports: each problem, where it is (the file and the field), and why.
//! The world and the game data are checked whole, and every problem found is reported.

use std::fmt;

use openswos_common::{TeamFileNumber, TeamId};
use openswos_competitions_json_resource::{CompetitionType, SymbolId};
use openswos_team_json_resource::EntryError;
use thiserror::Error;
use uuid::Uuid;

/// Where a problem is: a file of the package (`competitions/leagues.json`) and a field in it
/// (`leagues[3] (_englishLeague).playoff_links[1]`; empty for the file as a whole).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location {
    /// The file.
    pub file: String,
    /// The field.
    pub field: String,
}

impl Location {
    /// The file as a whole.
    pub(crate) fn file(file: &str) -> Self {
        Self {
            file: file.to_owned(),
            field: String::new(),
        }
    }

    /// A field of `file`.
    pub(crate) fn field(file: &str, field: &Field) -> Self {
        Self {
            file: file.to_owned(),
            field: field.0.clone(),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.field.is_empty() {
            write!(f, "{}", self.file)
        } else {
            write!(f, "{}: {}", self.file, self.field)
        }
    }
}

/// A field's path in its file, built a step at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field(String);

impl Field {
    /// A top-level key.
    pub(crate) fn key(name: &str) -> Self {
        Self(name.to_owned())
    }

    /// The element at `index` of this array.
    pub(crate) fn index(&self, index: usize) -> Self {
        Self(format!("{}[{index}]", self.0))
    }

    /// The record at `index` of this array, named by its id.
    pub(crate) fn record(&self, index: usize, id: &SymbolId) -> Self {
        Self(format!("{}[{index}] ({})", self.0, id.as_str()))
    }

    /// The key `name` of this object.
    pub(crate) fn then(&self, name: &str) -> Self {
        Self(format!("{}.{name}", self.0))
    }
}

/// What a world's id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A league (`leagues.json`).
    League,
    /// A cup (`cups.json`).
    Cup,
    /// A tournament (`tournaments.json`).
    Tournament,
    /// A play-off (`playoffs.json`).
    Playoff,
    /// A qualifying group (`qualifying.json`).
    QualifyingGroup,
    /// A league block (`league_blocks.json`).
    LeagueBlock,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::League => "league",
            Self::Cup => "cup",
            Self::Tournament => "tournament",
            Self::Playoff => "play-off",
            Self::QualifyingGroup => "qualifying group",
            Self::LeagueBlock => "league block",
        })
    }
}

/// The kinds a reference may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Expected(pub &'static [Kind]);

impl Expected {
    /// A league.
    pub(crate) const LEAGUE: Self = Self(&[Kind::League]);
    /// A cup.
    pub(crate) const CUP: Self = Self(&[Kind::Cup]);
    /// A play-off.
    pub(crate) const PLAYOFF: Self = Self(&[Kind::Playoff]);
    /// A league block.
    pub(crate) const LEAGUE_BLOCK: Self = Self(&[Kind::LeagueBlock]);
    /// A qualifying group.
    pub(crate) const QUALIFYING_GROUP: Self = Self(&[Kind::QualifyingGroup]);
    /// A play-off or a league block: what a league's links name.
    pub(crate) const LEAGUE_LINK: Self = Self(&[Kind::Playoff, Kind::LeagueBlock]);
    /// Any competition.
    pub(crate) const COMPETITION: Self =
        Self(&[Kind::League, Kind::Cup, Kind::Tournament, Kind::Playoff]);

    /// Whether `kind` is one of them.
    pub(crate) fn allows(self, kind: Kind) -> bool {
        self.0.contains(&kind)
    }
}

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, kind) in self.0.iter().enumerate() {
            match index {
                0 => {}
                last if last + 1 == self.0.len() => f.write_str(" or ")?,
                _ => f.write_str(", ")?,
            }
            write!(f, "{kind}")?;
        }
        Ok(())
    }
}

/// Why a world or its game data is not valid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Problem {
    /// A file the world or its game data needs is missing.
    #[error("the file is missing")]
    MissingFile,
    /// A competition file holds another file's records.
    #[error("a `{found}` file, where the world reads `{expected}`")]
    WrongFileType {
        /// The `type` the file's name calls for.
        expected: &'static str,
        /// Its `type`.
        found: &'static str,
    },
    /// A competition file the world does not read.
    #[error("not one of the world's competition files")]
    UnknownFile,
    /// A record of a type its file does not hold.
    #[error("a {found:?} record, which this file does not hold")]
    WrongCompetitionType {
        /// The record's type.
        found: CompetitionType,
    },
    /// An id defined twice.
    #[error("{} is defined again (first at {first})", id.as_str())]
    DuplicateId {
        /// The id.
        id: SymbolId,
        /// Where it is defined first.
        first: Location,
    },
    /// A reference to an id nothing defines.
    #[error("no {expected} is called {}", id.as_str())]
    UnknownId {
        /// The id.
        id: SymbolId,
        /// What it should name.
        expected: Expected,
    },
    /// A reference to an id of the wrong kind.
    #[error("{} is a {found}, not a {expected}", id.as_str())]
    WrongKind {
        /// The id.
        id: SymbolId,
        /// What it names.
        found: Kind,
        /// What it should name.
        expected: Expected,
    },
    /// A country number the world does not have.
    #[error("country {} is not a country of the world", .0.0)]
    UnknownCountry(TeamFileNumber),
    /// A country's `CN_*` name the world does not have.
    #[error("{0} is not a country of the world")]
    UnknownCountryDefine(String),
    /// A country listed twice, differently.
    #[error("country {} is listed again, differently (first at {first})", number.0)]
    ConflictingCountry {
        /// Its number.
        number: TeamFileNumber,
        /// Where it is listed first.
        first: Location,
    },
    /// A country's `CN_*` name given to two countries.
    #[error("{define} is country {}'s too", first.0)]
    DuplicateDefine {
        /// The `CN_*` name.
        define: String,
        /// The country it names first.
        first: TeamFileNumber,
    },
    /// A roster path outside `teams/`.
    #[error("{0} is not a roster of the package (teams/<NAME>.json)")]
    NotARosterPath(String),
    /// A competition table numbered like another.
    #[error("table {} is listed again (first at {first})", number.0)]
    DuplicateTable {
        /// Its number.
        number: TeamFileNumber,
        /// Where it is listed first.
        first: Location,
    },
    /// A country's roster that the package does not hold.
    #[error("the package has no roster {0}")]
    MissingRoster(String),
    /// A team entry that does not convert.
    #[error(transparent)]
    Team(EntryError),
    /// A team identity held twice.
    #[error("team ({}, {}) is in the rosters again (first at {first})", team.file.0, team.ordinal.0)]
    DuplicateTeam {
        /// The identity.
        team: TeamId,
        /// Where it is first.
        first: Location,
    },
    /// A team's or a player's id used twice.
    #[error("{uuid} is used again (first at {first})")]
    DuplicateUuid {
        /// The id.
        uuid: Uuid,
        /// Where it is first.
        first: Location,
    },
    /// A team in another country's roster.
    #[error("the team is country {}'s, the roster country {}'s", found.0, roster.0)]
    ForeignTeam {
        /// The roster's country.
        roster: TeamFileNumber,
        /// The team's.
        found: TeamFileNumber,
    },
    /// A reference to a team the rosters do not hold.
    #[error("the rosters have no team ({}, {})", .0.file.0, .0.ordinal.0)]
    UnknownTeam(TeamId),
    /// A reference to a squad place a team does not have.
    #[error("a team has no player {0}")]
    UnknownPlayer(u8),
    /// A country of the world its rules say nothing of.
    #[error("country {} has no entry", .0.0)]
    NoCountryDetails(TeamFileNumber),
    /// A country listed twice where it is listed once.
    #[error("country {} is listed again (first at {first})", number.0)]
    RepeatedCountry {
        /// Its number.
        number: TeamFileNumber,
        /// Where it is listed first.
        first: Location,
    },
    /// A list of another length than its own.
    #[error("{found} entries, not {expected}")]
    WrongCount {
        /// The entries it must have.
        expected: usize,
        /// The entries it has.
        found: usize,
    },
    /// A number below the one before it, in a list that never goes down.
    #[error("{value} is below the one before ({previous})")]
    Descending {
        /// The number.
        value: u16,
        /// The one before.
        previous: u16,
    },
    /// A second team file numbered as a country's own.
    #[error("file {} is a country's own", .0.0)]
    FileTaken(TeamFileNumber),
    /// A round robin of fewer than two clubs.
    #[error("a round robin of {0} clubs: it takes two or more")]
    TooFewClubs(u8),
    /// A round robin of a size given again.
    #[error("a round robin of {teams} clubs again (first at {first})")]
    RepeatedSchedule {
        /// Its clubs.
        teams: u8,
        /// Where the size is given first.
        first: Location,
    },
    /// A pair that is not two of the round robin's clubs.
    #[error("[{home}, {away}] is no pair of {teams} clubs")]
    NoPair {
        /// The club at home.
        home: u8,
        /// The club away.
        away: u8,
        /// The round robin's clubs.
        teams: u8,
    },
    /// A pair a round robin plays again, either way round.
    #[error("[{home}, {away}] is played again (first at {first})")]
    RepeatedPair {
        /// The club at home.
        home: u8,
        /// The club away.
        away: u8,
        /// Where the pair is first.
        first: Location,
    },
    /// A round robin without some of its pairs.
    #[error("{missing} pairs of {teams} clubs are not played")]
    MissingPairs {
        /// Its clubs.
        teams: u8,
        /// The pairs it lacks.
        missing: usize,
    },
    /// A round robin said to play real rounds where a match day has a club twice.
    #[error("match day {day} has club {club} twice: no real rounds")]
    NotRounds {
        /// The match day, from 1.
        day: usize,
        /// The club.
        club: u8,
    },
    /// A competition given a round robin of its own again.
    #[error("the competition has a round robin of its own already (at {first})")]
    RepeatedCompetition {
        /// Where it is named first.
        first: Location,
    },
    /// A competition given a round robin of another size than its groups'.
    #[error("a round robin of {teams} clubs, where the competition plays {size}")]
    ScheduleSize {
        /// The round robin's clubs.
        teams: u8,
        /// A size the competition plays.
        size: u8,
    },
    /// A knockout given a round robin.
    #[error("the competition is a knockout: it plays no round robin")]
    NoRoundRobin,
}

/// One problem and where it is.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{location}: {problem}")]
pub struct ValidationError {
    /// Where.
    pub location: Location,
    /// Why.
    pub problem: Problem,
}

/// Every problem a world or its game data has: at least one.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub struct Invalid(pub Vec<ValidationError>);

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.0.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

/// The problems found so far.
#[derive(Debug, Default)]
pub(crate) struct Problems(Vec<ValidationError>);

impl Problems {
    /// Records `problem` at `location`.
    pub(crate) fn push(&mut self, location: Location, problem: Problem) {
        self.0.push(ValidationError { location, problem });
    }

    /// `value` when no problem was found, the problems otherwise.
    pub(crate) fn finish<T>(self, value: Option<T>) -> Result<T, Invalid> {
        match value {
            Some(value) if self.0.is_empty() => Ok(value),
            Some(_) | None => Err(Invalid(self.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use openswos_competitions_json_resource::SymbolId;

    use super::*;

    fn id(text: &str) -> SymbolId {
        SymbolId(text.to_owned())
    }

    #[test]
    fn a_field_path_names_indices_records_and_keys() {
        let field = Field::key("leagues")
            .record(3, &id("_englishLeague"))
            .then("playoff_links")
            .index(1);
        assert_eq!(field.0, "leagues[3] (_englishLeague).playoff_links[1]");
    }

    #[test]
    fn a_location_names_its_file_and_its_field() {
        let file = Location::file("competitions/cups.json");
        assert_eq!(file.to_string(), "competitions/cups.json");
        let field = Location::field("competitions/cups.json", &Field::key("count"));
        assert_eq!(field.to_string(), "competitions/cups.json: count");
    }

    #[test]
    fn the_expected_kinds_read_as_a_list() {
        assert_eq!(Expected::LEAGUE.to_string(), "league");
        assert_eq!(
            Expected::LEAGUE_LINK.to_string(),
            "play-off or league block"
        );
        assert_eq!(
            Expected::COMPETITION.to_string(),
            "league, cup, tournament or play-off"
        );
        assert!(Expected::LEAGUE_LINK.allows(Kind::LeagueBlock));
        assert!(!Expected::LEAGUE_LINK.allows(Kind::League));
    }

    #[test]
    fn every_kind_has_a_name() {
        let names = [
            Kind::League,
            Kind::Cup,
            Kind::Tournament,
            Kind::Playoff,
            Kind::QualifyingGroup,
            Kind::LeagueBlock,
        ]
        .map(|kind| kind.to_string());
        assert_eq!(
            names,
            [
                "league",
                "cup",
                "tournament",
                "play-off",
                "qualifying group",
                "league block"
            ]
        );
    }

    #[test]
    fn problems_read_with_their_ids() {
        let unknown = Problem::UnknownId {
            id: id("_nowhere"),
            expected: Expected::LEAGUE,
        };
        assert_eq!(unknown.to_string(), "no league is called _nowhere");
        let wrong = Problem::WrongKind {
            id: id("_euroCup"),
            found: Kind::Tournament,
            expected: Expected::CUP,
        };
        assert_eq!(wrong.to_string(), "_euroCup is a tournament, not a cup");
        let duplicate = Problem::DuplicateId {
            id: id("_euroCup"),
            first: Location::file("competitions/cups.json"),
        };
        assert_eq!(
            duplicate.to_string(),
            "_euroCup is defined again (first at competitions/cups.json)"
        );
    }

    #[test]
    fn every_problem_is_listed_a_line_each() {
        let mut problems = Problems::default();
        problems.push(Location::file("a.json"), Problem::MissingFile);
        problems.push(
            Location::field("b.json", &Field::key("x")),
            Problem::UnknownFile,
        );
        let invalid = problems.finish(Some(())).expect_err("two problems");
        assert_eq!(invalid.0.len(), 2);
        assert_eq!(
            invalid.to_string(),
            "a.json: the file is missing\nb.json: x: not one of the world's competition files"
        );
    }

    #[test]
    fn no_problem_gives_the_value() {
        assert_eq!(Problems::default().finish(Some(7)), Ok(7));
    }
}
