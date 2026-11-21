//! Upstream's `career_params.json` (World 1996-97's, `data/worlds/1996-97/rules/`, GPL): it round-trips,
//! sets every knob once, and states the original's values. When upstream's checkout is in
//! the cache (`just upstream_fetch`), the knobs are the ones its loader reads.

use std::collections::BTreeSet;

use openswos_career_params_json_resource::{
    CareerParamsDocument, CareerParamsJsonFormat, Member, PerDivisionKnob, SignedKnob, UnsignedKnob,
};
use openswos_test_utils::career_params_path;
use openswos_test_utils::fixtures::assert_round_trip;
use openswos_test_utils::skip::{Skip, report};

/// The notes of upstream's file: a comment, then one per group of knobs.
const UPSTREAM_NOTES: usize = 11;

/// What precedes a key the loader reads, in upstream's loader: `LDU("$.num_seasons", …)`.
const LOADER_KEY_PREFIX: &str = "(\"$.";

fn upstream() -> CareerParamsDocument {
    let path = career_params_path();
    let bytes = std::fs::read(&path).expect("the committed career_params.json");
    assert_round_trip::<CareerParamsJsonFormat>(&bytes)
}

/// Every knob's key.
fn knob_keys() -> BTreeSet<&'static str> {
    UnsignedKnob::ALL
        .iter()
        .map(|knob| knob.key())
        .chain(SignedKnob::ALL.iter().map(|knob| knob.key()))
        .chain(PerDivisionKnob::ALL.iter().map(|knob| knob.key()))
        .collect()
}

#[test]
fn upstreams_file_round_trips_and_sets_every_knob() {
    let document = upstream();
    let notes = document
        .members
        .iter()
        .filter(|member| matches!(member, Member::Note { .. }))
        .count();
    assert_eq!(notes, UPSTREAM_NOTES);
    let set: BTreeSet<&str> = document
        .members
        .iter()
        .filter(|member| !matches!(member, Member::Note { .. }))
        .map(Member::key)
        .collect();
    assert_eq!(set, knob_keys());
}

#[test]
fn upstreams_file_states_the_original_values() {
    let document = upstream();
    // Upstream src/career_params.c: the original game's values.
    for (knob, value) in [
        (UnsignedKnob::StartBalanceScale, 1000),
        (UnsignedKnob::GateLeagueMultHome, 10_000),
        (UnsignedKnob::WageScale, 96),
        (UnsignedKnob::NumSeasons, 20),
        (UnsignedKnob::ForeignPoolCapacity, 432),
        (UnsignedKnob::TransferQuota, 256),
        (UnsignedKnob::JobOfferPass, 42),
        (UnsignedKnob::TrialistCountMode, 0),
    ] {
        assert_eq!(document.unsigned(knob), Some(value), "{knob:?}");
    }
    for (knob, value) in [
        (SignedKnob::BoardRatioOk, 30),
        (SignedKnob::BoardRatioGood, 10),
        (SignedKnob::BoardRatioPoor, -10),
        (SignedKnob::BoardRatioTerrible, -30),
    ] {
        assert_eq!(document.signed(knob), Some(value), "{knob:?}");
    }
    for knob in PerDivisionKnob::ALL {
        assert_eq!(
            document.per_division(*knob),
            Some(&[0, 0, 0, 0][..]),
            "{knob:?}"
        );
    }
}

#[test]
fn the_knobs_are_the_ones_upstreams_loader_reads() {
    let loader = openswos_test_utils::cache_path()
        .join("upstream")
        .join("OpenSWOS")
        .join("src")
        .join("save_format_json.c");
    let Ok(text) = std::fs::read_to_string(&loader) else {
        report(&Skip::new(format!(
            "no {} (just upstream_fetch)",
            loader.display()
        )));
        return;
    };
    let body = text
        .split_once("void DataJson_LoadCareerParams(void)")
        .and_then(|(_, rest)| rest.split_once("#undef LDU"))
        .map(|(body, _)| body)
        .expect("upstream's career params loader");
    let read: BTreeSet<&str> = body
        .lines()
        .filter_map(|line| {
            let key = line.split_once(LOADER_KEY_PREFIX)?.1;
            let end = key.find(['"', '['])?;
            key.get(..end)
        })
        .collect();
    assert_eq!(read, knob_keys());
    let signed: BTreeSet<&str> = body
        .lines()
        .filter(|line| line.trim_start().starts_with("LDS(\""))
        .filter_map(|line| line.split_once(LOADER_KEY_PREFIX)?.1.split('"').next())
        .collect();
    assert_eq!(
        signed,
        SignedKnob::ALL.iter().map(|knob| knob.key()).collect()
    );
}
