//! Your own copy of the PC game (`just test_real_assets <game_dir>`): World 1996-97 assembled
//! from it is a valid package, written as stated, every team its derived ids and checksum.
//! Skipped without `OPENSWOS_GAME_DIR`.

use openswos_test_utils::data::game_dir;
use openswos_test_utils::{require, tactics_path};
use openswos_world_extract::check::{builtin_tactics, game_data};
use openswos_world_extract::commands::assemble_9697;
use openswos_world_package_resource::WorldPackage;

#[test]
fn world_1996_97_assembles_from_the_pc_game() {
    let game = require!(game_dir());
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = dir.path().join("1996-97");
    let written = assemble_9697(&game, Some(&out)).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(written.rosters, 71);
    assert!(written.teams > 1_500, "{} teams", written.teams);
    assert_eq!(written.pool, 900);

    let opened = WorldPackage::open_dir(&out).expect("the package opens");
    assert!(opened.fingerprint.is_match());
    assert_eq!(opened.fingerprint.stated, written.fingerprint);
    assert!(opened.ignored.is_empty(), "{:?}", opened.ignored);
    let tactics = builtin_tactics(&tactics_path()).expect("the tactics");
    let data = game_data(&opened.package, tactics).unwrap_or_else(|error| panic!("{error}"));
    assert!(
        data.rosters()
            .rosters()
            .iter()
            .flat_map(|roster| &roster.teams)
            .all(|team| team.canonical),
        "derived checksums are the teams'"
    );
}

#[test]
fn assembling_twice_gives_the_same_package() {
    let game = require!(game_dir());
    let dir = tempfile::tempdir().expect("a temp dir");
    let first = assemble_9697(&game, Some(&dir.path().join("a"))).expect("assembled");
    let second = assemble_9697(&game, Some(&dir.path().join("b"))).expect("assembled");
    assert_eq!(first.fingerprint, second.fingerprint);
}
