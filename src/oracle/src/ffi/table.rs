//! src/diy_competition.c's league table: `DIY_SortLeagueTableByStandings` over the rows of
//! `_diyCompetitionStart`, the teams named in `_selectedTeams`.

use super::bindings::diy_competition_c;
use crate::table::{STANDINGS_ROWS, StandingsRow};

/// The bytes of a standings row (upstream `DIYTeamStats`): the offsets the order holds count
/// in them.
const ROW_BYTES: u16 = 18;

/// The bytes of a team's name field (`TeamFileEx.teamName`).
const NAME_BYTES: usize = 17;

/// `DIY_SortLeagueTableByStandings` over `rows` (row `i` the selected team `rows[i].team`) in
/// `groups` groups of `per_group`, the teams named `names` (by selected team): each place's
/// row, group after group.
pub(crate) fn sort(
    rows: &[StandingsRow],
    names: &[Vec<u8>],
    groups: u16,
    per_group: u16,
) -> Vec<usize> {
    let places = usize::from(groups) * usize::from(per_group);
    assert!(
        rows.len() <= STANDINGS_ROWS && places <= rows.len(),
        "the rows upstream holds"
    );
    // SAFETY: plain writes and reads of globals the lock's holder owns, each within its
    // array (64 rows, 64 places, 160 selected teams of 17-byte names); the union's league view
    // is the one a league uses.
    unsafe {
        let competition = &raw mut diy_competition_c::_diyCompetitionStart;
        let teams =
            (&raw mut diy_competition_c::_selectedTeams).cast::<diy_competition_c::TeamFileEx>();
        for (index, name) in names.iter().enumerate() {
            let field = (&raw mut (*teams.add(index)).team.teamName).cast::<u8>();
            field.write_bytes(0, NAME_BYTES);
            field.copy_from_nonoverlapping(name.as_ptr(), name.len().min(NAME_BYTES - 1));
        }
        let standings = (&raw mut (*competition).__bindgen_anon_2.leagueStandings)
            .cast::<diy_competition_c::DIYTeamStats>();
        for (index, row) in rows.iter().enumerate() {
            standings
                .add(index)
                .write_unaligned(diy_competition_c::DIYTeamStats {
                    teamSelOffset: row.team,
                    gamesPlayed: row.played,
                    wins: row.won,
                    draws: row.drawn,
                    losses: row.lost,
                    goalsFor: row.goals_for,
                    goalsAgainst: row.goals_against,
                    points: row.points,
                });
        }
        let order = (&raw mut (*competition).matchData.league.teamEntryOffsets).cast::<u16>();
        for (place, offset) in (0..places).zip((0..).step_by(usize::from(ROW_BYTES))) {
            order.add(place).write_unaligned(offset);
        }
        (*competition).base.matchPairsPerRound = groups.cast_signed();
        (*competition).base.totalMatchesInRound = per_group.cast_signed();
        diy_competition_c::DIY_SortLeagueTableByStandings();
        (0..places)
            .map(|place| usize::from(order.add(place).read_unaligned() / ROW_BYTES))
            .collect()
    }
}
