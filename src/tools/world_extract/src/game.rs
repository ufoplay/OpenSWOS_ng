//! Your copy of the PC game: its `DATA/` folder, whose files installs spell in any case.

use std::path::{Path, PathBuf};

use openswos_common::{Importer, TeamFileNumber};
use openswos_poolplyr_resource::{PoolFile, PoolFileFormat};
use openswos_team_resource::{TeamFile, TeamFileFormat};
use openswos_test_utils::data::find_ignoring_case;
use openswos_world_package_resource::POOL_FILE;

use crate::error::ExtractError;

/// The game's data folder, in the game's folder.
const DATA_DIR: &str = "DATA";

/// The start of a team file's name (`TEAM.008`).
const TEAM_FILE_PREFIX: &str = "TEAM.";

/// The digits of a team file's number in its name (`008`).
const TEAM_FILE_DIGITS: usize = 3;

/// The PC game, installed: the folder holding `DATA/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameCopy {
    /// Its `DATA/` folder.
    data: PathBuf,
}

impl GameCopy {
    /// The game installed in `dir`.
    ///
    /// # Errors
    ///
    /// When `dir` holds no `DATA/` folder.
    pub fn open(dir: &Path) -> Result<Self, ExtractError> {
        let data = find_ignoring_case(dir, DATA_DIR)
            .map_err(|skip| ExtractError::GameFile(skip.to_string()))?;
        Ok(Self { data })
    }

    /// Its `DATA/` folder.
    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    /// The team file of `number` (`DATA/TEAM.008`).
    ///
    /// # Errors
    ///
    /// When the game lacks it or it does not decode.
    pub fn team_file(&self, number: TeamFileNumber) -> Result<TeamFile, ExtractError> {
        let (path, bytes) = self.read(&team_file_name(number))?;
        TeamFileFormat::import(&bytes).map_err(|source| ExtractError::TeamFile { path, source })
    }

    /// The foreign players' pool (`DATA/POOLPLYR.DAT`).
    ///
    /// # Errors
    ///
    /// When the game lacks it or it does not decode.
    pub fn pool(&self) -> Result<PoolFile, ExtractError> {
        let (path, bytes) = self.read(POOL_FILE)?;
        PoolFileFormat::import(&bytes).map_err(|source| ExtractError::Pool { path, source })
    }

    /// The file `name` of `DATA/`, and where it is.
    fn read(&self, name: &str) -> Result<(PathBuf, Vec<u8>), ExtractError> {
        let path = find_ignoring_case(&self.data, name)
            .map_err(|skip| ExtractError::GameFile(skip.to_string()))?;
        let bytes = std::fs::read(&path).map_err(|source| ExtractError::Read {
            path: path.clone(),
            source,
        })?;
        Ok((path, bytes))
    }
}

/// The name of the team file of `number`: `TEAM.008`.
#[must_use]
pub fn team_file_name(number: TeamFileNumber) -> String {
    format!(
        "{TEAM_FILE_PREFIX}{:0width$}",
        number.0,
        width = TEAM_FILE_DIGITS
    )
}

#[cfg(test)]
mod tests {
    use openswos_test_utils::assets_path;

    use super::*;

    /// A game folder spelt in lower case, holding `TEAM.008` and the pool.
    fn game() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temp dir");
        let data = dir.path().join("data");
        std::fs::create_dir(&data).expect("data/");
        for (from, to) in [
            ("TEAM/TEAM.008", "team.008"),
            ("POOLPLYR/POOLPLYR.DAT", "poolplyr.dat"),
        ] {
            std::fs::copy(assets_path().join(from), data.join(to)).expect("a fixture");
        }
        dir
    }

    #[test]
    fn team_files_are_named_by_three_digits() {
        assert_eq!(team_file_name(TeamFileNumber(8)), "TEAM.008");
        assert_eq!(team_file_name(TeamFileNumber(85)), "TEAM.085");
        assert_eq!(team_file_name(TeamFileNumber(197)), "TEAM.197");
    }

    #[test]
    fn files_are_found_in_any_case() {
        let dir = game();
        let game = GameCopy::open(dir.path()).expect("a game");
        assert_eq!(game.data_dir(), dir.path().join("data"));
        let england = game.team_file(TeamFileNumber(8)).expect("England");
        assert_eq!(england.teams.len(), 93);
        assert_eq!(game.pool().expect("the pool").players.len(), 900);
    }

    #[test]
    fn a_missing_folder_or_file_is_named() {
        let empty = tempfile::tempdir().expect("a temp dir");
        let error = GameCopy::open(empty.path()).expect_err("no DATA/");
        assert!(error.to_string().contains("DATA"), "{error}");
        let dir = game();
        let game = GameCopy::open(dir.path()).expect("a game");
        let error = game.team_file(TeamFileNumber(1)).expect_err("no TEAM.001");
        assert!(error.to_string().contains("TEAM.001"), "{error}");
    }

    #[test]
    fn a_broken_file_is_named() {
        let dir = game();
        std::fs::write(dir.path().join("data/team.008"), [0, 1]).expect("broken");
        std::fs::write(dir.path().join("data/poolplyr.dat"), [0; 5]).expect("broken");
        let game = GameCopy::open(dir.path()).expect("a game");
        let error = game.team_file(TeamFileNumber(8)).expect_err("too short");
        assert!(error.to_string().contains("team.008"), "{error}");
        let error = game.pool().expect_err("a partial record");
        assert!(error.to_string().contains("poolplyr.dat"), "{error}");
    }
}
