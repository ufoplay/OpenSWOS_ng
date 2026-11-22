//! A tournament of a world, resolved from the world's data (docs/PLAN.md M5 #4d2): what
//! [`start_tournament`](super::start_tournament) takes, as upstream's `InitDIYLeagueFromRecord`
//! finds it for a record: its name (the record's first, else the world's default), the
//! country's adjective before it unless it is a national teams' competition, the resets'
//! names, the suspensions' track, the world's own draw for it and whether its groups may be
//! odd.

use super::record::{RecordError, TournamentRecord};
use super::start::{ResetNames, TournamentStart};
use crate::competition::start::Pitch;
use crate::world::{CompetitionRef, Source, World};

/// A world's tournament, with what the world resolves for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTournament {
    /// The record.
    record: TournamentRecord,
    /// Its name.
    name: Vec<u8>,
    /// The country's adjective before it.
    prefix: Option<Vec<u8>>,
    /// The names the tournament's, a league's and a cup's resets write first.
    reset_names: [Vec<u8>; 3],
    /// Whether its suspensions are the continental track's.
    continental: bool,
    /// The places its first stage keeps when the world draws it by its own tables.
    own_draw: Option<u8>,
    /// Whether its groups may hold an odd number of teams.
    odd_groups: bool,
}

impl WorldTournament {
    /// `competition`, played from `source`, in `world`.
    ///
    /// The name is the record's first when its name table has one, else the world's default
    /// tournament name (upstream `comp_name_table`, `_aLeague_1`). The country's adjective
    /// comes before it unless it is a national teams' competition ([`World::name_prefix`]).
    ///
    /// # Errors
    ///
    /// [`RecordError`] when the competition is no tournament, or states more stages, round
    /// rules or seeds than its record holds.
    pub fn resolve(
        world: &World,
        competition: CompetitionRef,
        source: Source,
    ) -> Result<Self, RecordError> {
        let tournament = match competition {
            CompetitionRef::Tournament(tournament) => world.tournament(tournament),
            CompetitionRef::League(_) | CompetitionRef::Cup(_) | CompetitionRef::Playoff(_) => {
                let id = world.competition_id(competition).as_str().to_owned();
                return Err(RecordError::NotATournament(id));
            }
        };
        let seeds: Vec<[u8; 2]> = tournament
            .seeds
            .iter()
            .map(|seed| seed.to_bytes())
            .collect();
        let record = TournamentRecord::from_tournament(&tournament.record, &seeds)?;
        let texts = world.texts();
        let name = match tournament.names.first() {
            Some(Some(name)) => name.clone(),
            Some(None) | None => texts.tournament_name().to_vec(),
        };
        Ok(Self {
            record,
            name,
            prefix: world.name_prefix(competition, source).map(<[u8]>::to_vec),
            reset_names: [
                texts.tournament_reset_name().to_vec(),
                texts.league_reset_name().to_vec(),
                texts.cup_reset_name().to_vec(),
            ],
            continental: world.is_continental(competition, source),
            own_draw: world.own_draw(competition),
            odd_groups: world.odd_tournament_groups(),
        })
    }

    /// What [`start_tournament`](super::start_tournament) takes for it, on the session's
    /// `pitch`.
    #[must_use]
    pub fn start(&self, pitch: Pitch) -> TournamentStart<'_> {
        let [tournament, league, cup] = &self.reset_names;
        TournamentStart {
            record: &self.record,
            name: &self.name,
            prefix: self.prefix.as_deref(),
            reset_names: ResetNames {
                tournament,
                league,
                cup,
            },
            continental: self.continental,
            own_draw: self.own_draw,
            odd_groups: self.odd_groups,
            pitch,
        }
    }
}

#[cfg(test)]
mod tests {
    use openswos_competitions_json_resource::SymbolId;
    use openswos_world_rules_resource::OwnDraw;

    use super::*;
    use crate::fixture::{package, package_with};

    fn resolved(world: &World, id: &str, source: Source) -> Result<WorldTournament, RecordError> {
        let competition = world
            .competition(&SymbolId(id.to_owned()))
            .expect("a competition");
        WorldTournament::resolve(world, competition, source)
    }

    #[test]
    fn a_tournament_takes_its_name_its_seeds_and_the_world_s_texts() {
        let world = World::from_package(&package()).expect("a valid world");
        let tournament = resolved(&world, "_euroCup", Source::Record).expect("resolved");
        let start = tournament.start(Pitch::Seasonal);
        assert_eq!(start.name, b"EUROPEAN CHAMPIONS' CUP");
        assert_eq!(start.prefix, None, "a record of no country");
        assert_eq!(start.reset_names.tournament, b"DIY TOURNAMENT");
        assert_eq!(start.reset_names.league, b"DIY LEAGUE");
        assert_eq!(start.reset_names.cup, b"DIY CUP");
        assert!(start.continental);
        assert_eq!(start.own_draw, None);
        assert!(!start.odd_groups, "the small world refuses them");
        // TIRANA, Albania's first club.
        assert_eq!(&start.record.seeds[..2], &[[0, 0], [0, 0]]);
        assert_eq!(start.record.stage(0)[0], 2);
    }

    #[test]
    fn a_tournament_without_names_takes_the_default_and_its_own_draw() {
        let mut package = package_with(&[(
            "tournaments.json",
            r#""name": ["_aEuropeanChampi_1", "_aEuropeanCup"]"#,
            r#""name": []"#,
        )]);
        package
            .rules
            .draws
            .as_mut()
            .expect("the draws")
            .own_draws
            .push(OwnDraw {
                competitions: vec![SymbolId("_euroCup".to_owned())],
                first_stage: 16,
            });
        let world = World::from_package(&package).expect("a valid world");
        let tournament = resolved(&world, "_euroCup", Source::CareerCopy).expect("resolved");
        assert_eq!(tournament.name, b"LEAGUE");
        assert_eq!(tournament.own_draw, Some(16));
        assert!(tournament.continental, "its copy too");
    }

    #[test]
    fn only_a_tournament_is_resolved() {
        let world = World::from_package(&package()).expect("a valid world");
        for id in ["_albanianLeague", "_albanianCup", "_div1Playoffs"] {
            assert_eq!(
                resolved(&world, id, Source::Record),
                Err(RecordError::NotATournament(id.to_owned())),
                "{id}"
            );
        }
    }
}
