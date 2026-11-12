//! A world's club pools below the leagues: the team pools (upstream `TeamPoolState`,
//! include/team_pool.h) and the state championships' pools (upstream `RI_STATE`,
//! include/state_pool.h, include/states.h). Saved only by a career in a world with them.
//!
//! What the numbers mean is the world's competitions' (M5); they are kept as written, in
//! their fields' C widths. Lists keep the length written.

use serde::{Deserialize, Serialize};

/// A pooled country's state (upstream `TeamPoolState`): which club each swapped slot of
/// its team files now holds, and the clubs queued below.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamPool {
    /// The country.
    pub country: u8,
    /// The identities' scheme: 2 (a pool club is the slots' count + its index). Absent in
    /// saves made before it (a pool club was 256 + its index); upstream's loader converts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ids: Option<u8>,
    /// The clubs queued below (at most 64 read).
    pub queue: Vec<u16>,
    /// Per slot of the country's team files, its identity + 1 (0: its own record). The
    /// writer leaves out the zeros past the first 256.
    pub map: Vec<u16>,
}

/// The state championships' pools and results (upstream `RI_STATE`, `g_stateSeasons`,
/// `g_stateChampions`, `g_stateCup`, `g_stateRank0`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatePools {
    /// The season ends simulated.
    pub seasons: u16,
    /// Per team file, whether its pool started (8).
    pub started: Vec<u8>,
    /// Per pool, its queue's length (20).
    pub queued: Vec<u8>,
    /// The queues (192).
    pub queues: Vec<u16>,
    /// Per pool, its contest's overrides (20 × 16).
    #[serde(with = "crate::flat")]
    pub overrides: Vec<[u16; 16]>,
    /// The identities (640).
    pub identities: Vec<u16>,
    /// Last season's champion of each state (13; 0: none).
    pub champions: Vec<u16>,
    /// The state the career club plays (255: none).
    pub played_state: u8,
    /// Its played ranking (36 bytes).
    pub played_ranking: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::round_trip;

    #[test]
    fn a_team_pool_round_trips_with_or_without_its_scheme() {
        let pool: TeamPool =
            round_trip(r#"{"country":20,"ids":2,"queue":[512,513],"map":[0,513,0]}"#);
        assert_eq!(pool.ids, Some(2));
        assert_eq!(pool.queue, vec![512, 513]);
        let old: TeamPool = round_trip(r#"{"country":20,"queue":[256],"map":[]}"#);
        assert_eq!(old.ids, None);
    }

    #[test]
    fn the_state_pools_round_trip() {
        let pools: StatePools = round_trip(&format!(
            r#"{{"seasons":2,"started":[1,0],"queued":[3],"queues":[7,8,9],"overrides":[{}],"identities":[5],"champions":[0,77],"played_state":255,"played_ranking":[1,2]}}"#,
            ["4"; 16].join(",")
        ));
        assert_eq!(pools.overrides, vec![[4; 16]]);
        assert_eq!(pools.played_state, 255);
    }
}
