//! A team in a match (upstream src/match_sim.c, M5 #8a, #8b): a player's price for his place
//! (`GetPlayerPrice`, `CountNearnessFactor`), his match value
//! (`UpdatePlayerPositionAndReturnRole`), whether he is available (`IsPlayerInjured`), the
//! team's strength (`ComputeOldPlayerSkillForViewResult`), its standing in its league
//! (`CalcTeamPriceRankPercentOffset`), its form (`CalcTeamViewOffset`), its available players
//! and goalkeepers (`GetNumberOfAvailablePlayers`, `GetNumberOfAvailableSubstitutes`), the status
//! buckets a match gives its players (`InitPlayerRemapTableForTeam`), each from an image of the
//! team's record: a selected team's in `_selectedTeams[0]`, the career club's in `_careerTeam`
//! with its line-up (`_currentMatchPlayers`, `_careerNumPlayers`). The session's tactics are the
//! setup's, each slot of `_teamTacticsTable` pointed at its record for the call. A status
//! bucket's draws come from the second generator: set it with [`Oracle::set_rand2_state`] first.

use crate::Oracle;
use crate::cup::SelectedTeam;
use crate::ffi;
use crate::running::IMAGE_LEN;

/// The bytes of a tactics record (`sizeof(TeamTactics)`).
pub const TACTICS_LEN: usize = 370;

/// The slots of the session's tactics table (`_teamTacticsTable[19]`).
pub const TACTICS_SLOTS: usize = 19;

/// The places of the career club's line-up (`_currentMatchPlayers[32]`).
pub const CAREER_LINE_UP: usize = 32;

/// What a player's availability is checked by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// The mode (`_inEditTactics`).
    pub mode: u16,
    /// A training game (`_trainingGame`).
    pub training: bool,
    /// The testers' rule (`TestRules_Force`).
    pub testers_rule: bool,
}

/// A team and its session: the record, the career club's line-up, the tactics, the status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamSetup {
    /// The record: a selected team's 684 bytes, the career club's 1,292.
    pub team: Vec<u8>,
    /// The career club's line-up and players: `Some` for the career club.
    pub career: Option<([u8; CAREER_LINE_UP], u16)>,
    /// The session's tactics, each slot's record.
    pub tactics: Vec<[u8; TACTICS_LEN]>,
    /// The status.
    pub status: Status,
}

/// What a call leaves: its answer and the team's record after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reckoned<T> {
    /// The answer.
    pub answer: T,
    /// The record after.
    pub team: Vec<u8>,
}

/// A player's record's fields, as the C reads them (`PlayerFile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerFields {
    /// `nationality`, `cachedMatchValue`, `shirtNumber`, `price`, `form`.
    pub signed: [i8; 5],
    /// `positionAndFace`, `cardsInjuries`.
    pub flags: [u8; 2],
    /// `passing`, `velocityHeading`, `tacklingControl`, `speedFinishing`.
    pub skills: [i8; 4],
    /// `goalStats`.
    pub goals: [u8; 4],
}

/// A team record's fields, as the C reads them (`TeamFileEx`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamFields {
    /// `countryNumber`, `teamOrdinal`, `teamControls`, `writesZeroHere`, `andWith0xFE`,
    /// `tactics`, `league`, `playerValuesDirty`.
    pub bytes: [u8; 8],
    /// `globalTeamNumber`.
    pub global_number: i16,
    /// `playerNumbers`.
    pub line_up: [u8; 16],
    /// `players`.
    pub players: Vec<PlayerFields>,
}

/// What a team's rank in its league reads: the competition, the team's record, the teams'
/// prices (as the stubbed loader gives them), whether the career's manager selects a national
/// team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankSetup {
    /// The competition under way (`_diyCompetitionStart`).
    pub competition: Box<[u8; IMAGE_LEN]>,
    /// The team's record.
    pub team: Vec<u8>,
    /// The selected teams, by ordinal.
    pub teams: Vec<SelectedTeam>,
    /// The career club.
    pub career_team: SelectedTeam,
    /// `_isNationalSelector`.
    pub national_selector: bool,
}

/// The day a team's form is drawn on (`_currentSeasonShowing`, `_newRandomSeedCopy`,
/// `_viewRandomPhase`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayGlobals {
    /// `_currentSeasonShowing`.
    pub season: u16,
    /// `_newRandomSeedCopy`.
    pub seed: u8,
    /// `_viewRandomPhase`.
    pub phase: u8,
}

/// What a match's status buckets read besides the team.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketSetup {
    /// The stand-ins' table, or none (a null pointer).
    pub stand_ins: Option<[u8; CAREER_LINE_UP]>,
    /// `_gameType`: 1 a DIY competition, 3 a season, 4 a career.
    pub game_type: u16,
    /// Whether a computer club's records keep its cards, its injuries (`WorldRules_CpuOpen`).
    pub computer: (bool, bool),
}

/// The engine's tables as upstream compiles them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceTables {
    /// `_skillsPerQuadrant`.
    pub position_weights: Vec<u8>,
    /// `_quadrantDistanceFactor`.
    pub area_distances: Vec<u8>,
    /// `_priceSkillPonders`.
    pub area_ponders: Vec<u32>,
    /// `_quadrantCoordinatesToBallQuadrant`.
    pub spot_areas: Vec<i8>,
    /// `_skillValuePonders`.
    pub skill_ponders: Vec<u8>,
    /// `_plPricesIncreaseTable`.
    pub price_rises: Vec<u8>,
}

impl Oracle {
    /// `GetPlayerPrice` of record `slot` at line-up place `number` of `setup`'s team.
    #[must_use]
    pub fn player_price(&mut self, setup: &TeamSetup, slot: u16, number: u16) -> Reckoned<i16> {
        ffi::sim::player_price(setup, slot, number)
    }

    /// `UpdatePlayerPositionAndReturnRole` of line-up place `place` of `setup`'s team: its role
    /// (high word) and value (low word).
    #[must_use]
    pub fn match_value(&mut self, setup: &TeamSetup, place: u16) -> Reckoned<u32> {
        ffi::sim::match_value(setup, place)
    }

    /// `ComputeOldPlayerSkillForViewResult` of `setup`'s team, `substitutes` counted: the
    /// strength and `_teamSkillCurrent` after.
    #[must_use]
    pub fn team_strength(&mut self, setup: &TeamSetup, substitutes: u16) -> Reckoned<(u16, u16)> {
        ffi::sim::strength(setup, substitutes)
    }

    /// `CountNearnessFactor(area, ball, price)` of a player whose 35 spots are `spots`.
    #[must_use]
    pub fn nearness(&mut self, area: u16, ball: u16, price: u16, spots: &[u8; 35]) -> u16 {
        ffi::sim::nearness(area, ball, price, spots)
    }

    /// `IsPlayerInjured` of a player of `cards_injuries` under `status`.
    #[must_use]
    pub fn injured(&mut self, cards_injuries: u8, status: Status) -> u16 {
        ffi::sim::injured(cards_injuries, status)
    }

    /// A selected team's record `image`, every field read back through the C struct.
    #[must_use]
    pub fn team_fields(&mut self, image: &[u8; 684]) -> TeamFields {
        ffi::sim::team_fields(image)
    }

    /// `CalcTeamPriceRankPercentOffset` of `setup`'s team in its competition, or the reason
    /// code upstream halts with.
    ///
    /// # Errors
    ///
    /// Upstream's halt (0x6006: the team is not in its league).
    pub fn price_rank(&mut self, setup: &RankSetup) -> Result<i16, u16> {
        ffi::sim::price_rank(setup)
    }

    /// `CalcTeamViewOffset` of the selected team `record` on `day`: its first half's form (high
    /// word) and its second's (low word).
    #[must_use]
    pub fn form(&mut self, record: &[u8; 684], day: DayGlobals) -> u32 {
        ffi::sim::form(record, day)
    }

    /// `GetNumberOfAvailablePlayers(team, mode, places)` of `setup`'s team (its status's
    /// training game and testers' rule): those out (high word) and those available (low word).
    #[must_use]
    pub fn available_players(&mut self, setup: &TeamSetup, mode: i16, places: u16) -> u32 {
        ffi::sim::available_players(setup, mode, places)
    }

    /// `GetNumberOfAvailableSubstitutes(team, mode)` of `setup`'s team: the goalkeepers out
    /// (high word) and available (low word).
    #[must_use]
    pub fn available_keepers(&mut self, setup: &TeamSetup, mode: i16) -> u32 {
        ffi::sim::available_keepers(setup, mode)
    }

    /// `InitPlayerRemapTableForTeam` of `setup`'s team and `buckets`: the record after.
    #[must_use]
    pub fn status_buckets(&mut self, setup: &TeamSetup, buckets: &BucketSetup) -> Vec<u8> {
        ffi::sim::status_buckets(setup, buckets)
    }

    /// The session's tactics as compiled: each slot's record of `_teamTacticsTable`.
    #[must_use]
    pub fn compiled_tactics(&mut self) -> Vec<[u8; TACTICS_LEN]> {
        ffi::sim::compiled_tactics()
    }

    /// The engine's tables, as compiled.
    #[must_use]
    pub fn price_tables(&mut self) -> PriceTables {
        ffi::sim::price_tables()
    }
}
