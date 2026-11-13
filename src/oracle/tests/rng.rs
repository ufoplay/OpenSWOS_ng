//! `openswos_rng` against upstream's src/random.c (and `Randomize2`, src/game_utils.c): the
//! same table, the same draws in any order of calls, the same halt when the save stack
//! fills. Without upstream's checkout the oracle is not built and these tests are not either
//! (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

use openswos_oracle::Oracle;
use openswos_oracle::rng::{NothingSaved, Rand2SaveError, RandState, SAVED_RAND_STATE_LEN};
use openswos_rng::{
    Generator, RANDOM_TABLE, RANDOM_TABLE_LEN, Rng, SAVE_STACK_FULL_REASON, SAVE_STACK_LEN,
    SaveStackFull,
};
use proptest::collection::vec;
use proptest::prelude::*;

/// Upstream's 257th table byte, which nothing reads (src/swos_data_bulk_imports.c).
const UNREAD_TABLE_BYTE: u8 = 26;

/// The calls of a sequence.
const CALLS: usize = 64;

/// The bytes a save takes on upstream's stack.
const SAVE_BYTES: usize = SAVED_RAND_STATE_LEN / SAVE_STACK_LEN;

/// How often a sequence draws, compared with the other calls.
const DRAW_WEIGHT: u32 = 4;

/// A call to both sides.
#[derive(Debug, Clone, Copy)]
enum Call {
    Rand,
    Rand2,
    RandomWord,
    Randomize2 { value: u16, random_seed: u16 },
    Save,
    Restore,
    Fix(Option<u8>),
}

fn state() -> impl Strategy<Value = RandState> {
    (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(seed, xor_index, xor_key)| RandState {
        seed,
        xor_index,
        xor_key,
    })
}

fn call() -> impl Strategy<Value = Call> {
    prop_oneof![
        DRAW_WEIGHT => Just(Call::Rand),
        DRAW_WEIGHT => Just(Call::Rand2),
        1 => Just(Call::RandomWord),
        1 => (any::<u16>(), any::<u16>())
            .prop_map(|(value, random_seed)| Call::Randomize2 { value, random_seed }),
        1 => Just(Call::Save),
        1 => Just(Call::Restore),
        1 => any::<Option<u8>>().prop_map(Call::Fix),
    ]
}

fn generator(state: RandState) -> Generator {
    let RandState {
        seed,
        xor_index,
        xor_key,
    } = state;
    Generator {
        seed,
        xor_index,
        xor_key,
    }
}

/// Both sides at the same state.
fn both(primary: RandState, secondary: RandState) -> (Oracle, Rng) {
    let mut oracle = Oracle::lock();
    oracle.set_rand_state(primary);
    oracle.set_rand2_state(secondary);
    let mut rng = Rng::default();
    rng.primary = generator(primary);
    rng.secondary = generator(secondary);
    (oracle, rng)
}

/// Whether both sides are at the same state.
fn same_state(oracle: &Oracle, rng: &Rng) -> bool {
    generator(oracle.rand_state()) == rng.primary
        && generator(oracle.rand2_state()) == rng.secondary
        && oracle.saved_rand2_bytes() == rng.saved().len() * SAVE_BYTES
}

/// A save on both sides: whether the game goes on (upstream did not halt).
fn save(oracle: &mut Oracle, rng: &mut Rng) -> Result<bool, TestCaseError> {
    match (oracle.save_rand2_state(), rng.save_rand2_state()) {
        (Ok(()), Ok(())) => Ok(true),
        (Err(Rand2SaveError::Halted(SAVE_STACK_FULL_REASON)), Err(SaveStackFull)) => Ok(false),
        (upstream, port) => Err(TestCaseError::fail(format!(
            "a save: upstream {upstream:?}, the port {port:?}"
        ))),
    }
}

#[test]
fn the_table_is_upstreams() {
    let oracle = Oracle::lock();
    let table = oracle.random_table();
    let (read, unread) = table.split_at(RANDOM_TABLE_LEN);
    assert_eq!(read, RANDOM_TABLE.as_slice());
    assert_eq!(unread, [UNREAD_TABLE_BYTE].as_slice());
}

proptest! {
    #[test]
    fn the_port_draws_as_upstream(
        primary in state(),
        secondary in state(),
        calls in vec(call(), 0..=CALLS),
    ) {
        let (mut oracle, mut rng) = both(primary, secondary);
        for call in calls {
            match call {
                Call::Rand => prop_assert_eq!(rng.rand(), oracle.rand()),
                Call::Rand2 => prop_assert_eq!(rng.rand2(), oracle.rand2()),
                Call::RandomWord => prop_assert_eq!(rng.random_word(), oracle.random_word()),
                Call::Randomize2 { value, random_seed } => {
                    rng.randomize2(value, random_seed);
                    oracle.randomize2(value, random_seed);
                }
                Call::Save => {
                    if !save(&mut oracle, &mut rng)? {
                        // Upstream halted the game: nothing follows.
                        prop_assert!(same_state(&oracle, &rng));
                        return Ok(());
                    }
                }
                Call::Restore => {
                    let upstream = oracle.restore_rand2_state();
                    let port = rng.restore_rand2_state();
                    prop_assert_eq!(upstream.is_ok(), port.is_ok());
                }
                Call::Fix(value) => {
                    rng.fixed = value;
                    oracle.set_fixed_random(value);
                }
            }
            prop_assert!(same_state(&oracle, &rng), "after {:?}", call);
        }
    }
}

#[test]
fn the_save_that_fills_the_stack_halts_both() {
    let (mut oracle, mut rng) = both(RandState::default(), RandState::default());
    for _ in 1..SAVE_STACK_LEN {
        assert!(save(&mut oracle, &mut rng).expect("the same answer"));
        oracle.rand2();
        rng.rand2();
    }
    // The halt unwound back through the C.
    assert!(!save(&mut oracle, &mut rng).expect("the same answer"));
    assert!(same_state(&oracle, &rng));
    // The saves come back last first, on both.
    for _ in 0..SAVE_STACK_LEN {
        assert_eq!(oracle.restore_rand2_state(), Ok(()));
        assert_eq!(rng.restore_rand2_state(), Ok(()));
        assert!(same_state(&oracle, &rng));
    }
    assert_eq!(oracle.restore_rand2_state(), Err(NothingSaved));
}

#[test]
fn the_lock_resets_the_globals() {
    {
        let state = RandState {
            seed: 1,
            xor_index: 1,
            xor_key: 1,
        };
        let (mut oracle, _) = both(state, state);
        oracle.set_fixed_random(Some(1));
        assert_eq!(oracle.save_rand2_state(), Ok(()));
    }
    let mut oracle = Oracle::lock();
    let mut rng = Rng::default();
    assert!(same_state(&oracle, &rng));
    assert_eq!((oracle.rand(), oracle.rand2()), (rng.rand(), rng.rand2()));
}
