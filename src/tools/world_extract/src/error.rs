//! Why a package cannot be built or checked.

use std::path::PathBuf;

use openswos_core::Invalid;
use openswos_oracle::tables::{InstallError, Unavailable};
use openswos_oracle::unpack::UnpackError;
use openswos_poolplyr_resource::PoolFileError;
use openswos_team_resource::TeamFileError;
use openswos_world_package_resource::{FilesError, Fingerprint, WorldPackageError};
use thiserror::Error;

use crate::complete::GLOBAL_TEAM_NUMBERS;

/// Why a package cannot be built or checked.
#[derive(Debug, Error)]
pub enum ExtractError {
    /// The game folder lacks a file (or its `DATA/` folder).
    #[error("{0}")]
    GameFile(String),
    /// A file does not read.
    #[error("{path}: {source}")]
    Read {
        /// The file.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
    /// A file or folder cannot be written, moved or removed.
    #[error("{path}: {source}")]
    Write {
        /// The file or folder.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
    /// A team file of the game does not decode.
    #[error("{path}: {source}")]
    TeamFile {
        /// The file.
        path: PathBuf,
        /// Why.
        source: TeamFileError,
    },
    /// The game's pool does not decode.
    #[error("{path}: {source}")]
    Pool {
        /// The file.
        path: PathBuf,
        /// Why.
        source: PoolFileError,
    },
    /// A built-in tactics file does not decode, or is not a preset's.
    #[error("{path}: {reason}")]
    Tactics {
        /// The file.
        path: PathBuf,
        /// Why.
        reason: String,
    },
    /// A package does not open or write.
    #[error(transparent)]
    Package(#[from] WorldPackageError),
    /// A package's files do not read or write.
    #[error(transparent)]
    Files(#[from] FilesError),
    /// The committed package holds files a package does not.
    #[error("{dir}: not files of a world package: {}", paths.join(", "))]
    UnknownFiles {
        /// The package's folder.
        dir: PathBuf,
        /// The files.
        paths: Vec<String>,
    },
    /// The world, or its game data, is not valid.
    #[error("the world is not valid:\n{0}")]
    Invalid(#[from] Invalid),
    /// The package has no foreign players' pool it can read.
    #[error("the package has no foreign players' pool it can read")]
    NoPool,
    /// The oracle is not built, so upstream's compiled tables cannot be read.
    #[error(transparent)]
    Oracle(#[from] Unavailable),
    /// Upstream's countries' table has no entry for a country of the world.
    #[error("upstream's _countriesTable has no entry for country {0}")]
    NoCountryEntry(u8),
    /// Upstream's countries' table gives a country a continent code that is none.
    #[error("upstream's _countriesTable gives country {number} the continent {code}, not one")]
    NoContinent {
        /// The country.
        number: u8,
        /// The code.
        code: u8,
    },
    /// The package has no `countries.json`.
    #[error("the package has no countries.json")]
    NoCountries,
    /// A competition file the completion edits is missing from its package.
    #[error("the package has no competitions/{0} of its kind")]
    NoCompetitionFile(&'static str),
    /// A new country of the world without its roster.
    #[error("the world brings country {0} but no roster of it")]
    NoTeamFile(u8),
    /// A world's baked install failed.
    #[error(transparent)]
    Install(#[from] InstallError),
    /// A converted record's id is another record's.
    #[error("the id {0} is another record's")]
    IdTaken(String),
    /// A compiled record without a name.
    #[error("a compiled record has no name")]
    Unnamed,
    /// A compiled record's type that is none.
    #[error("a compiled record's type {0} is no competition type")]
    NoCompetitionType(i8),
    /// A compiled name that is not ASCII.
    #[error("a compiled name is not ASCII: {0:?}")]
    NotAscii(Vec<u8>),
    /// No record has a competition id an upstream list names.
    #[error("no record has the competition id {0}")]
    NoRecordOfId(u8),
    /// Records of one competition id: an upstream list's record is not one of them.
    #[error("several records have the competition id {0}")]
    RepeatedCompId(u8),
    /// An upstream list names the career's copy of a record the career index does not copy.
    #[error("the career index makes no copy of {0}")]
    NoCareerCopy(String),
    /// No tournament has an id upstream's code names.
    #[error("no tournament has the competition id {0}")]
    NoCompetitionOfId(u8),
    /// Tournaments that play one round robin with groups of several sizes.
    #[error("the groups of {0} are not all of one size")]
    GroupSizes(String),
    /// A compiled round robin shorter than its size's.
    #[error("a round robin needs {needed} pairs, the compiled table holds {held}")]
    ShortSchedule {
        /// The pairs of a round robin of its size.
        needed: usize,
        /// The table's.
        held: usize,
    },
    /// No round robin of a size.
    #[error("no round robin of {0} clubs")]
    NoSchedule(u8),
    /// The package a world is laid over lacks a rules file.
    #[error("the base package has no rules/{0}")]
    NoBaseRules(&'static str),
    /// The upstream checkout has no table of the world's cups.
    #[error("{0}: no `kWorld2627Cups` table: run `just upstream_fetch`")]
    NoCupTable(PathBuf),
    /// A row of that table that does not read.
    #[error("a row of upstream's table of the world's cups does not read: {0}")]
    CupRow(String),
    /// That table and the compiled cups disagree, from this row.
    #[error("upstream's table of the world's cups and its compiled cups disagree at row {0}")]
    CupsOutOfStep(usize),
    /// A cup's slot that is none (1 the national cup, 2 the league cup).
    #[error("a world cup's slot {0} is none")]
    CupSlot(u8),
    /// A cup's country without a table of its own.
    #[error("a world cup's country {0} has no table of its own")]
    NoCountryTable(u8),
    /// A cup past upstream's tables (96 countries of 12 entries): upstream would drop it.
    #[error("a world cup of country {0} is past upstream's cup tables")]
    CupTablesFull(u8),
    /// The world's pool does not unpack.
    #[error("the world's pool: {0}")]
    Unpack(#[from] UnpackError),
    /// A world's clubs run past the global team numbers (upstream panic 0x8F02).
    #[error("the world's team numbers run to {0}, past the {GLOBAL_TEAM_NUMBERS} there are")]
    TooManyTeamNumbers(u16),
    /// A download failed, or the network is off.
    #[error("download: {0}")]
    Download(String),
    /// The upstream checkout states no World 2026-27 fingerprint.
    #[error("{path}: no fingerprint in `{function}`: run `just upstream_fetch`")]
    NoPinnedFingerprint {
        /// The file read.
        path: PathBuf,
        /// The function looked for.
        function: &'static str,
    },
    /// A package's `WORLD.ini` states another fingerprint than its files'.
    #[error("the package states the fingerprint {stated}, its files have {computed}")]
    FingerprintMismatch {
        /// The stated one.
        stated: Fingerprint,
        /// The files'.
        computed: Fingerprint,
    },
    /// The package is not the data the upstream pin was built with.
    #[error(
        "package and upstream pin out of step: the package is {package}, the pin expects {pin}"
    )]
    OutOfStep {
        /// The package's fingerprint.
        package: Fingerprint,
        /// The pin's.
        pin: Fingerprint,
    },
}
