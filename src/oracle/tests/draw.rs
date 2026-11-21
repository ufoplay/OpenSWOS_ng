//! `openswos_core::competition::{draw, value}` against upstream's src/diy_competition.c: the
//! teams' values (`GetAveragePlayerPrice`, `DIY_ComputeAverageTeamStrengthTier`) and every
//! shuffle (`DIY_ShuffleTeamOrderList`, `DIY_ShuffleTeamGridPairs`,
//! `DIY_ShuffleLeagueTeamOrder`, `DIY_ShuffleTeams`, `DIY_SortAndShuffleTeamsByValue`), from
//! any state of the second generator: the same order, the same state after. Without upstream's
//! checkout the oracle is not built and these tests are not either (`tests/available.rs` says
//! so).
#![cfg(not(oracle_unavailable))]

use openswos_core::competition::draw::{Pots, seed_by_value, shuffle, shuffled_order};
use openswos_core::competition::value::{Prices, average_price, strength};
use openswos_oracle::Oracle;
use openswos_oracle::draw::{GRID_PAIRS, SeedPots, TOURNAMENT_TEAMS};
use openswos_oracle::rng::RandState;
use openswos_rng::{Generator, Rng};
use proptest::collection::vec;
use proptest::prelude::*;

/// The most places `DIY_ShuffleTeamOrderList` is given here: past 256, so the bytes wrap
/// (its order holds 698).
const MAX_ORDER: u16 = 300;

/// A state of the second generator.
fn state() -> impl Strategy<Value = RandState> {
    (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(seed, xor_index, xor_key)| RandState {
        seed,
        xor_index,
        xor_key,
    })
}

/// The port's generators, the second one at `state`.
fn rng(state: RandState) -> Rng {
    let mut rng = Rng::default();
    rng.secondary = Generator {
        seed: state.seed,
        xor_index: state.xor_index,
        xor_key: state.xor_key,
    };
    rng
}

/// The port's second generator's state.
fn state_of(rng: &Rng) -> RandState {
    RandState {
        seed: rng.secondary.seed,
        xor_index: rng.secondary.xor_index,
        xor_key: rng.secondary.xor_key,
    }
}

/// The oracle, its second generator at `state`.
fn oracle(state: RandState) -> Oracle {
    let mut oracle = Oracle::lock();
    oracle.set_rand2_state(state);
    oracle
}

/// A team's prices: small (equal values) or any byte.
fn prices() -> impl Strategy<Value = Prices> {
    prop_oneof![any::<[u8; 16]>(), (0u8..3).prop_map(|price| [price; 16]),]
}

/// A seeded draw: its teams' prices, their order (a permutation of them), its pots.
fn seeded_draw() -> impl Strategy<Value = (Vec<Prices>, Vec<u8>, SeedPots)> {
    (0..=TOURNAMENT_TEAMS).prop_flat_map(|teams| {
        let order = Just(
            (0..teams)
                .map(|team| u8::try_from(team).expect("64"))
                .collect::<Vec<_>>(),
        )
        .prop_shuffle();
        let teams_u16 = u16::try_from(teams).expect("64");
        let pots = prop_oneof![
            Just(SeedPots::Quarters),
            (0..=teams_u16).prop_flat_map(move |groups| {
                // No groups: any number a group, the pots holding none.
                let per_group = teams_u16.checked_div(groups).unwrap_or(teams_u16);
                (Just(groups), 0..=per_group)
                    .prop_map(|(groups, per_group)| SeedPots::Groups { groups, per_group })
            }),
        ];
        (vec(prices(), teams), order, pots)
    })
}

proptest! {
    #[test]
    fn a_team_s_value_is_upstream_s(prices in prices()) {
        prop_assert_eq!(average_price(&prices), Oracle::lock().average_price(&prices));
    }

    #[test]
    fn a_competition_s_strength_is_upstream_s(teams in vec(prices(), 1..=TOURNAMENT_TEAMS)) {
        let ours = strength(&teams).expect("teams");
        let (average, tier) = Oracle::lock().strength(&teams);
        prop_assert_eq!((ours.average, u16::from(ours.tier)), (average, tier));
    }

    #[test]
    fn an_order_is_shuffled_as_upstream_shuffles_it(state in state(), count in 0..=MAX_ORDER) {
        let mut oracle = oracle(state);
        let upstream = oracle.shuffled_order(count);
        let mut rng = rng(state);
        prop_assert_eq!(shuffled_order(usize::from(count), &mut rng), upstream);
        prop_assert_eq!(state_of(&rng), oracle.rand2_state());
    }

    #[test]
    fn the_grid_is_shuffled_as_upstream_shuffles_it(
        state in state(),
        places in (0..=GRID_PAIRS).prop_flat_map(|pairs| vec(any::<u8>(), 2 * pairs)),
    ) {
        let mut oracle = oracle(state);
        let upstream = oracle.shuffled_grid(&places);
        let (mut rng, mut ours) = (rng(state), places);
        shuffle(&mut ours, &mut rng);
        prop_assert_eq!(ours, upstream);
        prop_assert_eq!(state_of(&rng), oracle.rand2_state());
    }

    #[test]
    fn a_league_s_places_are_shuffled_as_upstream_shuffles_them(
        state in state(),
        places in vec((any::<u8>(), any::<u16>()), 0..=TOURNAMENT_TEAMS),
    ) {
        let mut oracle = oracle(state);
        let upstream = oracle.shuffled_league(&places);
        let (mut rng, mut ours) = (rng(state), places);
        shuffle(&mut ours, &mut rng);
        prop_assert_eq!(ours, upstream);
        prop_assert_eq!(state_of(&rng), oracle.rand2_state());
    }

    #[test]
    fn a_tournament_s_teams_are_shuffled_as_upstream_shuffles_them(
        state in state(),
        order in vec(any::<u8>(), 0..=TOURNAMENT_TEAMS),
    ) {
        let mut oracle = oracle(state);
        let upstream = oracle.shuffled_teams(&order);
        let (mut rng, mut ours) = (rng(state), order);
        shuffle(&mut ours, &mut rng);
        prop_assert_eq!(ours, upstream);
        prop_assert_eq!(state_of(&rng), oracle.rand2_state());
    }

    #[test]
    fn a_seeded_draw_is_upstream_s(state in state(), (teams, order, pots) in seeded_draw()) {
        let mut oracle = oracle(state);
        let upstream = oracle.seeded(&order, &teams, pots);
        let values: Vec<u16> = teams.iter().map(average_price).collect();
        let pots = match pots {
            SeedPots::Groups { groups, per_group } => Pots::Groups { groups, per_group },
            SeedPots::Quarters => Pots::Quarters,
        };
        let (mut rng, mut ours) = (rng(state), order);
        seed_by_value(&mut ours, &values, pots, &mut rng).expect("pots within the teams");
        prop_assert_eq!(ours, upstream);
        prop_assert_eq!(state_of(&rng), oracle.rand2_state());
    }
}
