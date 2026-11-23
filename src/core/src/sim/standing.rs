//! A team's standing in its competition and its form on the day (docs/PLAN.md M5 #8b; upstream
//! src/match_sim.c `CalcTeamPriceRankPercentOffset`, `CalcTeamViewOffset`; src/diy_competition.c
//! `FindTeamInDIY` and `GetMoreExpensiveTeamsCount`, their league's paths, the only ones the
//! simulation reaches). A league's team does better than its price rank says, or worse, by up
//! to a quarter; each team's form of the day is drawn from the season, the session's seed and
//! the view's month. A world's league in zones ranks a club within its zone
//! (`LeagueSplit_ZoneFind`, `LeagueSplit_ZoneOfTeam`, `LeagueSplit_ZoneScale`): with World
//! 2026-27's components (M5 #9).

use openswos_rng::random_byte;

use super::tables::{FORM_BIAS_ENTRIES, FORM_BIASES, FORM_OFFSETS};
use super::team::MatchTeam;
use super::value::SimError;
use crate::competition::running::{RunningCompetition, TeamBytes};
use crate::competition::start::LoadedTeam;
use crate::competition::value::average_price;

/// The controls of a team the computer plays (`TEAM_COMPUTER`).
const COMPUTER: u8 = 1;

/// A league's type (`competitionType`).
const LEAGUE: i16 = 0;

/// The games a computer club's league must have played for its rank to count.
const COMPUTER_GAMES: u16 = 3;

/// The games another club's league must have played.
const HUMAN_GAMES: u16 = 10;

/// The bytes of a standings row: an entry offset counts in them.
const ROW_BYTES: u32 = 18;

/// A percentage's whole.
const PERCENT: i32 = 100;

/// The most a rank moves a team's strength, in percent (then halved).
const MAX_RANK_PERCENT: i16 = 50;

/// The index a national selector's lookup gives a team not in the competition.
const NOT_FOUND: u16 = 0xFFFF;

/// How far past its form's draw a team's second half's form is drawn.
const SECOND_HALF: u8 = 7;

/// How far the view's month is shifted down into a computer club's bias.
const PHASE_SHIFT: u8 = 4;

/// The most a team's form moves it, either way.
const MAX_FORM: i16 = 12;

/// The day a match is simulated on: what a team's form is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Day {
    /// The season shown (`_currentSeasonShowing`: its low byte counts).
    pub season: u16,
    /// The session's seed (`_newRandomSeedCopy`, the tick's high byte when the competition
    /// started).
    pub seed: u8,
    /// The view's month (`_viewRandomPhase`).
    pub phase: u8,
}

/// A team's form on the day: its first half's and its second's (upstream's view offsets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Form {
    /// The first half's (normal time's).
    pub first: i16,
    /// The second half's (extra time's).
    pub second: i16,
}

/// The low half of `value`, signed.
fn low_i16(value: i32) -> i16 {
    let [low, high, _, _] = value.to_le_bytes();
    i16::from_le_bytes([low, high])
}

/// The league's team `index`, checked within the image.
fn league_team(competition: &RunningCompetition, index: usize) -> Result<TeamBytes, SimError> {
    if !RunningCompetition::has_league_team(index) {
        return Err(SimError::IndexPastCompetition(index));
    }
    Ok(competition.league_team(index))
}

/// `team`'s place in its league's standing (upstream `FindTeamInDIY`'s league path): the first
/// place whose row's team is it; [`NOT_FOUND`] for a national selector's.
fn league_place(
    competition: &RunningCompetition,
    team: TeamBytes,
    national_selector: bool,
) -> Result<u16, SimError> {
    let teams = competition.num_teams().cast_unsigned();
    for place in 0..teams {
        let index = usize::from(place);
        if !RunningCompetition::has_team_entry(index) {
            return Err(SimError::IndexPastCompetition(index));
        }
        // Faithful to the original: the entry sign-extended, then divided as unsigned.
        let entry = i32::from(competition.team_entry_offset(index).cast_signed()).cast_unsigned();
        let [low, high, _, _] = (entry / ROW_BYTES).to_le_bytes();
        let row = u16::from_le_bytes([low, high]);
        if league_team(competition, usize::from(row))? == team {
            return Ok(place);
        }
    }
    if national_selector {
        Ok(NOT_FOUND)
    } else {
        Err(SimError::NotInCompetition(team))
    }
}

/// The league's teams of a higher average price than `team`'s (upstream
/// `GetMoreExpensiveTeamsCount`'s league path), each found by `teams`.
fn more_expensive(
    competition: &RunningCompetition,
    team: TeamBytes,
    teams: &impl Fn(TeamBytes) -> Option<LoadedTeam>,
) -> Result<u16, SimError> {
    let price = |team: TeamBytes| {
        teams(team)
            .map(|loaded| average_price(&loaded.prices))
            .ok_or(SimError::UnknownTeam(team))
    };
    let own = price(team)?;
    let mut count: u16 = 0;
    for index in 0..competition.num_teams().cast_unsigned() {
        let other = league_team(competition, usize::from(index))?;
        if other != team && price(other)? > own {
            count += 1;
        }
    }
    Ok(count)
}

/// How much `team`'s rank by price moves its strength in its competition, in percent, halved
/// (upstream `CalcTeamPriceRankPercentOffset`): a league's team only, once its first row has
/// played 3 games (a computer club's league) or 10; the teams dearer than it less its place,
/// over the teams less one, up to 50 either way. A national selector's team not in the league
/// takes the place upstream's lookup gives it.
///
/// # Errors
///
/// [`SimError`] when an index is past the competition's image, the team is not in its league
/// (upstream halts, 0x6006), or a team is not loaded.
pub fn price_rank(
    competition: &RunningCompetition,
    team: &MatchTeam,
    teams: impl Fn(TeamBytes) -> Option<LoadedTeam>,
    national_selector: bool,
) -> Result<i16, SimError> {
    if competition.competition_type() != LEAGUE {
        return Ok(0);
    }
    let games = competition.row(0).line.played;
    let needed = if team.controls() == COMPUTER {
        COMPUTER_GAMES
    } else {
        HUMAN_GAMES
    };
    if games < needed {
        return Ok(0);
    }
    let id = [team.country(), team.ordinal()];
    let place = league_place(competition, id, national_selector)?;
    let dearer = more_expensive(competition, id, &teams)?;
    let difference = i32::from(dearer.wrapping_sub(place).cast_signed());
    let others = competition.num_teams().wrapping_sub(1);
    if others == 0 {
        return Ok(0);
    }
    let percent = low_i16(difference * PERCENT / i32::from(others));
    Ok(percent.clamp(-MAX_RANK_PERCENT, MAX_RANK_PERCENT) >> 1)
}

/// `team`'s form on `day` (upstream `CalcTeamViewOffset`): a draw from the random table by the
/// team's number, the season and the seed, moved on by the view's month, picks each half's
/// form (doubled); a computer club's is biased by a second pick, another's is a point lower;
/// each at most 12 either way.
#[must_use]
pub fn form(team: &MatchTeam, day: &Day) -> Form {
    let [number, _] = team.global_number().to_le_bytes();
    let [season, _] = day.season.to_le_bytes();
    let drawn = random_byte(number ^ season ^ day.seed);
    let at = drawn.wrapping_add(day.phase);
    let offset = |at: u8| 2 * i16::from(FORM_OFFSETS[usize::from(at)]);
    let (mut first, mut second) = (offset(at), offset(at.wrapping_add(SECOND_HALF)));
    if team.controls() == COMPUTER {
        let pick = (usize::from(drawn) + usize::from(day.phase >> PHASE_SHIFT)) % FORM_BIAS_ENTRIES;
        let bias = 2 * i16::from(FORM_BIASES[pick]);
        first += bias;
        second += bias;
    } else {
        first -= 1;
        second -= 1;
    }
    Form {
        first: first.clamp(-MAX_FORM, MAX_FORM),
        second: second.clamp(-MAX_FORM, MAX_FORM),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::running::StandingsRow;
    use crate::competition::table::Line;
    use crate::sim::team::RECORD_LEN;

    /// A league of four teams, `[1, n]`, in order, its first row `games` played.
    fn league(games: u16) -> RunningCompetition {
        let mut competition = RunningCompetition::default();
        competition.set_num_teams(4);
        for row in 0..4u8 {
            let index = usize::from(row);
            competition.set_league_team(index, [1, row]);
            competition.set_team_entry_offset(index, u16::from(row) * 18);
        }
        competition.set_row(
            0,
            StandingsRow {
                team: 0,
                line: Line {
                    played: games,
                    won: 0,
                    drawn: 0,
                    lost: 0,
                    goals_for: 0,
                    goals_against: 0,
                    points: 0,
                },
            },
        );
        competition
    }

    /// Team `[1, n]` priced `n`.
    fn teams(team: TeamBytes) -> Option<LoadedTeam> {
        let [_, ordinal] = team;
        Some(LoadedTeam {
            identity: i32::from(ordinal),
            controls: openswos_team_resource::TeamControls::Computer,
            prices: [ordinal; 16],
        })
    }

    /// Team `[1, ordinal]` of `controls`.
    fn team(ordinal: u8, controls: u8) -> MatchTeam {
        let mut team = MatchTeam::selected(&[0; RECORD_LEN]);
        team.set_country(1);
        team.set_ordinal(ordinal);
        team.set_controls(controls);
        team
    }

    #[test]
    fn a_team_ranked_above_its_price_plays_up() {
        // Team 0 is first, three dearer: (3 - 0) × 100 / 3, at most 50, halved.
        assert_eq!(
            price_rank(&league(3), &team(0, COMPUTER), teams, false),
            Ok(25)
        );
        // Team 3 is last, none dearer: -100, at least -50, halved.
        assert_eq!(
            price_rank(&league(3), &team(3, COMPUTER), teams, false),
            Ok(-25)
        );
        // Too early in the season for another club.
        assert_eq!(price_rank(&league(9), &team(0, 0), teams, false), Ok(0));
        let mut cup = league(3);
        cup.set_competition_type(1);
        assert_eq!(price_rank(&cup, &team(0, COMPUTER), teams, false), Ok(0));
    }

    #[test]
    fn a_team_not_in_its_league_is_refused_but_for_a_selector() {
        assert_eq!(
            price_rank(&league(3), &team(9, COMPUTER), teams, false),
            Err(SimError::NotInCompetition([1, 9]))
        );
        // Place 0xFFFF: no team dearer, less it, a word: 1 × 100 / 3, halved.
        assert_eq!(
            price_rank(&league(3), &team(9, COMPUTER), teams, true),
            Ok(16)
        );
    }

    #[test]
    fn a_computer_club_s_form_is_biased_another_s_a_point_lower() {
        let day = Day {
            season: 0,
            seed: 0,
            phase: 0,
        };
        let drawn = random_byte(0);
        let base = 2 * i16::from(FORM_OFFSETS[usize::from(drawn)]);
        let bias = 2 * i16::from(FORM_BIASES[usize::from(drawn) % FORM_BIAS_ENTRIES]);
        assert_eq!(form(&team(0, 0), &day).first, (base - 1).clamp(-12, 12));
        assert_eq!(
            form(&team(0, COMPUTER), &day).first,
            (base + bias).clamp(-12, 12)
        );
    }
}
