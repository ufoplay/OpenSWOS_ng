//! A cup started (docs/PLAN.md M5 #4c): a knockout's record, a cup's or a play-off's, made a
//! competition under way (upstream diy_competition.c `InitDIYCupFromRecord`, the type-1 branch
//! of `InitCompetitionFromRecord`, with `InitDIYCup`, `DIY_InitCupBracketState`,
//! `DIY_BuildTeamSelectionPointerCache`, `DIY_InitCupBracketProgressionTable` and
//! `DIY_ShuffleTeamGridPairs`).
//!
//! The texts and the record's classes come in resolved ([`CupStart`]): the cup's name, the
//! country's adjective that prefixes it, the name the reset writes first, whether its
//! suspensions are the continental track's. [`WorldKnockout`] resolves them from a world's
//! data.

mod world_knockout;

pub use world_knockout::WorldKnockout;

use openswos_competitions_json_resource::{CompetitionType, Cup, Playoff, RoundRules, SymbolId};
use openswos_rng::Rng;
use openswos_team_resource::TeamControls;
use thiserror::Error;

use super::draw::shuffle;
use super::running::{RunningCompetition, TeamBytes};
use super::start::{
    LoadedTeam, NIBBLE_MASK, NIBBLE_SHIFT, Pitch, copy_string, defaults as common, months,
    set_pitch, write_name,
};
use super::value::strength;

/// The round rule bytes a cup's record holds (upstream `CompetitionCup.roundRuleFlags[6]`).
const CUP_ROUND_RULES: usize = 6;

/// The round rule bytes a play-off's record holds (upstream `Playoffs.roundRuleFlags[2]`).
const PLAYOFF_ROUND_RULES: usize = 2;

/// The most teams a cup holds (upstream `cupTeamIds[64]`, `teamGridPairings[64]`).
const MAX_TEAMS: i8 = 64;

/// The places the reset numbers in order (`teamGridPairings[64]`).
const GRID_PLACES: u8 = 64;

/// The rounds whose rules the reset sets (`roundLegs[6]`, `roundExtraTime[6]`,
/// `roundPenalties[6]`).
const RULED_ROUNDS: usize = 6;

/// The round results the bracket's start clears (upstream clears 32 bytes of
/// `cupRoundResults[100]`).
const CLEARED_RESULTS: usize = 32;

/// How far a round rule byte's legs are shifted (its bits 7-6).
const LEGS_SHIFT: u8 = 6;

/// How far its extra time is shifted (bits 5-4).
const EXTRA_TIME_SHIFT: u8 = 4;

/// How far its penalties are shifted (bits 3-2).
const PENALTIES_SHIFT: u8 = 2;

/// A rule's two bits.
const RULE_MASK: u8 = 0b11;

/// The legs code of a round played home and away (`roundLegs`).
const TWO_LEGS: u16 = 2;

/// The penalties code of a single-leg round a draw settles on penalties (`roundPenalties`):
/// one without it keeps a date for a replay.
const PENALTIES: u16 = 1;

/// How far a replay's month is past the last match's, in eighths of a month.
const REPLAY_STEP: u8 = 2;

/// The cup reset's own defaults (upstream `InitDIYCup`; the ones a league's shares are
/// [`common`]'s).
mod defaults {
    /// A cup (`competitionType`, `DIY_CUP`).
    pub const COMPETITION_TYPE: i16 = 1;
    /// The slot's cache key (`compSlot`; upstream writes it and `slotId` as the word 1).
    pub const COMP_SLOT: u8 = 1;
    /// The sub-round counter.
    pub const SUB_ROUND_COUNTER: i16 = 1;
    /// The teams.
    pub const TEAMS: i16 = 16;
    /// The rounds.
    pub const ROUNDS: i16 = 4;
    /// The fewest substitutes.
    pub const MIN_SUBSTITUTES: u16 = 3;
    /// A win's points (where a cup's layout reads its bench's size).
    pub const POINTS_FOR_WIN: u16 = 5;
    /// Each round's legs, extra time and penalties.
    pub const ROUND_RULE: u16 = 1;
}

/// What the engine reads of a knockout's record, in upstream's cup layout (`CompCupHeader`
/// and its round rules): a cup's (`cups.json`, of type 1) or a play-off's (`playoffs.json`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CupRecord {
    /// Its competition id (`compId`).
    pub comp_id: u8,
    /// Its first month (`beginMonth`).
    pub begin_month: i8,
    /// Its last month (`endMonth`).
    pub end_month: i8,
    /// 0 when its draw is shuffled; otherwise its teams are paired in their order
    /// (`seedReorderFlag`).
    pub seed_reorder: i8,
    /// Nonzero: the rounds it states, set after the bracket's own (`numRoundsOverride`).
    pub num_rounds_override: i8,
    /// Its teams (`teams`).
    pub teams: i8,
    /// The away goals' rule (`awayGoalsMode`).
    pub away_goals_mode: i8,
    /// The substitutes in the high nibble, a win's points in the low one, where a cup's layout
    /// reads its bench's size (`subsAndPointsForWin`).
    pub substitutes: u8,
    /// The base of its rounds' captions (`roundStringBase`).
    pub round_string_base: i8,
    /// Each round's rules, as many bytes as its record holds (legs, extra time and penalties
    /// in bits 7-6, 5-4 and 3-2): 6 a cup's, 2 a play-off's, those its file leaves out 0.
    pub round_rules: Vec<u8>,
}

/// A record no cup is started from.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecordError {
    /// A competition that is no knockout: a league, a tournament, or a cup played as one.
    #[error("{0} is no knockout")]
    NotAKnockout(String),
    /// More round rules than upstream's record holds.
    #[error("{id} states {rules} round rules, more than the {room} its record holds")]
    TooManyRules {
        /// The record's id.
        id: String,
        /// The rules it states.
        rules: usize,
        /// The rules upstream's record holds.
        room: usize,
    },
}

impl CupRecord {
    /// The record of `cup`, a knockout (`cups.json`, of type 1).
    ///
    /// # Errors
    ///
    /// [`RecordError`] when `cup` is played as a league or a tournament, or states more round
    /// rules than its record holds.
    pub fn from_cup(cup: &Cup) -> Result<Self, RecordError> {
        knockout(&cup.id, cup.comp.kind)?;
        Ok(Self {
            comp_id: cup.comp.comp_id.byte(),
            begin_month: cup.comp.begin_month.byte().cast_signed(),
            end_month: cup.comp.end_month.byte().cast_signed(),
            seed_reorder: cup.comp.seed_reorder_flag.byte().cast_signed(),
            num_rounds_override: cup.num_rounds_override.byte().cast_signed(),
            teams: cup.teams.byte().cast_signed(),
            away_goals_mode: cup.away_goals_mode.byte().cast_signed(),
            substitutes: u8::from(cup.substitutes) << NIBBLE_SHIFT | u8::from(cup.points_for_win),
            round_string_base: cup.round_string_base.byte().cast_signed(),
            round_rules: rule_bytes(&cup.id, &cup.round_rule_flags, CUP_ROUND_RULES)?,
        })
    }

    /// The record of `playoff` (`playoffs.json`), which upstream starts as a cup: its bench's
    /// size is where a cup's layout reads a win's points.
    ///
    /// # Errors
    ///
    /// [`RecordError`] when `playoff` is not of a knockout's type, or states more round rules
    /// than its record holds.
    pub fn from_playoff(playoff: &Playoff) -> Result<Self, RecordError> {
        knockout(&playoff.id, playoff.comp.kind)?;
        Ok(Self {
            comp_id: playoff.comp.comp_id.byte(),
            begin_month: playoff.comp.begin_month.byte().cast_signed(),
            end_month: playoff.comp.end_month.byte().cast_signed(),
            seed_reorder: playoff.comp.seed_reorder_flag.byte().cast_signed(),
            num_rounds_override: playoff.num_rounds_override.byte().cast_signed(),
            teams: playoff.teams.byte().cast_signed(),
            away_goals_mode: playoff.away_goals_mode.byte().cast_signed(),
            substitutes: u8::from(playoff.substitutes) << NIBBLE_SHIFT | u8::from(playoff.max_subs),
            round_string_base: playoff.round_string_base.byte().cast_signed(),
            round_rules: rule_bytes(&playoff.id, &playoff.round_rule_flags, PLAYOFF_ROUND_RULES)?,
        })
    }
}

/// `Ok` for a record of a knockout's type (upstream `InitCompetitionFromRecord` starts type 1
/// as a cup).
fn knockout(id: &SymbolId, kind: CompetitionType) -> Result<(), RecordError> {
    match kind {
        CompetitionType::Cup => Ok(()),
        CompetitionType::League | CompetitionType::Tournament => {
            Err(RecordError::NotAKnockout(id.as_str().to_owned()))
        }
    }
}

/// The bytes of `rules` as a record of `room` rules holds them: those left out 0.
fn rule_bytes(id: &SymbolId, rules: &[RoundRules], room: usize) -> Result<Vec<u8>, RecordError> {
    if rules.len() > room {
        return Err(RecordError::TooManyRules {
            id: id.as_str().to_owned(),
            rules: rules.len(),
            room,
        });
    }
    let mut bytes: Vec<u8> = rules.iter().map(|&rules| u8::from(rules)).collect();
    bytes.resize(room, 0);
    Ok(bytes)
}

/// What a cup is started from: its record, and what the world's data resolves for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CupStart<'a> {
    /// The cup's record.
    pub record: &'a CupRecord,
    /// The cup's name (the record's first, else the default cup name).
    pub name: &'a [u8],
    /// The country's adjective before the name: none for a national teams' competition or a
    /// record of no country.
    pub prefix: Option<&'a [u8]>,
    /// The name the reset writes first (upstream `_aDiyCup`): what is left of it past the name
    /// stays in the buffer.
    pub default_name: &'a [u8],
    /// Whether its suspensions are the continental track's (upstream `continental_record`).
    pub continental: bool,
    /// The session's pitch.
    pub pitch: Pitch,
}

/// Why a cup does not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StartError {
    /// An odd number of teams (upstream halts, 0x1313).
    #[error("a cup of {0} teams (upstream halts, 0x1313)")]
    OddTeams(i8),
    /// Fewer than two teams (upstream's months run on past its memory, or a negative count
    /// reads its record's names as round rules).
    #[error("a cup of {0} teams")]
    TooFewTeams(i8),
    /// More teams than a cup holds (upstream reads its pairings past their 64 places).
    #[error("a cup of {0} teams, more than the 64 it holds")]
    TooManyTeams(i8),
    /// A bracket of more rounds than the record has rules for (upstream reads on into its
    /// names).
    #[error("a bracket of {rounds} rounds, past the record's {rules} round rules")]
    TooFewRules {
        /// The bracket's rounds.
        rounds: usize,
        /// The record's rules.
        rules: usize,
    },
    /// A team of the cup that is not loaded.
    #[error("team {0:?} is not loaded")]
    UnknownTeam(TeamBytes),
}

/// Starts `start`'s cup in `competition` (upstream `InitDIYCupFromRecord`): the reset, the
/// record's fields and each of its bracket's rounds' rules, the name, the bracket from the
/// cup's teams (its `cup_team` slots, set before, found by `teams`), the record's rounds, then
/// the pairings shuffled by `rng` unless the record keeps its order. Upstream then hands the
/// cup to a world's components: World 2026-27's national cups (`WorldCups_CupMade`: their byes,
/// a later season's draw) join with their data (M5 #9), and a classic world's European cups
/// (`WorldEurope_DrawMade`: no tie of one country's clubs) with Europe 1984-85 (D10).
///
/// # Errors
///
/// [`StartError`] when the cup's teams are odd, fewer than two or more than it holds, its
/// bracket has rounds its record has no rules for, or a team is not loaded.
pub fn start_cup(
    competition: &mut RunningCompetition,
    start: &CupStart<'_>,
    teams: impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), StartError> {
    let record = start.record;
    let rounds = bracket_rounds(record.teams)?;
    if rounds > record.round_rules.len() {
        return Err(StartError::TooFewRules {
            rounds,
            rules: record.round_rules.len(),
        });
    }
    reset(competition, start.default_name);
    competition.set_substitutes_count(i16::from(!start.continental));
    competition.set_comp_id(record.comp_id);
    competition.set_sub_round_counter(i16::from(record.round_string_base));
    competition.set_start_month(record.begin_month);
    competition.set_end_month(record.end_month);
    set_pitch(competition, start.pitch);
    competition.set_num_teams(i16::from(record.teams));
    write_name(competition, start.prefix, start.name);
    competition.set_play_each_team(i16::from(record.away_goals_mode));
    competition.set_min_substitutes(u16::from(record.substitutes >> NIBBLE_SHIFT));
    competition.set_points_for_win(u16::from(record.substitutes & NIBBLE_MASK));
    for (round, &rules) in record.round_rules.iter().take(rounds).enumerate() {
        let rule = |shift: u8| u16::from(rules >> shift & RULE_MASK);
        competition.set_cup_round_legs(round, rule(LEGS_SHIFT));
        competition.set_cup_round_extra_time(round, rule(EXTRA_TIME_SHIFT));
        competition.set_cup_round_penalties(round, rule(PENALTIES_SHIFT));
    }
    bracket(competition, &teams)?;
    if record.num_rounds_override != 0 {
        competition.set_num_rounds(i16::from(record.num_rounds_override));
    }
    if record.seed_reorder == 0 {
        shuffle_pairings(competition, rng);
    }
    Ok(())
}

/// The rounds of a bracket of `teams`, as upstream counts them (the bits of half the teams),
/// once the teams are a cup's.
fn bracket_rounds(teams: i8) -> Result<usize, StartError> {
    if teams & 1 != 0 {
        return Err(StartError::OddTeams(teams));
    }
    if teams < 2 {
        return Err(StartError::TooFewTeams(teams));
    }
    if teams > MAX_TEAMS {
        return Err(StartError::TooManyTeams(teams));
    }
    Ok(bits(u16::from(teams.cast_unsigned()) >> 1))
}

/// The bits of `value` up to its highest set one.
fn bits(value: u16) -> usize {
    usize::try_from(u16::BITS - value.leading_zeros()).expect("16 bits at most")
}

/// The reset (upstream `InitDIYCup`): a DIY cup's defaults, the name `default_name`, every
/// round's rules 1, the pairings' places in order.
pub(super) fn reset(competition: &mut RunningCompetition, default_name: &[u8]) {
    competition.set_competition_type(defaults::COMPETITION_TYPE);
    competition.set_comp_slot(defaults::COMP_SLOT);
    competition.set_slot_id(0);
    competition.set_menu_state(0);
    competition.set_sub_round_counter(defaults::SUB_ROUND_COUNTER);
    competition.set_is_active(0);
    competition.set_match_started(0);
    competition.set_career_team_involved(0);
    competition.set_result_code(common::RESULT_CODE);
    competition.set_substitutes_count(1);
    competition.set_slot_state_a(common::SLOT_STATE_A);
    competition.set_slot_state_b(common::SLOT_STATE_B);
    competition.set_slot_state_c(common::SLOT_STATE_C);
    competition.set_comp_id(common::COMP_ID);
    competition.set_has_groups(0);
    copy_string(competition, default_name);
    competition.set_start_month(common::START_MONTH);
    competition.set_end_month(common::END_MONTH);
    competition.set_pitch_type_or_season(1);
    competition.set_pitch_type(common::PITCH_TYPE);
    competition.set_num_teams(defaults::TEAMS);
    competition.set_num_rounds(defaults::ROUNDS);
    competition.set_play_each_team(0);
    competition.set_min_substitutes(defaults::MIN_SUBSTITUTES);
    competition.set_points_for_win(defaults::POINTS_FOR_WIN);
    competition.set_cup_current_match_index(0);
    for round in 0..RULED_ROUNDS {
        competition.set_cup_round_legs(round, defaults::ROUND_RULE);
        competition.set_cup_round_extra_time(round, defaults::ROUND_RULE);
        competition.set_cup_round_penalties(round, defaults::ROUND_RULE);
    }
    for place in 0..GRID_PLACES {
        competition.set_grid_pairing(usize::from(place), place);
    }
}

/// The bracket's state (upstream `DIY_InitCupBracketState`): each team's identity and the
/// teams no computer controls, the pairs and rounds of half the teams, the first round's
/// format, the counters and results cleared, the month, the matches' months, the teams'
/// strength. Upstream also hands the month to the session (`_viewRandomPhase`), whose it is
/// (M7).
pub(super) fn bracket(
    competition: &mut RunningCompetition,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
) -> Result<(), StartError> {
    identities(competition, teams)?;
    let num_teams = competition.num_teams();
    let pairs = num_teams.cast_unsigned() >> 1;
    competition.set_match_pairs_per_round(pairs.cast_signed());
    competition.set_total_matches_in_round(pairs.cast_signed());
    competition.set_num_rounds(i16::try_from(bits(pairs)).expect("16 rounds at most"));
    competition.set_round_complete(0);
    competition.set_current_round_index(0);
    competition.set_is_active(1);
    competition.set_competition_finished(0);
    competition.set_competition_format(0);
    // The first round's (the round index just cleared); a final of one leg plays format 0.
    let mut format = competition.cup_round_legs(0);
    if pairs == 1 && format != TWO_LEGS {
        format = 0;
    }
    competition.set_round_format(format);
    competition.set_group_section_flag(0);
    competition.set_second_leg_counter(0);
    competition.set_cur_month_idx(0);
    competition.set_match_count(0);
    for result in 0..CLEARED_RESULTS {
        competition.set_cup_round_result(result, 0);
    }
    competition.set_view_random_phase(competition.start_month().cast_unsigned());
    match_months(competition);
    let count = usize::from(num_teams.cast_unsigned());
    let prices = (0..count)
        .map(|team| {
            let team = competition.cup_team(team);
            teams(team)
                .map(|loaded| loaded.prices)
                .ok_or(StartError::UnknownTeam(team))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let strength = strength(&prices).expect("a cup has two teams at least (bracket_rounds)");
    competition.set_avg_skill(strength.average.to_le_bytes()[0]);
    competition.set_strength_tier(strength.tier);
    Ok(())
}

/// Each team's identity, by its place (upstream `DIY_BuildTeamSelectionPointerCache`), and
/// the teams the computer does not control, counted in the round iterator. The pairings are
/// the reset's places in order, so each place's team is the cup's team of its index.
fn identities(
    competition: &mut RunningCompetition,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
) -> Result<(), StartError> {
    let count = usize::from(competition.num_teams().cast_unsigned());
    let mut not_computer: i16 = 0;
    for place in 0..count {
        let index = usize::from(competition.grid_pairing(place));
        let team = competition.cup_team(index);
        let loaded = teams(team).ok_or(StartError::UnknownTeam(team))?;
        competition.set_team_offset(index, loaded.identity);
        if loaded.controls != TeamControls::Computer {
            not_computer += 1;
        }
    }
    competition.set_round_iterator(not_computer);
    Ok(())
}

/// The matches' months (upstream `DIY_InitCupBracketProgressionTable`): two a round, but one a
/// single-leg round settled on penalties; the matches, a replay's date aside, from the first
/// month to the last; the replay's date after them. Faithful to the original: whether a
/// replay's date is kept is the lowest single-leg round's (the loop runs from the last round
/// down), though upstream names it the last round's.
fn match_months(competition: &mut RunningCompetition) {
    let rounds = usize::from(competition.num_rounds().cast_unsigned());
    let mut total: u16 = 0;
    let mut replay = false;
    for round in (0..rounds).rev() {
        total += 2;
        if competition.cup_round_legs(round) != TWO_LEGS {
            replay = true;
            if competition.cup_round_penalties(round) == PENALTIES {
                replay = false;
                total -= 1;
            }
        }
    }
    competition.set_total_matches(total);
    let count = total - u16::from(replay);
    let months = months(
        competition.start_month().cast_unsigned(),
        competition.end_month().cast_unsigned(),
        count,
    );
    for (index, month) in months.into_iter().enumerate() {
        competition.set_prog_table(index, month);
    }
    if replay {
        let last = usize::from(count) - 1;
        let month = competition.prog_table(last).wrapping_add(REPLAY_STEP);
        competition.set_prog_table(last + 1, month);
    }
}

/// The pairings shuffled (upstream `DIY_ShuffleTeamGridPairs`): the places of the first round's
/// pairs.
fn shuffle_pairings(competition: &mut RunningCompetition, rng: &mut Rng) {
    let places = usize::from(
        competition
            .match_pairs_per_round()
            .cast_unsigned()
            .wrapping_mul(2),
    );
    let mut pairings: Vec<u8> = (0..places)
        .map(|place| competition.grid_pairing(place))
        .collect();
    shuffle(&mut pairings, rng);
    for (place, index) in pairings.into_iter().enumerate() {
        competition.set_grid_pairing(place, index);
    }
}

#[cfg(test)]
mod tests {
    use openswos_common::Importer;
    use openswos_competitions_json_resource::{CompetitionsFile, CompetitionsJsonFormat};

    use super::*;

    /// Two legs, extra time and penalties (`{"legs": 2, "extra_time": 1, "penalties": 1}`).
    const TWO_LEGGED: u8 = 0x94;

    /// One leg, extra time and penalties.
    const ONE_LEG: u8 = 0x54;

    /// One leg, extra time, no penalties: a replay.
    const REPLAYED: u8 = 0x50;

    /// A cup of `teams`, its draw in order: two legs a round but a one-leg final.
    fn record(teams: i8) -> CupRecord {
        CupRecord {
            comp_id: 109,
            begin_month: 0x40,
            end_month: 0x20,
            seed_reorder: 1,
            num_rounds_override: 0,
            teams,
            away_goals_mode: 1,
            substitutes: 0x35,
            round_string_base: 3,
            round_rules: vec![TWO_LEGGED, ONE_LEG, 0, 0, 0, 0],
        }
    }

    fn start(record: &CupRecord) -> CupStart<'_> {
        CupStart {
            record,
            name: b"FA CUP",
            prefix: Some(b"ENGLISH"),
            default_name: b"DIY CUP",
            continental: false,
            pitch: Pitch::Seasonal,
        }
    }

    /// Each team `[0, n]` the selected team `n`, priced `n` throughout, the computer's but for
    /// team 1.
    fn teams(team: TeamBytes) -> Option<LoadedTeam> {
        let [_, ordinal] = team;
        Some(LoadedTeam {
            identity: i32::from(ordinal),
            controls: if ordinal == 1 {
                TeamControls::PlayerCoach
            } else {
                TeamControls::Computer
            },
            prices: [ordinal; 16],
        })
    }

    /// A competition whose cup slots hold teams `[0, 0]` to `[0, 7]`.
    fn competition() -> RunningCompetition {
        let mut competition = RunningCompetition::default();
        for ordinal in 0..8 {
            competition.set_cup_team(usize::from(ordinal), [0, ordinal]);
        }
        competition
    }

    fn started(start: &CupStart<'_>, rng: &mut Rng) -> RunningCompetition {
        let mut competition = competition();
        start_cup(&mut competition, start, teams, rng).expect("started");
        competition
    }

    #[test]
    fn a_cup_record_is_its_bytes() {
        let text = r#"{"type": "competition_cups", "count": 3, "competitions": [
            {"id": "_cup", "type": "cup", "comp": {"comp_id": 109, "type": 1, "team_number": 0, "begin_month": 88, "end_month": 32, "name_table_offset": 0, "reserved": 0, "team_list_offset": 0, "seed_reorder_flag": 0},
             "num_rounds_override": 2, "teams": 16, "away_goals_mode": 1, "substitutes": 3, "points_for_win": 5, "round_string_base": 1,
             "round_rule_flags": [{"legs": 2, "extra_time": 1, "penalties": 1}], "name": [], "team_seed_list": []},
            {"id": "_group", "type": "cup", "comp": {"comp_id": 5, "type": 0, "team_number": -1, "begin_month": 64, "end_month": 32, "name_table_offset": 15, "reserved": 0, "team_list_offset": 21, "seed_reorder_flag": 0},
             "num_rounds_override": 1, "teams": 2, "away_goals_mode": 3, "substitutes": 3, "points_for_win": 5, "round_string_base": 6,
             "round_rule_flags": [], "name": [], "team_seed_list": []},
            {"id": "_long", "type": "cup", "comp": {"comp_id": 110, "type": 1, "team_number": 0, "begin_month": 88, "end_month": 32, "name_table_offset": 0, "reserved": 0, "team_list_offset": 0, "seed_reorder_flag": 0},
             "num_rounds_override": 0, "teams": 2, "away_goals_mode": 1, "substitutes": 3, "points_for_win": 5, "round_string_base": 1,
             "round_rule_flags": [{"legs": 1, "extra_time": 1, "penalties": 1}, {"legs": 1, "extra_time": 1, "penalties": 1}, {"legs": 1, "extra_time": 1, "penalties": 1},
                                  {"legs": 1, "extra_time": 1, "penalties": 1}, {"legs": 1, "extra_time": 1, "penalties": 1}, {"legs": 1, "extra_time": 1, "penalties": 1},
                                  {"legs": 1, "extra_time": 1, "penalties": 1}], "name": [], "team_seed_list": []}]}"#;
        let CompetitionsFile::Cups(file) = CompetitionsJsonFormat::import(text.as_bytes())
            .expect("cups")
            .file
        else {
            panic!("cups");
        };
        let cup = CupRecord::from_cup(&file.competitions[0]).expect("a knockout");
        assert_eq!(
            (
                cup.comp_id,
                cup.teams,
                cup.num_rounds_override,
                cup.seed_reorder
            ),
            (109, 16, 2, 0)
        );
        assert_eq!((cup.substitutes, cup.round_string_base), (0x35, 1));
        assert_eq!(
            cup.round_rules,
            [TWO_LEGGED, 0, 0, 0, 0, 0],
            "six, the rest 0"
        );
        assert_eq!(
            CupRecord::from_cup(&file.competitions[1]),
            Err(RecordError::NotAKnockout("_group".to_owned()))
        );
        let error = CupRecord::from_cup(&file.competitions[2]).expect_err("seven rules");
        assert_eq!(
            error.to_string(),
            "_long states 7 round rules, more than the 6 its record holds"
        );
    }

    #[test]
    fn a_playoff_record_is_read_through_the_cups_layout() {
        let text = r#"{"type": "playoffs", "count": 1, "playoffs": [
            {"id": "_div1Playoffs", "comp": {"comp_id": 47, "type": 1, "team_number": 8, "begin_month": -128, "end_month": -124, "name_table_offset": 11, "reserved": 1, "team_list_offset": 17, "seed_reorder_flag": 1},
             "num_rounds_override": 1, "teams": 4, "away_goals_mode": 1, "substitutes": 3, "max_subs": 5, "round_string_base": 1,
             "round_rule_flags": [{"legs": 2, "extra_time": 1, "penalties": 1}], "name": ["_aDiv1Playoffs", "_aPlayoffs"]}]}"#;
        let CompetitionsFile::Playoffs(file) = CompetitionsJsonFormat::import(text.as_bytes())
            .expect("play-offs")
            .file
        else {
            panic!("play-offs");
        };
        let playoff = CupRecord::from_playoff(&file.playoffs[0]).expect("a knockout");
        assert_eq!(
            (playoff.begin_month, playoff.end_month, playoff.seed_reorder),
            (-128, -124, 1)
        );
        assert_eq!((playoff.teams, playoff.num_rounds_override), (4, 1));
        assert_eq!(
            playoff.substitutes, 0x35,
            "its bench where a cup's points are"
        );
        assert_eq!(playoff.round_rules, [TWO_LEGGED, 0]);
    }

    #[test]
    fn a_cup_starts_with_its_record_s_fields() {
        let record = record(4);
        let competition = started(&start(&record), &mut Rng::default());
        assert_eq!(competition.competition_type(), 1);
        assert_eq!((competition.comp_id(), competition.num_teams()), (109, 4));
        assert_eq!(competition.substitutes_count(), 1, "the domestic track");
        assert_eq!(competition.sub_round_counter(), 3, "the captions' base");
        assert_eq!(
            (competition.start_month(), competition.end_month()),
            (0x40, 0x20)
        );
        assert_eq!(competition.play_each_team(), 1, "the away goals' rule");
        assert_eq!(
            (competition.min_substitutes(), competition.points_for_win()),
            (3, 5)
        );
        assert_eq!(&competition.name()[..15], b"ENGLISH FA CUP\0");
        // Two rounds: two legs, then one with penalties; the rest the reset's 1s.
        let rules: Vec<(u16, u16, u16)> = (0..RULED_ROUNDS)
            .map(|round| {
                (
                    competition.cup_round_legs(round),
                    competition.cup_round_extra_time(round),
                    competition.cup_round_penalties(round),
                )
            })
            .collect();
        assert_eq!(
            rules,
            [
                (2, 1, 1),
                (1, 1, 1),
                (1, 1, 1),
                (1, 1, 1),
                (1, 1, 1),
                (1, 1, 1)
            ]
        );
        assert_eq!(
            (
                competition.match_pairs_per_round(),
                competition.total_matches_in_round(),
                competition.num_rounds()
            ),
            (2, 2, 2)
        );
        assert_eq!(competition.round_format(), 2, "the first round's legs");
        assert_eq!(competition.is_active(), 1);
        assert_eq!(competition.view_random_phase(), 0x40);
        // Two legs and a match: three months from 0x40 to 0x80 (0x20 wrapped).
        assert_eq!(competition.total_matches(), 3);
        let months: Vec<u8> = (0..3)
            .map(|match_| competition.prog_table(match_))
            .collect();
        assert_eq!(months, [0x40, 0x60, 0x80]);
        let identities: Vec<i32> = (0..4).map(|place| competition.team_offset(place)).collect();
        assert_eq!(identities, [0, 1, 2, 3]);
        assert_eq!(competition.round_iterator(), 1, "team 1 is a human's");
        // Prices 0 to 3: the average 1, tier 0.
        assert_eq!(
            (competition.avg_skill(), competition.strength_tier()),
            (1, 0)
        );
        let pairings: Vec<u8> = (0..8)
            .map(|place| competition.grid_pairing(place))
            .collect();
        assert_eq!(pairings, [0, 1, 2, 3, 4, 5, 6, 7], "kept in order");
    }

    #[test]
    fn the_pairings_are_shuffled_unless_the_record_keeps_them() {
        let mut record = record(8);
        record.round_rules[2] = ONE_LEG;
        record.seed_reorder = 0;
        let mut rng = Rng::default();
        let competition = started(&start(&record), &mut rng);
        let mut drawn = Rng::default();
        let mut expected: Vec<u8> = (0..8).collect();
        shuffle(&mut expected, &mut drawn);
        let pairings: Vec<u8> = (0..8)
            .map(|place| competition.grid_pairing(place))
            .collect();
        assert_eq!(pairings, expected);
        assert_eq!(rng, drawn, "eight draws");
        assert_eq!(
            competition.grid_pairing(8),
            8,
            "past the first round's places"
        );
    }

    #[test]
    fn a_single_leg_round_without_penalties_keeps_a_replays_date() {
        let mut record = record(4);
        record.round_rules = vec![REPLAYED, TWO_LEGGED, 0, 0, 0, 0];
        record.end_month = 0x50;
        record.num_rounds_override = 3;
        let competition = started(&start(&record), &mut Rng::default());
        // A match and its replay, two legs: four, the replay's date aside three months.
        assert_eq!(competition.total_matches(), 4);
        let months: Vec<u8> = (0..4)
            .map(|match_| competition.prog_table(match_))
            .collect();
        assert_eq!(months, [0x40, 0x48, 0x50, 0x52]);
        assert_eq!(
            competition.num_rounds(),
            3,
            "the record's, after the bracket's"
        );
    }

    #[test]
    fn a_final_of_one_leg_plays_format_0() {
        let mut record = record(2);
        record.round_rules = vec![ONE_LEG, 0, 0, 0, 0, 0];
        let competition = started(&start(&record), &mut Rng::default());
        assert_eq!(competition.round_format(), 0);
        assert_eq!(competition.total_matches(), 1);
        assert_eq!(competition.prog_table(0), 0x40, "one match, at the start");
    }

    #[test]
    fn a_continental_contest_and_a_fixed_pitch_are_set() {
        let record = record(2);
        let mut start = start(&record);
        start.continental = true;
        start.pitch = Pitch::Type(3);
        start.prefix = None;
        let competition = started(&start, &mut Rng::default());
        assert_eq!(competition.substitutes_count(), 0);
        assert_eq!(
            (competition.pitch_type_or_season(), competition.pitch_type()),
            (0, 3)
        );
        assert_eq!(
            &competition.name()[..8],
            b"FA CUP\0\0",
            "DIY CUP's tail gone"
        );
    }

    #[test]
    fn what_cannot_start_is_refused() {
        let refused = |record: &CupRecord| {
            let mut competition = competition();
            start_cup(&mut competition, &start(record), teams, &mut Rng::default())
                .expect_err("refused")
        };
        assert_eq!(refused(&record(5)), StartError::OddTeams(5));
        assert_eq!(refused(&record(-3)), StartError::OddTeams(-3));
        assert_eq!(refused(&record(0)), StartError::TooFewTeams(0));
        assert_eq!(refused(&record(-2)), StartError::TooFewTeams(-2));
        assert_eq!(refused(&record(66)), StartError::TooManyTeams(66));
        let mut playoff = record(8);
        playoff.round_rules.truncate(PLAYOFF_ROUND_RULES);
        assert_eq!(
            refused(&playoff),
            StartError::TooFewRules {
                rounds: 3,
                rules: 2
            }
        );
        let record = record(4);
        let error = start_cup(
            &mut competition(),
            &start(&record),
            |_| None,
            &mut Rng::default(),
        )
        .expect_err("no teams");
        assert_eq!(error, StartError::UnknownTeam([0, 0]));
    }

    #[test]
    fn a_bracket_has_the_bits_of_half_its_teams_as_rounds() {
        assert_eq!(bracket_rounds(2), Ok(1));
        assert_eq!(bracket_rounds(6), Ok(2), "three ties: two rounds");
        assert_eq!(bracket_rounds(16), Ok(4));
        assert_eq!(bracket_rounds(64), Ok(6));
    }
}
