//! Competitions' matches simulated (docs/PLAN.md M5 #8d; upstream src/diy_competition.c, the
//! simulated path of `DIY_PlayOrSimulateLeagueFixture` and `DIY_PlayCupMatchAndAdvance`, and
//! `getCachedOrCalcViewResult`, `getCachedOrCalcExtraTimeResult`,
//! `getCachedOrCalcPenaltyResult` with no result cached, the cache a played match's, M6). The
//! squads are kept by identity, as upstream keeps its loaded teams (`_selectedTeams`,
//! `_careerTeam`): each match's statuses, goals and own goals stay in them. Before each match
//! its sides' statuses count down ([`countdown`]) and their match flags' bit 0 is cleared; its
//! result is simulated ([`simulate`]) on the day of the competition's month (its
//! `viewRandomPhase`, which upstream mirrors into `_viewRandomPhase` with each of its writes),
//! the players' statuses in the competition's track.

use openswos_rng::Rng;

use super::countdown::countdown;
use super::result::{Session, Simulated, extra_time, penalties, simulate};
use super::team::MatchTeam;
use super::value::SimError;
use crate::competition::results::{MatchResults, MatchSetup, Sides};
use crate::competition::running::{RunningCompetition, TeamBytes};
use crate::competition::start::LoadedTeam;
use crate::competition::table::Score;

/// The bit of a team's match flags cleared before a simulated result (`andWith0xFE`).
const SIMULATED_FLAG: u8 = 0x01;

/// The teams a competition's matches are played by: the selected teams, by their index, and
/// the career club.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Squads {
    /// The selected teams, by their index among them.
    pub selected: Vec<MatchTeam>,
    /// The career club, when there is one.
    pub career: Option<MatchTeam>,
}

impl Squads {
    /// The squad of `identity`: the selected team of that index, the career club below 0.
    fn of(&mut self, identity: i32) -> Result<&mut MatchTeam, SimError> {
        match usize::try_from(identity) {
            Ok(index) => self.selected.get_mut(index),
            Err(_) => self.career.as_mut(),
        }
        .ok_or(SimError::NoSquad(identity))
    }

    /// The two sides' squads, the home side's first.
    fn pair(&mut self, sides: Sides) -> Result<[&mut MatchTeam; 2], SimError> {
        let Self { selected, career } = self;
        match (usize::try_from(sides.home), usize::try_from(sides.away)) {
            (Ok(home), Ok(away)) => {
                if home == away {
                    return Err(SimError::SameSquad(sides.home));
                }
                for (index, identity) in [(home, sides.home), (away, sides.away)] {
                    if index >= selected.len() {
                        return Err(SimError::NoSquad(identity));
                    }
                }
                let [home, away] = selected
                    .get_disjoint_mut([home, away])
                    .expect("two squads within the selected teams");
                Ok([home, away])
            }
            (Err(_), Ok(_)) => {
                let (career, away) = with_career(career, sides.home, selected, sides.away)?;
                Ok([career, away])
            }
            (Ok(_), Err(_)) => {
                let (career, home) = with_career(career, sides.away, selected, sides.home)?;
                Ok([home, career])
            }
            (Err(_), Err(_)) => Err(SimError::SameSquad(sides.home)),
        }
    }
}

/// The career club (its side `career_side`) and the selected team of side `other` (at or above
/// 0).
fn with_career<'s>(
    career: &'s mut Option<MatchTeam>,
    career_side: i32,
    selected: &'s mut [MatchTeam],
    other: i32,
) -> Result<(&'s mut MatchTeam, &'s mut MatchTeam), SimError> {
    let career = career.as_mut().ok_or(SimError::NoSquad(career_side))?;
    let squad = usize::try_from(other)
        .ok()
        .and_then(|index| selected.get_mut(index))
        .ok_or(SimError::NoSquad(other))?;
    Ok((career, squad))
}

/// Matches simulated: the squads that play them, the session they are played in, the loader of
/// the competition's teams (their prices, for a side's rank), the last normal time's state
/// (for its extra time).
#[derive(Debug, Clone)]
pub struct Simulation<'a, L> {
    /// The squads, as the matches leave them.
    pub squads: Squads,
    /// The session: its day's month replaced by the competition's at each match.
    pub session: Session<'a>,
    /// The loader of the competition's teams.
    teams: L,
    /// The last normal time's state.
    last: Option<Simulated>,
}

impl<'a, L: Fn(TeamBytes) -> Option<LoadedTeam>> Simulation<'a, L> {
    /// The matches of `squads` simulated in `session`, the competition's teams loaded by
    /// `teams`.
    #[must_use]
    pub fn new(squads: Squads, session: Session<'a>, teams: L) -> Self {
        Self {
            squads,
            session,
            teams,
            last: None,
        }
    }

    /// The squad of `identity` (-1 the career club).
    ///
    /// # Errors
    ///
    /// [`SimError::NoSquad`] when it names none.
    pub fn squad(&mut self, identity: i32) -> Result<&mut MatchTeam, SimError> {
        self.squads.of(identity)
    }
}

impl<L: Fn(TeamBytes) -> Option<LoadedTeam>> MatchResults for Simulation<'_, L> {
    fn normal_time(
        &mut self,
        competition: &RunningCompetition,
        sides: Sides,
        setup: &MatchSetup,
        rng: &mut Rng,
    ) -> Result<Score, SimError> {
        let mut session = self.session;
        session.rules.status.mode = competition.substitutes_count();
        session.day.phase = competition.view_random_phase();
        let [home, away] = self.squads.pair(sides)?;
        countdown([&mut *home, &mut *away], &session.rules, rng)?;
        for team in [&mut *home, &mut *away] {
            team.set_match_flags(team.match_flags() & !SIMULATED_FLAG);
        }
        let (score, simulated) =
            simulate(competition, setup, [home, away], &self.teams, &session, rng)?;
        self.last = Some(simulated);
        Ok(score)
    }

    fn extra_time(&mut self, sides: Sides, rng: &mut Rng) -> Result<Score, SimError> {
        let simulated = self.last.as_mut().ok_or(SimError::NoNormalTime)?;
        let [home, away] = self.squads.pair(sides)?;
        extra_time(simulated, [home, away], self.session.tactics, rng)
    }

    fn penalties(&mut self, _sides: Sides, rng: &mut Rng) -> Result<Score, SimError> {
        Ok(penalties(rng))
    }
}

#[cfg(test)]
mod tests {
    use openswos_common::{FixedName, Tactics};
    use openswos_tac_resource::{
        BALL_AREA_COLUMNS, OUTFIELD_PLAYERS, PlayerSpots, Spot, TeamTactics,
    };

    use super::*;
    use crate::competition::results::Venue;
    use crate::sim::formation::formation_orders;
    use crate::sim::standing::Day;
    use crate::sim::status::{ComputerStatuses, GameType, SimRules};
    use crate::sim::tables::AREAS;
    use crate::sim::team::{PLAYERS, RECORD_LEN};
    use crate::sim::value::StatusRules;

    /// Tactics whose player `n` (1 to 10) always stands in column 7, row 11 - `n`.
    fn rows() -> TeamTactics {
        let spots = |row: u8| {
            PlayerSpots([[Spot::from(0x70 | row); BALL_AREA_COLUMNS]; AREAS / BALL_AREA_COLUMNS])
        };
        TeamTactics {
            name: FixedName::from_bytes([0; 9]),
            players: std::array::from_fn(|player| spots(u8::try_from(10 - player).expect("10"))),
            pairs: [None; OUTFIELD_PLAYERS],
            ball_out_of_play: Tactics::FourFourTwo,
        }
    }

    /// A selected team the computer plays, its records in order, its first record a
    /// goalkeeper, an injured player's who heals by a step; its flags all set.
    fn squad(ordinal: u8) -> MatchTeam {
        let mut team = MatchTeam::selected(&[0; RECORD_LEN]);
        team.set_ordinal(ordinal);
        team.set_controls(1);
        team.set_match_flags(u8::MAX);
        for slot in 0..PLAYERS {
            team.set_player_number(slot, u8::try_from(slot).expect("16"));
            let position = if slot == 0 { 0 } else { 7 << 5 };
            team.set_position_and_face(slot, position);
        }
        team.set_cards_injuries(5, 0x10);
        team
    }

    /// No team is loaded: both sides the computer's, no rank is reckoned.
    fn no_teams(_team: TeamBytes) -> Option<LoadedTeam> {
        None
    }

    /// The match's setup.
    const SETUP: MatchSetup = MatchSetup {
        venue: Venue::Home,
        substitutes: (3, 5),
        column: 0,
    };

    #[test]
    fn a_normal_time_counts_down_clears_the_flags_then_simulates() {
        let tactics = [rows()];
        let orders = formation_orders(&tactics);
        let rules = SimRules {
            status: StatusRules {
                mode: 0,
                training: false,
                no_injuries_cards: false,
            },
            game: GameType::Competition,
            computer: ComputerStatuses {
                cards: true,
                injuries: true,
            },
        };
        let session = Session {
            tactics: &tactics,
            orders: &orders,
            rules,
            statuses_apply: true,
            national_selector: false,
            day: Day {
                season: 1996,
                seed: 7,
                phase: 0,
            },
        };
        let mut competition = RunningCompetition::default();
        competition.set_view_random_phase(9);
        let squads = Squads {
            selected: vec![squad(0), squad(1)],
            career: None,
        };
        let mut simulation = Simulation::new(squads.clone(), session, no_teams);
        let sides = Sides { home: 1, away: 0 };
        let mut rng = Rng::default();
        let score = simulation.normal_time(&competition, sides, &SETUP, &mut rng);
        // The same by hand: the countdown, the flags, the simulation on the competition's day.
        let (mut home, mut away) = (squads.selected[1].clone(), squads.selected[0].clone());
        let mut by_hand = Rng::default();
        countdown([&mut home, &mut away], &rules, &mut by_hand).expect("counted down");
        assert_eq!(home.cards_injuries(5), 0x0C, "a step healed");
        home.set_match_flags(0xFE);
        away.set_match_flags(0xFE);
        let day = Session {
            day: Day {
                phase: 9,
                ..session.day
            },
            ..session
        };
        let simulated = simulate(
            &competition,
            &SETUP,
            [&mut home, &mut away],
            no_teams,
            &day,
            &mut by_hand,
        );
        let (expected, _) = simulated.expect("simulated");
        assert_eq!(score, Ok(expected));
        assert_eq!(simulation.squads.selected, [away, home]);
        assert_eq!(rng, by_hand);
        let extra = simulation.extra_time(sides, &mut rng);
        assert!(extra.is_ok(), "{extra:?}");
        assert!(simulation.penalties(sides, &mut rng).is_ok());
    }

    #[test]
    fn the_sides_must_name_two_squads() {
        let tactics = [rows()];
        let orders = formation_orders(&tactics);
        let session = Session {
            tactics: &tactics,
            orders: &orders,
            rules: SimRules {
                status: StatusRules {
                    mode: 0,
                    training: false,
                    no_injuries_cards: false,
                },
                game: GameType::Competition,
                computer: ComputerStatuses {
                    cards: false,
                    injuries: false,
                },
            },
            statuses_apply: true,
            national_selector: false,
            day: Day {
                season: 0,
                seed: 0,
                phase: 0,
            },
        };
        let squads = Squads {
            selected: vec![squad(0), squad(1)],
            career: None,
        };
        let mut simulation = Simulation::new(squads, session, no_teams);
        let competition = RunningCompetition::default();
        let mut rng = Rng::default();
        let mut asked = |home, away| {
            simulation.normal_time(&competition, Sides { home, away }, &SETUP, &mut rng)
        };
        assert_eq!(asked(0, 2), Err(SimError::NoSquad(2)));
        assert_eq!(asked(1, 1), Err(SimError::SameSquad(1)));
        assert_eq!(asked(-1, -1), Err(SimError::SameSquad(-1)));
        assert_eq!(asked(0, -1), Err(SimError::NoSquad(-1)));
        assert_eq!(
            simulation.extra_time(Sides { home: 0, away: 1 }, &mut Rng::default()),
            Err(SimError::NoNormalTime)
        );
        assert_eq!(simulation.squad(1).map(|team| team.ordinal()), Ok(1));
        assert_eq!(
            simulation.squad(-1).map(|team| team.ordinal()),
            Err(SimError::NoSquad(-1))
        );
    }
}
