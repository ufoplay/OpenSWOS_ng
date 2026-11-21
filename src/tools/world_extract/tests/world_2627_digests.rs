//! World 2026-27's completion again, diffed against its committed digests
//! (`assets/WORLD_2026_27/completion.json`, M4 #5e). The completion's files may not be
//! committed; their SHA-256s may. An upstream bump, or a change to the completion, that changes
//! a file fails here and names it, line by line against the cached completion when the cache
//! is the one the digests know. Review, then write the digests again
//! (`OPENSWOS_BLESS=1 just test world_2627_digests`) and the cache (`just world_complete_2627`).
//! A file of its own: the baked install runs once a process. Skipped without the oracle or the
//! network.

use openswos_test_utils::assets_path;
use openswos_test_utils::fixtures::SidecarMode;
use openswos_world_extract::diff::{Digests, FilesDiff};
use openswos_world_extract::output::{WORLD_2026_27_DIR, cache_dir};
use openswos_world_package_resource::PackageFiles;

mod common;
use common::completed;

/// The completion's digests, under `assets/`.
const DIGESTS: &str = "WORLD_2026_27/completion.json";

#[test]
fn world_2627_digests_know_the_completion() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = dir.path().join("2026-27");
    if completed(&out).is_none() {
        return;
    }
    let fresh = PackageFiles::read_dir(&out).expect("the completion");
    let digests = Digests::of(&fresh);
    let path = assets_path().join(DIGESTS);
    match SidecarMode::from_env() {
        SidecarMode::Bless => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("the digests' folder");
            }
            std::fs::write(&path, digests.to_json()).expect("the digests written");
        }
        SidecarMode::Compare => {
            let text = std::fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!("{}: {error} (OPENSWOS_BLESS=1 writes it)", path.display())
            });
            let known = Digests::from_json(&text)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let diff = FilesDiff::between_digests(&known, &digests);
            if diff.is_empty() {
                return;
            }
            let cache = cache_dir(WORLD_2026_27_DIR);
            let detail = PackageFiles::read_dir(&cache)
                .ok()
                .filter(|cached| Digests::of(cached) == known)
                .map_or(diff, |cached| FilesDiff::between(&cached, &fresh));
            panic!(
                "World 2026-27's completion is not the one {DIGESTS} knows ({detail}). Review it \
                 against {}, then OPENSWOS_BLESS=1 just test world_2627_digests and just \
                 world_complete_2627",
                cache.display()
            );
        }
    }
}
