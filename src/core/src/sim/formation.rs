//! Each formation's outfield players by how defensive they are (docs/PLAN.md M5 #8c; upstream
//! src/menus.c `InitializeTacticsPositions`): a simulated result's stand-ins take their places
//! by it. A player's defensiveness sums, over the 20 ball areas of the half nearer his goal, how
//! far each of his spots is from the centre column or from the grid's far row, the larger.

use openswos_tac_resource::{BALL_AREA_COLUMNS, OUTFIELD_PLAYERS, TeamTactics};

use super::tables::{AREAS, ELEVEN};

/// The first ball area of the half nearer a team's goal.
const NEAR_HALF: usize = 15;

/// The pitch grid's centre column.
const CENTRE: i16 = 7;

/// The pitch grid's far row: a spot's distance from it counts.
const FAR_ROW: i16 = 15;

/// How far a spot's column is shifted in its byte.
const COLUMN_SHIFT: u8 = 4;

/// A nibble.
const NIBBLE: u8 = 0xF;

/// The formations whose orders a session reckons (upstream `_playersPositionsByTactics[18]`).
pub const FORMATIONS: usize = 18;

/// `tactics`' places by defensiveness (upstream `InitializeTacticsPositions` for one formation):
/// place 0 the goalkeeper's (0), then the outfield players' numbers (1 to 10), the most
/// defensive first (upstream's exchange sort, equals kept in their order).
#[must_use]
pub fn defensive_order(tactics: &TeamTactics) -> [u8; ELEVEN] {
    let mut order = [0u8; ELEVEN];
    let mut defensiveness = [0u16; ELEVEN];
    for player in 1..=OUTFIELD_PLAYERS {
        order[player] = u8::try_from(player).expect("ten players");
        let spots = &tactics.players[player - 1];
        defensiveness[player] = (NEAR_HALF..AREAS)
            .map(|area| {
                let spot = u8::from(spots.0[area / BALL_AREA_COLUMNS][area % BALL_AREA_COLUMNS]);
                let across = (i16::from(spot >> COLUMN_SHIFT & NIBBLE) - CENTRE).abs();
                let along = FAR_ROW - i16::from(spot & NIBBLE);
                u16::try_from(across.max(along)).expect("a distance on the grid")
            })
            .sum();
    }
    loop {
        let mut swapped = false;
        for place in 1..OUTFIELD_PLAYERS {
            if defensiveness[place] < defensiveness[place + 1] {
                defensiveness.swap(place, place + 1);
                order.swap(place, place + 1);
                swapped = true;
            }
        }
        if !swapped {
            return order;
        }
    }
}

/// The orders of the session's first 18 formations (`tactics`' presets and the user's; upstream
/// `InitializeTacticsPositions`, which the menus run at the start and when tactics are edited).
#[must_use]
pub fn formation_orders(tactics: &[TeamTactics]) -> Vec<[u8; ELEVEN]> {
    tactics
        .iter()
        .take(FORMATIONS)
        .map(defensive_order)
        .collect()
}

#[cfg(test)]
mod tests {
    use openswos_common::{FixedName, Tactics};
    use openswos_tac_resource::{PlayerSpots, Spot};

    use super::*;

    /// Tactics whose player `n` (1 to 10) always stands in column 7, row 11 - `n`.
    fn rows() -> TeamTactics {
        let spots = |row: u8| {
            PlayerSpots([[Spot::from(0x70 | row); BALL_AREA_COLUMNS]; AREAS / BALL_AREA_COLUMNS])
        };
        TeamTactics {
            name: FixedName::from_bytes([0; 9]),
            players: std::array::from_fn(|player| spots(u8::try_from(10 - player).expect("10"))),
            pairs: [None; OUTFIELD_PLAYERS],
            ball_out_of_play: Tactics::FourFourTwo,
        }
    }

    #[test]
    fn the_players_farthest_from_the_far_row_come_first() {
        // Player n stands in row 11 - n, 4 + n from the far row: the highest numbered the
        // farthest, the most defensive.
        assert_eq!(defensive_order(&rows()), [0, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn equals_keep_their_order() {
        let mut tactics = rows();
        let same = tactics.players[0];
        tactics.players = [same; OUTFIELD_PLAYERS];
        assert_eq!(
            defensive_order(&tactics),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
        assert_eq!(formation_orders(&[rows(), tactics]).len(), 2);
    }
}
