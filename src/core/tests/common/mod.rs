//! World 1996-97's package as far as the repository holds it: upstream's competition files
//! (`assets/COMPETITIONS_JSON/`, GPL), without rosters.

use openswos_test_utils::assets_path;
use openswos_world_package_resource::{PackageFiles, WorldPackage};

/// World 1996-97's `WORLD.ini`, as `world_extract` will write it (M4 #5); the fingerprint
/// is not checked here.
const WORLD_INI: &str = "tag = 9697\nname = World 1996-97\nfingerprint = 0000000000000000\n";

/// The package's files: `WORLD.ini` and upstream's ten competition files.
pub fn files() -> PackageFiles {
    let mut files = PackageFiles::default();
    files
        .insert("WORLD.ini", WORLD_INI.as_bytes().to_vec())
        .expect("a safe path");
    let dir = assets_path().join("COMPETITIONS_JSON");
    for entry in std::fs::read_dir(&dir).expect("the fixtures") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 name")
            .to_owned();
        let bytes = std::fs::read(&path).expect("readable");
        files
            .insert(&format!("competitions/{name}"), bytes)
            .expect("a safe path");
    }
    files
}

/// The package.
pub fn package() -> WorldPackage {
    WorldPackage::from_files(&files())
        .expect("the package opens")
        .package
}
