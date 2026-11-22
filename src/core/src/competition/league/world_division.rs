//! A division of a world's league, resolved from the world's data (docs/PLAN.md M5 #4b2): what
//! [`start_division`](super::start_division) takes, as upstream's
//! `InitDIYCompetitionFromTableEntry` finds it for a record: the division's name (the record's,
//! else the world's default), the country's adjective before it unless the competition is a
//! national teams' one, the reset's name, the suspensions' track.

use thiserror::Error;

use super::{DivisionStart, LeagueRecord};
use crate::competition::start::Pitch;
use crate::world::{CompetitionRef, NameBytes, Source, World};

/// The names a division has in a record's name table: its full name and its short one
/// (upstream `CompetitionLeague.name[2 * LEAGUE_MAX_DIVISIONS]`).
const NAMES_A_DIVISION: usize = 2;

/// A division of a world's league, or of a cup played as one, with what the world resolves
/// for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDivision {
    /// The record.
    record: LeagueRecord,
    /// The division's index.
    division: usize,
    /// Its name.
    name: Vec<u8>,
    /// The country's adjective before it.
    prefix: Option<Vec<u8>>,
    /// The name the reset writes first.
    default_name: Vec<u8>,
    /// Whether its suspensions are the continental track's.
    continental: bool,
}

/// Why a world has no such division.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResolveError {
    /// The competition is not played as a league: a cup, a tournament or a play-off.
    #[error("{0} is not played as a league")]
    NotALeague(String),
    /// The record has no such division.
    #[error("the league has no division {0}")]
    NoDivision(usize),
    /// The record names no division and the world has no default name for this one
    /// (upstream reads past `_divisionStringTable`).
    #[error("the league names no division and the world has no default name for division {0}")]
    NoDefaultName(usize),
}

impl WorldDivision {
    /// Division `division` of `competition`, played from `source`, in `world`.
    ///
    /// The name is the record's for the division when its name table has a first name, else
    /// the world's default for the division (upstream `comp_name_table`, `_divisionStringTable`):
    /// a null name is the empty one. The country's adjective comes before it unless the
    /// competition is a national teams' one, when the record's country has one (upstream
    /// `prependCountryNameIfClub`: no country, no adjective).
    ///
    /// # Errors
    ///
    /// [`ResolveError`] when the competition is not played as a league, it has no such
    /// division, or the division takes a default name the world does not have.
    pub fn resolve(
        world: &World,
        competition: CompetitionRef,
        source: Source,
        division: usize,
    ) -> Result<Self, ResolveError> {
        let (record, names) = match competition {
            CompetitionRef::League(league) => {
                let league = world.league(league);
                (LeagueRecord::from_league(&league.record), &league.names)
            }
            CompetitionRef::Cup(cup) => {
                let cup = world.cup(cup);
                let record = LeagueRecord::from_cup(&cup.record)
                    .map_err(|refused| ResolveError::NotALeague(refused.0))?;
                (record, &cup.names)
            }
            CompetitionRef::Tournament(_) | CompetitionRef::Playoff(_) => {
                let id = world.competition_id(competition).as_str().to_owned();
                return Err(ResolveError::NotALeague(id));
            }
        };
        if division >= record.divisions.len() {
            return Err(ResolveError::NoDivision(division));
        }
        let name = match names.first() {
            Some(Some(_)) => own_name(names, division),
            Some(None) | None => world
                .texts()
                .division_name(division)
                .ok_or(ResolveError::NoDefaultName(division))?
                .to_vec(),
        };
        Ok(Self {
            record,
            division,
            name,
            prefix: world.name_prefix(competition, source).map(<[u8]>::to_vec),
            default_name: world.texts().league_reset_name().to_vec(),
            continental: world.is_continental(competition, source),
        })
    }

    /// What [`start_division`](super::start_division) takes for it, on the session's `pitch`.
    #[must_use]
    pub fn start(&self, pitch: Pitch) -> DivisionStart<'_> {
        DivisionStart {
            record: &self.record,
            division: self.division,
            name: &self.name,
            prefix: self.prefix.as_deref(),
            default_name: &self.default_name,
            continental: self.continental,
            pitch,
        }
    }
}

/// The record's own name for `division`, two names a division: the empty name for a null one
/// or one past the table (upstream's table is zero-filled past the record's names).
fn own_name(names: &[NameBytes], division: usize) -> Vec<u8> {
    names
        .get(NAMES_A_DIVISION * division)
        .cloned()
        .flatten()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use openswos_competitions_json_resource::SymbolId;

    use super::*;
    use crate::fixture::{package, package_with};

    fn competition(world: &World, id: &str) -> CompetitionRef {
        world
            .competition(&SymbolId(id.to_owned()))
            .expect("a competition")
    }

    fn resolved(
        world: &World,
        id: &str,
        source: Source,
        division: usize,
    ) -> Result<WorldDivision, ResolveError> {
        WorldDivision::resolve(world, competition(world, id), source, division)
    }

    #[test]
    fn a_league_without_names_takes_the_default_and_its_countrys_adjective() {
        let world = World::from_package(&package()).expect("a valid world");
        let division = resolved(&world, "_albanianLeague", Source::Record, 0).expect("resolved");
        let start = division.start(Pitch::Seasonal);
        assert_eq!(start.name, b"PREMIER DIVISION");
        assert_eq!(start.prefix, Some(&b"ALBANIAN"[..]));
        assert_eq!(start.default_name, b"DIY LEAGUE");
        assert!(!start.continental);
        assert_eq!(start.division, 0);
        assert_eq!(start.record.comp_id, 37);
        assert_eq!(start.pitch, Pitch::Seasonal);
    }

    #[test]
    fn a_league_with_names_takes_its_divisions_full_name() {
        let world = World::from_package(&package_with(&[
            (
                "leagues.json",
                r#""divisions": [{"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0}],
             "reserved1": 0, "name": []},"#,
                r#""divisions": [{"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0},
                              {"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0},
                              {"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0}],
             "reserved1": 0, "name": ["_aDivision1", "D1", null, "D2"]},"#,
            ),
        ]))
        .expect("a valid world");
        let name = |division| {
            resolved(&world, "_albanianLeague", Source::Record, division)
                .expect("resolved")
                .name
        };
        assert_eq!(name(0), b"DIVISION 1", "a symbol's text");
        assert_eq!(name(1), b"", "a null name");
        assert_eq!(name(2), b"", "past the table");
    }

    #[test]
    fn a_national_teams_competition_takes_no_adjective() {
        // The friendly's career copy is a national teams' competition; give it a country.
        let world = World::from_package(&package_with(&[(
            "cups.json",
            r#"{"comp_id": 5, "type": 0, "team_number": -1,"#,
            r#"{"comp_id": 5, "type": 0, "team_number": 8,"#,
        )]))
        .expect("a valid world");
        let prefix = |source| {
            resolved(&world, "_friendly", source, 0)
                .expect("resolved")
                .prefix
        };
        assert_eq!(prefix(Source::Record), Some(b"ENGLISH".to_vec()));
        assert_eq!(prefix(Source::CareerCopy), None);
    }

    #[test]
    fn a_record_of_no_country_takes_no_adjective() {
        let world = World::from_package(&package()).expect("a valid world");
        let division = resolved(&world, "_friendly", Source::Record, 0).expect("resolved");
        assert_eq!(division.prefix, None);
        assert!(
            division.continental,
            "the friendly is the continental track's"
        );
    }

    #[test]
    fn only_a_league_or_a_cup_played_as_one_has_divisions() {
        let world = World::from_package(&package()).expect("a valid world");
        let refused = |id: &str| resolved(&world, id, Source::Record, 0).expect_err("refused");
        for id in ["_albanianCup", "_euroCup", "_div1Playoffs"] {
            assert_eq!(refused(id), ResolveError::NotALeague(id.to_owned()));
        }
        assert_eq!(
            resolved(&world, "_albanianLeague", Source::Record, 1).expect_err("one division"),
            ResolveError::NoDivision(1)
        );
    }

    #[test]
    fn a_division_past_the_default_names_is_refused() {
        let mut package = package_with(&[(
            "leagues.json",
            r#""divisions": [{"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0}],"#,
            r#""divisions": [{"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0},
                          {"teams": 2, "promoted": 0, "promoted_po": 0, "relegated": 0, "relegated_po": 0, "qualifier_link_offset": 0}],"#,
        )]);
        let strings = package.rules.strings.as_mut().expect("the strings");
        strings.division_names.truncate(1);
        let world = World::from_package(&package).expect("a valid world");
        assert_eq!(
            resolved(&world, "_albanianLeague", Source::Record, 1).expect_err("no default"),
            ResolveError::NoDefaultName(1)
        );
    }
}
