//! The world's own draws (`rules/draws.json`): the tournaments whose stages are drawn by their
//! own tables, which upstream picks by competition id (diy_competition.c
//! `buildCompSpecificTeamList`, `buildCompSpecificTeamShuffle`), and how their stages of groups
//! rank (`DIY_AdjustWorldCupBracketScores`).

use std::collections::BTreeMap;

use openswos_world_rules_resource as rules_file;
use openswos_world_rules_resource::Draws;

use super::competitions::TournamentRef;
use super::ids::Ids;
use super::rules::at;
use crate::competition::tournament::{
    BestPlaced, ORDER_PLACES, OwnDraw, PATTERN_GROUPS, Ranking, StageDraw,
};
use crate::validation::{Expected, Field, Kind, Problem, Problems};

/// Each tournament of `file`'s own draws, with its draw: `None` (each reported) when an id is
/// no tournament, a tournament is named again, or a draw is not valid ([`own_draw`]).
pub(crate) fn own_draws(
    file: &Draws,
    ids: &Ids,
    problems: &mut Problems,
) -> Option<BTreeMap<TournamentRef, OwnDraw>> {
    let mut draws = BTreeMap::new();
    let mut named_at = BTreeMap::new();
    let mut valid = true;
    for (index, draw) in file.own_draws.iter().enumerate() {
        let field = Field::key("own_draws").index(index);
        let own_draw = own_draw(draw, &field, problems);
        valid &= own_draw.is_some();
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
            if let Some(own_draw) = &own_draw {
                draws.insert(tournament, own_draw.clone());
            }
        }
    }
    valid.then_some(draws)
}

/// `draw` checked (at `field`): a stage at least; each stage's places 1 to 64, within the
/// order; a best placed draw's patterns of groups within a byte's bits, each ordering its best
/// placed. `None` (each problem reported) when it is not valid.
fn own_draw(draw: &rules_file::OwnDraw, field: &Field, problems: &mut Problems) -> Option<OwnDraw> {
    let stages_field = field.then("stages");
    if draw.stages.is_empty() {
        problems.push(at::<Draws>(&stages_field), Problem::NoStageDraws);
        return None;
    }
    let mut valid = true;
    let stages = draw
        .stages
        .iter()
        .enumerate()
        .map(|(stage, stage_draw)| {
            let checked = stage_draw_of(stage_draw, &stages_field.index(stage), problems);
            valid &= checked.is_some();
            checked
        })
        .collect::<Vec<_>>();
    let ranking = match draw.ranking {
        rules_file::Ranking::ByPlace => Ranking::ByPlace,
        rules_file::Ranking::ByGroup => Ranking::ByGroup,
    };
    valid.then(|| OwnDraw {
        ranking,
        stages: stages.into_iter().flatten().collect(),
    })
}

/// A place of the order checked (at `field`): within its 64.
fn place(place: u8, field: &Field, problems: &mut Problems) -> bool {
    let within = place < ORDER_PLACES;
    if !within {
        problems.push(at::<Draws>(field), Problem::DrawIndex(place));
    }
    within
}

/// A draw's length checked (at `field`): 1 to 64 places.
fn length(len: usize, field: &Field, problems: &mut Problems) -> bool {
    let within = (1..=usize::from(ORDER_PLACES)).contains(&len);
    if !within {
        problems.push(at::<Draws>(field), Problem::DrawLength(len));
    }
    within
}

/// A stage's draw checked (at `field`).
fn stage_draw_of(
    draw: &rules_file::StageDraw,
    field: &Field,
    problems: &mut Problems,
) -> Option<StageDraw> {
    match draw {
        rules_file::StageDraw::Keep(places) => {
            if (1..=ORDER_PLACES).contains(places) {
                Some(StageDraw::Keep(*places))
            } else {
                problems.push(
                    at::<Draws>(&field.then("keep")),
                    Problem::DrawPlaces(*places),
                );
                None
            }
        }
        rules_file::StageDraw::Order(order) => {
            let field = field.then("order");
            let mut valid = length(order.len(), &field, problems);
            for (index, &at) in order.iter().enumerate() {
                valid &= place(at, &field.index(index), problems);
            }
            valid.then(|| StageDraw::Order(order.clone()))
        }
        rules_file::StageDraw::BestPlaced(best) => {
            best_placed(best, &field.then("best_placed"), problems).map(StageDraw::BestPlaced)
        }
    }
}

/// A best placed draw checked (at `field`): its places within the order, its best placed (its
/// places of none) too from its first; each pattern's groups within a byte's bits, its order
/// ranking every best placed.
fn best_placed(
    best: &rules_file::BestPlaced,
    field: &Field,
    problems: &mut Problems,
) -> Option<BestPlaced> {
    let places_field = field.then("places");
    let mut valid = length(best.places.len(), &places_field, problems);
    for (index, at) in best.places.iter().enumerate() {
        if let Some(at) = *at {
            valid &= place(at, &places_field.index(index), problems);
        }
    }
    let count = best.places.iter().filter(|at| at.is_none()).count();
    if usize::from(best.first) + count > usize::from(ORDER_PLACES) {
        problems.push(
            at::<Draws>(&field.then("first")),
            Problem::BestPlacedPast {
                first: best.first,
                best: count,
            },
        );
        valid = false;
    }
    let mut patterns = Vec::with_capacity(best.patterns.len());
    for (index, pattern) in best.patterns.iter().enumerate() {
        let field = field.then("patterns").index(index);
        let mut bits = 0u8;
        for (group_index, &group) in pattern.groups.iter().enumerate() {
            if group < PATTERN_GROUPS {
                bits |= 1 << group;
            } else {
                problems.push(
                    at::<Draws>(&field.then("groups").index(group_index)),
                    Problem::PatternGroup(group),
                );
                valid = false;
            }
        }
        let order_field = field.then("order");
        if pattern.order.len() != count {
            problems.push(
                at::<Draws>(&order_field),
                Problem::PatternOrder {
                    order: pattern.order.len(),
                    best: count,
                },
            );
            valid = false;
        }
        for (rank_index, &rank) in pattern.order.iter().enumerate() {
            if usize::from(rank) >= count {
                problems.push(
                    at::<Draws>(&order_field.index(rank_index)),
                    Problem::BestPlacedRank { rank, best: count },
                );
                valid = false;
            }
        }
        patterns.push((bits, pattern.order.clone()));
    }
    valid.then(|| BestPlaced {
        first: best.first,
        patterns,
        places: best.places.clone(),
    })
}
