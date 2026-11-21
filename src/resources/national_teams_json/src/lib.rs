#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod record;
pub mod teams;

use openswos_common::json::{Indent, NonAscii, TrailingNewline, to_text};
use openswos_common::{Exporter, Importer};

pub use record::{NATIONAL_TEAM_RECORD_LEN, NationalTeamRecord};
pub use teams::{
    InvalidJobOfferPick, JobOfferPick, ListedOther, MAX_NATIONALITIES, MAX_RECORDS,
    ManagerNationality, NationalTeams, NationalTeamsError, OTHER,
};

/// A `national_teams.json`, its indentation and whether its text ends with a newline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NationalTeamsDocument {
    /// The file's content.
    pub teams: NationalTeams,
    /// Its indentation: two spaces as upstream's other data JSON, one in World 2026-27's
    /// (upstream `scripts/worldgen/nations.py`).
    pub indent: Indent,
    /// Whether its text ends with a newline.
    pub trailing_newline: TrailingNewline,
}

/// The indentation of the files this port writes: two spaces, as upstream's other data JSON.
pub const DATA_JSON_INDENT: Indent = Indent::Two;

/// The `national_teams.json` format, as Python's `json.dump` writes it. OpenSWOS itself has no
/// writer for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NationalTeamsJsonFormat;

impl Importer for NationalTeamsJsonFormat {
    type Model = NationalTeamsDocument;
    type Error = serde_json::Error;

    fn import(bytes: &[u8]) -> Result<NationalTeamsDocument, serde_json::Error> {
        Ok(NationalTeamsDocument {
            teams: serde_json::from_slice(bytes)?,
            indent: Indent::of(bytes),
            trailing_newline: TrailingNewline::of(bytes),
        })
    }
}

impl Exporter for NationalTeamsJsonFormat {
    type Model = NationalTeamsDocument;
    type Error = serde_json::Error;

    fn export(model: &NationalTeamsDocument) -> Result<Vec<u8>, serde_json::Error> {
        Ok(to_text(
            &model.teams,
            model.indent,
            NonAscii::Raw,
            model.trailing_newline,
        )?
        .into_bytes())
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // records the tests just read
mod tests {
    use super::*;
    use openswos_common::Nationality;
    use proptest::prelude::*;

    /// Upstream's example (docs/DATA_PIPELINE.md), written out.
    const EXAMPLE: &str = r#"{ "records": [ { "country": 80, "ordinal": 0, "nationality": 0, "team_file": 0, "skill": 19 },
                               { "country": 80, "ordinal": 2, "nationality": 1, "team_file": 1, "skill": 24 } ],
  "nationalities": [ 8, 31 ],
  "job_offer_pick": 41 }"#;

    fn read(text: &str) -> Result<NationalTeamsDocument, serde_json::Error> {
        NationalTeamsJsonFormat::import(text.as_bytes())
    }

    #[test]
    fn reads_upstreams_example() {
        let document = read(EXAMPLE).expect("a file");
        let teams = &document.teams;
        assert_eq!(teams.records().len(), 2);
        assert_eq!(teams.records()[1].to_bytes(), [80, 2, 1, 1, 0, 24]);
        let nationalities: Vec<Nationality> = teams
            .nationalities()
            .expect("listed")
            .iter()
            .map(|listed| listed.nationality())
            .collect();
        assert_eq!(nationalities, vec![Nationality(8), Nationality(31)]);
        assert_eq!(teams.job_offer_pick().map(JobOfferPick::records), Some(41));
    }

    #[test]
    fn writes_json_dump_layout() {
        let text = "{\n  \"records\": [\n    {\n      \"country\": 80,\n      \"ordinal\": 0,\n      \"nationality\": 0,\n      \"team_file\": 255,\n      \"skill\": 19\n    }\n  ],\n  \"nationalities\": [],\n  \"job_offer_pick\": 1\n}\n";
        let document = read(text).expect("a file");
        assert_eq!(document.teams.records()[0].club_file, None);
        assert_eq!(document.teams.nationalities(), Some(&[][..]));
        assert_eq!(
            NationalTeamsJsonFormat::export(&document).ok(),
            Some(text.as_bytes().to_vec())
        );
    }

    /// World 2026-27's: a comment first, one space (upstream `scripts/worldgen/nations.py`).
    #[test]
    fn a_comment_and_one_space_round_trip() {
        let text = "{\n \"about\": \"World 2026-27: the 211 FIFA members\",\n \"records\": [\n  {\n   \"country\": 80,\n   \"ordinal\": 0,\n   \"nationality\": 0,\n   \"team_file\": 0,\n   \"skill\": 15\n  }\n ],\n \"job_offer_pick\": 91\n}\n";
        let document = read(text).expect("a file");
        assert_eq!(
            document.teams.about(),
            Some("World 2026-27: the 211 FIFA members")
        );
        assert_eq!(document.indent, Indent::One);
        assert_eq!(
            NationalTeamsJsonFormat::export(&document).ok(),
            Some(text.as_bytes().to_vec())
        );
    }

    #[test]
    fn the_optional_keys_may_be_absent() {
        let text = "{\n  \"records\": [\n    {\n      \"country\": 80,\n      \"ordinal\": 0,\n      \"nationality\": 0,\n      \"team_file\": 0,\n      \"skill\": 19\n    }\n  ]\n}";
        let document = read(text).expect("a file");
        assert_eq!(document.teams.nationalities(), None);
        assert_eq!(document.teams.job_offer_pick(), None);
        assert_eq!(
            NationalTeamsJsonFormat::export(&document).ok(),
            Some(text.as_bytes().to_vec())
        );
    }

    #[test]
    fn what_the_loader_refuses_does_not_read() {
        let record =
            r#"{"country": 80, "ordinal": 0, "nationality": 0, "team_file": 0, "skill": 19}"#;
        for text in [
            r#"{"nationalities": [8]}"#.to_owned(),
            r#"{"records": []}"#.to_owned(),
            r#"{"records": [1]}"#.to_owned(),
            format!(r#"{{"records": [{record}], "nationalities": [255]}}"#),
            format!(r#"{{"records": [{record}], "nationalities": [256]}}"#),
            format!(r#"{{"records": [{record}], "job_offer_pick": 0}}"#),
            format!(r#"{{"records": [{record}], "job_offer_pick": 257}}"#),
            format!(r#"{{"records": [{record}], "extra": 1}}"#),
            format!(
                r#"{{"records": [{}]}}"#,
                vec![record; MAX_RECORDS + 1].join(", ")
            ),
        ] {
            assert!(read(&text).is_err(), "{text}");
        }
        let error = read(r#"{"records": []}"#).expect_err("no records");
        assert!(
            error.to_string().starts_with("no national team records"),
            "{error}"
        );
    }

    fn any_teams() -> impl Strategy<Value = NationalTeams> {
        (
            proptest::collection::vec(any::<[u8; NATIONAL_TEAM_RECORD_LEN]>(), 1..=MAX_RECORDS),
            proptest::option::of(proptest::collection::vec(0..u8::MAX, 0..=MAX_NATIONALITIES)),
            proptest::option::of(1..=u16::from(u8::MAX) + 1),
            proptest::option::of("[ -~]{0,40}"),
        )
            .prop_map(|(records, nationalities, pick, about)| {
                let teams = NationalTeams::new(
                    records
                        .into_iter()
                        .map(NationalTeamRecord::from_bytes)
                        .collect(),
                    nationalities.map(|codes| {
                        codes
                            .into_iter()
                            .map(|code| ManagerNationality::try_from(code).expect("not OTHER"))
                            .collect()
                    }),
                    pick.map(|pick| JobOfferPick::try_from(pick).expect("1..=256")),
                )
                .expect("within the maxima");
                match about {
                    Some(about) => teams.with_about(about),
                    None => teams,
                }
            })
    }

    proptest! {
        #[test]
        fn every_file_round_trips(teams in any_teams(), newline: bool, one_space: bool) {
            let document = NationalTeamsDocument {
                teams,
                indent: if one_space { Indent::One } else { DATA_JSON_INDENT },
                trailing_newline: if newline { TrailingNewline::Present } else { TrailingNewline::Absent },
            };
            let bytes = NationalTeamsJsonFormat::export(&document).expect("serializes");
            prop_assert_eq!(NationalTeamsJsonFormat::import(&bytes).expect("reads back"), document);
        }
    }
}
