//! Where packages go, and writing them: into a new or empty folder, or over the package this
//! tool wrote there before.

use std::path::{Path, PathBuf};

use openswos_test_utils::cache_path;
use openswos_world_package_resource::{PackageFiles, WorldPackage};

use crate::error::ExtractError;

/// The assembled and completed packages' folder, under `target/cache/` (docs/PLAN.md §4).
const WORLDS_DIR: &str = "worlds";

/// World 1996-97's folder, under [`WORLDS_DIR`].
pub const WORLD_1996_97_DIR: &str = "1996-97";

/// World 2026-27's folder, under [`WORLDS_DIR`].
pub const WORLD_2026_27_DIR: &str = "2026-27";

/// What is appended to a folder's name to stage the package that replaces it.
const STAGING_SUFFIX: &str = ".new";

/// What to do with a folder that already holds files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Existing {
    /// Replace it: it is the package this tool wrote there before (the cache's).
    Replace,
    /// Refuse: a folder the user names must be new or empty.
    Refuse,
}

/// The cache folder of the world `dir_name` (`target/cache/worlds/1996-97`).
#[must_use]
pub fn cache_dir(dir_name: &str) -> PathBuf {
    cache_path().join(WORLDS_DIR).join(dir_name)
}

/// Writes `package` into `dir`. With [`Existing::Replace`] the package is written beside
/// `dir` first, then takes its place, so a failed write leaves the old package whole.
///
/// # Errors
///
/// When the package does not encode or a file cannot be written, or `dir` holds files and
/// `existing` is [`Existing::Refuse`].
pub fn write(package: &WorldPackage, dir: &Path, existing: Existing) -> Result<(), ExtractError> {
    write_files(&package.to_files()?, dir, existing)
}

/// Writes `files` into `dir`, as [`write`] writes a package's.
///
/// # Errors
///
/// When a file cannot be written, or `dir` holds files and `existing` is
/// [`Existing::Refuse`].
pub fn write_files(
    files: &PackageFiles,
    dir: &Path,
    existing: Existing,
) -> Result<(), ExtractError> {
    match existing {
        Existing::Refuse => Ok(files.write_dir(dir)?),
        Existing::Replace => {
            let staging = staging_dir(dir);
            remove_if_there(&staging)?;
            files.write_dir(&staging)?;
            remove_if_there(dir)?;
            std::fs::rename(&staging, dir).map_err(|source| ExtractError::Write {
                path: dir.to_path_buf(),
                source,
            })
        }
    }
}

/// The folder a package replacing `dir` is staged in: beside it, its name and `.new`.
fn staging_dir(dir: &Path) -> PathBuf {
    let mut name = dir.file_name().unwrap_or_default().to_os_string();
    name.push(STAGING_SUFFIX);
    dir.with_file_name(name)
}

/// Removes the folder `dir` and what it holds, if it exists.
fn remove_if_there(dir: &Path) -> Result<(), ExtractError> {
    if !dir.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(dir).map_err(|source| ExtractError::Write {
        path: dir.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use openswos_test_utils::world_1996_97_path;

    use super::*;
    use crate::assemble::committed;

    fn package() -> WorldPackage {
        committed(&world_1996_97_path()).expect("the committed package")
    }

    #[test]
    fn worlds_go_under_the_cache() {
        assert_eq!(
            cache_dir(WORLD_1996_97_DIR),
            cache_path().join("worlds").join("1996-97")
        );
        assert_eq!(
            staging_dir(Path::new("a/1996-97")),
            Path::new("a/1996-97.new")
        );
    }

    #[test]
    fn a_new_folder_is_written() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let out = dir.path().join("world");
        write(&package(), &out, Existing::Refuse).expect("written");
        assert!(out.join("WORLD.ini").is_file());
        assert!(out.join("competitions/leagues.json").is_file());
        assert!(out.join("rules/career_params.json").is_file());
    }

    #[test]
    fn a_folder_with_files_is_refused_or_replaced() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let out = dir.path().join("world");
        std::fs::create_dir(&out).expect("the folder");
        std::fs::write(out.join("old.txt"), "old").expect("an old file");
        let error = write(&package(), &out, Existing::Refuse).expect_err("not empty");
        assert!(error.to_string().contains("not empty"), "{error}");
        write(&package(), &out, Existing::Replace).expect("replaced");
        assert!(!out.join("old.txt").exists());
        assert!(out.join("WORLD.ini").is_file());
        assert!(!staging_dir(&out).exists());
    }
}
