//! The check every package passes before it is written: a valid world, and valid game data
//! with its rosters, its pool and the built-in tactics (`openswos_core`).

use std::collections::BTreeMap;
use std::path::Path;

use openswos_common::Importer;
use openswos_core::{GameData, World};
use openswos_tac_resource::TeamTactics;
use openswos_tactics_json_resource::{PresetFile, TacticsJsonFormat};
use openswos_world_package_resource::WorldPackage;

use crate::error::ExtractError;

/// The built-in tactics in `dir` (`data/tactics/`), by file.
///
/// # Errors
///
/// When a file does not read, does not decode, or is not a preset's.
pub fn builtin_tactics(dir: &Path) -> Result<BTreeMap<PresetFile, TeamTactics>, ExtractError> {
    let read_error = |path: &Path| {
        let path = path.to_path_buf();
        move |source| ExtractError::Read { path, source }
    };
    let mut tactics = BTreeMap::new();
    for entry in std::fs::read_dir(dir).map_err(read_error(dir))? {
        let path = entry.map_err(read_error(dir))?.path();
        let tactics_error = |reason: String| ExtractError::Tactics {
            path: path.clone(),
            reason,
        };
        let file = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(PresetFile::parse)
            .ok_or_else(|| tactics_error("not a built-in tactics file".to_owned()))?;
        let bytes = std::fs::read(&path).map_err(read_error(&path))?;
        let json =
            TacticsJsonFormat::import(&bytes).map_err(|error| tactics_error(error.to_string()))?;
        tactics.insert(file, json.tactics);
    }
    Ok(tactics)
}

/// `package`'s game data: its world, its rosters and its pool, with `tactics`.
///
/// # Errors
///
/// [`ExtractError::Invalid`] with every problem of the world or of its game data, or
/// [`ExtractError::NoPool`] when the package has no pool it can read.
pub fn game_data(
    package: &WorldPackage,
    tactics: BTreeMap<PresetFile, TeamTactics>,
) -> Result<GameData, ExtractError> {
    let world = World::from_package(package)?;
    let pool = package.pool.clone().ok_or(ExtractError::NoPool)?;
    Ok(GameData::new(world, &package.teams, pool, tactics)?)
}

#[cfg(test)]
mod tests {
    use openswos_poolplyr_resource::PoolFileFormat;
    use openswos_test_utils::{assets_path, tactics_path, world_1996_97_path};

    use super::*;
    use crate::assemble::committed;

    #[test]
    fn the_builtin_tactics_are_upstreams_twenty_files() {
        let tactics = builtin_tactics(&tactics_path()).expect("the tactics");
        assert_eq!(tactics.len(), 20);
    }

    #[test]
    fn a_file_not_a_presets_is_named() {
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::write(dir.path().join("notes.txt"), "hello").expect("a stray file");
        let error = builtin_tactics(dir.path()).expect_err("not a preset");
        assert!(error.to_string().contains("notes.txt"), "{error}");
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::write(dir.path().join("442.json"), "{}").expect("a broken file");
        let error = builtin_tactics(dir.path()).expect_err("not tactics");
        assert!(error.to_string().contains("442.json"), "{error}");
    }

    #[test]
    fn game_data_needs_the_pool_and_the_rosters() {
        let tactics = builtin_tactics(&tactics_path()).expect("the tactics");
        let mut package = committed(&world_1996_97_path()).expect("the committed package");
        assert!(matches!(
            game_data(&package, tactics.clone()),
            Err(ExtractError::NoPool)
        ));
        let pool = std::fs::read(assets_path().join("POOLPLYR/POOLPLYR.DAT")).expect("a pool");
        package.pool = Some(PoolFileFormat::import(&pool).expect("the pool"));
        let Err(ExtractError::Invalid(invalid)) = game_data(&package, tactics) else {
            panic!("no rosters: not valid");
        };
        assert!(
            invalid.to_string().contains("countries[0].team_file"),
            "{invalid}"
        );
    }
}
