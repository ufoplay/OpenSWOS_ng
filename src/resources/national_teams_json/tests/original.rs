//! The original's national teams, as upstream compiles them (`_nationalTeams`,
//! `_nationalityOrdinalsArray`, src/swos_data_bulk_imports.c; `_numNationalTeams`,
//! `_nat_teams_rand_limit1`, src/swos_amiga_data_imports.c): they make a file that
//! round-trips, and every record its 6 bytes. Read from upstream's checkout in the cache
//! (`just upstream_fetch`); skipped without it.

use std::collections::BTreeSet;
use std::path::Path;

use openswos_common::json::TrailingNewline;
use openswos_common::{Exporter, Importer, Nationality, TeamFileNumber};
use openswos_national_teams_json_resource::{
    JobOfferPick, ManagerNationality, NATIONAL_TEAM_RECORD_LEN, NationalTeamRecord, NationalTeams,
    NationalTeamsDocument, NationalTeamsJsonFormat, OTHER,
};
use openswos_test_utils::skip::{Skip, report};

/// The records the original counts (`_numNationalTeams`, 0x3F).
const ORIGINAL_RECORDS: usize = 63;

/// The nationalities the original's manager may take, OTHER aside.
const ORIGINAL_NATIONALITIES: usize = 63;

/// The original's job-offer pick (`_nat_teams_rand_limit1`, 0x29).
const ORIGINAL_JOB_OFFER_PICK: u16 = 41;

/// The radix of the hex literals.
const HEX_RADIX: u32 = 16;

/// The team files of the original's national teams: one per continent.
const NATIONAL_TEAM_FILES: std::ops::RangeInclusive<u8> = 80..=85;

/// The values of the C array initialiser starting at `declaration` in `text`: hex,
/// decimal or character literals.
fn c_array(text: &str, declaration: &str) -> Vec<u8> {
    let start = text.find(declaration).expect("the declaration");
    let body = text
        .get(start..)
        .and_then(|rest| rest.split_once('{'))
        .and_then(|(_, rest)| rest.split_once("};"))
        .map(|(body, _)| body)
        .expect("an initialiser");
    let mut uncommented = String::new();
    let mut rest = body;
    while let Some((before, after)) = rest.split_once("/*") {
        uncommented.push_str(before);
        rest = after.split_once("*/").map_or("", |(_, after)| after);
    }
    uncommented.push_str(rest);
    uncommented
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| {
            if let Some(hex) = token.strip_prefix("0x") {
                u8::from_str_radix(hex, HEX_RADIX).expect("a hex byte")
            } else if let Some(character) = token.strip_prefix('\'') {
                character.bytes().next().expect("a character literal")
            } else {
                token.parse().expect("a decimal byte")
            }
        })
        .collect()
}

fn original() -> Option<(Vec<u8>, NationalTeams)> {
    let path = openswos_test_utils::cache_path()
        .join("upstream")
        .join("OpenSWOS")
        .join("src")
        .join("swos_data_bulk_imports.c");
    let Ok(bytes) = std::fs::read(&path) else {
        report(&Skip::new(format!(
            "no {} (just upstream_fetch)",
            Path::new(&path).display()
        )));
        return None;
    };
    // The file is Latin-1: its tables are ASCII.
    let text = String::from_utf8_lossy(&bytes);
    let table = c_array(&text, "uint8_t _nationalTeams[");
    let counted = table
        .get(..ORIGINAL_RECORDS * NATIONAL_TEAM_RECORD_LEN)
        .expect("the counted records")
        .to_vec();
    let (chunks, _) = counted.as_chunks::<NATIONAL_TEAM_RECORD_LEN>();
    let records = chunks
        .iter()
        .copied()
        .map(NationalTeamRecord::from_bytes)
        .collect();
    let nationalities = c_array(&text, "uint8_t _nationalityOrdinalsArray[")
        .into_iter()
        .take_while(|code| Nationality(*code) != OTHER)
        .map(|code| ManagerNationality::try_from(code).expect("not OTHER"))
        .collect();
    let teams = NationalTeams::new(
        records,
        Some(nationalities),
        Some(JobOfferPick::try_from(ORIGINAL_JOB_OFFER_PICK).expect("a pick")),
    )
    .expect("the original's tables fit");
    Some((counted, teams))
}

#[test]
fn the_originals_tables_make_a_file() {
    let Some((bytes, teams)) = original() else {
        return;
    };
    let table: Vec<u8> = teams
        .records()
        .iter()
        .flat_map(|record| record.to_bytes())
        .collect();
    assert_eq!(table, bytes);
    let document = NationalTeamsDocument {
        teams,
        trailing_newline: TrailingNewline::Present,
    };
    let text = NationalTeamsJsonFormat::export(&document).expect("serializes");
    assert_eq!(
        NationalTeamsJsonFormat::import(&text).expect("reads back"),
        document
    );
}

#[test]
fn the_originals_tables_agree() {
    let Some((_, teams)) = original() else { return };
    let records = teams.records();
    assert_eq!(records.len(), ORIGINAL_RECORDS);
    let first = records.first().expect("a record");
    assert_eq!(first.to_bytes(), [80, 0, 0, 0, 0, 19], "upstream's example");
    assert!(records.iter().all(|record| {
        let TeamFileNumber(file) = record.team.file;
        NATIONAL_TEAM_FILES.contains(&file)
    }));
    assert!(records.iter().all(|record| record.club_file.is_some()));
    let nationalities: BTreeSet<Nationality> =
        records.iter().map(|record| record.nationality).collect();
    assert_eq!(
        nationalities.len(),
        ORIGINAL_RECORDS,
        "one national team each"
    );
    let listed = teams.nationalities().expect("listed");
    assert_eq!(listed.len(), ORIGINAL_NATIONALITIES);
    assert!(
        listed
            .iter()
            .all(|listed| nationalities.contains(&listed.nationality())),
        "every nationality a manager may take has a national team"
    );
}
