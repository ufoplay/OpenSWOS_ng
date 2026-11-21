//! A world's national and league cups (upstream src/world_cups.c `WorldCups_Install`), laid
//! as upstream lays them at start-up. In the world's order, each cup goes into its country's
//! table in its slot, and its record joins `cups.json`, replacing 96/97's of its id.
//!
//! The records are the oracle's, field for field. Their ids are spelled only by the C:
//! upstream's generated table (`kWorld2627Cups` in src/world_data.c) names each row's record
//! (`&kWorld2627_italianCup`, the record `_italianCup`). Each row's country and slot are
//! checked against the oracle's. Their draws' other fields (the real bracket, the quotas, the
//! European place) join `rules/national_cups.json` with the M5 unit that reads them.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use openswos_competitions_json_resource::{
    CompetitionsDocument, CompetitionsFile, Cup, CupKind, CupsFile, SymbolId,
};
use openswos_oracle::tables::{CupRecord, WorldCupEntry};
use openswos_team_json_resource::TeamJsonDocument;

use super::competitions::tables_mut;
use super::records::{ID_PREFIX, header, names, round_rules, seeds, signed, subs_and_points};
use crate::error::ExtractError;
use crate::fetch::WORLD_DATA_SOURCE;

/// The cups' file.
const CUPS: &str = "cups.json";

/// What starts upstream's generated table of the world's cups, in [`WORLD_DATA_SOURCE`].
const CUPS_TABLE: &str = "kWorld2627Cups[] = {";

/// What ends a C array's initialiser.
const TABLE_END: &str = "};";

/// What starts a row of the table.
const ROW_START: char = '{';

/// What separates a row's fields.
const FIELD_SEPARATOR: char = ',';

/// What precedes a row's record: its address, the world's prefix, then the record's id less
/// its leading underscore (`&kWorld2627_italianCup`).
const RECORD_PREFIX: &str = "&kWorld2627";

/// The countries upstream gives a table of their own (src/world_cups.c
/// `WORLD_CUP_COUNTRIES`): a cup past them is dropped.
const TABLE_COUNTRIES: usize = 96;

/// The entries of such a table (src/world_cups.c `s_tables[][12]`): its leagues, the
/// separator, its cups and the end.
const TABLE_ENTRIES: usize = 12;

/// The entries of a table that are no competition: the separator before its cups, and its
/// end.
const TABLE_MARKERS: usize = 2;

/// A row of upstream's table: the cup's country, its slot and its record's id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CupRow {
    /// Its country's number.
    pub country: u8,
    /// Its slot (`WorldCup.slot`).
    pub slot: u8,
    /// Its record's id.
    pub id: SymbolId,
}

/// Where a cup goes in its country's table (include/world_cups.h `WorldCup.slot`): after
/// its leagues and the separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CupSlot {
    /// Slot 1: the national cup.
    National = 1,
    /// Slot 2: the league cup.
    League = 2,
}

/// A slot that is none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InvalidCupSlot(u8);

openswos_common::byte_enum!(CupSlot, InvalidCupSlot, [National, League]);

impl CupSlot {
    /// Its place among a table's cups.
    fn index(self) -> usize {
        match self {
            Self::National => 0,
            Self::League => 1,
        }
    }
}

/// The rows of upstream's table of the world's cups, read from the checkout in `checkout`.
///
/// # Errors
///
/// When its [`WORLD_DATA_SOURCE`] does not read, has no table, or a row does not read.
pub fn cup_rows(checkout: &Path) -> Result<Vec<CupRow>, ExtractError> {
    let path = checkout.join(WORLD_DATA_SOURCE);
    let text = std::fs::read_to_string(&path).map_err(|source| ExtractError::Read {
        path: path.clone(),
        source,
    })?;
    rows(&text).ok_or(ExtractError::NoCupTable(path))?
}

/// The rows of the table in `text`, none without the table.
fn rows(text: &str) -> Option<Result<Vec<CupRow>, ExtractError>> {
    let (_, table) = text.split_once(CUPS_TABLE)?;
    let (table, _) = table.split_once(TABLE_END)?;
    Some(
        table
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with(ROW_START))
            .map(row)
            .collect(),
    )
}

/// A row of the table (`{ 20, 1, &kWorld2627_italianCup, 16, …`).
fn row(line: &str) -> Result<CupRow, ExtractError> {
    let wrong = || ExtractError::CupRow(line.to_owned());
    let mut fields = line
        .trim_start_matches(ROW_START)
        .split(FIELD_SEPARATOR)
        .map(str::trim);
    let mut number = || fields.next().and_then(|field| field.parse::<u8>().ok());
    let (country, slot) = (number().ok_or_else(wrong)?, number().ok_or_else(wrong)?);
    let id = fields
        .next()
        .and_then(|field| field.strip_prefix(RECORD_PREFIX))
        .filter(|id| id.starts_with(ID_PREFIX))
        .ok_or_else(wrong)?;
    Ok(CupRow {
        country,
        slot,
        id: SymbolId(id.to_owned()),
    })
}

/// Installs `cups` (the world's, in its order, each with its row of `rows`) into
/// `competitions`, their seeds named after their clubs in `teams`, as `WorldCups_Install`
/// does: a cup takes its slot in its country's table, the table growing by one when the slot
/// is the next, and a league cup whose country has no national cup is dropped.
pub(super) fn install(
    competitions: &mut BTreeMap<String, CompetitionsDocument>,
    cups: &[WorldCupEntry],
    rows: &[CupRow],
    teams: &BTreeMap<String, TeamJsonDocument>,
) -> Result<(), ExtractError> {
    if rows.len() != cups.len() {
        return Err(ExtractError::CupsOutOfStep(rows.len().min(cups.len())));
    }
    let tables = tables_mut(competitions)?;
    let mut countries = BTreeSet::new();
    let mut records = Vec::new();
    for (index, (cup, row)) in cups.iter().zip(rows).enumerate() {
        if (row.country, row.slot) != (cup.country, cup.slot) {
            return Err(ExtractError::CupsOutOfStep(index));
        }
        let slot = CupSlot::try_from(cup.slot)
            .map_err(|InvalidCupSlot(slot)| ExtractError::CupSlot(slot))?;
        countries.insert(cup.country);
        let table = tables
            .countries
            .iter_mut()
            .find(|table| table.number.0 == cup.country && table.leagues.is_some())
            .ok_or(ExtractError::NoCountryTable(cup.country))?;
        let leagues = table.leagues.as_ref().map_or(0, Vec::len);
        let list = table.cups.get_or_insert_with(Vec::new);
        match slot.index().cmp(&list.len()) {
            Ordering::Less => {
                if let Some(taken) = list.get_mut(slot.index()) {
                    *taken = row.id.clone();
                }
            }
            Ordering::Equal => list.push(row.id.clone()),
            Ordering::Greater => continue,
        }
        if countries.len() > TABLE_COUNTRIES || leagues + list.len() + TABLE_MARKERS > TABLE_ENTRIES
        {
            return Err(ExtractError::CupTablesFull(cup.country));
        }
        records.push(record(&cup.record, row.id.clone(), teams)?);
    }
    let file = cups_mut(competitions)?;
    for record in records {
        match file
            .competitions
            .iter_mut()
            .find(|found| found.id == record.id)
        {
            Some(found) => *found = record,
            None => file.competitions.push(record),
        }
    }
    file.count = file.competitions.len();
    Ok(())
}

/// `record` as a cup of `cups.json`, its id `id`, its seeds named after their clubs in
/// `teams`.
fn record(
    record: &CupRecord,
    id: SymbolId,
    teams: &BTreeMap<String, TeamJsonDocument>,
) -> Result<Cup, ExtractError> {
    let (substitutes, points_for_win) = subs_and_points(record.subs_and_points_for_win);
    Ok(Cup {
        id,
        kind: CupKind::Cup,
        comp: header(&record.comp)?,
        num_rounds_override: signed(record.num_rounds_override),
        teams: signed(record.teams),
        away_goals_mode: signed(record.away_goals_mode),
        substitutes,
        points_for_win,
        round_string_base: signed(record.round_string_base),
        round_rule_flags: round_rules(&record.round_rule_flags),
        name: names(&record.names)?,
        team_seed_list: seeds(&record.team_seed_list, teams),
    })
}

/// The cups' file of `competitions`.
fn cups_mut(
    competitions: &mut BTreeMap<String, CompetitionsDocument>,
) -> Result<&mut CupsFile, ExtractError> {
    let Some(CompetitionsFile::Cups(file)) = competitions
        .get_mut(CUPS)
        .map(|document| &mut document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(CUPS));
    };
    Ok(file)
}

#[cfg(test)]
mod tests {
    use openswos_common::TeamFileNumber;
    use openswos_competitions_json_resource::{CompetitionType, CountryCompetitions};
    use openswos_oracle::tables::CompetitionBase;
    use openswos_test_utils::world_1996_97_path;

    use super::*;
    use crate::assemble::committed;
    use crate::complete::records::tests::oceania;

    /// Two rows of upstream's table as scripts/world_import.py generates it.
    const TABLE: &str = "static const WorldCup kWorld2627Cups[] = {\n    { 20, 1, \
        &kWorld2627_italianCup, 16, 8, 0, 4, kWorld2627_italianCupFlips, 2, \
        kWorld2627_italianCupQuota, kWorld2627_italianCupOrder1, 0, 1 },   /* _italianCup */\n    \
        { 40, 2, &kWorld2627_worldWalLeagueCup, 16, 8, 0, 4, \
        kWorld2627_worldWalLeagueCupFlips, 1, kWorld2627_worldWalLeagueCupQuota, \
        kWorld2627_worldWalLeagueCupOrder1, 0, 1 },   /* _worldWalLeagueCup */\n};\n#endif\n";

    /// The Coppa Italia as world_data.c compiles it (`kWorld2627_italianCup`), its list cut to
    /// two clubs, in `country`'s slot `slot`.
    fn cup(country: u8, slot: u8) -> WorldCupEntry {
        WorldCupEntry {
            country,
            slot,
            record: CupRecord {
                comp: CompetitionBase {
                    comp_id: 132,
                    kind: 1,
                    team_number: 20,
                    begin_month: 56,
                    end_month: 32,
                    reserved1: 15,
                    is_playoff: 0,
                    reserved2: 21,
                    seed_reorder_flag: 1,
                },
                num_rounds_override: 0,
                teams: 16,
                away_goals_mode: 0,
                subs_and_points_for_win: 53,
                round_string_base: 2,
                round_rule_flags: vec![84, 84, -108, 84, 0, 0],
                names: vec![Some(b"COPPA ITALIA".to_vec()), Some(b"COPPA".to_vec())],
                team_seed_list: vec![(85, 0), (85, 1)],
            },
        }
    }

    fn row(country: u8, slot: u8, id: &str) -> CupRow {
        CupRow {
            country,
            slot,
            id: SymbolId(id.to_owned()),
        }
    }

    fn base() -> BTreeMap<String, CompetitionsDocument> {
        committed(&world_1996_97_path())
            .expect("the committed package")
            .competitions
    }

    /// Country `number`'s cups in `competitions`.
    fn table_cups(competitions: &BTreeMap<String, CompetitionsDocument>, number: u8) -> Vec<&str> {
        let Some(CompetitionsFile::CountryCompetitions(tables)) = competitions
            .get("country_competitions.json")
            .map(|d| &d.file)
        else {
            panic!("the tables");
        };
        tables
            .countries
            .iter()
            .find(|table| table.number.0 == number)
            .and_then(|table| table.cups.as_ref())
            .map(|cups| cups.iter().map(SymbolId::as_str).collect())
            .unwrap_or_default()
    }

    /// The cup of id `id` in `competitions`.
    fn cup_record<'a>(
        competitions: &'a BTreeMap<String, CompetitionsDocument>,
        id: &str,
    ) -> Option<&'a Cup> {
        let Some(CompetitionsFile::Cups(cups)) = competitions.get(CUPS).map(|d| &d.file) else {
            panic!("the cups");
        };
        cups.competitions.iter().find(|cup| cup.id.as_str() == id)
    }

    #[test]
    fn the_rows_name_each_cup_s_record() {
        let read = rows(TABLE).expect("a table").expect("rows");
        assert_eq!(
            read,
            [row(20, 1, "_italianCup"), row(40, 2, "_worldWalLeagueCup")]
        );
        assert!(rows("no table").is_none());
        let error = rows(&TABLE.replace("&kWorld2627_italianCup", "kWorld2627_italianCup"))
            .expect("a table")
            .expect_err("no address");
        assert!(
            error.to_string().contains("{ 20, 1, kWorld2627_italianCup"),
            "{error}"
        );
        let error = rows(&TABLE.replace("{ 20, 1,", "{ 20, x,"))
            .expect("a table")
            .expect_err("no slot");
        assert!(matches!(error, ExtractError::CupRow(_)));
    }

    #[test]
    fn the_rows_are_read_from_the_checkout() {
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::create_dir(dir.path().join("src")).expect("src/");
        let source = dir.path().join(WORLD_DATA_SOURCE);
        std::fs::write(&source, TABLE).expect("the source");
        assert_eq!(cup_rows(dir.path()).expect("rows").len(), 2);
        std::fs::write(&source, "int x;").expect("the source");
        let error = cup_rows(dir.path()).expect_err("no table");
        assert!(matches!(error, ExtractError::NoCupTable(path) if path == source));
        let missing = cup_rows(&dir.path().join("none")).expect_err("no file");
        assert!(matches!(missing, ExtractError::Read { .. }));
    }

    #[test]
    fn a_slot_is_a_place_among_the_cups() {
        assert_eq!(CupSlot::try_from(1).map(CupSlot::index), Ok(0));
        assert_eq!(CupSlot::try_from(2).map(CupSlot::index), Ok(1));
        assert_eq!(CupSlot::try_from(3), Err(InvalidCupSlot(3)));
    }

    #[test]
    fn a_record_is_the_cup_upstream_compiles() {
        let cup = record(
            &cup(20, 1).record,
            SymbolId("_italianCup".to_owned()),
            &oceania(),
        )
        .expect("a cup");
        assert_eq!(cup.id.as_str(), "_italianCup");
        assert_eq!(cup.comp.kind, CompetitionType::Cup);
        assert_eq!(
            (cup.comp.comp_id.byte(), cup.comp.team_number.byte()),
            (132, 20)
        );
        assert_eq!((cup.teams.byte(), cup.round_string_base.byte()), (16, 2));
        assert_eq!(
            (u8::from(cup.substitutes), u8::from(cup.points_for_win)),
            (3, 5),
            "53 is 0x35"
        );
        assert_eq!(cup.round_rule_flags.len(), 6, "as compiled");
        assert_eq!(u8::from(cup.round_rule_flags[2]), 0x94);
        assert_eq!(
            cup.name,
            [Some("COPPA ITALIA".to_owned()), Some("COPPA".to_owned())]
        );
        let teams: Vec<_> = cup
            .team_seed_list
            .iter()
            .map(|seed| seed.team.as_deref())
            .collect();
        assert_eq!(teams, [Some("AUSTRALIA"), Some("FIJI")]);
    }

    #[test]
    fn a_cup_takes_its_slot_and_its_record_replaces_its_id() {
        let mut competitions = base();
        let cups = [cup(20, 1), cup(40, 2)];
        let rows = [row(20, 1, "_italianCup"), row(40, 2, "_worldWalLeagueCup")];
        let before = match competitions.get(CUPS).map(|d| &d.file) {
            Some(CompetitionsFile::Cups(file)) => file.count,
            _ => panic!("the cups"),
        };
        install(&mut competitions, &cups, &rows, &oceania()).expect("installed");
        assert_eq!(table_cups(&competitions, 20), ["_italianCup"]);
        assert_eq!(
            table_cups(&competitions, 40),
            ["_welshCup", "_worldWalLeagueCup"],
            "the next slot grows the table"
        );
        let italian = cup_record(&competitions, "_italianCup").expect("replaced");
        assert_eq!(italian.teams.byte(), 16);
        assert_eq!(italian.team_seed_list.len(), 2);
        assert!(cup_record(&competitions, "_worldWalLeagueCup").is_some());
        let Some(CompetitionsFile::Cups(file)) = competitions.get(CUPS).map(|d| &d.file) else {
            panic!("the cups");
        };
        assert_eq!(file.count, before + 1);
        assert_eq!(file.count, file.competitions.len());
    }

    #[test]
    fn a_league_cup_without_a_national_cup_is_dropped() {
        let mut competitions = base();
        install(
            &mut competitions,
            &[cup(48, 2)],
            &[row(48, 2, "_worldChiLeagueCup")],
            &oceania(),
        )
        .expect("installed");
        assert!(table_cups(&competitions, 48).is_empty());
        assert!(cup_record(&competitions, "_worldChiLeagueCup").is_none());
    }

    #[test]
    fn what_upstream_cannot_install_is_refused() {
        let install_one = |cup: WorldCupEntry, row: CupRow| {
            install(&mut base(), &[cup], &[row], &oceania()).expect_err("refused")
        };
        let error = install_one(cup(20, 1), row(20, 2, "_italianCup"));
        assert!(matches!(error, ExtractError::CupsOutOfStep(0)));
        let error = install(&mut base(), &[cup(20, 1)], &[], &oceania()).expect_err("no rows");
        assert!(matches!(error, ExtractError::CupsOutOfStep(0)));
        let error = install_one(cup(20, 3), row(20, 3, "_italianCup"));
        assert!(matches!(error, ExtractError::CupSlot(3)));
        let error = install_one(cup(80, 1), row(80, 1, "_europeCup"));
        assert!(
            matches!(error, ExtractError::NoCountryTable(80)),
            "a region's"
        );
    }

    #[test]
    fn a_cup_past_upstream_s_tables_is_refused() {
        let mut competitions = base();
        let Some(CompetitionsFile::CountryCompetitions(tables)) = competitions
            .get_mut("country_competitions.json")
            .map(|d| &mut d.file)
        else {
            panic!("the tables");
        };
        let leagues = |count: usize| Some(vec![SymbolId("_x".to_owned()); count]);
        tables.countries = (0..=u8::try_from(TABLE_COUNTRIES).expect("a byte"))
            .map(|number| CountryCompetitions {
                number: TeamFileNumber(number),
                name: String::new(),
                table: SymbolId(format!("_t{number}")),
                leagues: leagues(1),
                cups: None,
            })
            .collect();
        let every: Vec<_> = (0..=u8::try_from(TABLE_COUNTRIES).expect("a byte")).collect();
        let cups: Vec<_> = every.iter().map(|&number| cup(number, 1)).collect();
        let rows: Vec<_> = every
            .iter()
            .map(|&number| row(number, 1, &format!("_c{number}")))
            .collect();
        let error =
            install(&mut competitions.clone(), &cups, &rows, &oceania()).expect_err("97 countries");
        assert!(matches!(error, ExtractError::CupTablesFull(96)), "{error}");
        if let Some(CompetitionsFile::CountryCompetitions(tables)) = competitions
            .get_mut("country_competitions.json")
            .map(|d| &mut d.file)
            && let Some(first) = tables.countries.first_mut()
        {
            first.leagues = leagues(TABLE_ENTRIES - TABLE_MARKERS);
        }
        let error =
            install(&mut competitions, &cups[..1], &rows[..1], &oceania()).expect_err("13 entries");
        assert!(matches!(error, ExtractError::CupTablesFull(0)), "{error}");
    }
}
