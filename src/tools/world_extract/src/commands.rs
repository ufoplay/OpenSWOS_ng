//! The tool's commands, whole: each builds or fetches a package, checks it, and says what it
//! did.

use std::fmt;
use std::path::{Path, PathBuf};

use openswos_common::Importer;
use openswos_poolplyr_resource::{PoolFile, PoolFileFormat};
use openswos_test_utils::{root_path, tactics_path, world_1996_97_path};
use openswos_upstream::workspace::{CHECKOUT_DIR_ENV, DEFAULT_CHECKOUT_DIR};
use openswos_world_package_resource::{Fingerprint, POOL_FILE, TEAMS_FOLDER, WorldPackage};

use openswos_core::World;

use crate::assemble::world_1996_97;
use crate::check::{builtin_tactics, game_data};
use crate::error::ExtractError;
use crate::extract::committed_files;
use crate::fetch::{Fetched, world_2627};
use crate::game::GameCopy;
use crate::output::{
    Existing, WORLD_1996_97_DIR, WORLD_2026_27_DIR, cache_dir, write, write_files,
};

/// A package written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// Its name.
    pub name: String,
    /// Its folder.
    pub dir: PathBuf,
    /// Its fingerprint, as its `WORLD.ini` states it.
    pub fingerprint: Fingerprint,
    /// Its rosters.
    pub rosters: usize,
    /// Its teams.
    pub teams: usize,
    /// Its pool's players.
    pub pool: usize,
}

impl fmt::Display for Written {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} rosters, {} teams, a pool of {} players; fingerprint {}; written to {}",
            self.name,
            self.rosters,
            self.teams,
            self.pool,
            self.fingerprint,
            self.dir.display()
        )
    }
}

/// `just world_extract_9697`: World 1996-97 assembled from your copy of the PC game in
/// `game_dir`, checked, and written into `out` (a new or empty folder), or by default into
/// `target/cache/worlds/1996-97/`, replacing the package there.
///
/// # Errors
///
/// When the package cannot be built, is not valid, or cannot be written.
pub fn assemble_9697(game_dir: &Path, out: Option<&Path>) -> Result<Written, ExtractError> {
    let game = GameCopy::open(game_dir)?;
    let package = world_1996_97(&world_1996_97_path(), &game)?;
    let data = game_data(&package, builtin_tactics(&tactics_path())?)?;
    let (dir, existing) = match out {
        Some(dir) => (dir.to_path_buf(), Existing::Refuse),
        None => (cache_dir(WORLD_1996_97_DIR), Existing::Replace),
    };
    write(&package, &dir, existing)?;
    let check = WorldPackage::open_dir(&dir)?.fingerprint;
    if !check.is_match() {
        return Err(ExtractError::FingerprintMismatch {
            stated: check.stated,
            computed: check.computed,
        });
    }
    let rosters = data.rosters().rosters();
    Ok(Written {
        name: package.identity.name.clone(),
        dir,
        fingerprint: check.stated,
        rosters: rosters.len(),
        teams: rosters.iter().map(|roster| roster.teams.len()).sum(),
        pool: data.pool().len(),
    })
}

/// The committed package, extracted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    /// Its folder.
    pub dir: PathBuf,
    /// Its files.
    pub files: usize,
}

impl fmt::Display for Extracted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "World 1996-97 extracted from the upstream checkout: {} files written to {}",
            self.files,
            self.dir.display()
        )
    }
}

/// `just world_extract_9697_data`: World 1996-97's committed package extracted again from the
/// upstream checkout ([`upstream_checkout`]) and its oracle, checked (a valid world), and
/// written into `out` (a new or empty folder), or by default over `data/worlds/1996-97/`.
///
/// # Errors
///
/// When the oracle is not built, the package cannot be extracted, is not valid, or cannot
/// be written.
pub fn extract_9697(out: Option<&Path>) -> Result<Extracted, ExtractError> {
    let package = crate::extract::world_1996_97(&upstream_checkout())?;
    World::from_package(&package)?;
    let files = committed_files(&package)?;
    let (dir, existing) = match out {
        Some(dir) => (dir.to_path_buf(), Existing::Refuse),
        None => (world_1996_97_path(), Existing::Replace),
    };
    write_files(&files, &dir, existing)?;
    Ok(Extracted {
        dir,
        files: files.iter().count(),
    })
}

/// A completed world, written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed {
    /// Its name.
    pub name: String,
    /// Its folder.
    pub dir: PathBuf,
    /// Its countries.
    pub countries: usize,
    /// Its leagues.
    pub leagues: usize,
    /// Its rosters.
    pub rosters: usize,
    /// Its rosters' teams.
    pub teams: usize,
    /// Its pool's players.
    pub pool: usize,
}

impl fmt::Display for Completed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} completed: {} countries, {} leagues, {} rosters, {} teams, a pool of {} \
             players; written to {}",
            self.name,
            self.countries,
            self.leagues,
            self.rosters,
            self.teams,
            self.pool,
            self.dir.display()
        )
    }
}

/// `just world_complete_2627`: World 2026-27's package downloaded and checked against the
/// pin ([`fetch_2627`]), its pool unpacked by upstream's decoder, laid over World 1996-97's
/// committed package with what its baked install leaves (the oracle), checked (a valid world
/// with valid game data), and written into `out` (a new or empty folder), or by default into
/// `target/cache/worlds/2026-27/`, replacing the package there.
///
/// # Errors
///
/// When the download, the oracle or the merge fails, the world is not valid, or the package
/// cannot be written.
pub fn complete_2627(out: Option<&Path>) -> Result<Completed, ExtractError> {
    // The oracle first: without it there is nothing to complete, and nothing to download.
    openswos_oracle::tables::team_file_bases()?;
    let fetched = fetch_2627()?;
    // Unpacked before the install, after which the oracle serves nothing else.
    let pool = fetched
        .packed_pool
        .as_deref()
        .map(unpack_pool)
        .transpose()?;
    let baked = openswos_oracle::tables::world_2627()?;
    let base = crate::assemble::committed(&world_1996_97_path())?;
    let package = crate::complete::world_2627(&base, &fetched.opened.package, &baked, pool)?;
    let data = game_data(&package, builtin_tactics(&tactics_path())?)?;
    let world = data.world();
    let (dir, existing) = match out {
        Some(dir) => (dir.to_path_buf(), Existing::Refuse),
        None => (cache_dir(WORLD_2026_27_DIR), Existing::Replace),
    };
    write(&package, &dir, existing)?;
    let rosters = data.rosters().rosters();
    Ok(Completed {
        name: package.identity.name.clone(),
        dir,
        countries: world.countries().iter().count(),
        leagues: world.leagues().len(),
        rosters: rosters.len(),
        teams: rosters.iter().map(|roster| roster.teams.len()).sum(),
        pool: data.pool().len(),
    })
}

/// A world's packed foreign pool, unpacked by upstream's decoder and read.
fn unpack_pool(packed: &[u8]) -> Result<PoolFile, ExtractError> {
    let unpacked = openswos_oracle::unpack::rnc(packed)?;
    PoolFileFormat::import(&unpacked).map_err(|source| ExtractError::Pool {
        path: PathBuf::from(format!("{TEAMS_FOLDER}/{POOL_FILE}")),
        source,
    })
}

/// `just world_fetch_2627`: World 2026-27's package downloaded and checked against the
/// upstream checkout ([`upstream_checkout`]).
///
/// # Errors
///
/// As [`world_2627`].
pub fn fetch_2627() -> Result<Fetched, ExtractError> {
    world_2627(&upstream_checkout())
}

/// The upstream checkout: `OPENSWOS_UPSTREAM_DIR` (from the workspace root when relative), else
/// `target/cache/upstream/OpenSWOS`, as the `upstream` tool finds it.
#[must_use]
pub fn upstream_checkout() -> PathBuf {
    let dir = std::env::var_os(CHECKOUT_DIR_ENV)
        .map_or_else(|| PathBuf::from(DEFAULT_CHECKOUT_DIR), PathBuf::from);
    root_path().join(dir)
}

/// What [`fetch_2627`] says it did.
#[must_use]
pub fn describe_fetched(fetched: &Fetched) -> String {
    let package = &fetched.opened.package;
    let competitions: Vec<&str> = package.competitions.keys().map(String::as_str).collect();
    format!(
        "{}: {}; fingerprint {}, the upstream pin's; rosters: {}; competitions: {}; files not \
         read: {}",
        package.identity.name,
        fetched.zip.display(),
        fetched.opened.fingerprint.stated,
        package.teams.len(),
        competitions.join(", "),
        fetched.opened.ignored.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checkout_is_the_upstream_tools() {
        if std::env::var_os(CHECKOUT_DIR_ENV).is_none() {
            assert_eq!(
                upstream_checkout(),
                root_path().join("target/cache/upstream/OpenSWOS")
            );
        }
    }

    #[test]
    fn a_folder_without_the_game_is_refused() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let error = assemble_9697(dir.path(), Some(&dir.path().join("out"))).expect_err("no game");
        assert!(error.to_string().contains("DATA"), "{error}");
        assert!(!dir.path().join("out").exists(), "nothing written");
    }

    #[test]
    fn a_written_package_says_what_it_holds() {
        let written = Written {
            name: "World 1996-97".to_owned(),
            dir: PathBuf::from("out"),
            fingerprint: Fingerprint::try_from("0123456789abcdef".to_owned()).expect("valid"),
            rosters: 71,
            teams: 1580,
            pool: 900,
        };
        assert_eq!(
            written.to_string(),
            "World 1996-97: 71 rosters, 1580 teams, a pool of 900 players; fingerprint \
             0123456789abcdef; written to out"
        );
    }
}
