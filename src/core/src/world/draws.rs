//! The world's own draws (`rules/draws.json`): the tournaments whose stages are drawn by their
//! own tables, which upstream picks by competition id (diy_competition.c
//! `buildCompSpecificTeamList`, `buildCompSpecificTeamShuffle`).

use std::collections::BTreeMap;

use openswos_world_rules_resource::Draws;

use super::competitions::TournamentRef;
use super::ids::Ids;
use super::rules::at;
use crate::validation::{Expected, Field, Kind, Problem, Problems};

/// The most places a first stage's own draw keeps (upstream copies them through its draw's
/// 64-byte scratch).
const MAX_PLACES: u8 = 64;

/// Each tournament of `file`'s own draws, with the places its first stage keeps: `None` (each
/// reported) when an id is no tournament, a tournament is named again, or a draw keeps no
/// place or more than 64.
pub(crate) fn own_draws(
    file: &Draws,
    ids: &Ids,
    problems: &mut Problems,
) -> Option<BTreeMap<TournamentRef, u8>> {
    let mut draws = BTreeMap::new();
    let mut named_at = BTreeMap::new();
    let mut valid = true;
    for (index, draw) in file.own_draws.iter().enumerate() {
        let field = Field::key("own_draws").index(index);
        if !(1..=MAX_PLACES).contains(&draw.first_stage) {
            problems.push(
                at::<Draws>(&field.then("first_stage")),
                Problem::DrawPlaces(draw.first_stage),
            );
            valid = false;
        }
        for (named, id) in draw.competitions.iter().enumerate() {
            let location = at::<Draws>(&field.then("competitions").index(named));
            let Some(target) = ids.resolve(id, Expected::TOURNAMENT, || location.clone(), problems)
            else {
                valid = false;
                continue;
            };
            debug_assert_eq!(target.kind, Kind::Tournament);
            let tournament = TournamentRef(target.index);
            if let Some(first) = named_at.insert(tournament, location.clone()) {
                problems.push(location, Problem::RepeatedDraw { first });
                valid = false;
                continue;
            }
            draws.insert(tournament, draw.first_stage);
        }
    }
    valid.then_some(draws)
}
