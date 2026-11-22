//! The world's classes of competition (`rules/competition_classes.json`): the national teams'
//! competitions and the contests of the continental suspensions' track. Upstream's lists name
//! records by address, a competition's own record and the copy a career plays it in apart
//! (src/swos_amiga_ptr_imports.c): so does each class here.

use std::collections::BTreeSet;

use openswos_competitions_json_resource::SymbolId;
use openswos_world_rules_resource::CompetitionClasses;

use super::competitions::{CompetitionRef, competition_ref};
use super::ids::Ids;
use super::rules::at;
use super::tables::CareerContest;
use crate::validation::{Field, Problem, Problems};

/// The record a competition is played from: its own, or the copy a career makes of it
/// (upstream `_careerContests`: a career plays its contests in copies).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    /// Its own record.
    Record,
    /// The career's copy of it.
    CareerCopy,
}

/// The classes, each the competitions in it with the record they are played from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Classes {
    /// The national teams' competitions.
    pub(crate) national_teams: BTreeSet<(CompetitionRef, Source)>,
    /// The contests of the continental suspensions' track.
    pub(crate) continental: BTreeSet<(CompetitionRef, Source)>,
}

/// The classes of `file`, each id a competition's, or the name of a copy `career` makes:
/// `None` (each reported) when one is neither.
pub(crate) fn classes(
    file: &CompetitionClasses,
    ids: &Ids,
    career: &[CareerContest],
    problems: &mut Problems,
) -> Option<Classes> {
    let mut class = |name: &str, listed: &[SymbolId]| {
        let resolved: Vec<Option<(CompetitionRef, Source)>> = listed
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let found = contest(id, ids, career);
                if found.is_none() {
                    let location = at::<CompetitionClasses>(&Field::key(name).index(index));
                    problems.push(location, Problem::UnknownContest(id.clone()));
                }
                found
            })
            .collect();
        resolved.into_iter().collect::<Option<BTreeSet<_>>>()
    };
    let national_teams = class("national_teams", &file.national_teams);
    let continental = class("continental", &file.continental);
    Some(Classes {
        national_teams: national_teams?,
        continental: continental?,
    })
}

/// What `id` names: a competition's own record, or the copy of the career contest whose copy
/// it names.
fn contest(id: &SymbolId, ids: &Ids, career: &[CareerContest]) -> Option<(CompetitionRef, Source)> {
    competition_ref(ids, id)
        .map(|competition| (competition, Source::Record))
        .or_else(|| {
            career
                .iter()
                .find(|contest| contest.copy.as_ref() == Some(id))
                .and_then(|contest| contest.competition)
                .map(|competition| (competition, Source::CareerCopy))
        })
}
