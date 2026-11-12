//! Real recordings (`OPENSWOS_SAVES_DIR`, `just test_saves`): every `.SRP` in the folder
//! reads, writes back byte for byte, and its chunks decode and encode back to their
//! content: the match, the frames (as many as the header says), the sounds, the seek
//! table, the chapters. Skipped without the folder; recordings are never committed (they
//! hold the game's data).

use openswos_common::Importer;
use openswos_srp_resource::{
    FourCc, SrpFormat, encode_audio, encode_chapters, encode_frames, encode_seek,
};
use openswos_test_utils::data::saves_dir;
use openswos_test_utils::fixtures::{assert_round_trip, list_fixtures};
use openswos_test_utils::require;

/// The extension of recordings.
const SRP_EXTENSION: &str = ".SRP";

#[test]
fn every_recording_round_trips_and_decodes() {
    let dir = require!(saves_dir());
    let files = list_fixtures(&dir, |name| {
        name.to_ascii_uppercase().ends_with(SRP_EXTENSION)
    });
    for path in files {
        let bytes = std::fs::read(&path).expect("a readable recording");
        let file = assert_round_trip::<SrpFormat>(&bytes);
        let data = |fourcc| file.chunk(fourcc).expect("the chunk").data().to_vec();
        if let Some(meta) = file.meta() {
            let meta = meta.expect("the match decodes");
            assert_eq!(meta.encode(), data(FourCc::META), "{}", path.display());
        }
        if let Some(frames) = file.frames() {
            let frames = frames.expect("the frames decode");
            assert_eq!(
                u32::try_from(frames.len()).ok(),
                Some(file.header.frame_count),
                "{}",
                path.display()
            );
            assert_eq!(encode_frames(&frames).expect("encodes"), data(FourCc::FRMS));
        }
        if let Some(audio) = file.audio() {
            assert_eq!(encode_audio(&audio.expect("decodes")), data(FourCc::AUDI));
        }
        if let Some(seek) = file.seek() {
            assert_eq!(
                encode_seek(&seek.expect("decodes")).expect("encodes"),
                data(FourCc::SEEK)
            );
        }
        if let Some(chapters) = file.chapters() {
            let encoded = encode_chapters(&chapters.expect("decodes")).expect("encodes");
            assert_eq!(encoded, data(FourCc::CHAP));
        }
        assert!(SrpFormat::import(&bytes).is_ok());
    }
}
