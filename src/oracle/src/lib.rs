#![doc = include_str!("../readme.md")]

#[cfg(not(oracle_unavailable))]
mod ffi;
#[cfg(not(oracle_unavailable))]
pub mod math;
#[cfg(not(oracle_unavailable))]
pub mod rng;

#[cfg(not(oracle_unavailable))]
pub use oracle::Oracle;

/// The lock on upstream's globals.
#[cfg(not(oracle_unavailable))]
mod oracle {
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use crate::ffi;

    /// One test at a time: upstream's C globals are the process's.
    static LOCK: Mutex<()> = Mutex::new(());

    /// Upstream's C, held: its globals reset when taken, its functions called through it.
    #[derive(Debug)]
    pub struct Oracle {
        _held: MutexGuard<'static, ()>,
    }

    impl Oracle {
        /// Takes the oracle, waiting for another test to let it go, and resets the globals
        /// its units touch to upstream's values.
        #[must_use]
        pub fn lock() -> Self {
            // A test that failed while holding it leaves nothing the reset does not undo.
            let held = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
            ffi::reset();
            Self { _held: held }
        }
    }
}
