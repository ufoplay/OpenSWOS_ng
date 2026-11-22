//! A package's `rules/` files, each read into its type (the ones this crate knows so far).

use thiserror::Error;

use crate::calendar::Calendar;
use crate::competition_classes::CompetitionClasses;
use crate::country_details::CountryDetails;
use crate::draws::Draws;
use crate::file::{RulesFile, read, write};
use crate::fixtures::Fixtures;
use crate::season::Season;
use crate::strings::Strings;
use crate::team_files::TeamFiles;

/// A file of `rules/` this crate reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RulesFileKind {
    /// `calendar.json`.
    Calendar,
    /// `country_details.json`.
    CountryDetails,
    /// `team_files.json`.
    TeamFiles,
    /// `season.json`.
    Season,
    /// `fixtures.json`.
    Fixtures,
    /// `strings.json`.
    Strings,
    /// `competition_classes.json`.
    CompetitionClasses,
    /// `draws.json`.
    Draws,
}

impl RulesFileKind {
    /// Every kind, in the order the files are written.
    pub const ALL: [Self; 8] = [
        Self::Calendar,
        Self::CountryDetails,
        Self::TeamFiles,
        Self::Season,
        Self::Fixtures,
        Self::Strings,
        Self::CompetitionClasses,
        Self::Draws,
    ];

    /// Its file name in `rules/`.
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Calendar => Calendar::FILE_NAME,
            Self::CountryDetails => CountryDetails::FILE_NAME,
            Self::TeamFiles => TeamFiles::FILE_NAME,
            Self::Season => Season::FILE_NAME,
            Self::Fixtures => Fixtures::FILE_NAME,
            Self::Strings => Strings::FILE_NAME,
            Self::CompetitionClasses => CompetitionClasses::FILE_NAME,
            Self::Draws => Draws::FILE_NAME,
        }
    }

    /// The kind whose file is called `name`.
    #[must_use]
    pub fn of(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.file_name() == name)
    }
}

/// A `rules/` file that does not read.
#[derive(Debug, Error)]
#[error("{file}: {source}")]
pub struct RulesError {
    /// The file's name.
    pub file: &'static str,
    /// Why.
    pub source: serde_json::Error,
}

/// A package's `rules/` files: each one it has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rules {
    /// `calendar.json`.
    pub calendar: Option<Calendar>,
    /// `country_details.json`.
    pub country_details: Option<CountryDetails>,
    /// `team_files.json`.
    pub team_files: Option<TeamFiles>,
    /// `season.json`.
    pub season: Option<Season>,
    /// `fixtures.json`.
    pub fixtures: Option<Fixtures>,
    /// `strings.json`.
    pub strings: Option<Strings>,
    /// `competition_classes.json`.
    pub competition_classes: Option<CompetitionClasses>,
    /// `draws.json`.
    pub draws: Option<Draws>,
}

impl Rules {
    /// Reads `bytes` as the file of `kind`, in its place.
    ///
    /// # Errors
    ///
    /// When the bytes are not the file's JSON.
    pub fn read(&mut self, kind: RulesFileKind, bytes: &[u8]) -> Result<(), RulesError> {
        let error = |source| RulesError {
            file: kind.file_name(),
            source,
        };
        match kind {
            RulesFileKind::Calendar => self.calendar = Some(read(bytes).map_err(error)?),
            RulesFileKind::CountryDetails => {
                self.country_details = Some(read(bytes).map_err(error)?);
            }
            RulesFileKind::TeamFiles => self.team_files = Some(read(bytes).map_err(error)?),
            RulesFileKind::Season => self.season = Some(read(bytes).map_err(error)?),
            RulesFileKind::Fixtures => self.fixtures = Some(read(bytes).map_err(error)?),
            RulesFileKind::Strings => self.strings = Some(read(bytes).map_err(error)?),
            RulesFileKind::CompetitionClasses => {
                self.competition_classes = Some(read(bytes).map_err(error)?);
            }
            RulesFileKind::Draws => self.draws = Some(read(bytes).map_err(error)?),
        }
        Ok(())
    }

    /// Each file it has, written, by name, in [`RulesFileKind::ALL`]'s order.
    #[must_use]
    pub fn files(&self) -> Vec<(&'static str, Vec<u8>)> {
        RulesFileKind::ALL
            .into_iter()
            .filter_map(|kind| self.written(kind).map(|bytes| (kind.file_name(), bytes)))
            .collect()
    }

    /// The file of `kind`, written, if it has it.
    fn written(&self, kind: RulesFileKind) -> Option<Vec<u8>> {
        match kind {
            RulesFileKind::Calendar => self.calendar.as_ref().map(write),
            RulesFileKind::CountryDetails => self.country_details.as_ref().map(write),
            RulesFileKind::TeamFiles => self.team_files.as_ref().map(write),
            RulesFileKind::Season => self.season.as_ref().map(write),
            RulesFileKind::Fixtures => self.fixtures.as_ref().map(write),
            RulesFileKind::Strings => self.strings.as_ref().map(write),
            RulesFileKind::CompetitionClasses => self.competition_classes.as_ref().map(write),
            RulesFileKind::Draws => self.draws.as_ref().map(write),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_its_file_name() {
        for kind in RulesFileKind::ALL {
            assert_eq!(RulesFileKind::of(kind.file_name()), Some(kind));
        }
        assert_eq!(RulesFileKind::of("career_params.json"), None);
        assert_eq!(RulesFileKind::of("europe.json"), None);
    }

    #[test]
    fn the_files_read_into_their_places_and_write_back() {
        let mut rules = Rules::default();
        assert!(rules.files().is_empty());
        let calendar = b"{\n  \"first_year\": 2026\n}\n";
        let fixtures = b"{\n  \"odd_tournament_groups\": true,\n  \"schedules\": [],\n  \
            \"competition_schedules\": []\n}\n";
        rules
            .read(RulesFileKind::Fixtures, fixtures)
            .expect("fixtures");
        rules
            .read(RulesFileKind::Calendar, calendar)
            .expect("a calendar");
        assert_eq!(
            rules.calendar.map(|calendar| calendar.first_year),
            Some(2026)
        );
        assert_eq!(
            rules
                .fixtures
                .as_ref()
                .map(|fixtures| fixtures.odd_tournament_groups),
            Some(true)
        );
        assert_eq!(
            rules.files(),
            [
                ("calendar.json", calendar.to_vec()),
                ("fixtures.json", fixtures.to_vec())
            ]
        );
    }

    #[test]
    fn a_broken_file_is_named() {
        let error = Rules::default()
            .read(RulesFileKind::Season, b"[]")
            .expect_err("not a season");
        assert!(error.to_string().starts_with("season.json: "), "{error}");
    }
}
