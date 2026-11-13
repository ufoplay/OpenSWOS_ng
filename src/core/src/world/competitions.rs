//! The world's competitions: the leagues, cups, tournaments and play-offs, each the record
//! its file holds with its references resolved. What their numbers mean is the competition
//! engine's (M5); here each record is checked to be of a type its file holds.

use openswos_common::TeamId;
use openswos_competitions_json_resource::{
    CompetitionType, Cup as CupRecord, League as LeagueRecord, Playoff as PlayoffRecord, SymbolId,
    TeamSeed, Tournament as TournamentRecord,
};

use super::countries::Countries;
use super::files::{CompetitionFiles, Slot};
use super::ids::Ids;
use super::tables::BlockRef;
use crate::validation::{Expected, Field, Kind, Location, Problem, Problems};

/// A league of a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeagueRef(pub(crate) usize);

/// A cup of a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CupRef(pub(crate) usize);

/// A tournament of a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TournamentRef(pub(crate) usize);

/// A play-off of a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayoffRef(pub(crate) usize);

/// A competition of a world, of any kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompetitionRef {
    /// A league.
    League(LeagueRef),
    /// A cup.
    Cup(CupRef),
    /// A tournament.
    Tournament(TournamentRef),
    /// A play-off.
    Playoff(PlayoffRef),
}

/// What a league's link names: its play-offs, or the block of leagues it heads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeagueLink {
    /// A play-off.
    Playoff(PlayoffRef),
    /// A league block.
    Block(BlockRef),
}

/// A league: its record, and its links resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct League {
    /// The record, as `leagues.json` holds it.
    pub record: LeagueRecord,
    /// Its `playoff_links`, resolved (`None` for a `null` link); `None` when it has none.
    pub playoff_links: Option<Vec<Option<LeagueLink>>>,
}

/// A cup: its record, and the teams its draw starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cup {
    /// The record, as `cups.json` holds it.
    pub record: CupRecord,
    /// Its `team_seed_list`, each team by its identity.
    pub seeds: Vec<TeamId>,
}

/// A tournament: its record, and the teams its draw starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tournament {
    /// The record, as `tournaments.json` holds it.
    pub record: TournamentRecord,
    /// Its `team_seed_list`, each team by its identity.
    pub seeds: Vec<TeamId>,
}

/// A play-off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playoff {
    /// The record, as `playoffs.json` holds it.
    pub record: PlayoffRecord,
}

/// `check` applied to every item, all of them (so every problem is reported); `None` when
/// any fails.
pub(crate) fn each<T, U>(
    items: &[T],
    mut check: impl FnMut(usize, &T) -> Option<U>,
) -> Option<Vec<U>> {
    let checked: Vec<Option<U>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| check(index, item))
        .collect();
    checked.into_iter().collect()
}

/// The competition `id` names, if any.
pub(crate) fn competition_ref(ids: &Ids, id: &SymbolId) -> Option<CompetitionRef> {
    ids.get(id).and_then(|target| match target.kind {
        Kind::League => Some(CompetitionRef::League(LeagueRef(target.index))),
        Kind::Cup => Some(CompetitionRef::Cup(CupRef(target.index))),
        Kind::Tournament => Some(CompetitionRef::Tournament(TournamentRef(target.index))),
        Kind::Playoff => Some(CompetitionRef::Playoff(PlayoffRef(target.index))),
        Kind::QualifyingGroup | Kind::LeagueBlock => None,
    })
}

/// What resolving needs: the ids, the countries and where problems go.
pub(crate) struct Resolver<'a> {
    /// The ids.
    pub(crate) ids: &'a Ids,
    /// The countries.
    pub(crate) countries: &'a Countries,
    /// The problems.
    pub(crate) problems: &'a mut Problems,
}

impl Resolver<'_> {
    /// What `id` names, which must be one of `expected`.
    fn target(
        &mut self,
        id: &SymbolId,
        expected: Expected,
        location: impl FnOnce() -> Location,
    ) -> Option<super::ids::Target> {
        self.ids.resolve(id, expected, location, self.problems)
    }

    /// The competition `id` names.
    pub(crate) fn competition(
        &mut self,
        id: &SymbolId,
        location: impl FnOnce() -> Location,
    ) -> Option<CompetitionRef> {
        self.target(id, Expected::COMPETITION, location)
            .and_then(|_| competition_ref(self.ids, id))
    }

    /// The league `id` names.
    pub(crate) fn league(
        &mut self,
        id: &SymbolId,
        location: impl FnOnce() -> Location,
    ) -> Option<LeagueRef> {
        self.target(id, Expected::LEAGUE, location)
            .map(|target| LeagueRef(target.index))
    }

    /// The cup `id` names.
    pub(crate) fn cup(
        &mut self,
        id: &SymbolId,
        location: impl FnOnce() -> Location,
    ) -> Option<CupRef> {
        self.target(id, Expected::CUP, location)
            .map(|target| CupRef(target.index))
    }

    /// The play-off `id` names.
    pub(crate) fn playoff(
        &mut self,
        id: &SymbolId,
        location: impl FnOnce() -> Location,
    ) -> Option<PlayoffRef> {
        self.target(id, Expected::PLAYOFF, location)
            .map(|target| PlayoffRef(target.index))
    }

    /// The league block `id` names.
    pub(crate) fn block(
        &mut self,
        id: &SymbolId,
        location: impl FnOnce() -> Location,
    ) -> Option<BlockRef> {
        self.target(id, Expected::LEAGUE_BLOCK, location)
            .map(|target| BlockRef(target.index))
    }

    /// The play-off or league block a league's link names.
    fn league_link(
        &mut self,
        id: &SymbolId,
        location: impl FnOnce() -> Location,
    ) -> Option<LeagueLink> {
        let target = self.target(id, Expected::LEAGUE_LINK, location)?;
        match target.kind {
            Kind::Playoff => Some(LeagueLink::Playoff(PlayoffRef(target.index))),
            Kind::LeagueBlock => Some(LeagueLink::Block(BlockRef(target.index))),
            Kind::League | Kind::Cup | Kind::Tournament | Kind::QualifyingGroup => None,
        }
    }

    /// Whether a record's `type` is one its file holds.
    fn check_type(
        &mut self,
        found: CompetitionType,
        holds: &[CompetitionType],
        location: impl FnOnce() -> Location,
    ) -> Option<()> {
        if holds.contains(&found) {
            Some(())
        } else {
            self.problems
                .push(location(), Problem::WrongCompetitionType { found });
            None
        }
    }

    /// The teams a draw starts from, each by its country's number and its place in the
    /// country's team file.
    fn seeds(&mut self, seeds: &[TeamSeed], path: &str, field: &Field) -> Option<Vec<TeamId>> {
        each(seeds, |index, seed| {
            match self.countries.resolve(&seed.country) {
                Ok(country) => Some(TeamId {
                    file: country.number,
                    ordinal: seed.ordinal,
                }),
                Err(problem) => {
                    let location = Location::field(path, &field.index(index).then("country"));
                    self.problems.push(location, problem);
                    None
                }
            }
        })
    }
}

/// The leagues of `leagues.json`.
pub(crate) fn leagues(
    files: &CompetitionFiles<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Vec<League>> {
    let path = Slot::Leagues.path();
    each(&files.leagues.leagues, |index, record| {
        let field = Field::key("leagues").record(index, &record.id);
        let kind = resolver.check_type(record.comp.kind, &[CompetitionType::League], || {
            Location::field(&path, &field.then("comp").then("type"))
        });
        // The country a World 2026-27 league brings must be one of the world's countries.
        let new_country = match &record.new {
            Some(new) => resolver
                .countries
                .check(
                    new.country,
                    || Location::field(&path, &field.then("new").then("country")),
                    resolver.problems,
                )
                .map(|_| ()),
            None => Some(()),
        };
        let links_field = field.then("playoff_links");
        let playoff_links = match &record.playoff_links {
            Some(links) => each(links, |link, id| match id {
                Some(id) => resolver
                    .league_link(id, || Location::field(&path, &links_field.index(link)))
                    .map(Some),
                None => Some(None),
            })
            .map(Some),
            None => Some(None),
        };
        kind?;
        new_country?;
        Some(League {
            record: record.clone(),
            playoff_links: playoff_links?,
        })
    })
}

/// The cups of `cups.json`: knockout cups, and the qualifying groups, friendlies and tours
/// authored as cups, which play as leagues.
pub(crate) fn cups(files: &CompetitionFiles<'_>, resolver: &mut Resolver<'_>) -> Option<Vec<Cup>> {
    let path = Slot::Cups.path();
    each(&files.cups.competitions, |index, record| {
        let field = Field::key("competitions").record(index, &record.id);
        let kind = resolver.check_type(
            record.comp.kind,
            &[CompetitionType::League, CompetitionType::Cup],
            || Location::field(&path, &field.then("comp").then("type")),
        );
        let seeds = resolver.seeds(&record.team_seed_list, &path, &field.then("team_seed_list"));
        kind?;
        Some(Cup {
            record: record.clone(),
            seeds: seeds?,
        })
    })
}

/// The tournaments of `tournaments.json`.
pub(crate) fn tournaments(
    files: &CompetitionFiles<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Vec<Tournament>> {
    let path = Slot::Tournaments.path();
    each(&files.tournaments.competitions, |index, record| {
        let field = Field::key("competitions").record(index, &record.id);
        let kind = resolver.check_type(record.comp.kind, &[CompetitionType::Tournament], || {
            Location::field(&path, &field.then("comp").then("type"))
        });
        let seeds = resolver.seeds(&record.team_seed_list, &path, &field.then("team_seed_list"));
        kind?;
        Some(Tournament {
            record: record.clone(),
            seeds: seeds?,
        })
    })
}

/// The play-offs of `playoffs.json`, which play as cups.
pub(crate) fn playoffs(
    files: &CompetitionFiles<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Vec<Playoff>> {
    let path = Slot::Playoffs.path();
    each(&files.playoffs.playoffs, |index, record| {
        let field = Field::key("playoffs").record(index, &record.id);
        resolver.check_type(record.comp.kind, &[CompetitionType::Cup], || {
            Location::field(&path, &field.then("comp").then("type"))
        })?;
        Some(Playoff {
            record: record.clone(),
        })
    })
}
