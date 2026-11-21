//! World 1996-97's committed package (`data/worlds/1996-97/`) is what `world_extract` extracts
//! from the pinned upstream checkout and its compiled tables: every file, byte for byte (M4
//! #5e). After an upstream bump that changes them, the failure names each file that differs
//! and its first line that does; `just world_extract_9697_data` writes them again. Skipped
//! without the oracle (no checkout, `just upstream_fetch`).

use openswos_test_utils::skip::{Skip, report};
use openswos_test_utils::world_1996_97_path;
use openswos_world_extract::commands::{extract_9697, upstream_checkout};
use openswos_world_extract::diff::FilesDiff;
use openswos_world_extract::error::ExtractError;
use openswos_world_extract::extract::{committed_files, world_1996_97};
use openswos_world_package_resource::PackageFiles;

/// The committed package's files: ten competition files, the career params, five rules.
const COMMITTED_FILES: usize = 16;

#[test]
fn the_committed_package_is_the_extraction() {
    let package = match world_1996_97(&upstream_checkout()) {
        Ok(package) => package,
        Err(ExtractError::Oracle(unavailable)) => {
            report(&Skip::new(unavailable.to_string()));
            return;
        }
        Err(error) => panic!("{error}"),
    };
    let extracted = committed_files(&package).expect("the files");
    let committed = PackageFiles::read_dir(&world_1996_97_path()).expect("the committed package");
    let diff = FilesDiff::between(&committed, &extracted);
    assert!(
        diff.is_empty(),
        "data/worlds/1996-97 is not the extraction ({diff}): review, then just \
         world_extract_9697_data"
    );
}

#[test]
fn the_extraction_writes_a_new_folder() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = dir.path().join("1996-97");
    match extract_9697(Some(&out)) {
        Ok(extracted) => {
            assert_eq!(extracted.files, COMMITTED_FILES);
            assert!(out.join("rules/season.json").is_file());
            assert!(!out.join("WORLD.ini").exists());
        }
        Err(ExtractError::Oracle(unavailable)) => report(&Skip::new(unavailable.to_string())),
        Err(error) => panic!("{error}"),
    }
}
