#![doc = include_str!("../readme.md")]

#[cfg(not(oracle_unavailable))]
pub mod cup;
#[cfg(not(oracle_unavailable))]
pub mod cup_play;
#[cfg(not(oracle_unavailable))]
pub mod draw;
#[cfg(not(oracle_unavailable))]
mod ffi;
#[cfg(not(oracle_unavailable))]
pub mod knockouts;
#[cfg(not(oracle_unavailable))]
pub mod league;
#[cfg(not(oracle_unavailable))]
pub mod math;
#[cfg(not(oracle_unavailable))]
pub mod play;
#[cfg(not(oracle_unavailable))]
pub mod rng;
#[cfg(not(oracle_unavailable))]
pub mod rules_library;
#[cfg(not(oracle_unavailable))]
pub mod running;
#[cfg(not(oracle_unavailable))]
pub mod sim;
#[cfg(not(oracle_unavailable))]
pub mod sim_play;
#[cfg(not(oracle_unavailable))]
pub mod sim_result;
#[cfg(not(oracle_unavailable))]
pub mod stage;
#[cfg(not(oracle_unavailable))]
pub mod style;
#[cfg(not(oracle_unavailable))]
pub mod table;
pub mod tables;
#[cfg(not(oracle_unavailable))]
pub mod tournament;
pub mod unpack;

#[cfg(not(oracle_unavailable))]
pub use oracle::Oracle;

/// The lock on upstream's globals.
#[cfg(not(oracle_unavailable))]
mod oracle {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use crate::ffi;

    /// One test at a time: upstream's C globals are the process's.
    static LOCK: Mutex<()> = Mutex::new(());

    /// Whether a world's baked install ran in this process: it wrote 96/97's tables for good.
    static WORLD_INSTALLED: AtomicBool = AtomicBool::new(false);

    /// Records that a world's baked install ran (`tables::world_2627`).
    pub(crate) fn mark_world_installed() {
        WORLD_INSTALLED.store(true, Ordering::SeqCst);
    }

    /// Upstream's C, held: its globals reset when taken, its functions called through it.
    #[derive(Debug)]
    pub struct Oracle {
        _held: MutexGuard<'static, ()>,
    }

    impl Oracle {
        /// Takes the oracle, waiting for another test to let it go, and resets the globals
        /// its units touch to upstream's values.
        ///
        /// # Panics
        ///
        /// When a world's baked install ran in this process (`tables::world_2627`): 96/97's
        /// tables are gone from it, and no reset brings them back.
        #[must_use]
        pub fn lock() -> Self {
            assert!(
                !WORLD_INSTALLED.load(Ordering::SeqCst),
                "upstream's globals hold World 2026-27's baked install: this process serves \
                 nothing else (run it in a process of its own)"
            );
            // A test that failed while holding it leaves nothing the reset does not undo.
            let held = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
            ffi::reset();
            Self { _held: held }
        }
    }
}
