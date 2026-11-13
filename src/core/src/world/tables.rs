//! The tables that tie the competitions together, their references resolved: the qualifying
//! groups (`qualifying.json`), the league blocks (`league_blocks.json`), the career index
//! (`career.json`), each country's competitions (`country_competitions.json`) and the UEFA
//! Cup berths (`seed_buckets.json`).

use std::collections::BTreeMap;

use openswos_common::TeamFileNumber;
use openswos_common::json::JsonByte;
use openswos_competitions_json_resource::{
    CompCupEntry, CountryCompetitions, Marker, SeedBucket as SeedBucketRecord, SymbolId,
};

use super::competitions::{CompetitionRef, CupRef, LeagueRef, PlayoffRef, Resolver, each};
use super::files::{CompetitionFiles, Slot};
use crate::validation::{Expected, Field, Location, Problem};

/// The byte ending a qualifying group's countries (upstream's `0xFF`).
const END_OF_COUNTRIES: u8 = 0xFF;

/// A qualifying group of a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupRef(pub(crate) usize);

/// A league block of a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockRef(pub(crate) usize);

/// A region's or a qualifying round's competitions and countries (upstream
/// `QualifyingGroup`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifyingGroup {
    /// Its id.
    pub id: SymbolId,
    /// Its competitions, and the markers between them, as listed.
    pub entries: Vec<GroupEntry>,
    /// Its countries, up to the byte ending them (what the game reads).
    pub countries: Vec<TeamFileNumber>,
}

/// An entry of a qualifying group's competitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupEntry {
    /// A competition.
    Competition(CompetitionRef),
    /// A marker between them.
    Marker(Marker),
}

/// A block of leagues played together, with their play-offs (upstream `Leagues`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueBlock {
    /// Its id.
    pub id: SymbolId,
    /// Its leagues.
    pub leagues: Vec<BlockLeague>,
    /// Its play-offs.
    pub playoffs: Option<PlayoffRef>,
    /// The block after it.
    pub next: Option<BlockRef>,
    /// Upstream `qualifierIndexPairs`, as written.
    pub qualifier_index_pairs: Option<Vec<JsonByte>>,
}

/// A league of a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockLeague {
    /// The league.
    pub league: Option<LeagueRef>,
    /// Upstream `linkParams`, as written.
    pub link_params: Vec<JsonByte>,
}

/// A competition of the career index (upstream `careerContests`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareerContest {
    /// The competition; `None` ends upstream's list.
    pub competition: Option<CompetitionRef>,
    /// The name of the copy a career plays it in, which the career makes.
    pub copy: Option<SymbolId>,
}

/// The competitions of one slot of upstream's `_competitionsTable`: a country's, or a
/// region's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountryTable {
    /// Its slot: a country's number, or a region's.
    pub number: TeamFileNumber,
    /// Its name.
    pub name: String,
    /// What it holds.
    pub content: TableContent,
}

/// What a slot of the competition table holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableContent {
    /// A country's own leagues and cups.
    Own {
        /// Its table's name in upstream's C data.
        table: SymbolId,
        /// Its leagues.
        leagues: Vec<LeagueRef>,
        /// Its cups.
        cups: Vec<CupRef>,
    },
    /// A region's: a qualifying group's.
    Region(GroupRef),
}

/// A coefficient tier of the UEFA Cup berths (upstream `EuroCupQualSeedBucketBlock`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedBucket {
    /// Its upstream array's name (`bucket15`).
    pub tier: String,
    /// Its berths in all.
    pub size: u8,
    /// Its countries' berths, in draw order.
    pub allocation: Vec<Berths>,
}

/// A country's berths in a tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Berths {
    /// The country.
    pub country: TeamFileNumber,
    /// Its berths.
    pub berths: u8,
}

/// The qualifying groups of `qualifying.json`.
pub(crate) fn qualifying_groups(
    files: &CompetitionFiles<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Vec<QualifyingGroup>> {
    let path = Slot::Qualifying.path();
    each(&files.qualifying.groups, |index, record| {
        let field = Field::key("groups").record(index, &record.id);
        let entries_field = field.then("comp_cup");
        let entries = each(&record.comp_cup, |entry, value| match value {
            CompCupEntry::Competition(id) => resolver
                .competition(id, || Location::field(&path, &entries_field.index(entry)))
                .map(GroupEntry::Competition),
            CompCupEntry::Marker(marker) => Some(GroupEntry::Marker(*marker)),
        });
        let countries_field = field.then("countries");
        let listed: Vec<(usize, TeamFileNumber)> = record
            .countries
            .iter()
            .map(|byte| byte.byte())
            .take_while(|byte| *byte != END_OF_COUNTRIES)
            .map(TeamFileNumber)
            .enumerate()
            .collect();
        let countries = each(&listed, |_, (country, number)| {
            resolver.countries.check(
                *number,
                || Location::field(&path, &countries_field.index(*country)),
                resolver.problems,
            )
        });
        Some(QualifyingGroup {
            id: record.id.clone(),
            entries: entries?,
            countries: countries?,
        })
    })
}

/// The league blocks of `league_blocks.json`.
pub(crate) fn league_blocks(
    files: &CompetitionFiles<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Vec<LeagueBlock>> {
    let path = Slot::LeagueBlocks.path();
    each(&files.league_blocks.groups, |index, record| {
        let field = Field::key("groups").record(index, &record.id);
        let leagues_field = field.then("leagues");
        let leagues = each(&record.leagues, |league, entry| {
            let resolved = match &entry.league {
                Some(id) => resolver
                    .league(id, || {
                        Location::field(&path, &leagues_field.index(league).then("league"))
                    })
                    .map(Some),
                None => Some(None),
            };
            Some(BlockLeague {
                league: resolved?,
                link_params: entry.link_params.clone(),
            })
        });
        // Absent and `null` both mean none.
        let playoffs = match record.playoffs.as_ref().and_then(Option::as_ref) {
            Some(id) => resolver
                .playoff(id, || Location::field(&path, &field.then("playoffs")))
                .map(Some),
            None => Some(None),
        };
        let next = match record.next_group.as_ref().and_then(Option::as_ref) {
            Some(id) => resolver
                .block(id, || Location::field(&path, &field.then("next_group")))
                .map(Some),
            None => Some(None),
        };
        Some(LeagueBlock {
            id: record.id.clone(),
            leagues: leagues?,
            playoffs: playoffs?,
            next: next?,
            qualifier_index_pairs: record.qualifier_index_pairs.clone(),
        })
    })
}

/// The career index of `career.json`.
pub(crate) fn career(
    files: &CompetitionFiles<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Vec<CareerContest>> {
    let path = Slot::Career.path();
    each(&files.career.contests, |index, record| {
        let competition = match &record.competition {
            Some(id) => resolver
                .competition(id, || {
                    Location::field(
                        &path,
                        &Field::key("contests").index(index).then("competition"),
                    )
                })
                .map(Some),
            None => Some(None),
        };
        Some(CareerContest {
            competition: competition?,
            copy: record.competition_copy.clone(),
        })
    })
}

/// The competition tables of `country_competitions.json`. A slot with leagues or cups is a
/// country's own, and its number must be a country's; one with neither is a region's, its
/// table a qualifying group (upstream `DataJson_InstallCompetitionGraph`).
pub(crate) fn country_tables(
    files: &CompetitionFiles<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Vec<CountryTable>> {
    let path = Slot::CountryCompetitions.path();
    let mut first_at: BTreeMap<TeamFileNumber, usize> = BTreeMap::new();
    each(&files.country_competitions.countries, |index, record| {
        let field = Field::key("countries").index(index);
        let CountryCompetitions {
            number,
            name,
            table,
            leagues,
            cups,
        } = record;
        let unique = match first_at.get(number) {
            Some(first) => {
                resolver.problems.push(
                    Location::field(&path, &field.then("number")),
                    Problem::DuplicateTable {
                        number: *number,
                        first: Location::field(&path, &Field::key("countries").index(*first)),
                    },
                );
                None
            }
            None => {
                first_at.insert(*number, index);
                Some(())
            }
        };
        let content = if leagues.is_none() && cups.is_none() {
            resolver
                .ids
                .resolve(
                    table,
                    Expected::QUALIFYING_GROUP,
                    || Location::field(&path, &field.then("table")),
                    resolver.problems,
                )
                .map(|target| TableContent::Region(GroupRef(target.index)))
        } else {
            let country = resolver.countries.check(
                *number,
                || Location::field(&path, &field.then("number")),
                resolver.problems,
            );
            let leagues_field = field.then("leagues");
            let leagues = each(leagues.as_deref().unwrap_or_default(), |league, id| {
                resolver.league(id, || Location::field(&path, &leagues_field.index(league)))
            });
            let cups_field = field.then("cups");
            let cups = each(cups.as_deref().unwrap_or_default(), |cup, id| {
                resolver.cup(id, || Location::field(&path, &cups_field.index(cup)))
            });
            country?;
            Some(TableContent::Own {
                table: table.clone(),
                leagues: leagues?,
                cups: cups?,
            })
        };
        unique?;
        Some(CountryTable {
            number: *number,
            name: name.clone(),
            content: content?,
        })
    })
}

/// The UEFA Cup berths of `seed_buckets.json`.
pub(crate) fn seed_buckets(
    files: &CompetitionFiles<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Vec<SeedBucket>> {
    let path = Slot::SeedBuckets.path();
    each(
        &files.seed_buckets.euro_cup_qual_seed_buckets,
        |index, record| {
            let SeedBucketRecord {
                tier,
                size,
                allocation,
            } = record;
            let allocation_field = Field::key("euro_cup_qual_seed_buckets")
                .index(index)
                .then("allocation");
            let allocation = each(allocation, |entry, berths| {
                resolver
                    .countries
                    .check(
                        berths.number,
                        || Location::field(&path, &allocation_field.index(entry).then("number")),
                        resolver.problems,
                    )
                    .map(|country| Berths {
                        country,
                        berths: berths.berths,
                    })
            });
            Some(SeedBucket {
                tier: tier.clone(),
                size: *size,
                allocation: allocation?,
            })
        },
    )
}
