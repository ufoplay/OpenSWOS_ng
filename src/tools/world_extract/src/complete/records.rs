//! What the compiled records the completion writes share (the oracle's `TournamentRecord` and
//! `CupRecord`): their base, their bytes, their names and their seeds, as upstream's JSON
//! writes them.

use std::collections::BTreeMap;

use openswos_common::{Nibble, TeamFileNumber, TeamOrdinal, json::JsonByte};
use openswos_competitions_json_resource::{
    CompetitionHeader, CompetitionType, Names, RoundRules, SeedCountry, TeamSeed,
};
use openswos_oracle::tables::CompetitionBase;
use openswos_team_json_resource::TeamJsonDocument;

use crate::error::ExtractError;

/// What starts an id (upstream's C symbols: `_euroCup`).
pub(super) const ID_PREFIX: char = '_';

/// How far a byte's high nibble is shifted.
const NIBBLE_SHIFT: u8 = 4;

/// A nibble's bits.
const NIBBLE_MASK: u8 = 0x0F;

/// A record's base as the JSON's `comp` header writes it.
///
/// # Errors
///
/// [`ExtractError::NoCompetitionType`] when its type is none.
pub(super) fn header(comp: &CompetitionBase) -> Result<CompetitionHeader, ExtractError> {
    Ok(CompetitionHeader {
        comp_id: JsonByte::from(comp.comp_id),
        kind: CompetitionType::try_from(comp.kind.cast_unsigned())
            .map_err(|_| ExtractError::NoCompetitionType(comp.kind))?,
        team_number: signed(comp.team_number),
        begin_month: signed(comp.begin_month),
        end_month: signed(comp.end_month),
        name_table_offset: signed(comp.reserved1),
        reserved: signed(comp.is_playoff),
        team_list_offset: signed(comp.reserved2),
        seed_reorder_flag: signed(comp.seed_reorder_flag),
    })
}

/// A signed byte as the JSON writes it.
pub(super) fn signed(byte: i8) -> JsonByte {
    JsonByte::try_from(i16::from(byte)).expect("a signed byte is a byte")
}

/// Upstream's `subsAndPointsForWin` split as the JSON splits it: the substitutes (its high
/// nibble), then a win's points (its low one).
pub(super) fn subs_and_points(byte: i8) -> (Nibble, Nibble) {
    let byte = byte.cast_unsigned();
    (nibble(byte >> NIBBLE_SHIFT), nibble(byte & NIBBLE_MASK))
}

/// Each round's rule byte, split.
pub(super) fn round_rules(flags: &[i8]) -> Vec<RoundRules> {
    flags
        .iter()
        .map(|&flags| RoundRules::from(flags.cast_unsigned()))
        .collect()
}

/// A record's names as their text, `None` for a null pointer.
///
/// # Errors
///
/// [`ExtractError::NotAscii`] for a name that is not ASCII, as upstream's are.
pub(super) fn names(names: &[Option<Vec<u8>>]) -> Result<Names, ExtractError> {
    names
        .iter()
        .map(|name| name.as_deref().map(text).transpose())
        .collect()
}

/// A seed list's clubs, by country number, each named after its club in `teams` when it has
/// it.
pub(super) fn seeds(
    list: &[(u8, u8)],
    teams: &BTreeMap<String, TeamJsonDocument>,
) -> Vec<TeamSeed> {
    list.iter()
        .map(|&(country, ordinal)| TeamSeed {
            country: SeedCountry::Number(TeamFileNumber(country)),
            ordinal: TeamOrdinal(ordinal),
            team: club_name(teams, country, ordinal),
        })
        .collect()
}

/// The nibble `value` (under 16).
fn nibble(value: u8) -> Nibble {
    Nibble::try_from(value).expect("masked to a nibble")
}

/// A name's text.
fn text(bytes: &[u8]) -> Result<String, ExtractError> {
    if bytes.is_ascii() {
        Ok(bytes.iter().copied().map(char::from).collect())
    } else {
        Err(ExtractError::NotAscii(bytes.to_vec()))
    }
}

/// The name of club `ordinal` of country `country` in `teams`, if it has it.
fn club_name(
    teams: &BTreeMap<String, TeamJsonDocument>,
    country: u8,
    ordinal: u8,
) -> Option<String> {
    teams
        .values()
        .filter(|document| document.file.country == Some(country))
        .flat_map(|document| &document.file.teams)
        .find(|entry| entry.team.team_ordinal == ordinal)
        .map(|entry| entry.team.team_name.clone())
}

#[cfg(test)]
pub(super) mod tests {
    use openswos_common::Importer;
    use openswos_team_json_resource::TeamJsonFormat;
    use openswos_test_utils::assets_path;

    use super::*;

    /// Oceania's roster (`assets/TEAM_JSON/OCEANIA.json`, country 85): AUSTRALIA, FIJI…
    pub(in crate::complete) fn oceania() -> BTreeMap<String, TeamJsonDocument> {
        let bytes = std::fs::read(assets_path().join("TEAM_JSON/OCEANIA.json")).expect("a roster");
        BTreeMap::from([(
            "OCEANIA.json".to_owned(),
            TeamJsonFormat::import(&bytes).expect("a roster"),
        )])
    }

    /// The base of World 2026-27's Champions League as world_data.c compiles it.
    pub(in crate::complete) fn base() -> CompetitionBase {
        CompetitionBase {
            comp_id: 0,
            kind: 2,
            team_number: -1,
            begin_month: 64,
            end_month: 32,
            reserved1: 35,
            is_playoff: 0,
            reserved2: 41,
            seed_reorder_flag: 1,
        }
    }

    #[test]
    fn the_base_is_the_header_byte_for_byte() {
        let comp = header(&base()).expect("a header");
        assert_eq!(comp.kind, CompetitionType::Tournament);
        assert_eq!(comp.comp_id.byte(), 0);
        assert_eq!(i16::from(comp.team_number), -1);
        assert_eq!(
            (
                comp.name_table_offset.byte(),
                comp.team_list_offset.byte(),
                comp.seed_reorder_flag.byte()
            ),
            (35, 41, 1)
        );
        let none = CompetitionBase { kind: 3, ..base() };
        assert_eq!(
            header(&none).expect_err("no type").to_string(),
            "a compiled record's type 3 is no competition type"
        );
    }

    #[test]
    fn bytes_split_as_the_json_splits_them() {
        assert_eq!(i16::from(signed(-108)), -108);
        let (substitutes, points) = subs_and_points(0x35);
        assert_eq!((u8::from(substitutes), u8::from(points)), (3, 5));
        let (substitutes, points) = subs_and_points(i8::MIN);
        assert_eq!((u8::from(substitutes), u8::from(points)), (8, 0));
        // -108 is 0x94: two legs, extra time, penalties.
        let rules = round_rules(&[84, -108, 0]);
        assert_eq!(
            rules.iter().map(|&rule| u8::from(rule)).collect::<Vec<_>>(),
            [84, 0x94, 0]
        );
    }

    #[test]
    fn names_are_their_text() {
        assert_eq!(
            names(&[Some(b"COPPA ITALIA".to_vec()), None]).expect("names"),
            [Some("COPPA ITALIA".to_owned()), None]
        );
        let error = names(&[Some(vec![0xC9])]).expect_err("not ASCII");
        assert!(matches!(error, ExtractError::NotAscii(bytes) if bytes == [0xC9]));
    }

    #[test]
    fn a_seed_is_named_after_its_club() {
        let list = seeds(&[(85, 0), (85, 200), (8, 0)], &oceania());
        let named: Vec<_> = list.iter().map(|seed| seed.team.as_deref()).collect();
        assert_eq!(named, [Some("AUSTRALIA"), None, None]);
        assert_eq!(list[0].country, SeedCountry::Number(TeamFileNumber(85)));
        assert_eq!(list[1].ordinal, TeamOrdinal(200));
    }
}
