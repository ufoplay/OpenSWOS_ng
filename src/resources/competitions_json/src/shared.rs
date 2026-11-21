//! What the competition records share: ids, the `comp` header, round rules, seeded teams.

use openswos_common::json::{JsonByte, code};
use openswos_common::{TeamFileNumber, TeamOrdinal};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The largest value of a 2-bit field.
const RULE_BITS_MAX: u8 = 0b11;

/// Where a round rule byte's legs start (bits 7-6).
const LEGS_SHIFT: u8 = 6;

/// Where its extra time starts (bits 5-4).
const EXTRA_TIME_SHIFT: u8 = 4;

/// Where its penalties start (bits 3-2).
const PENALTIES_SHIFT: u8 = 2;

/// An object's id: its C symbol name upstream (`_euroCup`, `_englishLeagues1`), by which
/// the files name each other.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SymbolId(pub String);

impl SymbolId {
    /// The id's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How a competition is played (upstream `Competition.type`; the switch of
/// src/competition_data.c `comp_name_table`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum CompetitionType {
    /// Round robin: the leagues, and the qualifying groups, friendlies and tours that
    /// `cups.json` holds.
    League = 0,
    /// Knockout: the cups and the play-offs.
    Cup = 1,
    /// Stages of groups, then knockout.
    Tournament = 2,
}

/// A code that is not a competition type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a competition type")]
pub struct InvalidCompetitionType(pub u8);

openswos_common::byte_enum!(
    CompetitionType,
    InvalidCompetitionType,
    [League, Cup, Tournament]
);

/// The base of every competition record, `comp` (upstream `Competition`, 9 bytes). Each
/// byte is as the C initialiser spells it (`-128` or `128`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompetitionHeader {
    /// The competition's identity (upstream `compId`, the `COMPID_*` values).
    pub comp_id: JsonByte,
    /// How it is played.
    #[serde(rename = "type", with = "code")]
    pub kind: CompetitionType,
    /// The team file its teams come from (upstream `teamNumber`); `0xFF` a `.TMD` pool or
    /// another special file.
    pub team_number: JsonByte,
    /// When its season starts: a calendar position in eighths of a month.
    pub begin_month: JsonByte,
    /// When its season ends, likewise.
    pub end_month: JsonByte,
    /// Upstream's old self-relative offset of the name table: no longer read.
    pub name_table_offset: JsonByte,
    /// Byte 6: 1 on the play-off records, which share the cups' type (upstream
    /// `isPlayoff`).
    pub reserved: JsonByte,
    /// Upstream's old self-relative offset of the team list: no longer read.
    pub team_list_offset: JsonByte,
    /// 0 to shuffle the draw, otherwise the seed order as given (upstream
    /// `seedReorderFlag`).
    pub seed_reorder_flag: JsonByte,
}

/// A 2-bit field of a round's rule byte: `0..=3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct RuleBits(u8);

/// A value too big for 2 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} does not fit 2 bits (0..=3)")]
pub struct InvalidRuleBits(pub u8);

impl RuleBits {
    /// The value, `0..=3`.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for RuleBits {
    type Error = InvalidRuleBits;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= RULE_BITS_MAX {
            Ok(Self(value))
        } else {
            Err(InvalidRuleBits(value))
        }
    }
}

impl From<RuleBits> for u8 {
    fn from(bits: RuleBits) -> Self {
        bits.0
    }
}

/// A round's rule byte, split as upstream splits it (save_format_json.c `dj_round_rules`):
/// bits 7-6, 5-4, 3-2, then 1-0, written only when set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoundRules {
    /// Bits 7-6: the legs.
    pub legs: RuleBits,
    /// Bits 5-4: extra time.
    pub extra_time: RuleBits,
    /// Bits 3-2: penalties.
    pub penalties: RuleBits,
    /// Bits 1-0, written only when not 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<RuleBits>,
}

impl From<u8> for RoundRules {
    /// The rules a round rule byte packs (upstream `roundRuleFlags`), split as the JSON
    /// splits them: the low bits only when not 0.
    fn from(byte: u8) -> Self {
        let bits = |shift: u8| RuleBits((byte >> shift) & RULE_BITS_MAX);
        let low = bits(0);
        Self {
            legs: bits(LEGS_SHIFT),
            extra_time: bits(EXTRA_TIME_SHIFT),
            penalties: bits(PENALTIES_SHIFT),
            low: (low.0 != 0).then_some(low),
        }
    }
}

impl From<RoundRules> for u8 {
    /// The round rule byte that packs `rules`.
    fn from(rules: RoundRules) -> Self {
        (rules.legs.0 << LEGS_SHIFT)
            | (rules.extra_time.0 << EXTRA_TIME_SHIFT)
            | (rules.penalties.0 << PENALTIES_SHIFT)
            | rules.low.map_or(0, |low| low.0)
    }
}

/// The country of a seeded team: its `CN_*` name, or its number (upstream's loader takes
/// either).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SeedCountry {
    /// The `CN_*` name (`"CN_SCOTLAND"`).
    Define(String),
    /// The number: its team file.
    Number(TeamFileNumber),
}

/// A team a competition's draw starts from (upstream `teamSeedList`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamSeed {
    /// Its country.
    pub country: SeedCountry,
    /// Its place in its country's team file.
    pub ordinal: TeamOrdinal,
    /// Its name, for whoever reads the file: the loader does not read it.
    pub team: Option<String>,
}

/// A record's name slots: string symbols upstream (`_aDivision1`, translated on load), the
/// names themselves in World 2026-27; `null` for an empty slot.
pub type Names = Vec<Option<String>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_rule_byte_splits_and_packs_back() {
        // 0x94: two legs, extra time, penalties (upstream's two-legged tie).
        let rules = RoundRules::from(0x94);
        assert_eq!(
            (
                rules.legs.0,
                rules.extra_time.0,
                rules.penalties.0,
                rules.low
            ),
            (2, 1, 1, None)
        );
        for byte in 0..=u8::MAX {
            assert_eq!(u8::from(RoundRules::from(byte)), byte);
        }
        assert_eq!(RoundRules::from(0x03).low, Some(RuleBits(3)));
    }

    #[test]
    fn competition_types_follow_upstream() {
        for (code, kind) in (0u8..).zip(CompetitionType::ALL.iter().copied()) {
            assert_eq!(u8::from(kind), code);
            assert_eq!(CompetitionType::try_from(code), Ok(kind));
        }
        assert_eq!(CompetitionType::try_from(3), Err(InvalidCompetitionType(3)));
        assert_eq!(
            InvalidCompetitionType(3).to_string(),
            "3 is not a competition type"
        );
    }

    #[test]
    fn the_header_keeps_its_bytes_as_written() {
        let text = r#"{"comp_id":-128,"type":1,"team_number":-1,"begin_month":120,"end_month":-120,"name_table_offset":15,"reserved":0,"team_list_offset":21,"seed_reorder_flag":1}"#;
        let header: CompetitionHeader = serde_json::from_str(text).expect("a header");
        assert_eq!(header.kind, CompetitionType::Cup);
        assert_eq!(header.comp_id.byte(), 128);
        assert_eq!(header.team_number.byte(), 0xFF);
        assert_eq!(serde_json::to_string(&header).ok(), Some(text.to_owned()));
    }

    #[test]
    fn round_rules_write_low_only_when_set() {
        for text in [
            r#"{"legs":2,"extra_time":1,"penalties":1}"#,
            r#"{"legs":0,"extra_time":0,"penalties":0,"low":2}"#,
        ] {
            let rules: RoundRules = serde_json::from_str(text).expect("rules");
            assert_eq!(serde_json::to_string(&rules).ok(), Some(text.to_owned()));
        }
        assert!(
            serde_json::from_str::<RoundRules>(r#"{"legs":4,"extra_time":0,"penalties":0}"#)
                .is_err()
        );
        assert_eq!(RuleBits::try_from(3).map(RuleBits::value), Ok(3));
        assert_eq!(RuleBits::try_from(4), Err(InvalidRuleBits(4)));
        assert_eq!(
            InvalidRuleBits(4).to_string(),
            "4 does not fit 2 bits (0..=3)"
        );
    }

    #[test]
    fn a_seed_names_its_country_or_numbers_it() {
        for text in [
            r#"{"country":"CN_SCOTLAND","ordinal":33,"team":"RANGERS"}"#,
            r#"{"country":67,"ordinal":33,"team":null}"#,
        ] {
            let seed: TeamSeed = serde_json::from_str(text).expect("a seed");
            assert_eq!(seed.ordinal, TeamOrdinal(33));
            assert_eq!(serde_json::to_string(&seed).ok(), Some(text.to_owned()));
        }
        let seed: TeamSeed =
            serde_json::from_str(r#"{"country":67,"ordinal":0,"team":null}"#).expect("a seed");
        assert_eq!(seed.country, SeedCountry::Number(TeamFileNumber(67)));
    }

    #[test]
    fn ids_are_their_text() {
        let id: SymbolId = serde_json::from_str(r#""_euroCup""#).expect("an id");
        assert_eq!(id.as_str(), "_euroCup");
    }
}
