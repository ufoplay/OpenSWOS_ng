//! The file's content: the records, the manager's nationalities, the job-offer pick.
//!
//! Upstream's loader (`DataJson_LoadNationalTeams`, src/save_format_json.c) refuses a file
//! whole when a record or nationality does not read or a list is too long. So does this
//! crate. Its `job_offer_pick` outside 1..=256 the loader ignores; it does not read here.

use openswos_common::Nationality;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::record::NationalTeamRecord;

/// The most records a file may hold (upstream `NATIONAL_TEAMS_MAX`, SDL build).
pub const MAX_RECORDS: usize = 256;

/// The most nationalities a file may list: upstream's `NATIONALITIES_MAX` (128, SDL build)
/// less the OTHER the game adds.
pub const MAX_NATIONALITIES: usize = 127;

/// The nationality the game adds after the listed ones: OTHER.
pub const OTHER: Nationality = Nationality(0xFF);

/// The fewest records a random national job may be drawn from.
const JOB_OFFER_PICK_MIN: u16 = 1;

/// The most records a random national job may be drawn from: the table's 256.
const JOB_OFFER_PICK_MAX: u16 = 256;

/// A nationality the career manager may take: any but OTHER, which the game adds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct ManagerNationality(Nationality);

/// OTHER, given as a listed nationality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("255 is OTHER, which the game adds: it is not listed")]
pub struct ListedOther;

impl ManagerNationality {
    /// The nationality.
    #[must_use]
    pub fn nationality(self) -> Nationality {
        self.0
    }
}

impl TryFrom<u8> for ManagerNationality {
    type Error = ListedOther;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        let nationality = Nationality(code);
        if nationality == OTHER {
            Err(ListedOther)
        } else {
            Ok(Self(nationality))
        }
    }
}

impl From<ManagerNationality> for u8 {
    fn from(nationality: ManagerNationality) -> Self {
        nationality.0.0
    }
}

/// How many of the first records a random national job offer is drawn from (upstream
/// `_nat_teams_rand_limit1`): 1..=256.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct JobOfferPick(u16);

/// A job-offer pick outside 1..=256.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is no job-offer pick ({JOB_OFFER_PICK_MIN}..={JOB_OFFER_PICK_MAX})")]
pub struct InvalidJobOfferPick(pub u16);

impl JobOfferPick {
    /// The records.
    #[must_use]
    pub fn records(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for JobOfferPick {
    type Error = InvalidJobOfferPick;

    fn try_from(records: u16) -> Result<Self, Self::Error> {
        if (JOB_OFFER_PICK_MIN..=JOB_OFFER_PICK_MAX).contains(&records) {
            Ok(Self(records))
        } else {
            Err(InvalidJobOfferPick(records))
        }
    }
}

impl From<JobOfferPick> for u16 {
    fn from(pick: JobOfferPick) -> Self {
        pick.0
    }
}

/// Why lists are not a national-teams file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NationalTeamsError {
    /// No records.
    #[error("no national team records")]
    NoRecords,
    /// More records than the game holds.
    #[error("{0} national team records: at most {MAX_RECORDS}")]
    TooManyRecords(usize),
    /// More nationalities than the game holds.
    #[error("{0} nationalities: at most {MAX_NATIONALITIES}")]
    TooManyNationalities(usize),
}

/// A `national_teams.json`: the national teams and the career manager's nationalities, in
/// place of the original's.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "TeamsText", into = "TeamsText")]
pub struct NationalTeams {
    about: Option<String>,
    records: Vec<NationalTeamRecord>,
    nationalities: Option<Vec<ManagerNationality>>,
    job_offer_pick: Option<JobOfferPick>,
}

impl NationalTeams {
    /// The file's content.
    ///
    /// # Errors
    ///
    /// When there are no records, or more records or nationalities than the game holds.
    pub fn new(
        records: Vec<NationalTeamRecord>,
        nationalities: Option<Vec<ManagerNationality>>,
        job_offer_pick: Option<JobOfferPick>,
    ) -> Result<Self, NationalTeamsError> {
        if records.is_empty() {
            return Err(NationalTeamsError::NoRecords);
        }
        if records.len() > MAX_RECORDS {
            return Err(NationalTeamsError::TooManyRecords(records.len()));
        }
        if let Some(listed) = &nationalities
            && listed.len() > MAX_NATIONALITIES
        {
            return Err(NationalTeamsError::TooManyNationalities(listed.len()));
        }
        Ok(Self {
            about: None,
            records,
            nationalities,
            job_offer_pick,
        })
    }

    /// These teams with `about`, the file's comment.
    #[must_use]
    pub fn with_about(self, about: String) -> Self {
        Self {
            about: Some(about),
            ..self
        }
    }

    /// The file's comment (World 2026-27's says where its records come from), which the game
    /// does not read. `None`: the key is absent.
    #[must_use]
    pub fn about(&self) -> Option<&str> {
        self.about.as_deref()
    }

    /// The national teams, in order: a nationality's national team is the first record
    /// with it (upstream `GetNationalTeamNumber`, src/teams.c).
    #[must_use]
    pub fn records(&self) -> &[NationalTeamRecord] {
        &self.records
    }

    /// The nationalities the career manager may take, in the menu's order (the game adds
    /// OTHER last). `None`: the game keeps the original's. An empty list does too, but is
    /// kept as written.
    #[must_use]
    pub fn nationalities(&self) -> Option<&[ManagerNationality]> {
        self.nationalities.as_deref()
    }

    /// The first records a random national job offer is drawn from. `None`: the game
    /// keeps the original's.
    #[must_use]
    pub fn job_offer_pick(&self) -> Option<JobOfferPick> {
        self.job_offer_pick
    }
}

/// The file, as the JSON names its keys.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TeamsText {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    about: Option<String>,
    records: Vec<NationalTeamRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nationalities: Option<Vec<ManagerNationality>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    job_offer_pick: Option<JobOfferPick>,
}

impl TryFrom<TeamsText> for NationalTeams {
    type Error = NationalTeamsError;

    fn try_from(text: TeamsText) -> Result<Self, Self::Error> {
        let TeamsText {
            about,
            records,
            nationalities,
            job_offer_pick,
        } = text;
        let teams = Self::new(records, nationalities, job_offer_pick)?;
        Ok(match about {
            Some(about) => teams.with_about(about),
            None => teams,
        })
    }
}

impl From<NationalTeams> for TeamsText {
    fn from(teams: NationalTeams) -> Self {
        let NationalTeams {
            about,
            records,
            nationalities,
            job_offer_pick,
        } = teams;
        Self {
            about,
            records,
            nationalities,
            job_offer_pick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> NationalTeamRecord {
        NationalTeamRecord::from_bytes([80, 0, 0, 0, 0, 19])
    }

    #[test]
    fn a_file_has_records_within_the_maxima() {
        assert_eq!(
            NationalTeams::new(Vec::new(), None, None),
            Err(NationalTeamsError::NoRecords)
        );
        assert!(NationalTeams::new(vec![record(); MAX_RECORDS], None, None).is_ok());
        assert_eq!(
            NationalTeams::new(vec![record(); MAX_RECORDS + 1], None, None),
            Err(NationalTeamsError::TooManyRecords(MAX_RECORDS + 1))
        );
        let nationality = ManagerNationality::try_from(8).expect("a nationality");
        assert!(
            NationalTeams::new(
                vec![record()],
                Some(vec![nationality; MAX_NATIONALITIES]),
                None
            )
            .is_ok()
        );
        assert_eq!(
            NationalTeams::new(
                vec![record()],
                Some(vec![nationality; MAX_NATIONALITIES + 1]),
                None
            ),
            Err(NationalTeamsError::TooManyNationalities(
                MAX_NATIONALITIES + 1
            ))
        );
    }

    #[test]
    fn other_is_not_listed() {
        assert_eq!(ManagerNationality::try_from(0xFF), Err(ListedOther));
        assert_eq!(
            ManagerNationality::try_from(0xFE).map(ManagerNationality::nationality),
            Ok(Nationality(0xFE))
        );
    }

    #[test]
    fn a_job_offer_pick_is_one_to_256_records() {
        assert_eq!(JobOfferPick::try_from(0), Err(InvalidJobOfferPick(0)));
        assert_eq!(JobOfferPick::try_from(1).map(JobOfferPick::records), Ok(1));
        assert_eq!(
            JobOfferPick::try_from(256).map(JobOfferPick::records),
            Ok(256)
        );
        assert_eq!(JobOfferPick::try_from(257), Err(InvalidJobOfferPick(257)));
    }
}
