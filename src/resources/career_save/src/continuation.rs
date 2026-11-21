//! What a career save keeps to go on exactly (upstream `[career] exact_continuation`,
//! src/save_format_json.c): the first random generator, the clocks at the career menu the
//! save was made at, and the European qualifying draws' seed groups as the season ends'
//! shuffles left them. Written only with the setting on: a save is otherwise as before.
//!
//! Kept as written, in the fields' C widths.

use serde::{Deserialize, Serialize};

/// The state a career needs to go on exactly (the second generator is in the scalars:
/// `seed_2`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactContinuation {
    /// The first generator's seed (upstream `_seed`).
    pub seed: u8,
    /// Its XOR table's index (upstream `_randXorIndex`).
    pub rand_xor_index: u8,
    /// Its XOR key (upstream `_randXorKey`).
    pub rand_xor_key: u8,
    /// The clock at the career menu the save was made at (upstream `_currentTick`).
    pub current_tick: u16,
    /// The game clock there (upstream `_currentGameTick`).
    pub current_game_tick: u16,
    /// The European qualifying draws' seed groups (upstream's three seed-group tables, their
    /// groups in order), each its bytes: the season end shuffles them in place.
    pub euro_seed_groups: Vec<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};

    #[test]
    fn the_state_round_trips() {
        let state: ExactContinuation = round_trip(
            r#"{"seed":7,"rand_xor_index":3,"rand_xor_key":255,"current_tick":65535,"current_game_tick":12,"euro_seed_groups":[[4,6,11],[],[15]]}"#,
        );
        assert_eq!(state.euro_seed_groups.len(), 3);
        assert!(rejects::<ExactContinuation>(
            r#"{"seed":256,"rand_xor_index":3,"rand_xor_key":255,"current_tick":0,"current_game_tick":0,"euro_seed_groups":[]}"#
        ));
    }
}
