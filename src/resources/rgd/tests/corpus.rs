//! swos-port's recordings (downloaded into `target/cache/`, D8): each header reads and
//! writes back byte for byte, and every frame holds the invariants a right layout gives
//! (upstream `rgd_convert.py --verify`): frame numbers run on from the first, the clock
//! never goes back and stays within 200 minutes, its first digit is 0, no team scores
//! more than 30. Six recordings end as upstream's table says (docs/TESTS.md: frames,
//! minutes, score). Version 1.2's one recording does not read: its frames are not known.
//! Skipped when a recording can be neither found in the cache nor downloaded.

use std::fs::File;
use std::io::{BufReader, Read};

use flate2::read::GzDecoder;
use openswos_common::ByteWriter;
use openswos_rgd_resource::{RgdError, RgdReader, VersionError};
use openswos_test_utils::cache::{fetch, rgd_recording};

/// The most minutes a match's clock shows (extra time included, with margin).
const MAX_MINUTES: u16 = 200;

/// The most goals a team scores in a recording, with margin.
const MAX_GOALS: u8 = 30;

/// How a recording ends.
struct Ending {
    frames: usize,
    minutes: u16,
    goals: (u8, u8),
}

/// The recording `file_name`'s stream, decompressed.
fn open(file_name: &str) -> Option<GzDecoder<BufReader<File>>> {
    let download = rgd_recording(file_name).expect("a recording of the corpus");
    let path = match fetch(download) {
        Ok(path) => path,
        Err(skip) => {
            openswos_test_utils::skip::report(&skip);
            return None;
        }
    };
    let file = File::open(&path).expect("the cached recording");
    Some(GzDecoder::new(BufReader::new(file)))
}

/// Reads `file_name` whole: checks its header round-trips and its frames' invariants,
/// and returns how it ends and its teams' names.
fn check(file_name: &str) -> Option<(Ending, [String; 2])> {
    let raw_header = {
        let mut stream = open(file_name)?;
        let mut version = [0; 2];
        stream.read_exact(&mut version).expect("a version");
        let [major, minor] = version;
        let len = openswos_rgd_resource::RgdVersion::new(major, minor)
            .expect("a version")
            .header_len();
        let mut bytes = version.to_vec();
        bytes.resize(len, 0);
        stream
            .read_exact(&mut bytes[version.len()..])
            .expect("a header");
        bytes
    };
    let mut reader = RgdReader::new(open(file_name)?).expect("a recording");
    let mut writer = ByteWriter::new();
    reader.header().write(&mut writer).expect("writes");
    assert!(
        writer.into_bytes() == raw_header,
        "{file_name}: the header writes back"
    );
    let names = [
        reader.header().top_team.name.text(),
        reader.header().bottom_team.name.text(),
    ];
    let mut frames = 0;
    let mut first_no = None;
    let mut minutes = 0;
    let mut goals = (0, 0);
    while let Some(frame) = reader.next_frame().expect("a frame") {
        let first = *first_no.get_or_insert(frame.frame_no());
        let expected = u64::from(first) + u64::try_from(frames).expect("a count");
        assert_eq!(
            u64::from(frame.frame_no()),
            expected,
            "{file_name}: frame {frames}'s number"
        );
        let [zero, ..] = frame.game_time();
        assert_eq!(zero, 0, "{file_name}: frame {frames}'s clock");
        assert!(
            frame.minutes() >= minutes,
            "{file_name}: frame {frames}: the clock went back"
        );
        assert!(frame.minutes() <= MAX_MINUTES);
        assert!(frame.team1_goals() <= MAX_GOALS && frame.team2_goals() <= MAX_GOALS);
        minutes = frame.minutes();
        goals = (frame.team1_goals(), frame.team2_goals());
        frames += 1;
    }
    assert!(frames > 0, "{file_name}: no frames");
    Some((
        Ending {
            frames,
            minutes,
            goals,
        },
        names,
    ))
}

/// A recording with a known ending.
fn ends_as(file_name: &str, frames: usize, minutes: u16, goals: (u8, u8)) {
    let Some((ending, _)) = check(file_name) else {
        return;
    };
    assert_eq!(ending.frames, frames, "{file_name}: frames");
    assert_eq!(ending.minutes, minutes, "{file_name}: minutes");
    assert_eq!(ending.goals, goals, "{file_name}: goals");
}

/// A recording between two teams.
fn plays(file_name: &str, top: &str, bottom: &str) {
    let Some((_, names)) = check(file_name) else {
        return;
    };
    assert_eq!(names, [top.to_owned(), bottom.to_owned()], "{file_name}");
}

#[test]
fn zkz_12065730_v1p5() {
    ends_as("zkz-12065730.RGD.gz", 1519, 3, (1, 0));
}

#[test]
fn zkz_11021807_v1p3() {
    ends_as("zkz-11021807.RGD.gz", 4064, 6, (0, 0));
}

#[test]
fn zkz_11054632_v1p1() {
    ends_as("zkz-11054632.RGD.gz", 7314, 8, (0, 0));
}

#[test]
fn playa_10090613_v1p1() {
    ends_as("playa-10090613.RGD.gz", 22_497, 90, (3, 0));
}

#[test]
fn zkz_10140228_v1p1() {
    ends_as("zkz-10140228.RGD.gz", 39_066, 120, (3, 3));
}

#[test]
fn zkz_12080203_v1p4() {
    ends_as("zkz-12080203.RGD.gz", 31_635, 90, (10, 10));
}

#[test]
fn the_other_recordings_read() {
    for (file_name, top, bottom) in [
        ("2xcpu-08030320.RGD.gz", "BRAZIL", "ITALY"),
        ("playa-10095214.RGD.gz", "BRAZIL", "URUGUAY"),
        ("playa-10144556.RGD.gz", "FORT. DUSSELDORF", "FREIBURG"),
        ("shootout-08200245.RGD.gz", "LEGIA WARSAW", "WIDZEW LODZ"),
        ("zkz-10173518.RGD.gz", "BRAZIL", "ARGENTINA"),
        ("zkz-10174427.RGD.gz", "AC MILAN", "JUVENTUS"),
        ("zkz-10190651.RGD.gz", "FORT. DUSSELDORF", "MSV DUISBURG"),
        ("zkz-11215003.RGD.gz", "JUVENTUS", "MANCHESTER UTD"),
    ] {
        plays(file_name, top, bottom);
    }
}

#[test]
fn version_1p2_does_not_read() {
    let Some(stream) = open("zkz-11122841.RGD.gz") else {
        return;
    };
    assert!(matches!(
        RgdReader::new(stream),
        Err(RgdError::Version(VersionError::Unresolved))
    ));
}
