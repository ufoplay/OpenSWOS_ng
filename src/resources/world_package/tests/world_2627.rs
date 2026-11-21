//! World 2026-27's package (downloaded into `target/cache/`, D8): it opens (its rosters, its
//! leagues, national teams, B teams and stadiums, its `mod.json` with the world's settings),
//! its files give the fingerprint its `WORLD.ini` states (the one upstream's build is pinned
//! to), and it writes back with every file it reads byte for byte. Skipped when the package
//! can be neither found in the cache nor downloaded.

use std::io::Read;
use std::path::PathBuf;

use openswos_test_utils::cache::{WORLD_2627_PACKAGE, fetch};
use openswos_world_package_resource::{
    COMPETITIONS_FOLDER, OpenedPackage, PackageFiles, TEAMS_FOLDER, WorldPackage, WorldTag,
};

/// The extension of the files a package reads in `teams/` and `competitions/`.
const JSON_EXTENSION: &str = ".json";

/// The fingerprint upstream's build is made with (`World_DataFingerprint`,
/// src/world_data.c at the pinned commit).
const UPSTREAM_FINGERPRINT: &str = "80e20cdd3e92ec9c";

/// The rosters of the package: 101 countries' clubs, the six continents' national teams.
const ROSTERS: usize = 107;

/// The files of `competitions/`: the leagues, the national teams, the B teams, the stadiums.
const COMPETITION_FILES: usize = 4;

/// The world's settings in its `mod.json` (upstream scripts/make_world_package.sh: the career's
/// four, the match's six).
const SETTINGS: usize = 10;

/// The national teams: the 211 FIFA members (upstream scripts/worldgen/nations.py).
const NATIONAL_TEAMS: usize = 211;

/// The clubs with a B team.
const B_TEAMS: usize = 20;

/// The Amiga's team files in `data/`, not read (D9).
const AMIGA_FILES: usize = 102;

/// The world's foreign players' pool, RNC-packed: not read (D9).
const POOL: &str = "teams/POOLPLYR.DAT";

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
    let manifest = &package.manifest.as_ref().expect("a mod.json").manifest;
    assert_eq!(manifest.version, UPSTREAM_FINGERPRINT);
    assert_eq!(manifest.settings.as_ref().map(Vec::len), Some(SETTINGS));
    let national = package.national_teams.as_ref().expect("its national teams");
    assert_eq!(national.teams.records().len(), NATIONAL_TEAMS);
    let bteams = &package.bteams.as_ref().expect("its B teams").content;
    assert_eq!(bteams.teams.len(), B_TEAMS);
    assert!(
        bteams
            .teams
            .iter()
            .all(|team| bteams.rules_of(team).is_some())
    );
    let stadiums = &package.stadiums.as_ref().expect("its stadiums").content;
    assert!(!stadiums.clubs.is_empty() && !stadiums.venues.is_empty());
    assert_eq!(opened.ignored.len(), AMIGA_FILES + 1, "data/ and the pool");
    assert!(
        opened
            .ignored
            .iter()
            .all(|path| path.starts_with("data/") || path == POOL)
    );
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
            if file.ends_with(JSON_EXTENSION) {
                assert!(written.get(file) == Some(bytes), "{file} differs");
                compared += 1;
            }
        }
    }
    assert_eq!(compared, ROSTERS + COMPETITION_FILES);
    let manifest = original.get("mod.json").expect("a mod.json");
    assert!(
        written.get("mod.json") == Some(manifest),
        "mod.json differs"
    );
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
