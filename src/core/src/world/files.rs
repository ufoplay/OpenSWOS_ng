//! The competition files a world reads: upstream's ten, each at its path with its `type`.

use openswos_competitions_json_resource::{
    CareerFile, CompetitionsFile, CountriesFile, CountryCompetitionsFile, CupsFile,
    LeagueBlocksFile, LeaguesFile, PlayoffsFile, QualifyingFile, SeedBucketsFile, TournamentsFile,
};
use openswos_world_package_resource::{COMPETITIONS_FOLDER, WorldPackage};

use crate::validation::{Location, Problem, Problems};

/// One of the world's competition files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Slot {
    /// `leagues.json`.
    Leagues,
    /// `cups.json`.
    Cups,
    /// `tournaments.json`.
    Tournaments,
    /// `playoffs.json`.
    Playoffs,
    /// `qualifying.json`.
    Qualifying,
    /// `league_blocks.json`.
    LeagueBlocks,
    /// `career.json`.
    Career,
    /// `country_competitions.json`.
    CountryCompetitions,
    /// `countries.json`.
    Countries,
    /// `seed_buckets.json`.
    SeedBuckets,
}

impl Slot {
    /// Every file, in the order a missing one is reported.
    const ALL: [Self; 10] = [
        Self::Leagues,
        Self::Cups,
        Self::Tournaments,
        Self::Playoffs,
        Self::Qualifying,
        Self::LeagueBlocks,
        Self::Career,
        Self::CountryCompetitions,
        Self::Countries,
        Self::SeedBuckets,
    ];

    /// Its name in `competitions/` (upstream's, src/save_format_json.c).
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Leagues => "leagues.json",
            Self::Cups => "cups.json",
            Self::Tournaments => "tournaments.json",
            Self::Playoffs => "playoffs.json",
            Self::Qualifying => "qualifying.json",
            Self::LeagueBlocks => "league_blocks.json",
            Self::Career => "career.json",
            Self::CountryCompetitions => "country_competitions.json",
            Self::Countries => "countries.json",
            Self::SeedBuckets => "seed_buckets.json",
        }
    }

    /// Its path in the package (`competitions/leagues.json`).
    pub(crate) fn path(self) -> String {
        format!("{COMPETITIONS_FOLDER}/{}", self.name())
    }

    /// Its `type` key.
    fn tag(self) -> &'static str {
        match self {
            Self::Leagues => "competition_leagues",
            Self::Cups => "competition_cups",
            Self::Tournaments => "competition_tournaments",
            Self::Playoffs => "playoffs",
            Self::Qualifying => "qualifying_groups",
            Self::LeagueBlocks => "league_blocks",
            Self::Career => "career_index",
            Self::CountryCompetitions => "country_competitions",
            Self::Countries => "countries",
            Self::SeedBuckets => "seed_buckets",
        }
    }

    /// The file named `name`, if the world reads it.
    fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|slot| slot.name() == name)
    }

    /// The file whose records `file` holds.
    fn of(file: &CompetitionsFile) -> Self {
        match file {
            CompetitionsFile::Leagues(_) => Self::Leagues,
            CompetitionsFile::Cups(_) => Self::Cups,
            CompetitionsFile::Tournaments(_) => Self::Tournaments,
            CompetitionsFile::Playoffs(_) => Self::Playoffs,
            CompetitionsFile::Qualifying(_) => Self::Qualifying,
            CompetitionsFile::LeagueBlocks(_) => Self::LeagueBlocks,
            CompetitionsFile::Career(_) => Self::Career,
            CompetitionsFile::CountryCompetitions(_) => Self::CountryCompetitions,
            CompetitionsFile::Countries(_) => Self::Countries,
            CompetitionsFile::SeedBuckets(_) => Self::SeedBuckets,
        }
    }
}

/// The world's competition files.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CompetitionFiles<'a> {
    /// `leagues.json`.
    pub(crate) leagues: &'a LeaguesFile,
    /// `cups.json`.
    pub(crate) cups: &'a CupsFile,
    /// `tournaments.json`.
    pub(crate) tournaments: &'a TournamentsFile,
    /// `playoffs.json`.
    pub(crate) playoffs: &'a PlayoffsFile,
    /// `qualifying.json`.
    pub(crate) qualifying: &'a QualifyingFile,
    /// `league_blocks.json`.
    pub(crate) league_blocks: &'a LeagueBlocksFile,
    /// `career.json`.
    pub(crate) career: &'a CareerFile,
    /// `country_competitions.json`.
    pub(crate) country_competitions: &'a CountryCompetitionsFile,
    /// `countries.json`.
    pub(crate) countries: &'a CountriesFile,
    /// `seed_buckets.json`.
    pub(crate) seed_buckets: &'a SeedBucketsFile,
}

/// The files as found, each `None` until found.
#[derive(Default)]
struct Found<'a> {
    leagues: Option<&'a LeaguesFile>,
    cups: Option<&'a CupsFile>,
    tournaments: Option<&'a TournamentsFile>,
    playoffs: Option<&'a PlayoffsFile>,
    qualifying: Option<&'a QualifyingFile>,
    league_blocks: Option<&'a LeagueBlocksFile>,
    career: Option<&'a CareerFile>,
    country_competitions: Option<&'a CountryCompetitionsFile>,
    countries: Option<&'a CountriesFile>,
    seed_buckets: Option<&'a SeedBucketsFile>,
}

impl<'a> Found<'a> {
    fn insert(&mut self, file: &'a CompetitionsFile) {
        match file {
            CompetitionsFile::Leagues(file) => self.leagues = Some(file),
            CompetitionsFile::Cups(file) => self.cups = Some(file),
            CompetitionsFile::Tournaments(file) => self.tournaments = Some(file),
            CompetitionsFile::Playoffs(file) => self.playoffs = Some(file),
            CompetitionsFile::Qualifying(file) => self.qualifying = Some(file),
            CompetitionsFile::LeagueBlocks(file) => self.league_blocks = Some(file),
            CompetitionsFile::Career(file) => self.career = Some(file),
            CompetitionsFile::CountryCompetitions(file) => self.country_competitions = Some(file),
            CompetitionsFile::Countries(file) => self.countries = Some(file),
            CompetitionsFile::SeedBuckets(file) => self.seed_buckets = Some(file),
        }
    }
}

/// The package's competition files, or `None` when one is missing or misplaced (each
/// reported).
pub(crate) fn pick<'a>(
    package: &'a WorldPackage,
    problems: &mut Problems,
) -> Option<CompetitionFiles<'a>> {
    let mut found = Found::default();
    for (name, document) in &package.competitions {
        let location = Location::file(&format!("{COMPETITIONS_FOLDER}/{name}"));
        let holds = Slot::of(&document.file);
        match Slot::named(name) {
            Some(slot) if slot == holds => found.insert(&document.file),
            Some(slot) => problems.push(
                location,
                Problem::WrongFileType {
                    expected: slot.tag(),
                    found: holds.tag(),
                },
            ),
            None => problems.push(location, Problem::UnknownFile),
        }
    }
    let leagues = required(found.leagues, Slot::Leagues, problems);
    let cups = required(found.cups, Slot::Cups, problems);
    let tournaments = required(found.tournaments, Slot::Tournaments, problems);
    let playoffs = required(found.playoffs, Slot::Playoffs, problems);
    let qualifying = required(found.qualifying, Slot::Qualifying, problems);
    let league_blocks = required(found.league_blocks, Slot::LeagueBlocks, problems);
    let career = required(found.career, Slot::Career, problems);
    let country_competitions = required(
        found.country_competitions,
        Slot::CountryCompetitions,
        problems,
    );
    let countries = required(found.countries, Slot::Countries, problems);
    let seed_buckets = required(found.seed_buckets, Slot::SeedBuckets, problems);
    Some(CompetitionFiles {
        leagues: leagues?,
        cups: cups?,
        tournaments: tournaments?,
        playoffs: playoffs?,
        qualifying: qualifying?,
        league_blocks: league_blocks?,
        career: career?,
        country_competitions: country_competitions?,
        countries: countries?,
        seed_buckets: seed_buckets?,
    })
}

/// `file`, reporting it missing when it is `None`.
fn required<T>(file: Option<T>, slot: Slot, problems: &mut Problems) -> Option<T> {
    if file.is_none() {
        problems.push(Location::file(&slot.path()), Problem::MissingFile);
    }
    file
}

#[cfg(test)]
mod tests {
    use super::Slot;

    #[test]
    fn every_file_is_named_and_typed_once() {
        for slot in Slot::ALL {
            assert_eq!(Slot::named(slot.name()), Some(slot));
            assert_eq!(slot.path(), format!("competitions/{}", slot.name()));
        }
        let mut tags = Slot::ALL.map(Slot::tag).to_vec();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), Slot::ALL.len());
        assert_eq!(Slot::named("rules.json"), None);
    }
}
