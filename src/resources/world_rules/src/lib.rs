#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod calendar;
pub mod competition_classes;
pub mod country_details;
pub mod draws;
pub mod file;
pub mod fixtures;
pub mod rules;
pub mod season;
pub mod strings;
pub mod team_files;

pub use calendar::Calendar;
pub use competition_classes::CompetitionClasses;
pub use country_details::{CountryDetail, CountryDetails};
pub use draws::{BestPlaced, Draws, OwnDraw, Pattern, Ranking, StageDraw};
pub use file::{RulesFile, read, write};
pub use fixtures::{CompetitionSchedule, Fixtures, Schedule};
pub use rules::{Rules, RulesError, RulesFileKind};
pub use season::{Season, SeasonEnd};
pub use strings::Strings;
pub use team_files::{SecondFile, TeamFiles};
