//! A knockout of a world, resolved from the world's data (docs/PLAN.md M5 #4c): what
//! [`start_cup`](super::start_cup) takes, as upstream's `InitDIYCupFromRecord` finds it for a
//! record: the cup's name (the record's first, else the world's default), the country's
//! adjective before it unless the competition is a national teams' one, the reset's name, the
//! suspensions' track.

use super::{CupRecord, CupStart, RecordError};
use crate::competition::start::Pitch;
use crate::world::{CompetitionRef, Source, World};

/// A world's cup or play-off, with what the world resolves for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldKnockout {
    /// The record.
    record: CupRecord,
    /// Its name.
    name: Vec<u8>,
    /// The country's adjective before it.
    prefix: Option<Vec<u8>>,
    /// The name the reset writes first.
    default_name: Vec<u8>,
    /// Whether its suspensions are the continental track's.
    continental: bool,
}

impl WorldKnockout {
    /// `competition`, played from `source`, in `world`.
    ///
    /// The name is the record's first when its name table has one, else the world's default
    /// cup name (upstream `comp_name_table`, `_aCup_0`). The country's adjective comes before
    /// it unless the competition is a national teams' one ([`World::name_prefix`]).
    ///
    /// # Errors
    ///
    /// [`RecordError`] when the competition is no knockout (a league, a tournament, a cup
    /// played as a league), or states more round rules than its record holds.
    pub fn resolve(
        world: &World,
        competition: CompetitionRef,
        source: Source,
    ) -> Result<Self, RecordError> {
        let (record, names) = match competition {
            CompetitionRef::Cup(cup) => {
                let cup = world.cup(cup);
                (CupRecord::from_cup(&cup.record)?, &cup.names)
            }
            CompetitionRef::Playoff(playoff) => {
                let playoff = world.playoff(playoff);
                (CupRecord::from_playoff(&playoff.record)?, &playoff.names)
            }
            CompetitionRef::League(_) | CompetitionRef::Tournament(_) => {
                let id = world.competition_id(competition).as_str().to_owned();
                return Err(RecordError::NotAKnockout(id));
            }
        };
        let name = match names.first() {
            Some(Some(name)) => name.clone(),
            Some(None) | None => world.texts().cup_name().to_vec(),
        };
        Ok(Self {
            record,
            name,
            prefix: world.name_prefix(competition, source).map(<[u8]>::to_vec),
            default_name: world.texts().cup_reset_name().to_vec(),
            continental: world.is_continental(competition, source),
        })
    }

    /// What [`start_cup`](super::start_cup) takes for it, on the session's `pitch`.
    #[must_use]
    pub fn start(&self, pitch: Pitch) -> CupStart<'_> {
        CupStart {
            record: &self.record,
            name: &self.name,
            prefix: self.prefix.as_deref(),
            default_name: &self.default_name,
            continental: self.continental,
            pitch,
        }
    }
}

#[cfg(test)]
mod tests {
    use openswos_competitions_json_resource::SymbolId;

    use super::*;
    use crate::fixture::{package, package_with};

    fn resolved(world: &World, id: &str, source: Source) -> Result<WorldKnockout, RecordError> {
        let competition = world
            .competition(&SymbolId(id.to_owned()))
            .expect("a competition");
        WorldKnockout::resolve(world, competition, source)
    }

    #[test]
    fn a_cup_without_names_takes_the_default_and_its_countrys_adjective() {
        let world = World::from_package(&package()).expect("a valid world");
        let cup = resolved(&world, "_albanianCup", Source::Record).expect("resolved");
        let start = cup.start(Pitch::Seasonal);
        assert_eq!(start.name, b"CUP");
        assert_eq!(start.prefix, Some(&b"ALBANIAN"[..]));
        assert_eq!(start.default_name, b"DIY CUP");
        assert!(!start.continental);
        assert_eq!(start.record.comp_id, 109);
        assert_eq!(start.record.round_rules, [0x54, 0, 0, 0, 0, 0]);
        assert_eq!(start.pitch, Pitch::Seasonal);
    }

    #[test]
    fn a_cup_with_names_takes_its_first() {
        let world = World::from_package(&package()).expect("a valid world");
        let cup = resolved(&world, "_cupWinnersCup", Source::Record).expect("resolved");
        assert_eq!(cup.name, b"EUROPEAN CUP WINNERS' CUP");
        assert_eq!(cup.prefix, None, "a record of no country");
        let mut package = package_with(&[(
            "cups.json",
            r#""name": ["_aEuropeanCupWin", "_aEuroCwCup"]"#,
            r#""name": [null, "_aEuroCwCup"]"#,
        )]);
        let classes = package
            .rules
            .competition_classes
            .as_mut()
            .expect("the classes");
        classes
            .continental
            .push(SymbolId("_cupWinnersCup".to_owned()));
        let world = World::from_package(&package).expect("a valid world");
        let cup = resolved(&world, "_cupWinnersCup", Source::Record).expect("resolved");
        assert_eq!(cup.name, b"CUP", "no first name: the default");
        assert!(cup.continental);
    }

    #[test]
    fn a_playoff_is_a_knockout_of_its_record() {
        let mut package = package();
        let world = World::from_package(&package).expect("a valid world");
        let playoff = resolved(&world, "_div1Playoffs", Source::Record).expect("resolved");
        let start = playoff.start(Pitch::Type(2));
        assert_eq!(start.name, b"DIVISION 1 PLAY-OFFS");
        assert_eq!(start.prefix, Some(&b"ENGLISH"[..]));
        assert_eq!(start.record.teams, 4);
        assert_eq!(
            start.record.substitutes, 0x33,
            "its bench where a cup's points are"
        );
        assert_eq!(start.record.round_rules, [0x94, 0]);
        let classes = package
            .rules
            .competition_classes
            .as_mut()
            .expect("the classes");
        classes
            .national_teams
            .push(SymbolId("_div1Playoffs".to_owned()));
        let world = World::from_package(&package).expect("a valid world");
        let playoff = resolved(&world, "_div1Playoffs", Source::Record).expect("resolved");
        assert_eq!(playoff.prefix, None, "a national teams' competition");
    }

    #[test]
    fn only_a_knockout_is_resolved() {
        let world = World::from_package(&package()).expect("a valid world");
        for id in ["_albanianLeague", "_friendly", "_euroCup"] {
            assert_eq!(
                resolved(&world, id, Source::Record),
                Err(RecordError::NotAKnockout(id.to_owned())),
                "{id}"
            );
        }
    }
}
