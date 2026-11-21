#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod calendar;
pub mod country_details;
pub mod file;
pub mod fixtures;
pub mod rules;
pub mod season;
pub mod team_files;

pub use calendar::Calendar;
pub use country_details::{CountryDetail, CountryDetails};
pub use file::{RulesFile, read, write};
pub use fixtures::{Fixtures, OddRoundRobin};
pub use rules::{Rules, RulesError, RulesFileKind};
pub use season::{Season, SeasonEnd};
pub use team_files::{SecondFile, TeamFiles};
