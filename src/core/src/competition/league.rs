//! A league started (docs/PLAN.md M5 #4b): a division of a league record made a competition
//! under way (upstream diy_competition.c `InitDIYCompetitionFromTableEntry`, with
//! `DIY_InitCompetitionDefaults`, `DIY_RecomputeCompetitionDerivedFields`,
//! `DIY_CacheSelectedTeamPointers`, `DIY_RecomputeCompetitionDerivedProgressionTable` and
//! `DIY_ShuffleLeagueTeamOrder`).
//!
//! The texts and the record's classes come in resolved ([`DivisionStart`]): the division's name,
//! the country's adjective that prefixes it, the default name the reset writes first, whether
//! its suspensions are the continental track's. Where the world's data states them is the next
//! step's (M5 #4b2).

use openswos_competitions_json_resource::{CompetitionType, Cup, League};
use openswos_rng::Rng;
use thiserror::Error;

use super::draw::shuffle;
use super::running::{NAME_LEN, RunningCompetition, STANDINGS_ROWS, StandingsRow, TeamBytes};
use super::schedule::Schedules;
use super::table::Line;
use super::value::{Prices, strength};

/// The bytes of a division in a league record (upstream `Division`: its clubs, promoted,
/// promoted through play-offs, relegated, relegated through play-offs, its qualifier link).
const DIVISION_BYTES: usize = 6;

/// The places the reset lays out in order (`shuffleAndPad[64]`, `teamEntryOffsets[64]`).
const PLACES: usize = 64;

/// The bytes of a standings row: a place's entry offset counts in them.
const ROW_BYTES: u16 = 18;

/// The longest name the name buffer takes, its end aside (`copyCompetitionName`).
const NAME_ROOM: usize = NAME_LEN - 1;

/// The bytes of upstream's name scratch (`copyCompetitionName`'s `tmp[96]`): the adjective,
/// the space and the name, its end aside, fit in them.
const NAME_SCRATCH: usize = 96;

/// How far a substitutes byte's high nibble (the fewest) is shifted.
const NIBBLE_SHIFT: u8 = 4;

/// A substitutes byte's low nibble (the most).
const NIBBLE_MASK: u8 = 0x0F;

/// The most match days whose months the image holds: from the months' table to its end.
const MAX_MATCH_DAYS: usize = super::running::IMAGE_LEN - super::running::PHASE_MONTHS;

/// The months the season's progression wraps by when it ends before it starts (upstream
/// `DIY_RecomputeCompetitionDerivedProgressionTable`: twelve months of eight steps).
const MONTHS_WRAP: u8 = 0x60;

/// The reset's defaults (upstream `DIY_InitCompetitionDefaults`).
mod defaults {
    /// No result yet (`resultCode`).
    pub const RESULT_CODE: i16 = -1;
    /// Slot state A (a menu ordinal).
    pub const SLOT_STATE_A: i16 = 3;
    /// Slot state B.
    pub const SLOT_STATE_B: i16 = 2;
    /// Slot state C (a menu ordinal).
    pub const SLOT_STATE_C: i16 = 8;
    /// No competition id.
    pub const COMP_ID: u8 = 0xFF;
    /// The first month: the season's start, in eighths of a month.
    pub const START_MONTH: i8 = 0x38;
    /// The last month.
    pub const END_MONTH: i8 = 0x20;
    /// No pitch type: the season's.
    pub const PITCH_TYPE: i16 = -1;
    /// The teams.
    pub const TEAMS: i16 = 2;
    /// A win's points.
    pub const POINTS_FOR_WIN: u16 = 2;
    /// The fewest substitutes.
    pub const MIN_SUBSTITUTES: u16 = 3;
    /// The most substitutes.
    pub const MAX_SUBSTITUTES: u16 = 5;
}

/// What the engine reads of a league record, in upstream's league layout (`CompLeagueHeader`
/// and its divisions).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LeagueRecord {
    /// Its competition id (`compId`).
    pub comp_id: u8,
    /// Its first month (`beginMonth`).
    pub begin_month: i8,
    /// Its last month (`endMonth`).
    pub end_month: i8,
    /// How many times each team plays the others (`rounds`; below 0, once).
    pub rounds: i8,
    /// A win's points (`pointsForWin`).
    pub points_for_win: i8,
    /// The substitutes: the fewest in the high nibble, the most in the low one
    /// (`subsAndMaxSubs`).
    pub substitutes: u8,
    /// Its divisions, each its six bytes (`Division`).
    pub divisions: Vec<[i8; DIVISION_BYTES]>,
}

/// A record that is no league.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0} is no record a league is started from")]
pub struct NotALeague(pub String);

impl LeagueRecord {
    /// The record of `league` (`leagues.json`).
    #[must_use]
    pub fn from_league(league: &League) -> Self {
        Self {
            comp_id: league.comp.comp_id.byte(),
            begin_month: league.comp.begin_month.byte().cast_signed(),
            end_month: league.comp.end_month.byte().cast_signed(),
            rounds: league.rounds.byte().cast_signed(),
            points_for_win: league.points_for_win.byte().cast_signed(),
            substitutes: u8::from(league.substitutes) << NIBBLE_SHIFT | u8::from(league.max_subs),
            divisions: league
                .divisions
                .iter()
                .map(|division| {
                    [
                        division.teams,
                        division.promoted,
                        division.promoted_po,
                        division.relegated,
                        division.relegated_po,
                        division.qualifier_link_offset,
                    ]
                    .map(|byte| byte.byte().cast_signed())
                })
                .collect(),
        }
    }

    /// The record of `cup`, a cup played as a league (`cups.json`'s qualifying groups,
    /// friendlies and tours, of type 0), as upstream reads its bytes through the league
    /// layout: its `num_rounds_override` byte is the divisions' count, `teams` the rounds,
    /// `away_goals_mode` a win's points, its substitutes byte the substitutes, and
    /// `round_string_base` with its first five round rules its one division. Faithful to the
    /// original, which authors these records in the cup's layout and plays them as leagues.
    ///
    /// # Errors
    ///
    /// [`NotALeague`] when `cup` is no league (a knockout).
    pub fn from_cup(cup: &Cup) -> Result<Self, NotALeague> {
        if cup.comp.kind != CompetitionType::League {
            return Err(NotALeague(cup.id.as_str().to_owned()));
        }
        let rule = |round: usize| {
            cup.round_rule_flags
                .get(round)
                .map_or(0, |&rules| u8::from(rules))
                .cast_signed()
        };
        Ok(Self {
            comp_id: cup.comp.comp_id.byte(),
            begin_month: cup.comp.begin_month.byte().cast_signed(),
            end_month: cup.comp.end_month.byte().cast_signed(),
            rounds: cup.teams.byte().cast_signed(),
            points_for_win: cup.away_goals_mode.byte().cast_signed(),
            substitutes: u8::from(cup.substitutes) << NIBBLE_SHIFT | u8::from(cup.points_for_win),
            divisions: vec![[
                cup.round_string_base.byte().cast_signed(),
                rule(0),
                rule(1),
                rule(2),
                rule(3),
                rule(4),
            ]],
        })
    }
}

/// The pitch the session plays on (upstream `_pitchType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pitch {
    /// The season's (`_pitchType` -2).
    Seasonal,
    /// This type, all season.
    Type(i16),
}

/// A team of the league, as upstream finds it (`VerifyTeamLoaded`): its identity in the
/// standings and its valued players' prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadedTeam {
    /// -1 the career club, otherwise its index among the selected teams.
    pub identity: i32,
    /// Its valued players' prices.
    pub prices: Prices,
}

/// What a division is started from: its record and place, and what the world's data resolves
/// for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivisionStart<'a> {
    /// The league's record.
    pub record: &'a LeagueRecord,
    /// The division's index.
    pub division: usize,
    /// The division's name (the record's for it, else the default division name).
    pub name: &'a [u8],
    /// The country's adjective before the name: none for a national teams' competition or a
    /// record of no country.
    pub prefix: Option<&'a [u8]>,
    /// The name the reset writes first (upstream `_aDiyLeague`): what is left of it past the
    /// name stays in the buffer.
    pub default_name: &'a [u8],
    /// Whether its suspensions are the continental track's (upstream `continental_record`).
    pub continental: bool,
    /// The session's pitch.
    pub pitch: Pitch,
}

/// Why a division does not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StartError {
    /// The record has no such division.
    #[error("the league has no division {0}")]
    NoDivision(usize),
    /// The world has no round robin of the division's size.
    #[error("no round robin of {0} clubs")]
    NoSchedule(i8),
    /// Its size's round robin is no real rounds, which a career's calendar cannot play
    /// (upstream halts, 0x1313).
    #[error("the round robin of {0} clubs plays no real rounds (upstream halts, 0x1313)")]
    NotRounds(i8),
    /// Fewer than two teams (upstream divides by zero).
    #[error("a league of {0} teams")]
    TooFewTeams(i16),
    /// A team of the league that is not loaded.
    #[error("team {0:?} is not loaded")]
    UnknownTeam(TeamBytes),
    /// A season of more match days than the image holds months for (upstream writes them on
    /// past the competition, into its buffer's scratch).
    #[error("a season of {0} match days")]
    TooManyMatchDays(u16),
}

/// Starts `start`'s division in `competition` (upstream `InitDIYCompetitionFromTableEntry`):
/// the reset, the record's and the division's fields, the name, the derived fields from the
/// league's teams (its `league_team` slots, set before, found by `teams`), then the places
/// shuffled by `rng`. A world league's split (upstream `LeagueSplit_Init`) joins with its data
/// (M5 #9).
///
/// # Errors
///
/// [`StartError`] when the division is not the record's, its size has no round robin or none
/// in real rounds, it has fewer than two teams, or a team is not loaded.
pub fn start_division(
    competition: &mut RunningCompetition,
    start: &DivisionStart<'_>,
    schedules: &Schedules,
    teams: impl Fn(TeamBytes) -> Option<LoadedTeam>,
    rng: &mut Rng,
) -> Result<(), StartError> {
    let record = start.record;
    let division = *record
        .divisions
        .get(start.division)
        .ok_or(StartError::NoDivision(start.division))?;
    let [clubs, promoted, promoted_po, relegated, relegated_po, _] = division;
    let schedule = schedules
        .of_size(clubs.cast_unsigned())
        .ok_or(StartError::NoSchedule(clubs))?;
    if !schedule.rounds() {
        return Err(StartError::NotRounds(clubs));
    }
    reset(competition, start.default_name);
    competition.set_substitutes_count(i16::from(!start.continental));
    competition.set_comp_id(record.comp_id);
    competition.set_start_month(record.begin_month);
    competition.set_end_month(record.end_month);
    // Faithful to upstream's build: it reads the rounds and the next byte as a word, little-
    // endian, so its sign is the points byte's (the original's 68k tested the rounds').
    if i16::from_le_bytes([
        record.rounds.cast_unsigned(),
        record.points_for_win.cast_unsigned(),
    ]) < 0
    {
        competition.set_has_groups_diy(1);
    }
    set_pitch(competition, start.pitch);
    competition.set_round_complete(i16::from(promoted.cast_unsigned()));
    competition.set_competition_finished(play_off_places(promoted_po));
    competition.set_current_round_index(i16::from(relegated.cast_unsigned()));
    competition.set_num_rounds(play_off_places(relegated_po));
    competition.set_num_teams(i16::from(clubs));
    competition.set_match_pairs_per_round(1);
    write_name(competition, start.prefix, start.name);
    competition.set_points_for_win(i16::from(record.points_for_win).cast_unsigned());
    // Faithful to the original: rounds below 0 play once, 0 plays none.
    let rounds = if record.rounds < 0 { 1 } else { record.rounds };
    competition.set_play_each_team(i16::from(rounds));
    competition.set_league_min_substitutes(u16::from(record.substitutes >> NIBBLE_SHIFT));
    competition.set_league_max_substitutes(u16::from(record.substitutes & NIBBLE_MASK));
    derive(competition, &teams)?;
    shuffle_places(competition, rng);
    Ok(())
}

/// The reset (upstream `DIY_InitCompetitionDefaults`): a DIY league's defaults, the name
/// `default_name`, the table's lines cleared (their teams kept), the places in order.
fn reset(competition: &mut RunningCompetition, default_name: &[u8]) {
    competition.set_competition_type(0);
    competition.set_comp_slot(0);
    competition.set_slot_id(0);
    competition.set_is_active(0);
    competition.set_match_started(0);
    competition.set_career_team_involved(0);
    competition.set_result_code(defaults::RESULT_CODE);
    competition.set_substitutes_count(1);
    competition.set_has_groups_diy(0);
    competition.set_slot_state_a(defaults::SLOT_STATE_A);
    competition.set_slot_state_b(defaults::SLOT_STATE_B);
    competition.set_slot_state_c(defaults::SLOT_STATE_C);
    competition.set_comp_id(defaults::COMP_ID);
    competition.set_has_groups(0);
    copy_string(competition, default_name);
    competition.set_start_month(defaults::START_MONTH);
    competition.set_end_month(defaults::END_MONTH);
    competition.set_pitch_type_or_season(1);
    competition.set_pitch_type(defaults::PITCH_TYPE);
    competition.set_num_teams(defaults::TEAMS);
    competition.set_play_each_team(1);
    competition.set_points_for_win(defaults::POINTS_FOR_WIN);
    competition.set_league_min_substitutes(defaults::MIN_SUBSTITUTES);
    competition.set_league_max_substitutes(defaults::MAX_SUBSTITUTES);
    competition.set_match_pairs_per_round(1);
    competition.set_sub_round_counter(0);
    competition.set_round_complete(0);
    competition.set_competition_finished(0);
    competition.set_current_round_index(0);
    competition.set_num_rounds(0);
    for row in 0..STANDINGS_ROWS {
        let team = competition.row(row).team;
        competition.set_row(
            row,
            StandingsRow {
                team,
                line: Line::default(),
            },
        );
    }
    for (place, offset) in (0..PLACES).zip((0..).step_by(usize::from(ROW_BYTES))) {
        competition.set_shuffle_and_pad(place, u8::try_from(place).expect("64 places"));
        competition.set_team_entry_offset(place, offset);
    }
}

/// `text` (up to its first 0) and its end into the name buffer, as upstream's `StringCopy`
/// does: the bytes after its end are left.
fn copy_string(competition: &mut RunningCompetition, text: &[u8]) {
    let mut name = competition.name();
    let text = until_nul(text);
    let len = text.len().min(NAME_ROOM);
    name[..len].copy_from_slice(&text[..len]);
    name[len] = 0;
    competition.set_name(name);
}

/// The competition's name into its buffer (upstream `copyCompetitionName`): the adjective and
/// a space, then the name, at most 35 bytes and their end; the bytes after its end are left.
fn write_name(competition: &mut RunningCompetition, prefix: Option<&[u8]>, name: &[u8]) {
    let mut text: Vec<u8> = prefix
        .map(|adjective| {
            let mut prefix = until_nul(adjective).to_vec();
            prefix.push(b' ');
            prefix
        })
        .unwrap_or_default();
    let room = (NAME_SCRATCH - 1).saturating_sub(text.len());
    text.extend(until_nul(name).iter().take(room));
    copy_string(competition, &text);
}

/// The pitch fields (upstream `setPitchTypeFields`): the season's, or one type all season.
fn set_pitch(competition: &mut RunningCompetition, pitch: Pitch) {
    match pitch {
        Pitch::Seasonal => competition.set_pitch_type_or_season(1),
        Pitch::Type(pitch_type) => {
            competition.set_pitch_type_or_season(0);
            competition.set_pitch_type(pitch_type);
        }
    }
}

/// A division's play-off places as upstream reads them: below 0, half their opposite.
fn play_off_places(places: i8) -> i16 {
    let byte = if places < 0 {
        places.wrapping_neg().cast_unsigned() >> 1
    } else {
        places.cast_unsigned()
    };
    i16::from(byte)
}

/// `bytes` up to their first 0.
fn until_nul(bytes: &[u8]) -> &[u8] {
    bytes.split(|&byte| byte == 0).next().unwrap_or_default()
}

/// The fields derived from the league's teams and size (upstream
/// `DIY_RecomputeCompetitionDerivedFields`): each team's identity in its row, the section's
/// counts, the season's matches and its months, the teams' strength.
fn derive(
    competition: &mut RunningCompetition,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
) -> Result<(), StartError> {
    let num_teams = competition.num_teams();
    let count = usize::from(num_teams.cast_unsigned());
    // Each team's identity in its row (upstream `DIY_CacheSelectedTeamPointers`).
    for place in (0..count).rev() {
        let index = usize::from(
            i16::from(competition.shuffle_and_pad(place).cast_signed()).cast_unsigned(),
        );
        let team = competition.league_team(index);
        let loaded = teams(team).ok_or(StartError::UnknownTeam(team))?;
        let line = competition.row(index).line;
        competition.set_row(
            index,
            StandingsRow {
                team: loaded.identity,
                line,
            },
        );
    }
    let pairs = competition.match_pairs_per_round().cast_unsigned();
    // Faithful to the original: the teams are sign-extended before the division.
    let groups = low_word(
        i32::from(num_teams)
            .cast_unsigned()
            .checked_div(u32::from(pairs))
            .ok_or(StartError::TooFewTeams(num_teams))?,
    );
    competition.set_total_matches_in_round(groups.cast_signed());
    competition.set_section_matches_progress(0);
    let half = groups >> 1;
    if half == 0 {
        return Err(StartError::TooFewTeams(num_teams));
    }
    competition.set_half_group_count(half);
    competition.set_matches_per_half(half.wrapping_mul(pairs));
    competition.set_half_groups_counter(0);
    competition.set_league_match_progress(0);
    competition.set_league_current_slot(0);
    let section = low_word((u32::from(groups.wrapping_sub(1)) * u32::from(groups)) >> 1);
    competition.set_total_matches_per_section(section);
    competition.set_total_matches_per_section_live(section);
    let season = low_word(i32::from(section.cast_signed()).cast_unsigned() / u32::from(half))
        .wrapping_mul(competition.play_each_team().cast_unsigned());
    competition.set_league_total_matches(season);
    competition.set_min_substitutes(competition.play_each_team().cast_unsigned());
    competition.set_matches_played_count(0);
    competition.set_is_active(1);
    competition.set_view_random_phase(competition.start_month().cast_unsigned());
    season_months(competition)?;
    let prices = (0..count)
        .map(|team| {
            let team = competition.league_team(team);
            teams(team)
                .map(|loaded| loaded.prices)
                .ok_or(StartError::UnknownTeam(team))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let strength = strength(&prices).ok_or(StartError::TooFewTeams(num_teams))?;
    competition.set_avg_skill(strength.average.to_le_bytes()[0]);
    competition.set_strength_tier(strength.tier);
    Ok(())
}

/// The low word of `value`, as upstream keeps a quotient.
fn low_word(value: u32) -> u16 {
    let [low, high, _, _] = value.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// Each match day's month, from the first month to the last (upstream
/// `DIY_RecomputeCompetitionDerivedProgressionTable`). Faithful to the original: past 128 match
/// days the months run on into the cup's teams.
fn season_months(competition: &mut RunningCompetition) -> Result<(), StartError> {
    let days = competition.league_total_matches();
    if days == 0 || usize::from(days) > MAX_MATCH_DAYS {
        return Err(StartError::TooManyMatchDays(days));
    }
    let start = competition.start_month().cast_unsigned();
    if days == 1 {
        competition.set_phase_start_month(0, start);
        return Ok(());
    }
    let mut end = competition.end_month().cast_unsigned();
    if end < start {
        end = end.wrapping_add(MONTHS_WRAP);
    }
    let range = u32::from(end.wrapping_sub(start));
    let last = days.wrapping_sub(1);
    for day in 0..=last {
        let step = low_word(u32::from(day) * range / u32::from(last));
        competition
            .set_phase_start_month(usize::from(day), step.to_le_bytes()[0].wrapping_add(start));
    }
    Ok(())
}

/// The places shuffled (upstream `DIY_ShuffleLeagueTeamOrder`): each place's index and its
/// row's offset together.
fn shuffle_places(competition: &mut RunningCompetition, rng: &mut Rng) {
    let count = usize::from(competition.num_teams().cast_unsigned());
    let mut places: Vec<(u8, u16)> = (0..count)
        .map(|place| {
            (
                competition.shuffle_and_pad(place),
                competition.team_entry_offset(place),
            )
        })
        .collect();
    shuffle(&mut places, rng);
    for (place, (index, offset)) in places.into_iter().enumerate() {
        competition.set_shuffle_and_pad(place, index);
        competition.set_team_entry_offset(place, offset);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use openswos_common::Importer;
    use openswos_competitions_json_resource::{CompetitionsFile, CompetitionsJsonFormat};

    use super::*;
    use crate::competition::schedule::{Pair, Schedule};

    /// Round robins of 2 and 4 clubs in real rounds, of 3 not.
    fn schedules() -> Schedules {
        let pair = |home, away| Pair { home, away };
        let two = Schedule::new(2, true, vec![pair(0, 1)]);
        let three = Schedule::new(3, false, vec![pair(0, 1), pair(1, 2), pair(2, 0)]);
        let pairs = vec![
            pair(0, 1),
            pair(2, 3),
            pair(0, 2),
            pair(1, 3),
            pair(3, 0),
            pair(1, 2),
        ];
        let four = Schedule::new(4, true, pairs);
        Schedules::new(
            BTreeMap::from([(2, two), (3, three), (4, four)]),
            BTreeMap::new(),
        )
    }

    /// A league of one division of `clubs`: 3 points a win, twice, subs 3 to 5.
    fn record(clubs: i8) -> LeagueRecord {
        LeagueRecord {
            comp_id: 40,
            begin_month: 0x40,
            end_month: 0x20,
            rounds: 2,
            points_for_win: 3,
            substitutes: 0x35,
            divisions: vec![[clubs, 1, -4, 2, 0, 0]],
        }
    }

    fn start(record: &LeagueRecord) -> DivisionStart<'_> {
        DivisionStart {
            record,
            division: 0,
            name: b"PREMIER",
            prefix: Some(b"ENGLISH"),
            default_name: b"DIY LEAGUE",
            continental: false,
            pitch: Pitch::Seasonal,
        }
    }

    /// Each team `[0, n]` the selected team `n`, priced `n` throughout.
    fn teams(team: TeamBytes) -> Option<LoadedTeam> {
        let [_, ordinal] = team;
        Some(LoadedTeam {
            identity: i32::from(ordinal),
            prices: [ordinal; 16],
        })
    }

    /// A competition whose league slots hold teams `[0, 0]` to `[0, 3]`.
    fn competition() -> RunningCompetition {
        let mut competition = RunningCompetition::default();
        for ordinal in 0..4 {
            competition.set_league_team(usize::from(ordinal), [0, ordinal]);
        }
        competition
    }

    #[test]
    fn a_league_record_is_its_bytes() {
        let text = r#"{"type": "competition_leagues", "count": 1, "leagues": [
            {"id": "_l", "comp": {"comp_id": 40, "type": 0, "team_number": 8, "begin_month": 64, "end_month": 32, "name_table_offset": 0, "reserved": 0, "team_list_offset": 0, "seed_reorder_flag": 0},
             "divisions_count": 1, "rounds": 2, "points_for_win": 3, "substitutes": 3, "max_subs": 5,
             "divisions": [{"teams": 20, "promoted": 0, "promoted_po": -4, "relegated": 3, "relegated_po": 0, "qualifier_link_offset": 0}],
             "reserved1": 0, "name": []}]}"#;
        let CompetitionsFile::Leagues(file) = CompetitionsJsonFormat::import(text.as_bytes())
            .expect("leagues")
            .file
        else {
            panic!("leagues");
        };
        let record = LeagueRecord::from_league(&file.leagues[0]);
        assert_eq!(
            (record.comp_id, record.rounds, record.substitutes),
            (40, 2, 0x35)
        );
        assert_eq!(record.divisions, [[20, 0, -4, 3, 0, 0]]);
    }

    #[test]
    fn a_cup_played_as_a_league_is_read_through_the_league_s_layout() {
        let text = r#"{"type": "competition_cups", "count": 2, "competitions": [
            {"id": "_group", "type": "cup", "comp": {"comp_id": 5, "type": 0, "team_number": -1, "begin_month": 64, "end_month": 32, "name_table_offset": 15, "reserved": 0, "team_list_offset": 21, "seed_reorder_flag": 0},
             "num_rounds_override": 1, "teams": 2, "away_goals_mode": 3, "substitutes": 3, "points_for_win": 5, "round_string_base": 6,
             "round_rule_flags": [{"legs": 0, "extra_time": 0, "penalties": 1}, {"legs": 0, "extra_time": 0, "penalties": 0, "low": 2}], "name": [], "team_seed_list": []},
            {"id": "_knockout", "type": "cup", "comp": {"comp_id": 109, "type": 1, "team_number": 0, "begin_month": 88, "end_month": 32, "name_table_offset": 0, "reserved": 0, "team_list_offset": 0, "seed_reorder_flag": 0},
             "num_rounds_override": 0, "teams": 16, "away_goals_mode": 1, "substitutes": 3, "points_for_win": 5, "round_string_base": 1,
             "round_rule_flags": [], "name": [], "team_seed_list": []}]}"#;
        let CompetitionsFile::Cups(file) = CompetitionsJsonFormat::import(text.as_bytes())
            .expect("cups")
            .file
        else {
            panic!("cups");
        };
        let group = LeagueRecord::from_cup(&file.competitions[0]).expect("a league");
        assert_eq!(
            (group.rounds, group.points_for_win, group.substitutes),
            (2, 3, 0x35)
        );
        // The division: the round string base, then the rules' bytes (4, then 2), then 0s.
        assert_eq!(group.divisions, [[6, 4, 2, 0, 0, 0]]);
        let error = LeagueRecord::from_cup(&file.competitions[1]).expect_err("a knockout");
        assert_eq!(
            error.to_string(),
            "_knockout is no record a league is started from"
        );
    }

    #[test]
    fn a_division_starts_with_its_record_s_fields() {
        let record = record(4);
        let mut competition = competition();
        let mut rng = Rng::default();
        start_division(
            &mut competition,
            &start(&record),
            &schedules(),
            teams,
            &mut rng,
        )
        .expect("started");
        assert_eq!((competition.comp_id(), competition.num_teams()), (40, 4));
        assert_eq!(competition.substitutes_count(), 1, "the domestic track");
        assert_eq!(
            (
                competition.round_complete(),
                competition.competition_finished()
            ),
            (1, 2),
            "promoted 1; -4 through play-offs, half its opposite"
        );
        assert_eq!(
            (competition.current_round_index(), competition.num_rounds()),
            (2, 0)
        );
        assert_eq!(
            (competition.points_for_win(), competition.play_each_team()),
            (3, 2)
        );
        assert_eq!(
            (
                competition.league_min_substitutes(),
                competition.league_max_substitutes()
            ),
            (3, 5)
        );
        assert_eq!(&competition.name()[..16], b"ENGLISH PREMIER\0");
        // 4 clubs: 6 matches a cycle, 2 a day, 2 cycles: 6 match days.
        assert_eq!(competition.league_total_matches(), 6);
        assert_eq!(competition.total_matches_per_section(), 6);
        assert_eq!(competition.is_active(), 1);
        // Prices 0 to 3: the average 1, tier 0.
        assert_eq!(
            (competition.avg_skill(), competition.strength_tier()),
            (1, 0)
        );
        let teams: Vec<i32> = (0..4).map(|row| competition.row(row).team).collect();
        assert_eq!(teams, [0, 1, 2, 3]);
        assert_eq!(competition.pitch_type_or_season(), 1, "the season's");
    }

    #[test]
    fn the_name_keeps_what_the_default_left_past_its_end() {
        let record = record(2);
        let mut start = start(&record);
        start.prefix = None;
        start.name = b"PREM";
        let mut competition = competition();
        start_division(
            &mut competition,
            &start,
            &schedules(),
            teams,
            &mut Rng::default(),
        )
        .expect("started");
        assert_eq!(
            &competition.name()[..11],
            b"PREM\0EAGUE\0",
            "DIY LEAGUE's tail"
        );
        let long = [b'A'; 40];
        start.name = &long;
        start_division(
            &mut competition,
            &start,
            &schedules(),
            teams,
            &mut Rng::default(),
        )
        .expect("started");
        assert_eq!(competition.name()[34], b'A');
        assert_eq!(competition.name()[35], 0, "35 bytes and the end");
    }

    #[test]
    fn what_cannot_start_is_refused() {
        let mut competition = competition();
        let mut rng = Rng::default();
        let mut refused = |clubs: i8, division: usize| {
            let record = record(clubs);
            let mut start = start(&record);
            start.division = division;
            start_division(&mut competition, &start, &schedules(), teams, &mut rng)
                .expect_err("refused")
        };
        assert_eq!(refused(4, 1), StartError::NoDivision(1));
        assert_eq!(refused(6, 0), StartError::NoSchedule(6));
        assert_eq!(refused(3, 0), StartError::NotRounds(3));
        let record = record(4);
        let error = start_division(
            &mut competition,
            &start(&record),
            &schedules(),
            |_| None,
            &mut rng,
        )
        .expect_err("no teams");
        assert_eq!(error, StartError::UnknownTeam([0, 3]));
    }

    #[test]
    fn play_off_places_below_zero_are_half_their_opposite() {
        assert_eq!(play_off_places(3), 3);
        assert_eq!(play_off_places(-4), 2);
        assert_eq!(play_off_places(-5), 2);
        assert_eq!(play_off_places(i8::MIN), 64);
    }

    #[test]
    fn a_continental_contest_and_a_fixed_pitch_are_set() {
        let record = record(2);
        let mut start = start(&record);
        start.continental = true;
        start.pitch = Pitch::Type(3);
        let mut competition = competition();
        start_division(
            &mut competition,
            &start,
            &schedules(),
            teams,
            &mut Rng::default(),
        )
        .expect("started");
        assert_eq!(competition.substitutes_count(), 0);
        assert_eq!(
            (competition.pitch_type_or_season(), competition.pitch_type()),
            (0, 3)
        );
    }
}
