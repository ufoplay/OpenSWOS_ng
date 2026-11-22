//! A cup started (upstream src/diy_competition.c `InitDIYCupFromRecord`): a knockout's record,
//! a cup's or a play-off's, made the competition under way, from an image of it before. The
//! pairings' shuffle draws from the second generator: set it with [`Oracle::set_rand2_state`]
//! first.

use crate::Oracle;
use crate::draw::Prices;
use crate::ffi;
use crate::league::CompiledRecord;
use crate::running::IMAGE_LEN;

/// The C struct a knockout's record is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// A cup's (`CompetitionCup`): six round rules.
    Cup,
    /// A play-off's (`Playoffs`): two round rules.
    Playoff,
}

/// A record a cup is started from, as its C struct lays it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnockoutBytes {
    /// Its struct.
    pub layout: Layout,
    /// Its `Competition` header's 9 bytes.
    pub header: [u8; 9],
    /// `numRoundsOverride`.
    pub num_rounds_override: i8,
    /// `teams`.
    pub teams: i8,
    /// `awayGoalsMode`.
    pub away_goals_mode: i8,
    /// `subsAndPointsForWin` (a play-off's `subsAndMaxSubs`).
    pub substitutes: u8,
    /// `roundStringBase`.
    pub round_string_base: i8,
    /// `roundRuleFlags`: those past the struct's (six a cup's, two a play-off's) are dropped.
    pub round_rules: Vec<i8>,
    /// `name`, none for a null pointer.
    pub names: [Option<Vec<u8>>; 2],
}

/// A team the stubbed loader gives: its players' prices and who controls it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedTeam {
    /// Its players' prices.
    pub prices: Prices,
    /// `teamControls`.
    pub controls: u8,
}

/// What `InitDIYCupFromRecord` is given and reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CupSetup {
    /// The competition's image before.
    pub before: Box<[u8; IMAGE_LEN]>,
    /// The record.
    pub record: KnockoutBytes,
    /// Whether the record is in the national teams' list.
    pub national: bool,
    /// Whether the record is in the continental track's list.
    pub continental: bool,
    /// The session's pitch type (`_pitchType`).
    pub pitch_type: u16,
    /// The selected teams, by ordinal: the team numbers of a country other than 0xFF load the
    /// selected team of their ordinal.
    pub teams: Vec<SelectedTeam>,
    /// The career club: the team numbers of country 0xFF load it.
    pub career_team: SelectedTeam,
}

/// What `InitDIYCupFromRecord` is given and reads for a compiled record: upstream's own lists
/// as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledCupSetup {
    /// The competition's image before.
    pub before: Box<[u8; IMAGE_LEN]>,
    /// The record.
    pub record: CompiledRecord,
    /// The session's pitch type (`_pitchType`).
    pub pitch_type: u16,
    /// The selected teams, by ordinal, as [`CupSetup::teams`].
    pub teams: Vec<SelectedTeam>,
    /// The career club.
    pub career_team: SelectedTeam,
}

impl Oracle {
    /// `InitDIYCupFromRecord` of `setup`: the competition's image after, or the reason code
    /// upstream halts with.
    ///
    /// # Errors
    ///
    /// Upstream's halt (0x1313: an odd number of teams).
    pub fn start_cup(&mut self, setup: &CupSetup) -> Result<Box<[u8; IMAGE_LEN]>, u16> {
        ffi::cup::start(setup)
    }

    /// `InitDIYCupFromRecord` of a compiled record (`setup`): the competition's image after, or
    /// the reason code upstream halts with; `None` when upstream's tables have no such record.
    pub fn start_compiled_cup(
        &mut self,
        setup: &CompiledCupSetup,
    ) -> Option<Result<Box<[u8; IMAGE_LEN]>, u16>> {
        ffi::cup::start_compiled(setup)
    }
}
