#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod bteams;
pub mod files;
pub mod fingerprint;
pub mod ini;
pub mod json_file;
pub mod manifest;
pub mod package;
pub mod stadiums;

pub use bteams::{BTeam, BTeams, BTeamsDocument, CallUpRules, CallUpWindow};

pub use files::{FilesError, MAX_PACKAGE_BYTES, PackageFiles};
pub use fingerprint::{
    FINGERPRINT_HEX_LEN, FINGERPRINTED_FOLDERS, Fingerprint, FingerprintCheck, InvalidFingerprint,
};
pub use ini::{IniEntry, IniError, WorldIdentity, WorldIni, WorldIniFormat, WorldTag};
pub use json_file::JsonFile;
pub use manifest::{ModManifest, ModManifestDocument, ModManifestFormat, ModSetting};
pub use package::{
    BTEAMS_JSON, CAREER_PARAMS_JSON, COMPETITIONS_FOLDER, MOD_JSON, NATIONAL_TEAMS_JSON,
    OpenedPackage, POOL_FILE, RULES_FOLDER, STADIUMS_JSON, TEAMS_FOLDER, WORLD_INI, WorldPackage,
    WorldPackageError,
};
pub use stadiums::{ClubStadium, Stadiums, StadiumsDocument, Venue};
