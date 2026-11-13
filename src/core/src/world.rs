//! The world: what a world package says the football world is, typed and validated
//! (docs/PLAN.md §5.2). Today a package gives its identity, its countries and its
//! competitions with the tables that tie them together; the rules upstream hardcodes join
//! as the inventory (M4 #4) and the competition and career ports (M5, M7) give them files.
//!
//! [`World::from_package`] checks the package whole and reports every problem, each naming
//! its file and field: the ten competition files at their paths, one definition per id,
//! every reference naming a record of the right kind, every country a country of the world,
//! each record of a type its file holds. References become typed handles ([`LeagueRef`],
//! [`CompetitionRef`]…) into the world that made them. The records' numbers are kept as
//! read: their meaning is the competition engine's (M5).

mod competitions;
mod countries;
mod files;
mod ids;
mod tables;

use openswos_competitions_json_resource::SymbolId;
use openswos_world_package_resource::{WorldPackage, WorldTag};

pub use competitions::{
    CompetitionRef, Cup, CupRef, League, LeagueLink, LeagueRef, Playoff, PlayoffRef, Tournament,
    TournamentRef,
};
pub use countries::{Countries, Country};
pub use tables::{
    Berths, BlockLeague, BlockRef, CareerContest, CountryTable, GroupEntry, GroupRef, LeagueBlock,
    QualifyingGroup, SeedBucket, TableContent,
};

use self::competitions::{Resolver, competition_ref};
pub(crate) use self::files::Slot;
use self::ids::Ids;
use crate::validation::{Invalid, Problems};

/// A world, valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct World {
    /// Its tag.
    tag: WorldTag,
    /// Its name.
    name: String,
    /// Its countries.
    countries: Countries,
    /// Its ids.
    ids: Ids,
    /// Its leagues, in `leagues.json`'s order.
    leagues: Vec<League>,
    /// Its cups, in `cups.json`'s order.
    cups: Vec<Cup>,
    /// Its tournaments, in `tournaments.json`'s order.
    tournaments: Vec<Tournament>,
    /// Its play-offs, in `playoffs.json`'s order.
    playoffs: Vec<Playoff>,
    /// Its qualifying groups.
    qualifying_groups: Vec<QualifyingGroup>,
    /// Its league blocks.
    league_blocks: Vec<LeagueBlock>,
    /// Its career index.
    career: Vec<CareerContest>,
    /// Its competition tables.
    country_tables: Vec<CountryTable>,
    /// Its UEFA Cup berths.
    seed_buckets: Vec<SeedBucket>,
}

impl World {
    /// The world `package` describes.
    ///
    /// # Errors
    ///
    /// [`Invalid`], with every problem the package has.
    pub fn from_package(package: &WorldPackage) -> Result<Self, Invalid> {
        let mut problems = Problems::default();
        let world = build(package, &mut problems);
        problems.finish(world)
    }

    /// Its tag (`9697`, `2627`).
    #[must_use]
    pub fn tag(&self) -> &WorldTag {
        &self.tag
    }

    /// Its name (`World 1996-97`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its countries.
    #[must_use]
    pub fn countries(&self) -> &Countries {
        &self.countries
    }

    /// The competition called `id`.
    #[must_use]
    pub fn competition(&self, id: &SymbolId) -> Option<CompetitionRef> {
        competition_ref(&self.ids, id)
    }

    /// The id of `competition`.
    #[must_use]
    pub fn competition_id(&self, competition: CompetitionRef) -> &SymbolId {
        match competition {
            CompetitionRef::League(league) => &self.league(league).record.id,
            CompetitionRef::Cup(cup) => &self.cup(cup).record.id,
            CompetitionRef::Tournament(tournament) => &self.tournament(tournament).record.id,
            CompetitionRef::Playoff(playoff) => &self.playoff(playoff).record.id,
        }
    }

    /// Its leagues.
    #[must_use]
    pub fn leagues(&self) -> &[League] {
        &self.leagues
    }

    /// The league `league`, which must be this world's.
    #[must_use]
    pub fn league(&self, league: LeagueRef) -> &League {
        &self.leagues[league.0]
    }

    /// Its cups.
    #[must_use]
    pub fn cups(&self) -> &[Cup] {
        &self.cups
    }

    /// The cup `cup`, which must be this world's.
    #[must_use]
    pub fn cup(&self, cup: CupRef) -> &Cup {
        &self.cups[cup.0]
    }

    /// Its tournaments.
    #[must_use]
    pub fn tournaments(&self) -> &[Tournament] {
        &self.tournaments
    }

    /// The tournament `tournament`, which must be this world's.
    #[must_use]
    pub fn tournament(&self, tournament: TournamentRef) -> &Tournament {
        &self.tournaments[tournament.0]
    }

    /// Its play-offs.
    #[must_use]
    pub fn playoffs(&self) -> &[Playoff] {
        &self.playoffs
    }

    /// The play-off `playoff`, which must be this world's.
    #[must_use]
    pub fn playoff(&self, playoff: PlayoffRef) -> &Playoff {
        &self.playoffs[playoff.0]
    }

    /// Its qualifying groups.
    #[must_use]
    pub fn qualifying_groups(&self) -> &[QualifyingGroup] {
        &self.qualifying_groups
    }

    /// The qualifying group `group`, which must be this world's.
    #[must_use]
    pub fn qualifying_group(&self, group: GroupRef) -> &QualifyingGroup {
        &self.qualifying_groups[group.0]
    }

    /// Its league blocks.
    #[must_use]
    pub fn league_blocks(&self) -> &[LeagueBlock] {
        &self.league_blocks
    }

    /// The league block `block`, which must be this world's.
    #[must_use]
    pub fn league_block(&self, block: BlockRef) -> &LeagueBlock {
        &self.league_blocks[block.0]
    }

    /// Its career index.
    #[must_use]
    pub fn career(&self) -> &[CareerContest] {
        &self.career
    }

    /// Its competition tables.
    #[must_use]
    pub fn country_tables(&self) -> &[CountryTable] {
        &self.country_tables
    }

    /// Its UEFA Cup berths.
    #[must_use]
    pub fn seed_buckets(&self) -> &[SeedBucket] {
        &self.seed_buckets
    }
}

/// The world, or `None` with the problems reported.
fn build(package: &WorldPackage, problems: &mut Problems) -> Option<World> {
    let files = files::pick(package, problems)?;
    let countries = Countries::build(files.countries, problems);
    let ids = Ids::build(&files, problems);
    let mut resolver = Resolver {
        ids: &ids,
        countries: &countries,
        problems,
    };
    let leagues = competitions::leagues(&files, &mut resolver);
    let cups = competitions::cups(&files, &mut resolver);
    let tournaments = competitions::tournaments(&files, &mut resolver);
    let playoffs = competitions::playoffs(&files, &mut resolver);
    let qualifying_groups = tables::qualifying_groups(&files, &mut resolver);
    let league_blocks = tables::league_blocks(&files, &mut resolver);
    let career = tables::career(&files, &mut resolver);
    let country_tables = tables::country_tables(&files, &mut resolver);
    let seed_buckets = tables::seed_buckets(&files, &mut resolver);
    Some(World {
        tag: package.identity.tag.clone(),
        name: package.identity.name.clone(),
        leagues: leagues?,
        cups: cups?,
        tournaments: tournaments?,
        playoffs: playoffs?,
        qualifying_groups: qualifying_groups?,
        league_blocks: league_blocks?,
        career: career?,
        country_tables: country_tables?,
        seed_buckets: seed_buckets?,
        countries,
        ids,
    })
}

#[cfg(test)]
mod tests {
    use openswos_common::{TeamFileNumber, TeamId, TeamOrdinal};
    use openswos_competitions_json_resource::{Marker, SymbolId};
    use openswos_world_package_resource::WorldPackage;

    use super::*;
    use crate::fixture::{ALBANIA, ENGLAND, package, package_with};

    fn id(text: &str) -> SymbolId {
        SymbolId(text.to_owned())
    }

    fn team(file: TeamFileNumber, ordinal: u8) -> TeamId {
        TeamId {
            file,
            ordinal: TeamOrdinal(ordinal),
        }
    }

    /// The problems `package` has, a line each.
    fn problems(package: &WorldPackage) -> Vec<String> {
        match World::from_package(package) {
            Ok(_) => Vec::new(),
            Err(invalid) => invalid.0.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn the_small_world_builds() {
        let world = World::from_package(&package()).expect("a valid world");
        assert_eq!(world.tag().0, "9697");
        assert_eq!(world.name(), "World 1996-97");
        assert_eq!(world.countries().iter().count(), 3);
        assert_eq!(world.countries().listed().count(), 4);
        assert_eq!(
            (
                world.leagues().len(),
                world.cups().len(),
                world.tournaments().len(),
                world.playoffs().len()
            ),
            (2, 3, 1, 1)
        );
        assert_eq!(world.seed_buckets().len(), 1);
    }

    #[test]
    fn competitions_are_found_by_id_and_name_it_back() {
        let world = World::from_package(&package()).expect("a valid world");
        for name in [
            "_albanianLeague",
            "_englishLeague",
            "_albanianCup",
            "_cupWinnersCup",
            "_friendly",
            "_euroCup",
            "_div1Playoffs",
        ] {
            let competition = world.competition(&id(name)).expect(name);
            assert_eq!(world.competition_id(competition), &id(name));
        }
        assert_eq!(
            world.competition(&id("_euroCup")),
            Some(CompetitionRef::Tournament(TournamentRef(0)))
        );
        assert_eq!(world.competition(&id("_europeTable")), None, "a group");
        assert_eq!(world.competition(&id("_nowhere")), None);
    }

    #[test]
    fn a_leagues_links_name_its_play_offs_and_its_block() {
        let world = World::from_package(&package()).expect("a valid world");
        let english = world.league(LeagueRef(1));
        assert_eq!(english.record.id, id("_englishLeague"));
        assert_eq!(
            english.playoff_links,
            Some(vec![
                Some(LeagueLink::Playoff(PlayoffRef(0))),
                Some(LeagueLink::Block(BlockRef(0))),
                None
            ])
        );
        assert_eq!(world.league(LeagueRef(0)).playoff_links, None);
        assert_eq!(world.playoff(PlayoffRef(0)).record.id, id("_div1Playoffs"));
    }

    #[test]
    fn seeds_are_teams_of_their_countries() {
        let world = World::from_package(&package()).expect("a valid world");
        assert_eq!(world.cup(CupRef(0)).seeds, []);
        assert_eq!(
            world.cup(CupRef(1)).seeds,
            [team(ENGLAND, 0), team(ALBANIA, 1)],
            "by define and by number"
        );
        assert_eq!(world.tournament(TournamentRef(0)).seeds, [team(ALBANIA, 0)]);
    }

    #[test]
    fn a_qualifying_group_lists_competitions_markers_and_countries() {
        let world = World::from_package(&package()).expect("a valid world");
        let europe = world.qualifying_group(GroupRef(0));
        assert_eq!(europe.id, id("_europeTable"));
        assert_eq!(
            europe.entries,
            [
                GroupEntry::Marker(Marker::LeagueMenu),
                GroupEntry::Competition(CompetitionRef::Tournament(TournamentRef(0))),
                GroupEntry::Competition(CompetitionRef::Cup(CupRef(1))),
                GroupEntry::Marker(Marker::End),
            ]
        );
        assert_eq!(
            europe.countries,
            [ALBANIA, ENGLAND],
            "up to the end byte; what follows it is not read"
        );
        assert_eq!(world.qualifying_group(GroupRef(1)).countries, []);
    }

    #[test]
    fn a_league_block_names_its_leagues_play_offs_and_next_block() {
        let world = World::from_package(&package()).expect("a valid world");
        let first = world.league_block(BlockRef(0));
        assert_eq!(
            first
                .leagues
                .iter()
                .map(|league| league.league)
                .collect::<Vec<_>>(),
            [Some(LeagueRef(1)), None]
        );
        assert_eq!(first.playoffs, Some(PlayoffRef(0)));
        assert_eq!(first.next, Some(BlockRef(1)));
        let second = world.league_block(BlockRef(1));
        assert_eq!(
            (second.playoffs, second.next),
            (None, None),
            "absent and null"
        );
    }

    #[test]
    fn the_career_index_names_competitions_and_their_copies() {
        let world = World::from_package(&package()).expect("a valid world");
        assert_eq!(
            world.career(),
            [
                CareerContest {
                    competition: Some(CompetitionRef::Tournament(TournamentRef(0))),
                    copy: Some(id("_euroCupCopy")),
                },
                CareerContest {
                    competition: Some(CompetitionRef::Cup(CupRef(2))),
                    copy: Some(id("_friendlyCopy")),
                },
                CareerContest {
                    competition: None,
                    copy: None,
                },
            ]
        );
    }

    #[test]
    fn the_competition_tables_are_countries_and_regions() {
        let world = World::from_package(&package()).expect("a valid world");
        let contents: Vec<(u8, &TableContent)> = world
            .country_tables()
            .iter()
            .map(|table| (table.number.0, &table.content))
            .collect();
        assert_eq!(
            contents,
            [
                (
                    0,
                    &TableContent::Own {
                        table: id("_albaniaTable"),
                        leagues: vec![LeagueRef(0)],
                        cups: vec![CupRef(0)],
                    }
                ),
                (
                    8,
                    &TableContent::Own {
                        table: id("_englandTable"),
                        leagues: vec![LeagueRef(1)],
                        cups: vec![],
                    }
                ),
                (80, &TableContent::Region(GroupRef(0))),
                (72, &TableContent::Region(GroupRef(1))),
            ],
            "a region's slot needs no country"
        );
    }

    #[test]
    fn the_berths_are_countries() {
        let world = World::from_package(&package()).expect("a valid world");
        let [bucket] = world.seed_buckets() else {
            panic!("one tier");
        };
        assert_eq!((bucket.tier.as_str(), bucket.size), ("bucket2", 2));
        assert_eq!(
            bucket.allocation,
            [
                Berths {
                    country: ENGLAND,
                    berths: 1
                },
                Berths {
                    country: ALBANIA,
                    berths: 1
                },
            ]
        );
    }

    #[test]
    fn missing_misplaced_and_unknown_files_are_reported() {
        let mut package = package();
        let leagues = package
            .competitions
            .remove("leagues.json")
            .expect("leagues");
        package.competitions.insert("cups.json".to_owned(), leagues);
        package.competitions.remove("seed_buckets.json");
        let career = package.competitions["career.json"].clone();
        package.competitions.insert("rules.json".to_owned(), career);
        assert_eq!(
            problems(&package),
            [
                "competitions/cups.json: a `competition_leagues` file, where the world reads \
             `competition_cups`",
                "competitions/rules.json: not one of the world's competition files",
                "competitions/leagues.json: the file is missing",
                "competitions/cups.json: the file is missing",
                "competitions/seed_buckets.json: the file is missing",
            ]
        );
    }

    #[test]
    fn an_id_defined_twice_is_reported() {
        let package = package_with(&[("playoffs.json", r#""_div1Playoffs""#, r#""_euroCup""#)]);
        assert_eq!(
            problems(&package),
            [
                "competitions/playoffs.json: playoffs[0]: _euroCup is defined again (first at \
             competitions/tournaments.json: competitions[0])",
                "competitions/leagues.json: leagues[1] (_englishLeague).playoff_links[0]: no \
             play-off or league block is called _div1Playoffs",
                "competitions/league_blocks.json: groups[0] (_englishLeagues1).playoffs: no \
             play-off is called _div1Playoffs",
            ]
        );
    }

    #[test]
    fn every_wrong_reference_is_reported_where_it_is() {
        let package = package_with(&[
            (
                "leagues.json",
                r#""_englishLeagues1", null"#,
                r#""_nowhere", null"#,
            ),
            (
                "qualifying.json",
                r#""_euroCup", "_cup"#,
                r#""_englishLeagues1", "_cup"#,
            ),
            (
                "league_blocks.json",
                r#""next_group": "_englishLeagues2""#,
                r#""next_group": "_div1Playoffs""#,
            ),
            (
                "league_blocks.json",
                r#"[{"league": "_englishLeague""#,
                r#"[{"league": "_albanianCup""#,
            ),
            ("career.json", r#""_friendly""#, r#""_europeTable""#),
            (
                "country_competitions.json",
                r#"["_englishLeague"]}"#,
                r#"["_englishLeague"], "cups": ["_euroCup"]}"#,
            ),
            (
                "country_competitions.json",
                r#""table": "_europeTable""#,
                r#""table": "_albanianLeague""#,
            ),
        ]);
        assert_eq!(
            problems(&package),
            [
                "competitions/leagues.json: leagues[1] (_englishLeague).playoff_links[1]: no \
             play-off or league block is called _nowhere",
                "competitions/qualifying.json: groups[0] (_europeTable).comp_cup[1]: \
             _englishLeagues1 is a league block, not a league, cup, tournament or play-off",
                "competitions/league_blocks.json: groups[0] (_englishLeagues1).leagues[0].league: \
             _albanianCup is a cup, not a league",
                "competitions/league_blocks.json: groups[0] (_englishLeagues1).next_group: \
             _div1Playoffs is a play-off, not a league block",
                "competitions/career.json: contests[1].competition: _europeTable is a qualifying \
             group, not a league, cup, tournament or play-off",
                "competitions/country_competitions.json: countries[1].cups[0]: _euroCup is a \
             tournament, not a cup",
                "competitions/country_competitions.json: countries[2].table: _albanianLeague is a \
             league, not a qualifying group",
            ]
        );
    }

    #[test]
    fn every_country_is_one_of_the_worlds() {
        let package = package_with(&[
            (
                "cups.json",
                r#"{"country": "CN_ENGLAND""#,
                r#"{"country": "CN_MARS""#,
            ),
            ("tournaments.json", r#""CN_ALBANIA""#, "5"),
            (
                "leagues.json",
                r#""reserved1": 0, "name": []}"#,
                r#""reserved1": 0, "name": [], "new": {"country": 47, "name": "ANDORRA", "byte": 0}}"#,
            ),
            ("qualifying.json", "[0, 8, 255, 99]", "[0, 9, 255]"),
            (
                "country_competitions.json",
                r#"{"number": 8, "name": "ENGLAND""#,
                r#"{"number": 9, "name": "ENGLAND""#,
            ),
            ("seed_buckets.json", r#""number": 0"#, r#""number": 3"#),
        ]);
        assert_eq!(
            problems(&package),
            [
                "competitions/leagues.json: leagues[0] (_albanianLeague).new.country: country 47 is \
             not a country of the world",
                "competitions/cups.json: competitions[1] (_cupWinnersCup).team_seed_list[0].country: \
             CN_MARS is not a country of the world",
                "competitions/tournaments.json: competitions[0] (_euroCup).team_seed_list[0].country: \
             country 5 is not a country of the world",
                "competitions/qualifying.json: groups[0] (_europeTable).countries[1]: country 9 is \
             not a country of the world",
                "competitions/country_competitions.json: countries[1].number: country 9 is not a \
             country of the world",
                "competitions/seed_buckets.json: euro_cup_qual_seed_buckets[0].allocation[1].number: \
             country 3 is not a country of the world",
            ]
        );
    }

    #[test]
    fn a_record_of_a_type_its_file_does_not_hold_is_reported() {
        let package = package_with(&[
            (
                "leagues.json",
                r#""comp_id": 37, "type": 0"#,
                r#""comp_id": 37, "type": 1"#,
            ),
            (
                "cups.json",
                r#""comp_id": 109, "type": 1"#,
                r#""comp_id": 109, "type": 2"#,
            ),
            (
                "tournaments.json",
                r#""comp_id": 0, "type": 2"#,
                r#""comp_id": 0, "type": 0"#,
            ),
            (
                "playoffs.json",
                r#""comp_id": 47, "type": 1"#,
                r#""comp_id": 47, "type": 0"#,
            ),
        ]);
        assert_eq!(
            problems(&package),
            [
                "competitions/leagues.json: leagues[0] (_albanianLeague).comp.type: a Cup record, \
             which this file does not hold",
                "competitions/cups.json: competitions[0] (_albanianCup).comp.type: a Tournament \
             record, which this file does not hold",
                "competitions/tournaments.json: competitions[0] (_euroCup).comp.type: a League \
             record, which this file does not hold",
                "competitions/playoffs.json: playoffs[0] (_div1Playoffs).comp.type: a League \
             record, which this file does not hold",
            ]
        );
    }

    #[test]
    fn a_table_slot_listed_twice_is_reported() {
        let package = package_with(&[(
            "country_competitions.json",
            r#"{"number": 72, "name": "CUSTOM""#,
            r#"{"number": 80, "name": "CUSTOM""#,
        )]);
        assert_eq!(
            problems(&package),
            [
                "competitions/country_competitions.json: countries[3].number: table 80 is listed \
             again (first at competitions/country_competitions.json: countries[2])"
            ]
        );
    }
}
