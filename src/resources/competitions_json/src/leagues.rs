//! `leagues.json`: the leagues (upstream `CompetitionLeague`).

use openswos_common::json::{JsonByte, code};
use openswos_common::{Nibble, TeamFileNumber};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::shared::{CompetitionHeader, Names, SymbolId};

/// A league.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct League {
    /// Its id.
    pub id: SymbolId,
    /// Its header.
    pub comp: CompetitionHeader,
    /// How many divisions it has (World 2026-27's Italy: 14).
    pub divisions_count: JsonByte,
    /// How many times each team meets each other.
    pub rounds: JsonByte,
    /// A win's points.
    pub points_for_win: JsonByte,
    /// The substitutes' bench: the high nibble of upstream's `subsAndMaxSubs`.
    pub substitutes: Nibble,
    /// The substitutions allowed: its low nibble.
    pub max_subs: Nibble,
    /// The divisions, top first.
    pub divisions: Vec<Division>,
    /// Upstream's trailing byte of the old per-variant records.
    pub reserved1: JsonByte,
    /// The divisions' names, two slots each.
    pub name: Names,
    /// The play-offs or league blocks it links to (the leagues with promotion play-offs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playoff_links: Option<Vec<Option<SymbolId>>>,
    /// The bytes of its promotion and qualification links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promotion_link_data: Option<Vec<JsonByte>>,
    /// A country new to the game, which the league brings (World 2026-27).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<NewCountry>,
}

/// A division of a league (upstream `Division`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Division {
    /// Its teams.
    pub teams: JsonByte,
    /// The teams promoted directly.
    pub promoted: JsonByte,
    /// The teams that go to the promotion play-offs.
    pub promoted_po: JsonByte,
    /// The teams relegated directly.
    pub relegated: JsonByte,
    /// The teams that go to the relegation play-offs.
    pub relegated_po: JsonByte,
    /// Upstream's self-relative offset to the division's qualification link (only the
    /// English block uses it).
    pub qualifier_link_offset: JsonByte,
}

/// A country new to the game, which a World 2026-27 league brings
/// (upstream scripts/world_import.py).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCountry {
    /// Its number: its team file.
    pub country: TeamFileNumber,
    /// Its name.
    pub name: String,
    /// Its continent: the first byte of its name in the game's country table.
    #[serde(rename = "byte", with = "code")]
    pub continent: Continent,
}

/// A country's continent: the first byte of its name in the game's country table (upstream
/// `_countriesTable`; the game takes 0 as European, src/career_eurocups.c). The codes are
/// upstream scripts/world_import.py's `CONTINENTS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Continent {
    /// Europe.
    Europe = 0,
    /// North America.
    NorthAmerica = 1,
    /// South America.
    SouthAmerica = 2,
    /// Asia.
    Asia = 3,
    /// Oceania.
    Oceania = 4,
    /// Africa.
    Africa = 5,
}

/// A code that is not a continent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a continent")]
pub struct InvalidContinent(pub u8);

openswos_common::byte_enum!(
    Continent,
    InvalidContinent,
    [Europe, NorthAmerica, SouthAmerica, Asia, Oceania, Africa]
);

#[cfg(test)]
mod tests {
    use super::*;

    /// A league as upstream writes it, without its optional tail.
    const LEAGUE: &str = r#"{"id":"_customsLeague","comp":{"comp_id":36,"type":0,"team_number":72,"begin_month":64,"end_month":32,"name_table_offset":33,"reserved":0,"team_list_offset":0,"seed_reorder_flag":0},"divisions_count":1,"rounds":2,"points_for_win":3,"substitutes":3,"max_subs":5,"divisions":[{"teams":20,"promoted":0,"promoted_po":0,"relegated":0,"relegated_po":0,"qualifier_link_offset":0}],"reserved1":0,"name":[null,"_aDivision1"]"#;

    fn league(tail: &str) -> String {
        format!("{LEAGUE}{tail}}}")
    }

    #[test]
    fn a_league_round_trips_with_or_without_its_tail() {
        for tail in [
            "",
            r#","playoff_links":["_div1Playoffs","_englishLeagues1"],"promotion_link_data":[-1,0,1]"#,
            r#","new":{"country":47,"name":"ANDORRA","byte":0}"#,
        ] {
            let text = league(tail);
            let league: League = serde_json::from_str(&text).expect("a league");
            assert_eq!(serde_json::to_string(&league).ok(), Some(text));
        }
    }

    #[test]
    fn decodes_the_packed_and_coded_fields() {
        let league: League = serde_json::from_str(&league(
            r#","new":{"country":47,"name":"ANDORRA","byte":5}"#,
        ))
        .expect("a league");
        assert_eq!(league.substitutes.value(), 3);
        assert_eq!(league.max_subs.value(), 5);
        assert_eq!(league.name, vec![None, Some("_aDivision1".to_owned())]);
        let new = league.new.expect("a new country");
        assert_eq!(new.country, TeamFileNumber(47));
        assert_eq!(new.continent, Continent::Africa);
    }

    #[test]
    fn continents_follow_world_import() {
        for (code, continent) in (0u8..).zip(Continent::ALL.iter().copied()) {
            assert_eq!(u8::from(continent), code);
            assert_eq!(Continent::try_from(code), Ok(continent));
        }
        assert_eq!(Continent::try_from(6), Err(InvalidContinent(6)));
        assert_eq!(InvalidContinent(6).to_string(), "6 is not a continent");
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(serde_json::from_str::<League>(&league(r#","extra":1"#)).is_err());
        assert!(
            serde_json::from_str::<League>(&league(r#","new":{"country":47,"name":"X","byte":6}"#))
                .is_err()
        );
    }
}
