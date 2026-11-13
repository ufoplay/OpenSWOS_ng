//! The world's ids: every league, cup, tournament, play-off, qualifying group and league
//! block has one, unique across the files, by which the files name each other (upstream's
//! registry, src/save_format_json.c `DataJson_InstallCompetitionGraph`).

use std::collections::BTreeMap;

use openswos_competitions_json_resource::SymbolId;

use super::files::{CompetitionFiles, Slot};
use crate::validation::{Expected, Field, Kind, Location, Problem, Problems};

/// What an id names: its kind and its place in its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Target {
    /// Its kind.
    pub(crate) kind: Kind,
    /// Its index in its file.
    pub(crate) index: usize,
}

/// The ids, each with what it names and where it is defined.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Ids(BTreeMap<SymbolId, (Target, Location)>);

impl Ids {
    /// The ids `files` define. An id defined again is reported, and the first kept.
    pub(crate) fn build(files: &CompetitionFiles<'_>, problems: &mut Problems) -> Self {
        let mut ids = Self::default();
        let mut add = |slot: Slot, array: &str, kind: Kind, records: Vec<&SymbolId>| {
            for (index, id) in records.into_iter().enumerate() {
                let location = Location::field(&slot.path(), &Field::key(array).index(index));
                if let Some((_, first)) = ids.0.get(id) {
                    problems.push(
                        location,
                        Problem::DuplicateId {
                            id: id.clone(),
                            first: first.clone(),
                        },
                    );
                } else {
                    ids.0.insert(id.clone(), (Target { kind, index }, location));
                }
            }
        };
        add(
            Slot::Leagues,
            "leagues",
            Kind::League,
            files
                .leagues
                .leagues
                .iter()
                .map(|record| &record.id)
                .collect(),
        );
        add(
            Slot::Cups,
            "competitions",
            Kind::Cup,
            files
                .cups
                .competitions
                .iter()
                .map(|record| &record.id)
                .collect(),
        );
        add(
            Slot::Tournaments,
            "competitions",
            Kind::Tournament,
            files
                .tournaments
                .competitions
                .iter()
                .map(|record| &record.id)
                .collect(),
        );
        add(
            Slot::Playoffs,
            "playoffs",
            Kind::Playoff,
            files
                .playoffs
                .playoffs
                .iter()
                .map(|record| &record.id)
                .collect(),
        );
        add(
            Slot::Qualifying,
            "groups",
            Kind::QualifyingGroup,
            files
                .qualifying
                .groups
                .iter()
                .map(|record| &record.id)
                .collect(),
        );
        add(
            Slot::LeagueBlocks,
            "groups",
            Kind::LeagueBlock,
            files
                .league_blocks
                .groups
                .iter()
                .map(|record| &record.id)
                .collect(),
        );
        ids
    }

    /// What `id` names.
    pub(crate) fn get(&self, id: &SymbolId) -> Option<Target> {
        self.0.get(id).map(|(target, _)| *target)
    }

    /// What `id` names, which must be one of `expected`; otherwise reported at `location`.
    pub(crate) fn resolve(
        &self,
        id: &SymbolId,
        expected: Expected,
        location: impl FnOnce() -> Location,
        problems: &mut Problems,
    ) -> Option<Target> {
        let problem = match self.get(id) {
            Some(target) if expected.allows(target.kind) => return Some(target),
            Some(target) => Problem::WrongKind {
                id: id.clone(),
                found: target.kind,
                expected,
            },
            None => Problem::UnknownId {
                id: id.clone(),
                expected,
            },
        };
        problems.push(location(), problem);
        None
    }
}
