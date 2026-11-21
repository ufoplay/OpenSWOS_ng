#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod dos;
pub mod save;
pub mod scalars;

pub use dos::{
    DOS_BODY_LEN, DOS_HEADER_LEN, DOS_MAX_TEAMS, DOS_NAME_LEN, DOS_REGION_LEN, DOS_TAIL_LEN,
    DosCompetitionHeader, DosCompetitionType, DosDiyError, DosDiyFile, DosDiyFormat,
    DosMatchOptions, InvalidCompetitionType, USER_TACTICS,
};
pub use save::{
    CompetitionKind, CompetitionSave, CompetitionSaveError, CompetitionSaveFormat, WorldPack,
};
pub use scalars::CompetitionScalars;
