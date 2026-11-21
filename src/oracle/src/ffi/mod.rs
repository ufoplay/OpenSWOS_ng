//! The C side, and the workspace's only `unsafe` (D3): upstream's units as bindgen declares
//! them, the Rust stubs of the externs they reference, and safe functions over both, one
//! module a unit. Every function here touches process-wide C globals: they are called only by
//! the holder of the oracle's lock ([`crate::Oracle`]).
#![allow(unsafe_code)]

pub(crate) mod math;
pub(crate) mod rng;
mod stubs;
pub(crate) mod style;
pub(crate) mod tables;
pub(crate) mod unpack;

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

    /// src/strings_data.c (data): the countries' table.
    pub(crate) mod strings_data_c {
        include!(concat!(env!("OUT_DIR"), "/strings_data_c.rs"));
    }

    /// src/swos_amiga_data_imports.c (data): the team files' global numbers.
    pub(crate) mod swos_amiga_data_imports_c {
        include!(concat!(env!("OUT_DIR"), "/swos_amiga_data_imports_c.rs"));
    }

    /// src/competition_data.c (data): the season end's countries.
    pub(crate) mod competition_data_c {
        include!(concat!(env!("OUT_DIR"), "/competition_data_c.rs"));
    }

    /// src/world_data.c: World 2026-27's baked tables.
    pub(crate) mod world_data_c {
        include!(concat!(env!("OUT_DIR"), "/world_data_c.rs"));
    }

    /// src/rnc.c: the RNC unpacker.
    pub(crate) mod rnc_c {
        include!(concat!(env!("OUT_DIR"), "/rnc_c.rs"));
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
    black_box(stubs::World_Current as extern "C-unwind" fn() -> _);
    black_box(stubs::FixtureRounds_Install as extern "C-unwind" fn());
    black_box(stubs::LeaguePhase_InstallCups as extern "C-unwind" fn(_));
    black_box(stubs::World_SyncSecondFiles as extern "C-unwind" fn());
}
