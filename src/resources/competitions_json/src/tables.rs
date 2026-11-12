//! The tables that tie competitions together: `qualifying.json` (upstream
//! `QualifyingGroup`), `league_blocks.json` (`Leagues`), `career.json`
//! (`CompetitionCareer`).

use openswos_common::json::{JsonByte, present};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::shared::SymbolId;

/// The `0xFC` marker as a qualifying group's list writes it.
const LEAGUE_MENU_MARKER: i16 = 252;

/// The league/cup separator as written.
const SEPARATOR_MARKER: i16 = -2;

/// The end marker as written.
const END_MARKER: i16 = -1;

/// A region's or a qualifying round's competitions (upstream `QualifyingGroup`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifyingGroup {
    /// Its id.
    pub id: SymbolId,
    /// Its competitions, and the markers between them (upstream `compCup`).
    pub comp_cup: Vec<CompCupEntry>,
    /// Its countries, `0xFF` ending the list as upstream keeps it.
    pub countries: Vec<JsonByte>,
}

/// An entry of a qualifying group's competition list: a competition, or one of the
/// in-band pointer values upstream's list holds between them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CompCupEntry {
    /// A competition.
    Competition(SymbolId),
    /// A marker.
    Marker(Marker),
}

/// The in-band pointer values of a qualifying group's competition list (upstream
/// scripts/export_competitions.py `comp_cup_token`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "i16", into = "i16")]
pub enum Marker {
    /// `NULL`: 0.
    Null,
    /// The league-menu marker: `0xFC`, written 252.
    LeagueMenu,
    /// The separator between leagues and cups: written -2.
    Separator,
    /// The end of the list: written -1.
    End,
}

/// A number that is no marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a competition-list marker (0, 252, -2, -1)")]
pub struct InvalidMarker(pub i16);

impl TryFrom<i16> for Marker {
    type Error = InvalidMarker;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Null),
            LEAGUE_MENU_MARKER => Ok(Self::LeagueMenu),
            SEPARATOR_MARKER => Ok(Self::Separator),
            END_MARKER => Ok(Self::End),
            other => Err(InvalidMarker(other)),
        }
    }
}

impl From<Marker> for i16 {
    fn from(marker: Marker) -> Self {
        match marker {
            Marker::Null => 0,
            Marker::LeagueMenu => LEAGUE_MENU_MARKER,
            Marker::Separator => SEPARATOR_MARKER,
            Marker::End => END_MARKER,
        }
    }
}

/// A block of leagues played together, with their play-offs (upstream `Leagues`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeagueBlock {
    /// Its id.
    pub id: SymbolId,
    /// Its leagues.
    pub leagues: Vec<BlockLeague>,
    /// Its play-offs: absent, `null`, or an id.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub playoffs: Option<Option<SymbolId>>,
    /// The block after it: absent, `null`, or an id.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub next_group: Option<Option<SymbolId>>,
    /// Upstream `qualifierIndexPairs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualifier_index_pairs: Option<Vec<JsonByte>>,
}

/// A league of a block.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockLeague {
    /// The league.
    pub league: Option<SymbolId>,
    /// Upstream `linkParams`.
    pub link_params: Vec<JsonByte>,
}

/// A competition of the career index (upstream `careerContests`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CareerContest {
    /// The competition.
    pub competition: Option<SymbolId>,
    /// The copy a career plays it in: made on load, not in the files.
    pub competition_copy: Option<SymbolId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_competition_list_mixes_ids_and_markers() {
        let text = r#"{"id":"_africaTable","comp_cup":[252,"_africanNationsCup",-2,0,-1],"countries":[3,255]}"#;
        let group: QualifyingGroup = serde_json::from_str(text).expect("a group");
        assert_eq!(
            group.comp_cup,
            vec![
                CompCupEntry::Marker(Marker::LeagueMenu),
                CompCupEntry::Competition(SymbolId("_africanNationsCup".to_owned())),
                CompCupEntry::Marker(Marker::Separator),
                CompCupEntry::Marker(Marker::Null),
                CompCupEntry::Marker(Marker::End),
            ]
        );
        assert_eq!(serde_json::to_string(&group).ok(), Some(text.to_owned()));
    }

    #[test]
    fn other_numbers_are_no_markers() {
        assert_eq!(Marker::try_from(5), Err(InvalidMarker(5)));
        assert_eq!(
            InvalidMarker(5).to_string(),
            "5 is not a competition-list marker (0, 252, -2, -1)"
        );
        assert!(serde_json::from_str::<CompCupEntry>("5").is_err());
    }

    #[test]
    fn a_block_keeps_absent_and_null_links_apart() {
        for text in [
            r#"{"id":"_dutchLeagues1","leagues":[{"league":"_dutchLeague","link_params":[0]}]}"#,
            r#"{"id":"_englishLeagues1","leagues":[],"playoffs":"_div2Playoffs","next_group":null,"qualifier_index_pairs":[-1,4]}"#,
        ] {
            let block: LeagueBlock = serde_json::from_str(text).expect("a block");
            assert_eq!(serde_json::to_string(&block).ok(), Some(text.to_owned()));
        }
    }

    #[test]
    fn a_contest_may_name_nothing() {
        let text = r#"{"competition":null,"competition_copy":"_euroCupCopy"}"#;
        let contest: CareerContest = serde_json::from_str(text).expect("a contest");
        assert_eq!(contest.competition, None);
        assert_eq!(serde_json::to_string(&contest).ok(), Some(text.to_owned()));
    }
}
