//! The round robins of a package's `rules/fixtures.json`: each size's, as upstream's table
//! points at them (`_fixtureSchedulePtrTable`, read through the oracle), and the European
//! Championships' own (`_euroChampFixtureData`), which upstream picks by competition id (src/
//! diy_competition.c `getCurrentFixtureTeamPtrs`): here, data that names the competitions.

use std::collections::BTreeMap;

use openswos_competitions_json_resource::{CompetitionsDocument, CompetitionsFile, SymbolId};
use openswos_oracle::tables::FixtureSchedule;
use openswos_world_rules_resource::{CompetitionSchedule, Schedule};

use crate::error::ExtractError;

/// The competitions that play the European Championships' round robin, by upstream's ids
/// (include/comp_identity.h `COMPID_europeanChampionships2`, `COMPID_europeanChampionships`;
/// diy_competition.c `getCurrentFixtureTeamPtrs` checks them).
const EURO_CHAMPIONSHIP_COMP_IDS: [u8; 2] = [21, 22];

/// The tournaments' file.
const TOURNAMENTS: &str = "tournaments.json";

/// The fewest clubs a group plays a round robin with: a tournament's stages past its final
/// hold its winner, one group of one club.
const MIN_GROUP_CLUBS: u8 = 2;

/// Each of upstream's round robins as the package writes it.
pub(crate) fn schedules(tables: &[FixtureSchedule]) -> Vec<Schedule> {
    tables
        .iter()
        .map(|table| Schedule {
            teams: table.teams,
            rounds: table.rounds,
            pairs: table
                .pairs
                .iter()
                .map(|&(home, away)| [home, away])
                .collect(),
        })
        .collect()
}

/// The European Championships' round robin: played by the tournaments of `competitions` with
/// upstream's ids, its clubs their groups', its pairs the first of `bytes`
/// (`_euroChampFixtureData`), whether it plays real rounds as their size's in `schedules`.
///
/// # Errors
///
/// When no tournament has one of the ids, their groups are not all of one size, `bytes` holds
/// fewer pairs than a round robin of that size, or `schedules` has none of it.
pub(crate) fn euro_championships(
    competitions: &BTreeMap<String, CompetitionsDocument>,
    bytes: &[u8],
    schedules: &[Schedule],
) -> Result<CompetitionSchedule, ExtractError> {
    let Some(CompetitionsFile::Tournaments(file)) =
        competitions.get(TOURNAMENTS).map(|document| &document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(TOURNAMENTS));
    };
    let mut ids = Vec::new();
    let mut sizes = Vec::new();
    for comp_id in EURO_CHAMPIONSHIP_COMP_IDS {
        let tournament = file
            .competitions
            .iter()
            .find(|tournament| tournament.comp.comp_id.byte() == comp_id)
            .ok_or(ExtractError::NoCompetitionOfId(comp_id))?;
        ids.push(tournament.id.clone());
        sizes.extend(
            tournament
                .rounds
                .iter()
                .filter(|round| round.group_count.byte() != 0)
                .map(|round| round.team_group_count.byte())
                .filter(|&size| size >= MIN_GROUP_CLUBS),
        );
    }
    let teams = match sizes.as_slice() {
        [first, rest @ ..] if rest.iter().all(|size| size == first) => *first,
        [] | [_, ..] => return Err(ExtractError::GroupSizes(ids_text(&ids))),
    };
    let held = bytes.len() / 2;
    let needed = usize::from(teams) * usize::from(teams.saturating_sub(1)) / 2;
    let pairs: Vec<[u8; 2]> = bytes.as_chunks().0.iter().take(needed).copied().collect();
    if pairs.len() < needed {
        return Err(ExtractError::ShortSchedule { needed, held });
    }
    let rounds = schedules
        .iter()
        .find(|schedule| schedule.teams == teams)
        .map(|schedule| schedule.rounds)
        .ok_or(ExtractError::NoSchedule(teams))?;
    Ok(CompetitionSchedule {
        competitions: ids,
        schedule: Schedule {
            teams,
            rounds,
            pairs,
        },
    })
}

/// `ids` for a message.
fn ids_text(ids: &[SymbolId]) -> String {
    ids.iter()
        .map(SymbolId::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use openswos_test_utils::world_1996_97_path;

    use super::*;
    use crate::assemble::committed;

    /// `_euroChampFixtureData` as upstream defines it.
    const EURO_CHAMPIONSHIP_FIXTURES: [u8; 28] = [
        1, 0, 3, 2, 2, 1, 0, 3, 1, 3, 0, 2, 5, 10, 15, 20, 25, 30, 34, 38, 43, 48, 53, 58, 62, 66,
        71, 76,
    ];

    fn base() -> BTreeMap<String, CompetitionsDocument> {
        committed(&world_1996_97_path())
            .expect("the committed package")
            .competitions
    }

    fn four(rounds: bool) -> Schedule {
        Schedule {
            teams: 4,
            rounds,
            pairs: Vec::new(),
        }
    }

    #[test]
    fn a_table_is_its_pairs() {
        let written = schedules(&[FixtureSchedule {
            teams: 3,
            rounds: false,
            pairs: vec![(0, 1), (2, 0), (1, 2)],
        }]);
        assert_eq!(
            written,
            [Schedule {
                teams: 3,
                rounds: false,
                pairs: vec![[0, 1], [2, 0], [1, 2]],
            }]
        );
    }

    #[test]
    fn the_european_championships_play_their_groups_own_order() {
        let schedule = euro_championships(&base(), &EURO_CHAMPIONSHIP_FIXTURES, &[four(true)])
            .expect("a schedule");
        let ids: Vec<&str> = schedule.competitions.iter().map(SymbolId::as_str).collect();
        assert_eq!(ids, ["_europeanChampionships2", "_europeanChampionships"]);
        assert_eq!(schedule.schedule.teams, 4, "groups of 4");
        assert!(schedule.schedule.rounds);
        assert_eq!(
            schedule.schedule.pairs,
            [[1, 0], [3, 2], [2, 1], [0, 3], [1, 3], [0, 2]],
            "6 pairs, the data after them not"
        );
    }

    #[test]
    fn what_does_not_fit_is_refused() {
        let error = euro_championships(&base(), &EURO_CHAMPIONSHIP_FIXTURES[..10], &[four(true)])
            .expect_err("5 pairs");
        assert!(matches!(
            error,
            ExtractError::ShortSchedule { needed: 6, held: 5 }
        ));
        let error =
            euro_championships(&base(), &EURO_CHAMPIONSHIP_FIXTURES, &[]).expect_err("no size");
        assert!(matches!(error, ExtractError::NoSchedule(4)));
        let error = euro_championships(&BTreeMap::new(), &EURO_CHAMPIONSHIP_FIXTURES, &[])
            .expect_err("no file");
        assert!(matches!(error, ExtractError::NoCompetitionFile(_)));
    }
}
