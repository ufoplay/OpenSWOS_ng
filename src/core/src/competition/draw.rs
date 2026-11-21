//! The draws (docs/PLAN.md M5 #3): the shuffle every draw makes, and the seeded draw that
//! orders teams by value into pots first (upstream diy_competition.c `DIY_ShuffleTeams`,
//! `DIY_ShuffleTeamOrderList`, `DIY_ShuffleTeamGridPairs`, `DIY_ShuffleLeagueTeamOrder`,
//! `DIY_SortAndShuffleTeamsByValue`). They draw from the second generator, in upstream's
//! order.

use openswos_rng::Rng;
use thiserror::Error;

/// How far a draw is shifted down: `Rand2()` gives 0 to 255, so `draw × n >> 8` is a place
/// below `n`.
const DRAW_SHIFT: u32 = u8::BITS;

/// The pots of a knockout's seeded draw: four, each a quarter of the teams.
const QUARTERS: usize = 4;

/// The most teams a quarter holds for a knockout's seeded draw to leave its pots unshuffled
/// (upstream returns before shuffling when a quarter is two teams or fewer).
const UNSHUFFLED_QUARTER: usize = 2;

/// A place below `count`, drawn from the second generator (`Rand2() × count >> 8`).
fn draw_place(count: usize, rng: &mut Rng) -> usize {
    let count = u32::try_from(count).expect("a draw's places fit a word");
    usize::try_from((u32::from(rng.rand2()) * count) >> DRAW_SHIFT).expect("below the count")
}

/// Shuffles `items` as every draw of upstream's does: from the last place to the first, each
/// swapped with a place drawn below their number.
pub fn shuffle<T>(items: &mut [T], rng: &mut Rng) {
    for place in (0..items.len()).rev() {
        let other = draw_place(items.len(), rng);
        items.swap(place, other);
    }
}

/// The places `0` to `count - 1`, shuffled (upstream `DIY_ShuffleTeamOrderList`). Faithful to
/// the original: the places are bytes, numbered on past 255 by wrapping.
#[must_use]
pub fn shuffled_order(count: usize, rng: &mut Rng) -> Vec<u8> {
    let mut order: Vec<u8> = std::iter::successors(Some(0u8), |place| Some(place.wrapping_add(1)))
        .take(count)
        .collect();
    shuffle(&mut order, rng);
    order
}

/// How a seeded draw cuts its teams, ordered by value, into pots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pots {
    /// A stage of groups (upstream `isLeagueFormat`): a pot for each place of a group, each
    /// of as many teams as there are groups, so every group draws one from each.
    Groups {
        /// Its groups.
        groups: u16,
        /// Each group's teams.
        per_group: u16,
    },
    /// A knockout: four pots, each a quarter of the teams; none shuffled when a quarter is
    /// two teams or fewer.
    Quarters,
}

/// Pots that hold more teams than the draw has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the pots hold {pots} teams, the draw {teams}")]
pub struct PotsPastTeams {
    /// The teams the pots hold.
    pub pots: usize,
    /// The draw's.
    pub teams: usize,
}

/// A seeded draw (upstream `DIY_SortAndShuffleTeamsByValue`): `order` (teams by index into
/// `values`) ordered by value, the highest first (a bubble sort: equal values keep their
/// order), then each of `pots` shuffled.
///
/// # Errors
///
/// [`PotsPastTeams`] when the pots hold more teams than `order` (upstream would shuffle past
/// them).
///
/// # Panics
///
/// When a team of `order` has no value in `values`.
pub fn seed_by_value(
    order: &mut [u8],
    values: &[u16],
    pots: Pots,
    rng: &mut Rng,
) -> Result<(), PotsPastTeams> {
    let (count, size) = match pots {
        Pots::Groups { groups, per_group } => (usize::from(per_group), usize::from(groups)),
        Pots::Quarters => (QUARTERS, order.len() >> 2),
    };
    if count * size > order.len() {
        return Err(PotsPastTeams {
            pots: count * size,
            teams: order.len(),
        });
    }
    for pass in (0..order.len().saturating_sub(1)).rev() {
        for place in 0..=pass {
            if values[usize::from(order[place])] < values[usize::from(order[place + 1])] {
                order.swap(place, place + 1);
            }
        }
    }
    if matches!(pots, Pots::Quarters) && size <= UNSHUFFLED_QUARTER || size == 0 {
        return Ok(());
    }
    for pot in order.chunks_exact_mut(size).take(count) {
        shuffle(pot, rng);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use openswos_rng::Generator;

    use super::*;

    fn rng() -> Rng {
        let mut rng = Rng::default();
        rng.secondary = Generator::new(17, 3);
        rng
    }

    /// The generators fixed at `value`, as a recording fixes them.
    fn fixed(value: u8) -> Rng {
        let mut rng = Rng::default();
        rng.fixed = Some(value);
        rng
    }

    #[test]
    fn a_shuffle_draws_once_a_place() {
        let mut items = [0, 1, 2, 3, 4, 5];
        let (mut ours, mut counted) = (rng(), rng());
        shuffle(&mut items, &mut ours);
        for _ in 0..items.len() {
            counted.rand2();
        }
        assert_eq!(ours, counted, "six draws");
        let mut sorted = items;
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2, 3, 4, 5], "the same items");
        let mut none: [u8; 0] = [];
        shuffle(&mut none, &mut ours);
    }

    #[test]
    fn a_fixed_draw_swaps_each_place_with_its_share() {
        // Rand2 fixed at 128: each place swaps with the middle (128 × 4 >> 8 = 2).
        let mut rng = fixed(128);
        let mut items = [0, 1, 2, 3];
        shuffle(&mut items, &mut rng);
        // 3↔2: 0 1 3 2; 2↔2; 1↔2: 0 3 1 2; 0↔2: 1 3 0 2.
        assert_eq!(items, [1, 3, 0, 2]);
        assert_eq!(shuffled_order(4, &mut rng), [1, 3, 0, 2]);
    }

    #[test]
    fn a_seeded_draw_orders_by_value_then_shuffles_each_pot() {
        let values = [5, 9, 9, 1, 7, 3, 8, 2];
        let mut order = [0, 1, 2, 3, 4, 5, 6, 7];
        // Rand2 at 0: each place swaps with the pot's first.
        seed_by_value(
            &mut order,
            &values,
            Pots::Groups {
                groups: 2,
                per_group: 4,
            },
            &mut fixed(0),
        )
        .expect("4 pots of 2");
        // By value: 1 2 (9, 9: kept) 6 4 0 5 7 3; each pot of two swapped.
        assert_eq!(order, [2, 1, 4, 6, 5, 0, 3, 7]);
    }

    #[test]
    fn a_knockout_s_small_quarters_are_not_shuffled() {
        let values = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut order = [0, 1, 2, 3, 4, 5, 6, 7];
        let mut rng = rng();
        let before = rng.clone();
        seed_by_value(&mut order, &values, Pots::Quarters, &mut rng).expect("quarters of 2");
        assert_eq!(order, [7, 6, 5, 4, 3, 2, 1, 0], "ordered only");
        assert_eq!(rng, before, "no draw");
        let mut twelve: Vec<u8> = (0..12).collect();
        let values: Vec<u16> = (0..12).collect();
        seed_by_value(&mut twelve, &values, Pots::Quarters, &mut rng).expect("quarters of 3");
        assert_ne!(rng, before, "quarters of 3 shuffled");
    }

    #[test]
    fn pots_past_the_teams_are_refused() {
        let mut order = [0, 1, 2];
        let error = seed_by_value(
            &mut order,
            &[0, 0, 0],
            Pots::Groups {
                groups: 2,
                per_group: 2,
            },
            &mut rng(),
        )
        .expect_err("4 pot places, 3 teams");
        assert_eq!(error, PotsPastTeams { pots: 4, teams: 3 });
        assert_eq!(error.to_string(), "the pots hold 4 teams, the draw 3");
    }
}
