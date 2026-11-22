//! A tournament's state (upstream `DIYTournamentState`, include/game_types.h) as the C reads
//! it, and the functions that write it (src/diy_competition.c `InitDIYTournament`,
//! `DIYTournament_RecalculateMatchesPerTeam`, `InitDIYLeagueFromRecord`): an image written into
//! `_diyFileBufferCopy`, every field read back through bindgen's declaration of the struct, so
//! a port's layout is checked against the compiler's. A start's draws come from the second
//! generator: set it with [`Oracle::set_rand2_state`] first.

use crate::Oracle;
use crate::cup::SelectedTeam;
use crate::draw::Prices;
use crate::ffi;
use crate::league::CompiledRecord;
use crate::running::{HeaderFields, IMAGE_LEN};

pub use crate::ffi::tournament::STATE_LEN;

/// A stage's slot (`DIYTournamentSlot`) as the C reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotFields {
    /// `_pad_00`.
    pub pad: [u8; 2],
    /// `doubleCount`, `isLeagueFormat`, `seedingType`, `numTeams`, `divisionFactor`,
    /// `playEachTeam`, `initConst168`, `groups`, `teamsPerGroup`, `formatFlag16E`,
    /// `initConst170`, `formatFlag172`, `matchesPerSlot`, in their order.
    pub words: [u16; 13],
    /// `progressionTable`.
    pub months: [u8; 90],
}

/// Every field of a tournament's state, as upstream's C reads it (team ids as the native word
/// their two bytes make).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentFields {
    /// The header.
    pub header: HeaderFields,
    /// `leagueWarningsEnabled`.
    pub league_warnings_enabled: u16,
    /// `teamIds`.
    pub team_ids: [u16; 64],
    /// `groupMap`.
    pub group_map: [u8; 64],
    /// `slots`.
    pub slots: Vec<SlotFields>,
    /// `teamIndexOrder`.
    pub team_index_order: [u8; 64],
}

/// What `DIYTournament_RecalculateMatchesPerTeam` reads: the tournament's state, the
/// competition under way, the loaded teams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecalculateSetup {
    /// The tournament's state before.
    pub state: Box<[u8; STATE_LEN]>,
    /// The competition under way before.
    pub competition: Box<[u8; IMAGE_LEN]>,
    /// The selected teams' prices, by ordinal: the team numbers of a country other than 0xFF
    /// load the selected team of their ordinal.
    pub teams: Vec<Prices>,
    /// The career club's prices: the team numbers of country 0xFF load it.
    pub career_team: Prices,
}

/// A tournament's record, as its C struct (`CompetitionTournament`) lays it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentBytes {
    /// Its `Competition` header's 9 bytes.
    pub header: [u8; 9],
    /// `seedingType`.
    pub seeding_type: i8,
    /// `menuState`.
    pub menu_state: i8,
    /// `playEachTeam`.
    pub play_each_team: i8,
    /// `awayGoalsMode`.
    pub away_goals_mode: i8,
    /// `subsAndPointsForWin`.
    pub substitutes: u8,
    /// `roundStringBase`.
    pub round_string_base: i8,
    /// `rounds`: each `teamCount`, `groupCount`, `teamGroupCount`.
    pub rounds: [[i8; 3]; 6],
    /// `reserved`.
    pub reserved: i8,
    /// `roundRuleFlags`.
    pub round_rules: [i8; 6],
    /// `name`, none for a null pointer.
    pub names: [Option<Vec<u8>>; 2],
    /// `teamSeedList` (32 at most; the rest `{0, 0}`).
    pub seeds: Vec<[u8; 2]>,
}

/// What `InitDIYLeagueFromRecord` is given and reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentSetup {
    /// The tournament's state before.
    pub state: Box<[u8; STATE_LEN]>,
    /// The competition under way before.
    pub competition: Box<[u8; IMAGE_LEN]>,
    /// The record.
    pub record: TournamentBytes,
    /// Whether the record is in the national teams' list.
    pub national: bool,
    /// Whether the record is in the continental track's list.
    pub continental: bool,
    /// Whether the world played is one whose groups may be odd (`World_Current` answers World
    /// 2026-27), else the original's.
    pub odd_groups: bool,
    /// The session's pitch type (`_pitchType`).
    pub pitch_type: u16,
    /// The selected teams, by ordinal: the team numbers of a country other than 0xFF load the
    /// selected team of their ordinal.
    pub teams: Vec<SelectedTeam>,
    /// The career club: the team numbers of country 0xFF load it.
    pub career_team: SelectedTeam,
}

/// What `InitDIYLeagueFromRecord` is given and reads for a compiled record: upstream's own lists
/// as they are, the original's world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledTournamentSetup {
    /// The tournament's state before.
    pub state: Box<[u8; STATE_LEN]>,
    /// The competition under way before.
    pub competition: Box<[u8; IMAGE_LEN]>,
    /// The record.
    pub record: CompiledRecord,
    /// The session's pitch type (`_pitchType`).
    pub pitch_type: u16,
    /// The selected teams, by ordinal, as [`TournamentSetup::teams`].
    pub teams: Vec<SelectedTeam>,
    /// The career club.
    pub career_team: SelectedTeam,
}

/// A tournament started: its state and the competition under way after.
pub type Started = (Box<[u8; STATE_LEN]>, Box<[u8; IMAGE_LEN]>);

impl Oracle {
    /// `InitDIYLeagueFromRecord` of `setup`: the tournament's state and the competition under
    /// way after, or the reason code upstream halts with.
    ///
    /// # Errors
    ///
    /// Upstream's halt (0x1313: a stage's groups of an odd number of teams in the original's
    /// world).
    pub fn start_tournament(&mut self, setup: &TournamentSetup) -> Result<Started, u16> {
        ffi::tournament::start(setup)
    }

    /// `InitDIYLeagueFromRecord` of a compiled record (`setup`): as
    /// [`Oracle::start_tournament`]; `None` when upstream's tables have no such record.
    pub fn start_compiled_tournament(
        &mut self,
        setup: &CompiledTournamentSetup,
    ) -> Option<Result<Started, u16>> {
        ffi::tournament::start_compiled(setup)
    }

    /// `image` written into `_diyFileBufferCopy`, every field read back through the C struct.
    #[must_use]
    pub fn tournament_fields(&mut self, image: &[u8; STATE_LEN]) -> TournamentFields {
        ffi::tournament::fields(image)
    }

    /// `InitDIYTournament` from the state `before`: the state after.
    #[must_use]
    pub fn reset_tournament(&mut self, before: &[u8; STATE_LEN]) -> Box<[u8; STATE_LEN]> {
        ffi::tournament::reset(before)
    }

    /// `DIYTournament_RecalculateMatchesPerTeam` of `setup`: the tournament's state and the
    /// competition under way after.
    #[must_use]
    pub fn recalculate_tournament(
        &mut self,
        setup: &RecalculateSetup,
    ) -> (Box<[u8; STATE_LEN]>, Box<[u8; IMAGE_LEN]>) {
        ffi::tournament::recalculate(setup)
    }
}
