#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod files;
pub mod fingerprint;
pub mod ini;
pub mod manifest;
pub mod package;

pub use files::{FilesError, MAX_PACKAGE_BYTES, PackageFiles};
pub use fingerprint::{
    FINGERPRINT_HEX_LEN, FINGERPRINTED_FOLDERS, Fingerprint, FingerprintCheck, InvalidFingerprint,
};
pub use ini::{IniEntry, IniError, WorldIdentity, WorldIni, WorldIniFormat, WorldTag};
pub use manifest::{ModManifest, ModManifestDocument, ModManifestFormat};
pub use package::{
    COMPETITIONS_FOLDER, MOD_JSON, OpenedPackage, TEAMS_FOLDER, WORLD_INI, WorldPackage,
    WorldPackageError,
};
