//! What the World 2026-27 tests share: the completion, or its skip.

use openswos_oracle::tables::InstallError;
use openswos_test_utils::skip::{Skip, report};
use openswos_world_extract::commands::{Completed, complete_2627};
use openswos_world_extract::error::ExtractError;

/// World 2026-27 completed and written into the new folder `dir`, or `None` (the skip
/// reported) without the oracle or the network. Once a process: the baked install writes the
/// oracle's 96/97 tables for good.
pub fn completed(dir: &std::path::Path) -> Option<Completed> {
    match complete_2627(Some(dir)) {
        Ok(completed) => Some(completed),
        Err(ExtractError::Oracle(unavailable)) => {
            report(&Skip::new(unavailable.to_string()));
            None
        }
        Err(ExtractError::Install(InstallError::Unavailable(unavailable))) => {
            report(&Skip::new(unavailable.to_string()));
            None
        }
        Err(ExtractError::Download(reason)) => {
            report(&Skip::new(reason));
            None
        }
        Err(error) => panic!("{error}"),
    }
}
