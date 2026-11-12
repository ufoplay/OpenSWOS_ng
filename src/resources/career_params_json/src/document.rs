//! The file: an object of knobs and notes, kept in the order written.
//!
//! Upstream's file is written by hand: two-space indent, one member a line, arrays on one
//! line, a blank line before each note but the first member, a newline at the end. The
//! writer lays every file out that way, so upstream's round-trips byte for byte.

use std::collections::BTreeSet;
use std::convert::Infallible;
use std::fmt;

use openswos_common::json::TrailingNewline;
use openswos_common::{Exporter, Importer};
use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use thiserror::Error;

use crate::knobs::{PerDivisionKnob, SignedKnob, UnsignedKnob};

/// What a note's key starts with. No knob's key does.
pub const NOTE_PREFIX: char = '_';

/// One level of indentation in upstream's file.
const INDENT: &str = "  ";

/// A note's key: anything that starts with `_` (`_finances_wages`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteKey(String);

/// A key that does not start with `_`, given as a note's.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("`{0}` is no note's key: it does not start with `{NOTE_PREFIX}`")]
pub struct NotANoteKey(pub String);

impl NoteKey {
    /// The key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NoteKey {
    type Error = NotANoteKey;

    fn try_from(key: String) -> Result<Self, Self::Error> {
        if key.starts_with(NOTE_PREFIX) {
            Ok(Self(key))
        } else {
            Err(NotANoteKey(key))
        }
    }
}

/// A member of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Member {
    /// A note for whoever edits the file. The game does not read it.
    Note {
        /// Its key.
        key: NoteKey,
        /// Its text.
        text: String,
    },
    /// An unsigned knob and its value.
    Unsigned(UnsignedKnob, u16),
    /// A signed knob and its value.
    Signed(SignedKnob, i16),
    /// A per-division knob and its values, the top division first. The game reads the
    /// first four; a division without one keeps its value.
    PerDivision(PerDivisionKnob, Vec<u16>),
}

impl Member {
    /// Its key.
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Note { key, .. } => key.as_str(),
            Self::Unsigned(knob, _) => knob.key(),
            Self::Signed(knob, _) => knob.key(),
            Self::PerDivision(knob, _) => knob.key(),
        }
    }
}

/// A `career_params.json`: its members in the order written, each key once. The game reads
/// the knobs it finds over its defaults (upstream `DataJson_LoadCareerParams`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareerParamsDocument {
    /// The members.
    pub members: Vec<Member>,
    /// Whether the text ends with a newline (upstream's does).
    pub trailing_newline: TrailingNewline,
}

impl CareerParamsDocument {
    /// The value of `knob`, if the file sets it.
    #[must_use]
    pub fn unsigned(&self, knob: UnsignedKnob) -> Option<u16> {
        self.members.iter().find_map(|member| match member {
            Member::Unsigned(found, value) => (*found == knob).then_some(*value),
            Member::Note { .. } | Member::Signed(..) | Member::PerDivision(..) => None,
        })
    }

    /// The value of `knob`, if the file sets it.
    #[must_use]
    pub fn signed(&self, knob: SignedKnob) -> Option<i16> {
        self.members.iter().find_map(|member| match member {
            Member::Signed(found, value) => (*found == knob).then_some(*value),
            Member::Note { .. } | Member::Unsigned(..) | Member::PerDivision(..) => None,
        })
    }

    /// The values of `knob`, if the file sets it.
    #[must_use]
    pub fn per_division(&self, knob: PerDivisionKnob) -> Option<&[u16]> {
        self.members.iter().find_map(|member| match member {
            Member::PerDivision(found, values) => (*found == knob).then_some(values.as_slice()),
            Member::Note { .. } | Member::Unsigned(..) | Member::Signed(..) => None,
        })
    }
}

/// The members, as read.
struct Members(Vec<Member>);

impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(MembersVisitor)
    }
}

struct MembersVisitor;

impl<'de> Visitor<'de> for MembersVisitor {
    type Value = Members;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object of career knobs and notes")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Members, A::Error> {
        let mut members = Vec::new();
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!("duplicate key `{key}`")));
            }
            let member = if let Some(knob) = UnsignedKnob::from_key(&key) {
                Member::Unsigned(knob, map.next_value()?)
            } else if let Some(knob) = SignedKnob::from_key(&key) {
                Member::Signed(knob, map.next_value()?)
            } else if let Some(knob) = PerDivisionKnob::from_key(&key) {
                Member::PerDivision(knob, map.next_value()?)
            } else {
                let key = NoteKey::try_from(key).map_err(|NotANoteKey(key)| {
                    de::Error::custom(format!(
                        "unknown key `{key}`: neither a knob nor a note (`{NOTE_PREFIX}…`)"
                    ))
                })?;
                Member::Note {
                    key,
                    text: map.next_value()?,
                }
            };
            members.push(member);
        }
        Ok(Members(members))
    }
}

/// `text` as a JSON string. Characters beyond ASCII are written as they are, as upstream's
/// file has them.
fn string(text: &str) -> String {
    serde_json::to_string(text).expect("a string serialises")
}

/// A member's value as written.
fn value_text(member: &Member) -> String {
    match member {
        Member::Note { text, .. } => string(text),
        Member::Unsigned(_, value) => value.to_string(),
        Member::Signed(_, value) => value.to_string(),
        Member::PerDivision(_, values) => {
            let values: Vec<String> = values.iter().map(u16::to_string).collect();
            format!("[{}]", values.join(", "))
        }
    }
}

/// The file's text, laid out as upstream's.
fn to_text(document: &CareerParamsDocument) -> String {
    let mut text = String::from("{");
    for (index, member) in document.members.iter().enumerate() {
        let first = index == 0;
        text.push_str(if first { "\n" } else { ",\n" });
        if !first && matches!(member, Member::Note { .. }) {
            text.push('\n');
        }
        text.push_str(&format!(
            "{INDENT}{}: {}",
            string(member.key()),
            value_text(member)
        ));
    }
    if !document.members.is_empty() {
        text.push('\n');
    }
    text.push('}');
    match document.trailing_newline {
        TrailingNewline::Present => text.push('\n'),
        TrailingNewline::Absent => {}
    }
    text
}

/// The `career_params.json` format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CareerParamsJsonFormat;

impl Importer for CareerParamsJsonFormat {
    type Model = CareerParamsDocument;
    type Error = serde_json::Error;

    fn import(bytes: &[u8]) -> Result<CareerParamsDocument, serde_json::Error> {
        let Members(members) = serde_json::from_slice(bytes)?;
        Ok(CareerParamsDocument {
            members,
            trailing_newline: TrailingNewline::of(bytes),
        })
    }
}

impl Exporter for CareerParamsJsonFormat {
    type Model = CareerParamsDocument;
    type Error = Infallible;

    fn export(model: &CareerParamsDocument) -> Result<Vec<u8>, Infallible> {
        Ok(to_text(model).into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// A file laid out as upstream's.
    const SMALL: &str = "{\n  \"_comment\": \"Career knobs — edit and relaunch.\",\n\n  \"_wages\": \"trialists and reserves\",\n  \"wage_trialist_pct\": 33,\n  \"wage_reserve_pct\": 66,\n\n  \"_board\": \"thresholds\",\n  \"board_ratio_poor\": -10,\n  \"sponsorship_weeks\": [18, 14, 10, 6]\n}\n";

    fn read(text: &str) -> Result<CareerParamsDocument, serde_json::Error> {
        CareerParamsJsonFormat::import(text.as_bytes())
    }

    fn write(document: &CareerParamsDocument) -> String {
        let bytes = CareerParamsJsonFormat::export(document).unwrap_or_else(|never| match never {});
        String::from_utf8(bytes).expect("UTF-8")
    }

    #[test]
    fn reads_members_in_order_and_writes_upstreams_layout() {
        let document = read(SMALL).expect("a file");
        let keys: Vec<&str> = document.members.iter().map(Member::key).collect();
        assert_eq!(
            keys,
            vec![
                "_comment",
                "_wages",
                "wage_trialist_pct",
                "wage_reserve_pct",
                "_board",
                "board_ratio_poor",
                "sponsorship_weeks"
            ]
        );
        assert_eq!(document.unsigned(UnsignedKnob::WageReservePct), Some(66));
        assert_eq!(document.unsigned(UnsignedKnob::NumSeasons), None);
        assert_eq!(document.signed(SignedKnob::BoardRatioPoor), Some(-10));
        assert_eq!(
            document.per_division(PerDivisionKnob::SponsorshipWeeks),
            Some(&[18, 14, 10, 6][..])
        );
        assert_eq!(document.trailing_newline, TrailingNewline::Present);
        assert_eq!(write(&document), SMALL);
    }

    #[test]
    fn any_layout_reads_and_is_written_in_upstreams() {
        let compact = r#"{"_comment":"Career knobs — edit and relaunch.","_wages":"trialists and reserves","wage_trialist_pct":33,"wage_reserve_pct":66,"_board":"thresholds","board_ratio_poor":-10,"sponsorship_weeks":[18,14,10,6]}"#;
        let mut document = read(compact).expect("a file");
        assert_eq!(document.trailing_newline, TrailingNewline::Absent);
        document.trailing_newline = TrailingNewline::Present;
        assert_eq!(write(&document), SMALL);
    }

    #[test]
    fn an_empty_file_is_an_empty_object() {
        let document = read("{}").expect("a file");
        assert!(document.members.is_empty());
        assert_eq!(write(&document), "{}");
    }

    #[test]
    fn per_division_values_keep_their_count() {
        for values in ["[]", "[7]", "[1, 2, 3, 4, 5]"] {
            let text = format!("{{\n  \"league_distribution_weeks\": {values}\n}}");
            let document = read(&text).expect("a file");
            assert_eq!(write(&document), text);
        }
    }

    #[test]
    fn what_the_game_would_not_read_as_a_knob_does_not_read() {
        for text in [
            r#"{"num_season": 20}"#,
            r#"{"num_seasons": 20, "num_seasons": 21}"#,
            r#"{"num_seasons": -1}"#,
            r#"{"num_seasons": 65536}"#,
            r#"{"num_seasons": 20.5}"#,
            r#"{"num_seasons": "20"}"#,
            r#"{"board_ratio_ok": 32768}"#,
            r#"{"sponsorship_weeks": 3}"#,
            r#"{"_note": 3}"#,
            r#"[1, 2]"#,
        ] {
            assert!(read(text).is_err(), "{text}");
        }
    }

    #[test]
    fn errors_name_the_key() {
        let error = read(r#"{"num_season": 20}"#).expect_err("unknown");
        assert!(
            error.to_string().starts_with("unknown key `num_season`"),
            "{error}"
        );
        let error = read(r#"{"_a": "x", "_a": "y"}"#).expect_err("duplicate");
        assert!(
            error.to_string().starts_with("duplicate key `_a`"),
            "{error}"
        );
    }

    #[test]
    fn note_keys_start_with_an_underscore() {
        assert_eq!(
            NoteKey::try_from("_x".to_owned()).map(|key| key.as_str().to_owned()),
            Ok("_x".to_owned())
        );
        assert_eq!(
            NoteKey::try_from("x".to_owned()),
            Err(NotANoteKey("x".to_owned()))
        );
    }

    fn any_document() -> impl Strategy<Value = CareerParamsDocument> {
        (
            proptest::collection::vec(proptest::option::of(any::<u16>()), UnsignedKnob::ALL.len()),
            proptest::collection::vec(proptest::option::of(any::<i16>()), SignedKnob::ALL.len()),
            proptest::collection::vec(
                proptest::option::of(proptest::collection::vec(any::<u16>(), 0..6)),
                PerDivisionKnob::ALL.len(),
            ),
            proptest::collection::vec(".*", 0..4),
            any::<bool>(),
        )
            .prop_flat_map(|(unsigned, signed, per_division, notes, newline)| {
                let mut members: Vec<Member> = Vec::new();
                members.extend(
                    UnsignedKnob::ALL
                        .iter()
                        .zip(unsigned)
                        .filter_map(|(knob, value)| Some(Member::Unsigned(*knob, value?))),
                );
                members.extend(
                    SignedKnob::ALL
                        .iter()
                        .zip(signed)
                        .filter_map(|(knob, value)| Some(Member::Signed(*knob, value?))),
                );
                members.extend(
                    PerDivisionKnob::ALL
                        .iter()
                        .zip(per_division)
                        .filter_map(|(knob, values)| Some(Member::PerDivision(*knob, values?))),
                );
                members.extend(
                    notes
                        .into_iter()
                        .enumerate()
                        .map(|(index, text)| Member::Note {
                            key: NoteKey::try_from(format!("{NOTE_PREFIX}note {index}"))
                                .expect("a note key"),
                            text,
                        }),
                );
                let trailing_newline = if newline {
                    TrailingNewline::Present
                } else {
                    TrailingNewline::Absent
                };
                Just(members)
                    .prop_shuffle()
                    .prop_map(move |members| CareerParamsDocument {
                        members,
                        trailing_newline,
                    })
            })
    }

    proptest! {
        #[test]
        fn every_document_round_trips(document in any_document()) {
            let text = write(&document);
            prop_assert_eq!(read(&text).expect("reads back"), document);
        }
    }
}
