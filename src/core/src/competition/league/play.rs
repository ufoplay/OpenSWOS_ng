//! A league played (docs/PLAN.md M5 #5): its fixtures in order, a result recorded and the
//! progress advanced, the table sorted, its winner and qualifiers, and its season's moves
//! (upstream diy_competition.c `getCurrentFixtureTeamPtrs`, `DIY_AdvanceLeagueProgressAfterMatch`,
//! the bookkeeping of `DIY_PlayOrSimulateLeagueFixture`, `DIY_SortLeagueTableByStandings`,
//! `DIY_UpdateLeagueQualifiers`, `DIY_UpdatePromotionRelegationScratchCounts`). The match
//! itself is played or simulated elsewhere (M5 #8, M6); the career walks a season's fixtures
//! into its calendar (`recordLeagueFixtures`) with M7.
//!
//! A world's league split, league phase and state championships pair, count, record and sort
//! their own way (`LeagueSplit_*`, `LeaguePhase_*`, `States_Pair`): they join with World
//! 2026-27's components (M5 #9).

use thiserror::Error;

use crate::competition::running::{IMAGE_LEN, PHASE_MONTHS, RunningCompetition, STANDINGS_ROWS};
use crate::competition::schedule::Schedule;
use crate::competition::table::{Line, Score, record, sort_groups};
use crate::competition::tournament::TournamentState;

/// The bytes of a standings row: the places' entries count in them.
const ROW_BYTES: u16 = 18;

/// The places of the league's order (`teamEntryOffsets[64]`, `shuffleAndPad[64]`).
const PLACES: usize = 64;

/// The identity of the career club in a standings row.
const CAREER_CLUB: i32 = -1;

/// The phases whose months the image holds: from the months' table to its end.
const MAX_PHASES: usize = IMAGE_LEN - PHASE_MONTHS;

/// A match of the league: its two sides' standings rows, the side at home first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fixture {
    /// The home side's row.
    home: usize,
    /// The away side's row.
    away: usize,
}

impl Fixture {
    /// The home side's standings row.
    #[must_use]
    pub fn home(self) -> usize {
        self.home
    }

    /// The away side's standings row.
    #[must_use]
    pub fn away(self) -> usize {
        self.away
    }
}

/// The teams a league's season moves: its promoted and its relegated, each its identity in
/// the standings, in upstream's order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Moves {
    /// The promoted, from the top of the table.
    pub promoted: Vec<i32>,
    /// The relegated, from the bottom of the table.
    pub relegated: Vec<i32>,
}

/// Why a league cannot be played on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PlayError {
    /// A round robin of another size than the league's groups.
    #[error("a round robin of {schedule} clubs for groups of {groups}")]
    WrongSchedule {
        /// The round robin's clubs.
        schedule: u8,
        /// A group's clubs.
        groups: i16,
    },
    /// The season played (upstream reads past its round robin).
    #[error("the season is over: match {0} is past the round robin")]
    SeasonOver(u16),
    /// A place past the league's order (upstream reads the fields after it).
    #[error("place {0} is past the league's 64")]
    PlacePastOrder(usize),
    /// A place naming no standings row (upstream reads past the rows).
    #[error("place {place} names row {row}")]
    NotARow {
        /// The place.
        place: usize,
        /// The row it names.
        row: i16,
    },
    /// A place's entry that is no standings row's (upstream's entries are rows, 18 bytes a
    /// row).
    #[error("entry {0} is no standings row's")]
    EntryNotRow(u16),
    /// A phase whose month is past the competition (upstream reads past it).
    #[error("phase {0}'s month is past the competition")]
    MonthPastImage(u16),
    /// A team of the table with no name.
    #[error("team {0} has no name")]
    UnknownTeam(i32),
}

/// The fixture the league plays next (upstream `getCurrentFixtureTeamPtrs`): the current pair
/// of `schedule` (the round robin the league's groups play) in the current group, each side
/// the row of its place; its sides the other way round in every other cycle.
///
/// # Errors
///
/// [`PlayError`] when `schedule` is not of the league's groups' size, the season is over, or a
/// place is past the order or names no row.
pub fn fixture(
    competition: &RunningCompetition,
    schedule: &Schedule,
) -> Result<Fixture, PlayError> {
    let groups = competition.total_matches_in_round();
    if i16::from(schedule.teams()) != groups {
        return Err(PlayError::WrongSchedule {
            schedule: schedule.teams(),
            groups,
        });
    }
    let base = usize::from(
        competition
            .sub_round_counter()
            .cast_unsigned()
            .wrapping_mul(groups.cast_unsigned()),
    );
    let day = competition.section_matches_progress();
    let pair = schedule
        .pairs()
        .get(usize::from(day))
        .ok_or(PlayError::SeasonOver(day))?;
    let row = |position: u8| {
        let place = base + usize::from(position);
        if place >= PLACES {
            return Err(PlayError::PlacePastOrder(place));
        }
        let row = i16::from(competition.shuffle_and_pad(place).cast_signed());
        usize::try_from(row)
            .ok()
            .filter(|&row| row < STANDINGS_ROWS)
            .ok_or(PlayError::NotARow { place, row })
    };
    let (first, second) = (row(pair.home)?, row(pair.away)?);
    // The cycle's flag (`minSubstitutes`, a cycle counted down): odd, the other way round.
    Ok(if competition.min_substitutes() & 1 != 0 {
        Fixture {
            home: second,
            away: first,
        }
    } else {
        Fixture {
            home: first,
            away: second,
        }
    })
}

/// The league's progress after `fixture` (upstream `DIY_AdvanceLeagueProgressAfterMatch`): its
/// sides' games counted; the group, the day, the phase and its month, the cycle advanced.
///
/// # Errors
///
/// [`PlayError::MonthPastImage`] when a new phase's month is past the competition. Nothing is
/// written then.
pub fn advance(competition: &mut RunningCompetition, fixture: Fixture) -> Result<(), PlayError> {
    let (mut slot, mut half) = (
        competition.league_current_slot(),
        competition.half_groups_counter(),
    );
    if slot != 0 {
        slot = 0;
        half = 0;
    }
    half = half.wrapping_add(1);
    let mut group = competition.sub_round_counter().wrapping_add(1);
    let day_done = group.cast_unsigned() == competition.match_pairs_per_round().cast_unsigned();
    let phase_done = day_done && half == competition.matches_per_half();
    let phase = competition.league_match_progress().wrapping_add(1);
    if phase_done && usize::from(phase) >= MAX_PHASES {
        return Err(PlayError::MonthPastImage(phase));
    }
    for row in [fixture.home, fixture.away] {
        let mut standings = competition.row(row);
        standings.line.count_game();
        competition.set_row(row, standings);
    }
    competition.set_half_groups_counter(half);
    if !day_done {
        competition.set_league_current_slot(slot);
        competition.set_sub_round_counter(group);
        return Ok(());
    }
    group = 0;
    competition.set_sub_round_counter(group);
    competition
        .set_section_matches_progress(competition.section_matches_progress().wrapping_add(1));
    competition.set_league_current_slot(0);
    if phase_done {
        competition.set_league_current_slot(1);
        competition.set_league_match_progress(phase);
        competition.set_view_random_phase(competition.phase_start_month(usize::from(phase)));
    }
    let live = competition.total_matches_per_section_live().wrapping_sub(1);
    competition.set_total_matches_per_section_live(live);
    if live != 0 {
        return Ok(());
    }
    let cycles = competition.min_substitutes().wrapping_sub(1);
    competition.set_min_substitutes(cycles);
    if cycles != 0 {
        competition.set_total_matches_per_section_live(competition.total_matches_per_section());
        competition.set_section_matches_progress(0);
    }
    Ok(())
}

/// The league's next fixture played to `score` (the bookkeeping of upstream
/// `DIY_PlayOrSimulateLeagueFixture` around the match): the match begun, the progress advanced,
/// the career club's part noted, both kept in the tournament's state when the league is its
/// stage, the match counted, the result in the table, and once the season is played the
/// table sorted and its winner and qualifiers found ([`update_qualifiers`], the teams named by
/// `name`). The career's own records of the match are M7's.
///
/// # Errors
///
/// [`PlayError`] when the fixture cannot be found (nothing written then), the progress cannot
/// advance, or the table cannot be sorted.
pub fn play_result(
    competition: &mut RunningCompetition,
    tournament: &mut TournamentState,
    schedule: &Schedule,
    score: Score,
    name: impl Fn(i32) -> Option<Vec<u8>>,
) -> Result<Fixture, PlayError> {
    let fixture = fixture(competition, schedule)?;
    competition.set_match_started(1);
    keep_progress(competition, tournament);
    advance(competition, fixture)?;
    let career = [fixture.home, fixture.away]
        .into_iter()
        .any(|row| competition.row(row).team == CAREER_CLUB);
    if career {
        competition.set_career_team_involved(1);
    }
    keep_progress(competition, tournament);
    competition.set_matches_played_count(competition.matches_played_count().wrapping_add(1));
    let points = competition.points_for_win();
    if fixture.home == fixture.away {
        // Faithful to the original: a row that plays itself takes both sides' counts.
        let mut row = competition.row(fixture.home);
        let (mut home, mut away) = (row.line, row.line);
        record(&mut home, &mut away, score, points);
        row.line = both_sides(row.line, home, away);
        competition.set_row(fixture.home, row);
    } else {
        let (mut home, mut away) = (competition.row(fixture.home), competition.row(fixture.away));
        record(&mut home.line, &mut away.line, score, points);
        competition.set_row(fixture.home, home);
        competition.set_row(fixture.away, away);
    }
    if competition.min_substitutes() == 0 {
        update_qualifiers(competition, tournament, name)?;
    }
    Ok(fixture)
}

/// A line that was `before` with both sides' counts of a match: `home`'s and `away`'s, each
/// recorded from it, added up as words.
fn both_sides(before: Line, home: Line, away: Line) -> Line {
    let sum = |before: u16, home: u16, away: u16| home.wrapping_add(away).wrapping_sub(before);
    Line {
        played: sum(before.played, home.played, away.played),
        won: sum(before.won, home.won, away.won),
        drawn: sum(before.drawn, home.drawn, away.drawn),
        lost: sum(before.lost, home.lost, away.lost),
        goals_for: sum(before.goals_for, home.goals_for, away.goals_for),
        goals_against: sum(before.goals_against, home.goals_against, away.goals_against),
        points: sum(before.points, home.points, away.points),
    }
}

/// The match's progress kept in the tournament's state when the league is its stage
/// (upstream `DIY_SyncProgressToDiyFileBufferCopy`).
fn keep_progress(competition: &RunningCompetition, tournament: &mut TournamentState) {
    if competition.has_groups() == 0 {
        return;
    }
    tournament.set_match_started(competition.match_started());
    tournament.set_career_team_involved(competition.career_team_involved());
}

/// The row `entry` names (a place's entry: its row's offset, 18 bytes a row).
fn entry_row(entry: u16) -> Result<usize, PlayError> {
    let row = usize::from(entry / ROW_BYTES);
    if !entry.is_multiple_of(ROW_BYTES) || row >= STANDINGS_ROWS {
        return Err(PlayError::EntryNotRow(entry));
    }
    Ok(row)
}

/// The row of place `place` of the table.
fn place_row(competition: &RunningCompetition, place: usize) -> Result<usize, PlayError> {
    if place >= PLACES {
        return Err(PlayError::PlacePastOrder(place));
    }
    entry_row(competition.team_entry_offset(place))
}

/// A count as upstream walks it: a word that, as a signed one, below 1 walks none.
fn walked(count: i16) -> usize {
    usize::try_from(count).unwrap_or_default()
}

/// The table sorted, group by group (upstream `DIY_SortLeagueTableByStandings`): each group's
/// places by points, goal difference, goals, then name (`name`, by the team's identity), as
/// [`sort_groups`] orders them.
///
/// # Errors
///
/// [`PlayError`] when the groups' places are past the order, a place's entry is no row's, or a
/// team has no name.
pub fn sort_table(
    competition: &mut RunningCompetition,
    name: impl Fn(i32) -> Option<Vec<u8>>,
) -> Result<(), PlayError> {
    let groups = walked(competition.match_pairs_per_round());
    let per_group = walked(competition.total_matches_in_round());
    let places = groups * per_group;
    if places > PLACES {
        return Err(PlayError::PlacePastOrder(places));
    }
    let mut rows = (0..places)
        .map(|place| place_row(competition, place))
        .collect::<Result<Vec<_>, _>>()?;
    let lines: Vec<_> = (0..STANDINGS_ROWS)
        .map(|row| competition.row(row).line)
        .collect();
    let mut names = vec![Vec::new(); STANDINGS_ROWS];
    for &row in &rows {
        let team = competition.row(row).team;
        names[row] = name(team).ok_or(PlayError::UnknownTeam(team))?;
    }
    let names: Vec<&[u8]> = names.iter().map(Vec::as_slice).collect();
    sort_groups(&mut rows, per_group, &lines, &names);
    for (place, row) in rows.into_iter().enumerate() {
        let entry = u16::try_from(row).expect("64 rows") * ROW_BYTES;
        competition.set_team_entry_offset(place, entry);
    }
    Ok(())
}

/// The table sorted, then its winner and its play-offs' qualifiers (upstream
/// `DIY_UpdateLeagueQualifiers`): the winner, the top of the table, unless the league is a
/// tournament's stage before its last (`tournament`, the state, tells); the places after the
/// promoted that go to the promotion play-offs, then the places above the relegated that go to
/// the relegation play-offs, each team into the qualifiers in turn.
///
/// # Errors
///
/// [`PlayError`] when the table cannot be sorted, or a qualifier's place is past the order or
/// its entry no row's.
pub fn update_qualifiers(
    competition: &mut RunningCompetition,
    tournament: &TournamentState,
    name: impl Fn(i32) -> Option<Vec<u8>>,
) -> Result<(), PlayError> {
    sort_table(competition, name)?;
    let last_stage = tournament.round_complete().cast_unsigned().wrapping_add(1)
        == tournament.total_matches_in_round().cast_unsigned();
    if competition.has_groups() == 0 || last_stage {
        let row = place_row(competition, 0)?;
        competition.set_result_code(i16::from_be_bytes(competition.league_team(row)));
    }
    let mut places = Vec::new();
    let promotions = usize::from(competition.competition_finished().cast_unsigned());
    let promoted = usize::from(competition.round_complete().cast_unsigned());
    places.extend((0..promotions).map(|place| promoted + place));
    let relegations = competition.num_rounds().cast_unsigned();
    if relegations != 0 {
        let first = competition
            .num_teams()
            .cast_unsigned()
            .wrapping_sub(competition.current_round_index().cast_unsigned())
            .wrapping_sub(relegations);
        places.extend((0..usize::from(relegations)).map(|place| usize::from(first) + place));
    }
    let rows = places
        .into_iter()
        .map(|place| place_row(competition, place))
        .collect::<Result<Vec<_>, _>>()?;
    for (qualifier, row) in rows.into_iter().enumerate() {
        let team = competition.league_team(row);
        competition.set_qualified_team_by_division(qualifier, u16::from_le_bytes(team));
    }
    Ok(())
}

/// The teams the league's season moves (upstream `DIY_UpdatePromotionRelegationScratchCounts`):
/// a league's promoted from the top of its table, its relegated from the bottom; none for a
/// cup or a tournament. Upstream counts each in its scratch of the teams' divisions, by global
/// team number, and flags the career club's: the season end's (M7).
///
/// # Errors
///
/// [`PlayError`] when a place is past the order or its entry no row's.
pub fn season_moves(competition: &RunningCompetition) -> Result<Moves, PlayError> {
    if competition.competition_type() != 0 {
        return Ok(Moves::default());
    }
    let team = |place: usize| place_row(competition, place).map(|row| competition.row(row).team);
    let promoted = (0..walked(competition.round_complete()))
        .map(team)
        .collect::<Result<Vec<_>, _>>()?;
    let teams = usize::from(competition.num_teams().cast_unsigned());
    let relegated = (0..walked(competition.current_round_index()))
        .map(|from_bottom| {
            let place = teams
                .checked_sub(from_bottom + 1)
                .ok_or(PlayError::PlacePastOrder(teams))?;
            team(place)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Moves {
        promoted,
        relegated,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use openswos_rng::Rng;
    use openswos_team_resource::TeamControls;

    use super::*;
    use crate::competition::league::{DivisionStart, LeagueRecord, start_division};
    use crate::competition::schedule::{Pair, Schedules};
    use crate::competition::start::{LoadedTeam, Pitch};

    /// The round robin of 4 clubs, in real rounds: three days of two pairs.
    fn schedules() -> Schedules {
        let pair = |home, away| Pair { home, away };
        let pairs = vec![
            pair(0, 1),
            pair(2, 3),
            pair(0, 2),
            pair(1, 3),
            pair(3, 0),
            pair(1, 2),
        ];
        Schedules::new(
            BTreeMap::from([(4, Schedule::new(4, true, pairs))]),
            BTreeMap::new(),
        )
    }

    /// Team `[0, n]` the selected team `n`; team 3 the career club.
    fn teams(team: [u8; 2]) -> Option<LoadedTeam> {
        let [_, ordinal] = team;
        Some(LoadedTeam {
            identity: if ordinal == 3 {
                CAREER_CLUB
            } else {
                i32::from(ordinal)
            },
            controls: TeamControls::Computer,
            prices: [ordinal; 16],
        })
    }

    /// Each team's name: `A`, `B`, `C`, the career club `Z`.
    fn name(identity: i32) -> Option<Vec<u8>> {
        Some(match identity {
            CAREER_CLUB => b"Z".to_vec(),
            0 => b"A".to_vec(),
            1 => b"B".to_vec(),
            _ => b"C".to_vec(),
        })
    }

    /// A league of 4 started, twice round, its places in order (a draw of no change): one
    /// promoted, one through the play-offs; one relegated, one through the play-offs.
    fn started() -> RunningCompetition {
        let record = LeagueRecord {
            comp_id: 40,
            begin_month: 0x40,
            end_month: 0x60,
            rounds: 2,
            points_for_win: 3,
            substitutes: 0x35,
            divisions: vec![[4, 1, 1, 1, 1, 0]],
        };
        let mut competition = RunningCompetition::default();
        for ordinal in 0..4 {
            competition.set_league_team(usize::from(ordinal), [0, ordinal]);
        }
        let start = DivisionStart {
            record: &record,
            division: 0,
            name: b"PREMIER",
            prefix: None,
            default_name: b"DIY LEAGUE",
            continental: false,
            pitch: Pitch::Seasonal,
        };
        let mut rng = Rng::default();
        rng.fixed = Some(255);
        start_division(&mut competition, &start, &schedules(), teams, &mut rng).expect("started");
        for place in 0..4 {
            competition.set_shuffle_and_pad(place, u8::try_from(place).expect("4"));
            competition.set_team_entry_offset(place, u16::try_from(place).expect("4") * ROW_BYTES);
        }
        competition
    }

    fn schedule() -> Schedule {
        schedules().of_size(4).expect("4").clone()
    }

    #[test]
    fn the_fixtures_follow_the_round_robin_the_other_way_round_in_the_second_cycle() {
        let mut competition = started();
        let schedule = schedule();
        let mut played = Vec::new();
        while competition.min_substitutes() != 0 {
            let fixture = fixture(&competition, &schedule).expect("a fixture");
            played.push((fixture.home(), fixture.away()));
            advance(&mut competition, fixture).expect("advanced");
        }
        assert_eq!(played.len(), 12, "six pairs, twice");
        // Two cycles counted down from 2: the first even, the second odd and swapped.
        assert_eq!(&played[..2], &[(0, 1), (2, 3)]);
        assert_eq!(&played[6..8], &[(1, 0), (3, 2)]);
        assert_eq!(competition.row(0).line.played, 6);
        assert_eq!(competition.league_match_progress(), 6, "six match days");
        assert_eq!(
            fixture(&competition, &schedule),
            Err(PlayError::SeasonOver(6)),
            "the last cycle's progress kept"
        );
    }

    #[test]
    fn a_day_ends_with_its_last_group_and_a_phase_with_its_matches() {
        let mut competition = started();
        let schedule = schedule();
        let first = fixture(&competition, &schedule).expect("a fixture");
        advance(&mut competition, first).expect("advanced");
        assert_eq!(
            (
                competition.sub_round_counter(),
                competition.section_matches_progress()
            ),
            (0, 1),
            "one group: each match its day's"
        );
        assert_eq!(
            competition.league_current_slot(),
            0,
            "half the day's matches"
        );
        let second = fixture(&competition, &schedule).expect("a fixture");
        advance(&mut competition, second).expect("advanced");
        assert_eq!(competition.league_current_slot(), 1, "the day played");
        assert_eq!(competition.league_match_progress(), 1);
        assert_eq!(
            competition.view_random_phase(),
            competition.phase_start_month(1),
            "the next day's month"
        );
    }

    #[test]
    fn a_result_is_recorded_and_the_season_s_end_finds_its_qualifiers() {
        let mut competition = started();
        let mut tournament = TournamentState::default();
        let schedule = schedule();
        // The home side wins every match by its row's number of goals plus one.
        let mut fixtures = Vec::new();
        while competition.min_substitutes() != 0 {
            let next = fixture(&competition, &schedule).expect("a fixture");
            let score = Score {
                home: u16::try_from(next.home()).expect("4") + 1,
                away: 0,
            };
            fixtures.push(
                play_result(&mut competition, &mut tournament, &schedule, score, name)
                    .expect("played"),
            );
        }
        assert_eq!(fixtures.len(), 12);
        assert_eq!(competition.matches_played_count(), 12);
        assert_eq!(
            competition.career_team_involved(),
            1,
            "the career club plays"
        );
        assert_eq!(competition.match_started(), 1);
        let points: Vec<u16> = (0..4).map(|row| competition.row(row).line.points).collect();
        // Each wins its three home matches.
        assert_eq!(points, [9, 9, 9, 9]);
        // All level on points: goal difference orders them, the higher rows scoring more.
        let order: Vec<u16> = (0..4)
            .map(|place| competition.team_entry_offset(place) / ROW_BYTES)
            .collect();
        assert_eq!(order, [3, 2, 1, 0]);
        assert_eq!(
            competition.result_code(),
            0x0003,
            "the career club's team bytes"
        );
        // One promoted, then one to the promotion play-offs; one to the relegation play-offs,
        // above the one relegated.
        assert_eq!(
            competition.qualified_team_by_division(0),
            u16::from_le_bytes([0, 2])
        );
        assert_eq!(
            competition.qualified_team_by_division(1),
            u16::from_le_bytes([0, 1])
        );
        let moves = season_moves(&competition).expect("moves");
        assert_eq!(moves.promoted, [CAREER_CLUB]);
        assert_eq!(moves.relegated, [0]);
    }

    #[test]
    fn a_stage_s_winner_waits_for_the_last_stage() {
        let mut competition = started();
        competition.set_has_groups(1);
        competition.set_result_code(-1);
        let mut tournament = TournamentState::default();
        tournament.set_round_complete(0);
        tournament.set_total_matches_in_round(2);
        update_qualifiers(&mut competition, &tournament, name).expect("updated");
        assert_eq!(competition.result_code(), -1, "a stage before the last");
        tournament.set_round_complete(1);
        update_qualifiers(&mut competition, &tournament, name).expect("updated");
        assert_ne!(competition.result_code(), -1, "the last stage");
        competition.set_match_started(1);
        competition.set_career_team_involved(1);
        keep_progress(&competition, &mut tournament);
        assert_eq!(
            (
                tournament.match_started(),
                tournament.career_team_involved()
            ),
            (1, 1),
            "a stage's progress kept in the tournament"
        );
    }

    #[test]
    fn what_cannot_be_played_is_refused() {
        let competition = started();
        let three = Schedule::new(3, false, vec![Pair { home: 0, away: 1 }]);
        assert_eq!(
            fixture(&competition, &three),
            Err(PlayError::WrongSchedule {
                schedule: 3,
                groups: 4
            })
        );
        let mut broken = competition.clone();
        broken.set_shuffle_and_pad(0, 0x80);
        assert_eq!(
            fixture(&broken, &schedule()),
            Err(PlayError::NotARow {
                place: 0,
                row: -128
            })
        );
        let mut broken = competition.clone();
        broken.set_team_entry_offset(1, 19);
        assert_eq!(
            sort_table(&mut broken, name),
            Err(PlayError::EntryNotRow(19))
        );
        assert_eq!(
            sort_table(&mut competition.clone(), |_| None),
            Err(PlayError::UnknownTeam(0))
        );
        let mut broken = competition.clone();
        broken.set_league_current_slot(0);
        broken.set_half_groups_counter(1);
        broken.set_sub_round_counter(0);
        broken.set_match_pairs_per_round(1);
        broken.set_league_match_progress(u16::try_from(MAX_PHASES).expect("a word"));
        let before = broken.clone();
        let fixture = fixture(&competition, &schedule()).expect("a fixture");
        assert_eq!(
            advance(&mut broken, fixture),
            Err(PlayError::MonthPastImage(2171))
        );
        assert_eq!(broken, before, "nothing written");
    }

    #[test]
    fn a_cup_moves_no_team() {
        let mut competition = started();
        competition.set_competition_type(1);
        assert_eq!(season_moves(&competition), Ok(Moves::default()));
    }

    #[test]
    fn a_row_that_plays_itself_takes_both_sides() {
        let before = Line::default();
        let (mut home, mut away) = (before, before);
        record(&mut home, &mut away, Score { home: 2, away: 1 }, 3);
        let both = both_sides(before, home, away);
        assert_eq!((both.won, both.lost, both.points), (1, 1, 3));
        assert_eq!((both.goals_for, both.goals_against), (3, 3));
    }
}
