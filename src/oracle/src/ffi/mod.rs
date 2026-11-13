//! The C side, and the workspace's only `unsafe` (D3): upstream's units as bindgen declares
//! them, the Rust stubs of the externs they reference, and safe functions over both, one
//! module a unit. Every function here touches process-wide C globals: they are called only by
//! the holder of the oracle's lock ([`crate::Oracle`]).
#![allow(unsafe_code)]

pub(crate) mod math;
pub(crate) mod rng;
mod stubs;
pub(crate) mod style;

use std::hint::black_box;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use stubs::UpstreamPanic;

/// Upstream's units and headers, as bindgen declares them (`<file name>`, its dot an
/// underscore).
#[allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    missing_docs,
    clippy::all,
    clippy::pedantic
)]
mod bindings {
    /// src/random.c.
    pub(crate) mod random_c {
        include!(concat!(env!("OUT_DIR"), "/random_c.rs"));
    }

    /// src/game_utils.c.
    pub(crate) mod game_utils_c {
        include!(concat!(env!("OUT_DIR"), "/game_utils_c.rs"));
    }

    /// src/math68k.c.
    pub(crate) mod math68k_c {
        include!(concat!(env!("OUT_DIR"), "/math68k_c.rs"));
    }

    /// include/math68k.h, its `static inline` functions through their wrappers.
    pub(crate) mod math68k_h {
        include!(concat!(env!("OUT_DIR"), "/math68k_h.rs"));
    }

    /// src/dos_gameplay.c.
    pub(crate) mod dos_gameplay_c {
        include!(concat!(env!("OUT_DIR"), "/dos_gameplay_c.rs"));
    }
}

/// Calls `call`, turning upstream's halt into its reason code.
fn halting(call: impl FnOnce()) -> Result<(), u16> {
    catch_unwind(AssertUnwindSafe(call)).map_err(|payload| {
        match payload.downcast::<UpstreamPanic>() {
            Ok(halt) => halt.0,
            Err(other) => resume_unwind(other),
        }
    })
}

/// Resets the globals the units touch to upstream's values, and keeps every stub linked.
pub(crate) fn reset() {
    stubs::reset();
    rng::reset();
    style::reset();
    black_box(stubs::PanicHaltLoop as extern "C-unwind" fn() -> !);
    black_box(stubs::TacticsData_SelectRelease as extern "C-unwind" fn(_));
    black_box(stubs::Tweaks_On as extern "C-unwind" fn(_) -> _);
}
