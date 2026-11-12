//! World 2026-27's package (downloaded into `target/cache/`, D8): it opens, its files give
//! the fingerprint its `WORLD.ini` states (the one upstream's build is pinned to), and it
//! writes back with every roster and competition file byte for byte. Skipped when the
//! package can be neither found in the cache nor downloaded.

use std::io::Read;
use std::path::PathBuf;

use openswos_test_utils::cache::{WORLD_2627_PACKAGE, fetch};
use openswos_world_package_resource::{
    COMPETITIONS_FOLDER, OpenedPackage, PackageFiles, TEAMS_FOLDER, WorldPackage, WorldTag,
};

/// The fingerprint upstream's build is made with (`World_DataFingerprint`,
/// src/world_data.c at the pinned commit).
const UPSTREAM_FINGERPRINT: &str = "efe1464b6d1d62b8";

/// The rosters of the package.
const ROSTERS: usize = 101;

/// The cached package's path.
fn package_path() -> Option<PathBuf> {
    match fetch(&WORLD_2627_PACKAGE) {
        Ok(path) => Some(path),
        Err(skip) => {
            openswos_test_utils::skip::report(&skip);
            None
        }
    }
}

fn open() -> Option<(PathBuf, OpenedPackage)> {
    let path = package_path()?;
    let opened = WorldPackage::open_zip(&path).expect("World 2026-27's package opens");
    Some((path, opened))
}

#[test]
fn the_package_is_the_one_upstream_is_built_with() {
    let Some((_, opened)) = open() else { return };
    assert!(opened.fingerprint.is_match(), "{:?}", opened.fingerprint);
    assert_eq!(opened.fingerprint.stated.as_str(), UPSTREAM_FINGERPRINT);
    let package = &opened.package;
    assert_eq!(package.identity.tag, WorldTag("2627".to_owned()));
    assert_eq!(package.identity.name, "World 2026-27");
    assert_eq!(package.teams.len(), ROSTERS);
    assert_eq!(
        package.competitions.keys().collect::<Vec<_>>(),
        vec!["leagues.json"]
    );
    assert!(
        package.manifest.is_none(),
        "the pinned package predates mod.json"
    );
    assert_eq!(
        opened.ignored.len(),
        ROSTERS,
        "data/: the Amiga's team files"
    );
    assert!(opened.ignored.iter().all(|path| path.starts_with("data/")));
}

#[test]
fn the_package_writes_back_its_files_byte_for_byte() {
    let Some((path, opened)) = open() else { return };
    let original = PackageFiles::read_zip(std::io::BufReader::new(
        std::fs::File::open(&path).expect("the cached package"),
    ))
    .expect("a zip");
    let dir = tempfile::tempdir().expect("a temporary folder");
    let target = dir.path().join("2627");
    opened.package.write_dir(&target).expect("written");
    let written = PackageFiles::read_dir(&target).expect("read back");
    let mut compared = 0;
    for folder in [TEAMS_FOLDER, COMPETITIONS_FOLDER] {
        for (file, bytes) in original.in_folder(folder) {
            assert!(written.get(file) == Some(bytes), "{file} differs");
            compared += 1;
        }
    }
    assert_eq!(compared, ROSTERS + 1);
    let reopened = WorldPackage::open_dir(&target).expect("reopens");
    assert!(reopened.fingerprint.is_match());
    assert!(reopened.ignored.is_empty());
    assert_eq!(reopened.package, opened.package);
    // Without data/, the written package is other data: its own fingerprint.
    assert_ne!(reopened.fingerprint.stated.as_str(), UPSTREAM_FINGERPRINT);
}

#[test]
fn the_pin_is_upstreams() {
    let world_data = openswos_test_utils::cache_path()
        .join("upstream")
        .join("OpenSWOS")
        .join("src")
        .join("world_data.c");
    let Ok(mut file) = std::fs::File::open(&world_data) else {
        openswos_test_utils::skip::report(&openswos_test_utils::skip::Skip::new(format!(
            "no {} (just upstream_fetch)",
            world_data.display()
        )));
        return;
    };
    let mut text = String::new();
    file.read_to_string(&mut text).expect("readable");
    assert!(
        text.contains(&format!("return \"{UPSTREAM_FINGERPRINT}\";")),
        "upstream's World_DataFingerprint is no longer {UPSTREAM_FINGERPRINT}"
    );
}
