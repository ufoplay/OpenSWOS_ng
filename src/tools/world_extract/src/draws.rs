//! A world's own draws (docs/PLAN.md M5 #4d2, #7): `rules/draws.json`, the tournaments upstream
//! draws by its own tables, picked by competition id (src/diy_competition.c
//! `DIY_BuildTeamGridAndIndexMaps`, `DIY_ShuffleTeamsAndBuildCompTeamList`,
//! `buildCompSpecificTeamList`, `buildCompSpecificTeamShuffle`), and whose first stage's
//! qualifiers it ranks its own way (`DIY_BuildSeededBracketAndUpdateScores`,
//! `DIY_CopyTeamGridFromCompToDIY`: `DIY_AdjustWorldCupBracketScores`), each the world's
//! tournament records of the ids. The tables are upstream's compiled ones (`own_draw_tables`);
//! the counts its code copies are stated here.

use std::collections::BTreeMap;

use openswos_competitions_json_resource::{CompetitionsDocument, CompetitionsFile};
use openswos_oracle::tables::OwnDrawTables;
use openswos_world_rules_resource::{BestPlaced, Draws, OwnDraw, Pattern, Ranking, StageDraw};

use crate::error::ExtractError;

/// The tournaments' file.
const TOURNAMENTS: &str = "tournaments.json";

/// The World Cup's ids (include/comp_identity.h `COMPID_worldCup`).
const WORLD_CUP: &[u8] = &[4];

/// The European Championships' ids (`COMPID_europeanChampionships2`,
/// `COMPID_europeanChampionships`).
const EUROPEAN_CHAMPIONSHIPS: &[u8] = &[21, 22];

/// The places the World Cup's first stage keeps (`buildCompSpecificTeamList`'s first case
/// copies 24).
const WORLD_CUP_FIRST_STAGE: u8 = 24;

/// The places the European Championships' first stage keeps (`buildCompSpecificTeamShuffle`'s
/// first case copies 16).
const EUROPEAN_FIRST_STAGE: u8 = 16;

/// The places a final keeps (both builders' last case copies a word: two places).
const FINAL: u8 = 2;

/// The first of the World Cup's best placed teams: the place its pattern's groups are read
/// from (`teamIndexOrder[0xC]`), and what its map's `-1` entries add to a pattern's byte (12).
const BEST_PLACED_FIRST: u8 = 12;

/// The best placed a pattern orders: the four places read from [`BEST_PLACED_FIRST`], a
/// pattern's four bytes after its groups' bits.
const BEST_PLACED: usize = 4;

/// The bytes of a pattern of `_diyGroupPatternLookup`: its groups' bits, then its order.
const PATTERN_LEN: usize = 1 + BEST_PLACED;

/// The World Cup's semi-finals' order (diy_competition.c `diyCompSpecificShuffle4`: a table of
/// the code's own, upstream's fix of the original's `{2, 1, 0, 0}`, which read a code label
/// as data).
const WORLD_CUP_SEMI_FINALS: [u8; 4] = [2, 1, 0, 3];

/// The groups of a pattern's bits: each bit set, from bit 0.
fn groups(bits: u8) -> Vec<u8> {
    (0..u8::BITS)
        .filter(|&bit| bits >> bit & 1 != 0)
        .map(|bit| u8::try_from(bit).expect("a byte's bit"))
        .collect()
}

/// The patterns of `_diyGroupPatternLookup`, up to its end (a byte below 0 where a pattern's
/// groups would be).
fn patterns(table: &[u8]) -> Vec<Pattern> {
    let (patterns, _) = table.as_chunks::<PATTERN_LEN>();
    patterns
        .iter()
        .take_while(|[bits, ..]| bits.cast_signed() >= 0)
        .map(|[bits, order @ ..]| Pattern {
            groups: groups(*bits),
            order: order.to_vec(),
        })
        .collect()
}

/// The World Cup's best placed draw: its map's entries a place before, or (below 0) the next
/// of the matched pattern's.
fn best_placed(tables: &OwnDrawTables) -> BestPlaced {
    BestPlaced {
        first: BEST_PLACED_FIRST,
        patterns: patterns(&tables.group_patterns),
        places: tables
            .best_placed_map
            .iter()
            .map(|&place| u8::try_from(place).ok())
            .collect(),
    }
}

/// The World Cup's stages (`buildCompSpecificTeamList`'s cases) and its ranking.
fn world_cup(tables: &OwnDrawTables) -> (Ranking, Vec<StageDraw>) {
    (
        Ranking::ByGroup,
        vec![
            StageDraw::Keep(WORLD_CUP_FIRST_STAGE),
            StageDraw::BestPlaced(best_placed(tables)),
            StageDraw::Order(tables.tournament_order_8.clone()),
            StageDraw::Order(WORLD_CUP_SEMI_FINALS.to_vec()),
            StageDraw::Keep(FINAL),
        ],
    )
}

/// The European Championships' stages (`buildCompSpecificTeamShuffle`'s cases), ranked as the
/// engine ranks.
fn european_championships(tables: &OwnDrawTables) -> (Ranking, Vec<StageDraw>) {
    (
        Ranking::ByPlace,
        vec![
            StageDraw::Keep(EUROPEAN_FIRST_STAGE),
            StageDraw::Order(tables.order_8.clone()),
            StageDraw::Order(tables.order_4.clone()),
            StageDraw::Keep(FINAL),
        ],
    )
}

/// `rules/draws.json`: each own draw upstream's code names, its stages from upstream's
/// `tables`, its competitions the tournaments of `competitions` with its ids, in the ids'
/// order; a draw of no tournament left out.
///
/// # Errors
///
/// [`ExtractError::NoCompetitionFile`] when `competitions` has no tournaments.
pub(crate) fn draws(
    competitions: &BTreeMap<String, CompetitionsDocument>,
    tables: &OwnDrawTables,
) -> Result<Draws, ExtractError> {
    let Some(CompetitionsFile::Tournaments(file)) =
        competitions.get(TOURNAMENTS).map(|document| &document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(TOURNAMENTS));
    };
    let own_draws = [
        (WORLD_CUP, world_cup(tables)),
        (EUROPEAN_CHAMPIONSHIPS, european_championships(tables)),
    ]
    .into_iter()
    .map(|(comp_ids, (ranking, stages))| OwnDraw {
        competitions: comp_ids
            .iter()
            .flat_map(|&comp_id| {
                file.competitions
                    .iter()
                    .filter(move |tournament| tournament.comp.comp_id.byte() == comp_id)
                    .map(|tournament| tournament.id.clone())
            })
            .collect(),
        ranking,
        stages,
    })
    .filter(|draw| !draw.competitions.is_empty())
    .collect();
    Ok(Draws { own_draws })
}

#[cfg(test)]
mod tests {
    use openswos_common::Importer;
    use openswos_competitions_json_resource::{CompetitionsJsonFormat, SymbolId};

    use super::*;

    /// Tournaments of the ids 22, 4 and 0.
    fn competitions() -> BTreeMap<String, CompetitionsDocument> {
        let record = |id: &str, comp_id: u8| {
            format!(
                r#"{{"id": "{id}", "type": "tournament", "comp": {{"comp_id": {comp_id}, "type": 2, "team_number": -1, "begin_month": 64, "end_month": 32, "name_table_offset": 35, "reserved": 0, "team_list_offset": 41, "seed_reorder_flag": 1}},
                 "seeding_type": 1, "menu_state": 0, "play_each_team": 1, "away_goals_mode": 3, "substitutes": 3, "points_for_win": 5, "round_string_base": 1,
                 "rounds": [], "header_reserved": 0, "round_rule_flags": [], "name": [], "team_seed_list": []}}"#
            )
        };
        let text = format!(
            r#"{{"type": "competition_tournaments", "count": 3, "competitions": [{}, {}, {}]}}"#,
            record("_europeanChampionships", 22),
            record("_worldCup", 4),
            record("_euroCup", 0)
        );
        let document = CompetitionsJsonFormat::import(text.as_bytes()).expect("tournaments");
        BTreeMap::from([(TOURNAMENTS.to_owned(), document)])
    }

    /// Tables of two patterns, then the end.
    fn tables() -> OwnDrawTables {
        OwnDrawTables {
            order_8: vec![1, 0, 3, 2, 5, 4, 7, 6],
            order_4: vec![3, 2, 1, 0],
            tournament_order_8: vec![7, 6, 5, 4, 3, 2, 1, 0],
            group_patterns: vec![0b1111, 0, 1, 2, 3, 0b11_0000, 3, 2, 1, 0, 0xFF],
            best_placed_map: vec![-1, 3, -1, 5],
        }
    }

    #[test]
    fn the_own_draws_are_the_tournaments_of_their_ids_drawn_by_upstream_s_tables() {
        let draws = draws(&competitions(), &tables()).expect("draws");
        let id = |text: &str| SymbolId(text.to_owned());
        assert_eq!(
            draws.own_draws,
            [
                OwnDraw {
                    competitions: vec![id("_worldCup")],
                    ranking: Ranking::ByGroup,
                    stages: vec![
                        StageDraw::Keep(24),
                        StageDraw::BestPlaced(BestPlaced {
                            first: 12,
                            patterns: vec![
                                Pattern {
                                    groups: vec![0, 1, 2, 3],
                                    order: vec![0, 1, 2, 3],
                                },
                                Pattern {
                                    groups: vec![4, 5],
                                    order: vec![3, 2, 1, 0],
                                },
                            ],
                            places: vec![None, Some(3), None, Some(5)],
                        }),
                        StageDraw::Order(vec![7, 6, 5, 4, 3, 2, 1, 0]),
                        StageDraw::Order(vec![2, 1, 0, 3]),
                        StageDraw::Keep(2),
                    ],
                },
                OwnDraw {
                    competitions: vec![id("_europeanChampionships")],
                    ranking: Ranking::ByPlace,
                    stages: vec![
                        StageDraw::Keep(16),
                        StageDraw::Order(vec![1, 0, 3, 2, 5, 4, 7, 6]),
                        StageDraw::Order(vec![3, 2, 1, 0]),
                        StageDraw::Keep(2),
                    ],
                },
            ]
        );
    }

    #[test]
    fn a_world_without_tournaments_is_refused() {
        let error = draws(&BTreeMap::new(), &tables()).expect_err("no tournaments");
        assert!(matches!(
            error,
            ExtractError::NoCompetitionFile(TOURNAMENTS)
        ));
    }
}
