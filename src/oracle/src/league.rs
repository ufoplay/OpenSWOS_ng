//! A league started (upstream src/diy_competition.c `InitDIYCompetitionFromTableEntry`): a
//! division of a record made the competition under way, from an image of it before. The
//! places' shuffle draws from the second generator: set it with
//! [`Oracle::set_rand2_state`] first.

use crate::Oracle;
use crate::draw::Prices;
use crate::ffi;
use crate::running::IMAGE_LEN;

/// A record a league is started from, as its C struct lays it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordBytes {
    /// A league's (`CompetitionLeague`).
    League {
        /// Its `Competition` header's 9 bytes.
        header: [u8; 9],
        /// `divisionsCount`.
        divisions_count: i8,
        /// `rounds`.
        rounds: i8,
        /// `pointsForWin`.
        points_for_win: i8,
        /// `subsAndMaxSubs`.
        substitutes: u8,
        /// `divisions` (16 at most), each its six bytes.
        divisions: Vec<[i8; 6]>,
        /// `name` (32 at most): two a division, none for a null pointer.
        names: Vec<Option<Vec<u8>>>,
    },
    /// A cup's, played as a league (`CompetitionCup`, of type 0).
    Cup {
        /// Its `Competition` header's 9 bytes.
        header: [u8; 9],
        /// `numRoundsOverride`.
        num_rounds_override: i8,
        /// `teams`.
        teams: i8,
        /// `awayGoalsMode`.
        away_goals_mode: i8,
        /// `subsAndPointsForWin`.
        substitutes: u8,
        /// `roundStringBase`.
        round_string_base: i8,
        /// `roundRuleFlags`.
        round_rules: [i8; 6],
        /// `name`, none for a null pointer.
        names: [Option<Vec<u8>>; 2],
    },
}

/// What `InitDIYCompetitionFromTableEntry` is given and reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivisionSetup {
    /// The competition's image before.
    pub before: Box<[u8; IMAGE_LEN]>,
    /// The record.
    pub record: RecordBytes,
    /// The division (`idx`).
    pub division: u16,
    /// The divisions' default names (`_divisionStringTable`), four.
    pub division_names: [Vec<u8>; 4],
    /// Whether the record is in the national teams' list.
    pub national: bool,
    /// Whether the record is in the continental track's list.
    pub continental: bool,
    /// The session's pitch type (`_pitchType`).
    pub pitch_type: u16,
    /// The selected teams' prices, by ordinal: the team numbers of a country other than 0xFF
    /// load the selected team of their ordinal.
    pub teams: Vec<Prices>,
    /// The career club's prices: the team numbers of country 0xFF load it.
    pub career_team: Prices,
}

impl Oracle {
    /// `InitDIYCompetitionFromTableEntry` of `setup`: the competition's image after, or the
    /// reason code upstream halts with.
    ///
    /// # Errors
    ///
    /// Upstream's halt (0x1313: the division's round robin is no real rounds).
    pub fn start_division(&mut self, setup: &DivisionSetup) -> Result<Box<[u8; IMAGE_LEN]>, u16> {
        ffi::league::start(setup)
    }
}
