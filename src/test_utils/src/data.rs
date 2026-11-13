//! Your own copy of the game data, outside the repository (docs/PLAN.md §6). Its folder
//! reaches the tests through an environment variable (a `just` recipe passes its command
//! line argument that way); without it, the tests that need it are skipped.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::skip::Skip;

/// Environment variable naming the installed PC game: the folder holding `DATA/` and
/// `SPRITE.DAT` (e.g. a GOG install).
pub const GAME_DIR_ENV: &str = "OPENSWOS_GAME_DIR";

/// Environment variable naming an OpenSWOS Setup output folder.
pub const SETUP_ASSETS_DIR_ENV: &str = "OPENSWOS_ASSETS_DIR";

/// Environment variable naming a folder of OpenSWOS saves (`.CAR`, `.SEA`, `.DIY`…): yours,
/// or ones the golden-career autopilot wrote. Never committed: they hold the game's data.
pub const SAVES_DIR_ENV: &str = "OPENSWOS_SAVES_DIR";

/// The installed PC game's folder.
///
/// # Errors
///
/// A [`Skip`] when [`GAME_DIR_ENV`] is unset or not a folder.
pub fn game_dir() -> Result<PathBuf, Skip> {
    dir_from(GAME_DIR_ENV, std::env::var_os(GAME_DIR_ENV))
}

/// An OpenSWOS Setup output folder.
///
/// # Errors
///
/// A [`Skip`] when [`SETUP_ASSETS_DIR_ENV`] is unset or not a folder.
pub fn setup_assets_dir() -> Result<PathBuf, Skip> {
    dir_from(SETUP_ASSETS_DIR_ENV, std::env::var_os(SETUP_ASSETS_DIR_ENV))
}

/// A folder of OpenSWOS saves.
///
/// # Errors
///
/// A [`Skip`] when [`SAVES_DIR_ENV`] is unset or not a folder.
pub fn saves_dir() -> Result<PathBuf, Skip> {
    dir_from(SAVES_DIR_ENV, std::env::var_os(SAVES_DIR_ENV))
}

/// The name of OpenSWOS's settings file (upstream src/swos_io.c `swos_ini_path`).
pub const SETTINGS_FILE_NAME: &str = "SWOS.ini";

/// OpenSWOS's settings file: `SWOS.ini` beside the saves folder, in the user's game folder
/// that holds it (upstream src/swos_io.c `swos_ini_path`).
///
/// # Errors
///
/// A [`Skip`] when the saves folder is not given or has no settings file beside it.
pub fn settings_file() -> Result<PathBuf, Skip> {
    let saves = saves_dir()?;
    saves
        .parent()
        .map(|folder| folder.join(SETTINGS_FILE_NAME))
        .filter(|path| path.is_file())
        .ok_or_else(|| {
            Skip::new(format!(
                "{} has no {SETTINGS_FILE_NAME} beside it",
                saves.display()
            ))
        })
}

/// A file of the installed PC game, by its path in the game folder (`/`-separated),
/// matched without regard to case: installs spell `DATA/TEAM.008` differently.
///
/// # Errors
///
/// A [`Skip`] when the game folder is not given or lacks the file.
pub fn game_file(relative: &str) -> Result<PathBuf, Skip> {
    find_ignoring_case(&game_dir()?, relative)
}

/// The folder an environment variable's value names.
///
/// # Errors
///
/// A [`Skip`] when the value is missing or not a folder.
pub fn dir_from(variable: &str, value: Option<OsString>) -> Result<PathBuf, Skip> {
    let Some(value) = value else {
        return Err(Skip::new(format!("set {variable} to run this test")));
    };
    let dir = PathBuf::from(value);
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err(Skip::new(format!(
            "{variable}={} is not a folder",
            dir.display()
        )))
    }
}

/// The path of `relative` (`/`-separated) under `dir`, each component matched without
/// regard to ASCII case.
///
/// # Errors
///
/// A [`Skip`] when a component is not there.
pub fn find_ignoring_case(dir: &Path, relative: &str) -> Result<PathBuf, Skip> {
    let missing = || Skip::new(format!("{} has no {relative}", dir.display()));
    let mut path = dir.to_path_buf();
    for component in relative
        .split('/')
        .filter(|component| !component.is_empty())
    {
        let entries = std::fs::read_dir(&path).map_err(|_| missing())?;
        let found = entries
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(component)
            })
            .ok_or_else(missing)?;
        path = found.path();
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unset_variable_skips() {
        let skip = dir_from(GAME_DIR_ENV, None).expect_err("unset");
        assert_eq!(skip.reason(), "set OPENSWOS_GAME_DIR to run this test");
    }

    #[test]
    fn a_variable_naming_no_folder_skips() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("missing");
        let skip =
            dir_from(GAME_DIR_ENV, Some(missing.clone().into_os_string())).expect_err("no folder");
        assert!(skip.reason().contains("is not a folder"), "{skip}");
    }

    #[test]
    fn a_variable_naming_a_folder_gives_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let found =
            dir_from(GAME_DIR_ENV, Some(dir.path().as_os_str().to_owned())).expect("a folder");
        assert_eq!(found, dir.path());
    }

    #[test]
    fn files_are_found_whatever_their_case() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("data")).expect("dir");
        std::fs::write(dir.path().join("data").join("Team.008"), b"").expect("file");
        let found = find_ignoring_case(dir.path(), "DATA/TEAM.008").expect("found");
        assert_eq!(found, dir.path().join("data").join("Team.008"));
    }

    #[test]
    fn a_missing_file_skips() {
        let dir = tempfile::tempdir().expect("temp dir");
        let skip = find_ignoring_case(dir.path(), "DATA/TEAM.008").expect_err("missing");
        assert!(skip.reason().ends_with("has no DATA/TEAM.008"), "{skip}");
    }
}
