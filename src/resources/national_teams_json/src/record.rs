//! A national team's record (upstream `NationalTeamRecord`, include/game_types.h): 6 bytes
//! in the game's table, a named object in the JSON.
//!
//! | offset | bytes | field | JSON key |
//! |---|---|---|---|
//! | 0 | 1 | the national team's team file | `country` |
//! | 1 | 1 | its place in that file | `ordinal` |
//! | 2 | 1 | its nationality | `nationality` |
//! | 3 | 1 | the team file of the nation's clubs; `0xFF` none | `team_file` |
//! | 4 | 2 | its skill, big-endian | `skill` |

use openswos_common::{Nationality, TeamFileNumber, TeamId};
use serde::{Deserialize, Serialize};

/// Bytes of a record in the game's table.
pub const NATIONAL_TEAM_RECORD_LEN: usize = 6;

/// The team-file byte of a nation whose clubs the game does not have (upstream
/// `NATIONAL_TEAM_NO_FILE`, include/national_teams.h): its national selector's ADD PLAYER
/// has no HOME, only ABROAD.
const NO_CLUB_FILE: u8 = 0xFF;

/// A national team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "RecordText", into = "RecordText")]
pub struct NationalTeamRecord {
    /// The national team: its team file (`country`) and place in it.
    pub team: TeamId,
    /// Its nationality. A nationality's national team is the first record with it
    /// (upstream `GetNationalTeamNumber`, src/teams.c).
    pub nationality: Nationality,
    /// The team file of the nation's clubs, if the game has them.
    pub club_file: Option<TeamFileNumber>,
    /// Its skill (upstream `skillOrPrice`): national job offers come from teams within the
    /// manager's reputation window of it (src/job_offers.c).
    pub skill: u16,
}

impl NationalTeamRecord {
    /// The record the game's table holds in `bytes`.
    #[must_use]
    pub fn from_bytes(bytes: [u8; NATIONAL_TEAM_RECORD_LEN]) -> Self {
        let [file, ordinal, nationality, club_file, skill_high, skill_low] = bytes;
        Self {
            team: TeamId::from_bytes([file, ordinal]),
            nationality: Nationality(nationality),
            club_file: (club_file != NO_CLUB_FILE).then_some(TeamFileNumber(club_file)),
            skill: u16::from_be_bytes([skill_high, skill_low]),
        }
    }

    /// The record as the game's table holds it.
    #[must_use]
    pub fn to_bytes(self) -> [u8; NATIONAL_TEAM_RECORD_LEN] {
        let Self {
            team,
            nationality: Nationality(nationality),
            club_file,
            skill,
        } = self;
        let [file, ordinal] = team.to_bytes();
        let club_file = club_file.map_or(NO_CLUB_FILE, |TeamFileNumber(file)| file);
        let [skill_high, skill_low] = skill.to_be_bytes();
        [file, ordinal, nationality, club_file, skill_high, skill_low]
    }
}

/// A record as the JSON names its fields.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordText {
    country: u8,
    ordinal: u8,
    nationality: u8,
    team_file: u8,
    skill: u16,
}

impl From<RecordText> for NationalTeamRecord {
    fn from(text: RecordText) -> Self {
        let RecordText {
            country,
            ordinal,
            nationality,
            team_file,
            skill,
        } = text;
        let [skill_high, skill_low] = skill.to_be_bytes();
        Self::from_bytes([
            country,
            ordinal,
            nationality,
            team_file,
            skill_high,
            skill_low,
        ])
    }
}

impl From<NationalTeamRecord> for RecordText {
    fn from(record: NationalTeamRecord) -> Self {
        let [
            country,
            ordinal,
            nationality,
            team_file,
            skill_high,
            skill_low,
        ] = record.to_bytes();
        Self {
            country,
            ordinal,
            nationality,
            team_file,
            skill: u16::from_be_bytes([skill_high, skill_low]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn the_skill_is_big_endian() {
        let record = NationalTeamRecord::from_bytes([80, 0, 0, 0, 0, 19]);
        assert_eq!(record.team, TeamId::from_bytes([80, 0]));
        assert_eq!(record.nationality, Nationality(0));
        assert_eq!(record.club_file, Some(TeamFileNumber(0)));
        assert_eq!(record.skill, 19);
    }

    #[test]
    fn no_club_file_is_0xff() {
        let record = NationalTeamRecord::from_bytes([80, 3, 120, NO_CLUB_FILE, 1, 0]);
        assert_eq!(record.club_file, None);
        assert_eq!(record.skill, 256);
        assert_eq!(record.to_bytes(), [80, 3, 120, NO_CLUB_FILE, 1, 0]);
    }

    #[test]
    fn reads_upstreams_example() {
        let text =
            r#"{ "country": 80, "ordinal": 0, "nationality": 0, "team_file": 0, "skill": 19 }"#;
        let record: NationalTeamRecord = serde_json::from_str(text).expect("a record");
        assert_eq!(record.to_bytes(), [80, 0, 0, 0, 0, 19]);
        assert_eq!(
            serde_json::to_string(&record).expect("serializes"),
            r#"{"country":80,"ordinal":0,"nationality":0,"team_file":0,"skill":19}"#
        );
    }

    #[test]
    fn a_record_needs_every_field_in_range() {
        for text in [
            r#"{"country": 80, "ordinal": 0, "nationality": 0, "team_file": 0}"#,
            r#"{"country": 256, "ordinal": 0, "nationality": 0, "team_file": 0, "skill": 19}"#,
            r#"{"country": 80, "ordinal": 0, "nationality": -1, "team_file": 0, "skill": 19}"#,
            r#"{"country": 80, "ordinal": 0, "nationality": 0, "team_file": 0, "skill": 65536}"#,
            r#"{"country": 80, "ordinal": 0, "nationality": 0, "team_file": 0, "skill": 19, "x": 1}"#,
        ] {
            assert!(
                serde_json::from_str::<NationalTeamRecord>(text).is_err(),
                "{text}"
            );
        }
    }

    proptest! {
        #[test]
        fn bytes_round_trip(bytes: [u8; NATIONAL_TEAM_RECORD_LEN]) {
            let record = NationalTeamRecord::from_bytes(bytes);
            prop_assert_eq!(record.to_bytes(), bytes);
            let text = serde_json::to_string(&record).expect("serializes");
            prop_assert_eq!(serde_json::from_str::<NationalTeamRecord>(&text).expect("reads back"), record);
        }
    }
}
