//! The built-in tactics (`data/tactics/`: upstream's `bin/data_json/tactics/`, GPL): they
//! round-trip, name their presets, and hold the releases upstream describes.

use std::collections::BTreeMap;

use openswos_common::{Importer, Tactics};
use openswos_tac_resource::{TacFileFormat, TeamTactics};
use openswos_tactics_json_resource::{Preset, PresetFile, Release, TacticsJson, TacticsJsonFormat};
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::{assets_path, tactics_path};

/// The files upstream ships: the twelve presets, and two more releases of four of them.
const UPSTREAM_FILES: usize = 20;

/// Every built-in tactics file, by file.
fn all() -> BTreeMap<PresetFile, TacticsJson> {
    list_fixtures(&tactics_path(), |name| name.ends_with(".json"))
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a UTF-8 file name");
            let file = PresetFile::parse(name).unwrap_or_else(|| panic!("{name} is a preset file"));
            let bytes = std::fs::read(&path).expect("a readable tactics file");
            (file, assert_round_trip::<TacticsJsonFormat>(&bytes))
        })
        .collect()
}

fn get(
    files: &BTreeMap<PresetFile, TacticsJson>,
    preset: Preset,
    release: Release,
) -> &TeamTactics {
    &files
        .get(&PresetFile { preset, release })
        .unwrap_or_else(|| panic!("{preset:?} {release:?}"))
        .tactics
}

/// The position bytes in which two tactics differ.
fn differing_positions(a: &TeamTactics, b: &TeamTactics) -> usize {
    a.players
        .iter()
        .zip(&b.players)
        .flat_map(|(a, b)| a.0.iter().flatten().zip(b.0.iter().flatten()))
        .filter(|(a, b)| a != b)
        .count()
}

#[test]
fn every_file_round_trips_and_names_its_preset() {
    let files = all();
    assert_eq!(files.len(), UPSTREAM_FILES);
    for (file, json) in &files {
        assert_eq!(json.id, file.preset.id(), "{file:?}");
        assert_eq!(json.label, file.preset.label(), "{file:?}");
    }
}

#[test]
fn each_preset_plays_its_own_slot_while_the_ball_is_out() {
    let files = all();
    for preset in Preset::ALL {
        assert_eq!(
            get(&files, preset, Release::Shipped).ball_out_of_play,
            Tactics::from(preset),
            "{preset:?}"
        );
    }
}

#[test]
fn the_releases_differ_where_upstream_says() {
    let files = all();
    // src/tactics_data.c: the DOS release re-tuned 4-4-2 in 68 position bytes, Sweep in
    // 64, 4-3-3 in 10; its 3-4-3 is the Amiga's.
    for (preset, dos_changes) in [
        (Preset::FourFourTwo, 68),
        (Preset::Sweep, 64),
        (Preset::FourThreeThree, 10),
        (Preset::ThreeFourThree, 0),
    ] {
        let original = get(&files, preset, Release::Original);
        let dos = get(&files, preset, Release::Dos);
        let shipped = get(&files, preset, Release::Shipped);
        assert_eq!(
            differing_positions(original, dos),
            dos_changes,
            "{preset:?}"
        );
        assert_ne!(original, shipped, "{preset:?}: OpenSWOS corrects it");
        assert_eq!(original.pairs, dos.pairs, "{preset:?}");
    }
    // The grade letter of the name: DOS grades 4-4-2 "B" where the Amiga says "A".
    assert_eq!(
        get(&files, Preset::FourFourTwo, Release::Original)
            .name
            .as_bytes()[5],
        b'A'
    );
    assert_eq!(
        get(&files, Preset::FourFourTwo, Release::Dos)
            .name
            .as_bytes()[5],
        b'B'
    );
}

#[test]
fn swos_2020s_cpu_5_3_2_is_the_built_in_one() {
    let path = assets_path().join("TAC").join("04_5-3-2.tac");
    let bytes = std::fs::read(&path).expect("the SWOS 2020 tactics file");
    let tac = TacFileFormat::import(&bytes).expect("a tactics file");
    assert_eq!(
        &tac.tactics,
        get(&all(), Preset::FiveThreeTwo, Release::Shipped)
    );
}
