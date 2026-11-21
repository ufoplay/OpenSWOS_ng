//! src/diy_competition.c's teams' values and draws: `GetAveragePlayerPrice`,
//! `DIY_ComputeAverageTeamStrengthTier`, and the shuffles over `_diyCompetitionStart` and a
//! tournament's state.

use super::bindings::diy_competition_c::{self as diy, DIYTournamentState, TeamFileEx};
use super::stubs::SELECTED_TEAMS;
use crate::draw::{Prices, SeedPots};

/// The pairs `teamGridPairings` holds (64 places).
pub const GRID_PAIRS: usize = 32;

/// The places a tournament's order holds (`teamIndexOrder[64]`).
pub const TOURNAMENT_TEAMS: usize = 64;

/// The selected team of ordinal `ordinal`, which the stubbed loader gives.
pub(crate) fn loaded(ordinal: usize) -> *mut TeamFileEx {
    (&raw mut diy::_selectedTeams)
        .cast::<TeamFileEx>()
        .wrapping_add(ordinal)
}

/// Writes `prices` as `team`'s players' prices.
pub(crate) fn set_prices(team: *mut TeamFileEx, prices: &Prices) {
    // SAFETY: a plain write to a team record of a global the lock's holder owns.
    unsafe {
        for (player, &price) in (*team).players.iter_mut().zip(prices) {
            player.price = price.cast_signed();
        }
    }
}

/// Loads `teams`' prices as the teams of ordinals 0 onwards.
fn load(teams: &[Prices]) {
    assert!(teams.len() <= SELECTED_TEAMS, "the teams the stub holds");
    for (ordinal, prices) in teams.iter().enumerate() {
        set_prices(loaded(ordinal), prices);
    }
}

/// `GetAveragePlayerPrice` of a team with `prices`.
pub(crate) fn average_price(prices: &Prices) -> u16 {
    load(std::slice::from_ref(prices));
    // SAFETY: the function reads the team record it is given.
    unsafe { diy::GetAveragePlayerPrice(loaded(0)) }
}

/// `DIY_ComputeAverageTeamStrengthTier` of `teams` (ordinals 0 onwards, country 0): its
/// average and tier.
pub(crate) fn strength(teams: &[Prices]) -> (u16, u16) {
    load(teams);
    let table: Vec<u8> = (0..teams.len())
        .flat_map(|ordinal| [0, u8::try_from(ordinal).expect("64 teams")])
        .collect();
    let count = u16::try_from(teams.len()).expect("64 teams");
    // SAFETY: the function reads `count` big-endian team numbers from the table and the
    // loaded teams the stub gives.
    let packed =
        unsafe { diy::DIY_ComputeAverageTeamStrengthTier(count, table.as_ptr().cast_mut()) };
    let [average_high, average_low, tier_high, tier_low] = packed.to_be_bytes();
    (
        u16::from_be_bytes([average_high, average_low]),
        u16::from_be_bytes([tier_high, tier_low]),
    )
}

/// `DIY_ShuffleTeamOrderList` of `count` places.
pub(crate) fn shuffled_order(count: u16) -> Vec<u8> {
    // SAFETY: plain writes and reads of a global the lock's holder owns; the order holds 698
    // places.
    unsafe {
        let competition = &raw mut diy::_diyCompetitionStart;
        (*competition).base.matchPairsPerRound = count.cast_signed();
        diy::DIY_ShuffleTeamOrderList();
        let order = (&raw const (*competition).__bindgen_anon_1.teamShuffleOrder).cast::<u8>();
        std::slice::from_raw_parts(order, usize::from(count)).to_vec()
    }
}

/// `DIY_ShuffleTeamGridPairs` of `places` (two a pair).
pub(crate) fn shuffled_grid(places: &[u8]) -> Vec<u8> {
    assert!(
        places.len() <= 2 * GRID_PAIRS && places.len().is_multiple_of(2),
        "whole pairs the grid holds"
    );
    // SAFETY: plain writes and reads of a global the lock's holder owns, within the grid.
    unsafe {
        let competition = &raw mut diy::_diyCompetitionStart;
        (*competition).base.matchPairsPerRound = i16::try_from(places.len() / 2).expect("32 pairs");
        let grid = (&raw mut (*competition).teamGridPairings).cast::<u8>();
        grid.copy_from_nonoverlapping(places.as_ptr(), places.len());
        diy::DIY_ShuffleTeamGridPairs();
        std::slice::from_raw_parts(grid, places.len()).to_vec()
    }
}

/// `DIY_ShuffleLeagueTeamOrder` of a league's `places` (each its shuffle index and its row's
/// offset).
pub(crate) fn shuffled_league(places: &[(u8, u16)]) -> Vec<(u8, u16)> {
    assert!(
        places.len() <= TOURNAMENT_TEAMS,
        "the places a league holds"
    );
    // SAFETY: plain writes and reads of a global the lock's holder owns, within its 64 places;
    // the union's league view is the one a league uses.
    unsafe {
        let competition = &raw mut diy::_diyCompetitionStart;
        (*competition).base.numTeams = i16::try_from(places.len()).expect("64 places");
        let league = &raw mut (*competition).matchData.league;
        let indices = (&raw mut (*league).shuffleAndPad).cast::<u8>();
        let offsets = (&raw mut (*league).teamEntryOffsets).cast::<u16>();
        for (place, &(index, offset)) in places.iter().enumerate() {
            indices.add(place).write(index);
            offsets.add(place).write_unaligned(offset);
        }
        diy::DIY_ShuffleLeagueTeamOrder();
        (0..places.len())
            .map(|place| {
                (
                    indices.add(place).read(),
                    offsets.add(place).read_unaligned(),
                )
            })
            .collect()
    }
}

/// A tournament's state, its first slot holding `teams` teams in `order`.
fn tournament(order: &[u8]) -> Box<DIYTournamentState> {
    assert!(
        order.len() <= TOURNAMENT_TEAMS,
        "the places a tournament holds"
    );
    // SAFETY: a state of zeroed bytes is a valid one (its fields are integers and bytes).
    let mut state: Box<DIYTournamentState> = Box::new(unsafe { std::mem::zeroed() });
    state.slots[0].numTeams = u16::try_from(order.len()).expect("64 teams");
    state.teamIndexOrder[..order.len()].copy_from_slice(order);
    state
}

/// The state's order, its first `teams` places.
fn order_of(state: &DIYTournamentState, teams: usize) -> Vec<u8> {
    state.teamIndexOrder[..teams].to_vec()
}

/// `DIY_ShuffleTeams` of a tournament's first slot, its teams in `order`.
pub(crate) fn shuffled_teams(order: &[u8]) -> Vec<u8> {
    let mut state = tournament(order);
    let base = (&raw mut *state).cast::<u8>();
    // SAFETY: the function reads the first slot through `slot` (the state's base, slot 0) and
    // shuffles the state's order within its teams.
    unsafe { diy::DIY_ShuffleTeams(base, base) };
    order_of(&state, order.len())
}

/// `DIY_SortAndShuffleTeamsByValue` of a tournament's first slot: its teams in `order`, team
/// `i` (by index into the teams) the loaded team of ordinal `i` with `teams[i]`'s prices, cut
/// into `pots`.
pub(crate) fn seeded(order: &[u8], teams: &[Prices], pots: SeedPots) -> Vec<u8> {
    load(teams);
    let mut state = tournament(order);
    for (index, id) in state.teamIds.iter_mut().take(teams.len()).enumerate() {
        // Big-endian {country 0, ordinal}, as upstream reads them.
        *id = u16::from_ne_bytes([0, u8::try_from(index).expect("64 teams")]);
    }
    let slot = &mut state.slots[0];
    match pots {
        SeedPots::Groups { groups, per_group } => {
            slot.isLeagueFormat = 1;
            slot.groups = groups;
            slot.teamsPerGroup = per_group;
        }
        SeedPots::Quarters => slot.isLeagueFormat = 0,
    }
    let base = (&raw mut *state).cast::<u8>();
    // SAFETY: as `shuffled_teams`; the teams' numbers lead to the loaded teams the stub gives.
    unsafe { diy::DIY_SortAndShuffleTeamsByValue(base, base) };
    order_of(&state, order.len())
}
