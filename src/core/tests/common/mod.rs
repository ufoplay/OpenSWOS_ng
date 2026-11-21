//! World 1996-97's package as far as the repository holds it (`data/worlds/1996-97/`:
//! upstream's competition files and career params, GPL, and the rules `world_extract`
//! extracted), without rosters.

use openswos_test_utils::world_1996_97_path;
use openswos_world_package_resource::{PackageFiles, WorldPackage};

/// World 1996-97's `WORLD.ini`, its tag and name as `world_extract` writes them (the
/// fingerprint, which covers the rosters, is not checked here).
const WORLD_INI: &str = "tag = 9697\nname = World 1996-97\nfingerprint = 0000000000000000\n";

/// The package's files: `WORLD.ini` and the committed package's.
pub fn files() -> PackageFiles {
    let mut files = PackageFiles::read_dir(&world_1996_97_path()).expect("the committed package");
    files
        .insert("WORLD.ini", WORLD_INI.as_bytes().to_vec())
        .expect("a safe path");
    files
}

/// The package.
pub fn package() -> WorldPackage {
    let opened = WorldPackage::from_files(&files()).expect("the package opens");
    assert!(opened.ignored.is_empty(), "{:?}", opened.ignored);
    opened.package
}
