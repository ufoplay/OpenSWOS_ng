//! What the checks of competitions simulated to their end share (`play.rs`, `cup.rs`,
//! `stages.rs`; M5 #8d): England's teams of the real team file (`assets/TEAM/TEAM.008`) cycled
//! as the selected teams, team `[0, n]` the selected team `n` (as the stubbed loader gives it),
//! or the selected teams numbered in turn by a competition's own teams (as
//! [`Oracle::assign_teams`] makes the loader give them), their controls by turns, the
//! computer's mostly, the match flag a simulated result clears set; a career club of team
//! [`CAREER_TEAM`], its reserves another team's players, its line-up past its first 16; the
//! session a competition's
//! index varies (the kind of game, whether a world opens a computer club's statuses, whether a
//! match's statuses apply, the day); the port's simulation of it and upstream's
//! ([`Oracle::set_simulation`]); the squads and the second generators alike after each match.
#![allow(dead_code, reason = "each check uses a part of it")]

use std::ops::Range;

use openswos_common::Importer;
use openswos_core::competition::results::{MatchResults, MatchSetup, Sides, Venue};
use openswos_core::competition::running::RunningCompetition;
use openswos_core::competition::start::LoadedTeam;
use openswos_core::competition::table::Score;
use openswos_core::sim::tables::ELEVEN;
use openswos_core::sim::{
    CAREER_LINE_UP, CAREER_RECORD_LEN, CareerLineUp, ComputerStatuses, Day, GameType, MatchTeam,
    Session, SimError, SimRules, Simulation, Squads, StatusRules,
};
use openswos_oracle::Oracle;
use openswos_oracle::cup::SelectedTeam;
use openswos_oracle::cup_play::Taken;
use openswos_oracle::draw::Prices;
use openswos_oracle::play::Teams;
use openswos_oracle::rng::RandState;
use openswos_oracle::sim::{Status, TACTICS_LEN};
use openswos_oracle::sim_play::SimSession;
use openswos_oracle::sim_result::Side;
use openswos_rng::Rng;
use openswos_tac_resource::TeamTactics;
use openswos_team_resource::{TeamControls, TeamFileFormat};
use openswos_test_utils::assets_path;

/// The selected teams the checks hold (upstream `SELECTED_TEAMS_MAX`).
pub(crate) const SELECTED: usize = 160;

/// The career club's team number: of the country the stubbed loader gives it for.
pub(crate) const CAREER_TEAM: [u8; 2] = [0xFF, 1];

/// The career club's place among a competition's teams: its second.
pub(crate) const CAREER_PLACE: usize = 1;

/// The career club's global team number.
const CAREER_NUMBER: u16 = 0x0123;

/// A team record's country and ordinal (`countryNumber`, `teamOrdinal`).
const TEAM_ID: Range<usize> = 0..2;

/// A team record's global team number (`globalTeamNumber`).
const GLOBAL_NUMBER: Range<usize> = 2..4;

/// A team record's controls (`teamControls`).
const CONTROLS: usize = 4;

/// A team record's match flags (`andWith0xFE`).
const MATCH_FLAGS: usize = 23;

/// The match flag cleared before a simulated result, set in every squad so the clearing
/// shows.
const SIMULATED_FLAG: u8 = 0x01;

/// A team record's name field (`teamName`).
const NAME: Range<usize> = 5..22;

/// The bytes of a name the oracle writes into the field (the rest zeros).
const NAME_ROOM: usize = 16;

/// The offset of a record's players.
const PLAYERS: usize = 76;

/// The bytes of a player's record.
const PLAYER_LEN: usize = 38;

/// A player's price, in his record.
const PRICE: usize = 32;

/// The players whose prices count (`Prices`).
const VALUED: usize = 16;

/// The selected teams' controls by turns: the computer's mostly, a player's, a coach's.
const CONTROLS_BY_TURN: [u8; 6] = [1, 0, 1, 2, 1, 3];

/// The career club's controls: a player-coach's.
const CAREER_CONTROLS: u8 = 3;

/// How far the career club's line-up is moved along its records: place `n` fields record
/// `n + 5`, past its first 16 into its reserves.
const CAREER_SHIFT: usize = 5;

/// The players the career club holds.
const CAREER_PLAYERS: u16 = 20;

/// The season the checks' matches are played in, before a competition's index is added.
const SEASON: u16 = 1996;

/// The loader of a competition's teams: the squads' prices and controls.
pub(crate) type Loader = Box<dyn Fn([u8; 2]) -> Option<LoadedTeam>>;

/// The port's simulation of the checks.
pub(crate) type Sim<'a> = Simulation<'a, Loader>;

/// `image`'s name field as the oracle writes it: its first 16 bytes before a 0, the rest
/// zeros.
fn normalised_name(image: &mut [u8]) {
    let field = &mut image[NAME];
    let len = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(field.len())
        .min(NAME_ROOM);
    field[len..].fill(0);
}

/// England's teams: the squads, the career club among them, and the team numbers assigned to
/// the selected teams in turn.
pub(crate) struct England {
    /// The squads.
    pub(crate) squads: Squads,
    /// The team numbers assigned to the selected teams in turn.
    numbers: Vec<[u8; 2]>,
}

/// England's squads of the real team file, numbered by their ordinals ([`england_numbered`]).
pub(crate) fn england() -> England {
    england_numbered(&[])
}

/// England's squads of the real team file: the selected teams cycled through its teams, each
/// of the number `numbers` gives it in turn, else `[0, n]`; the career club its first team
/// with its second's players as reserves.
pub(crate) fn england_numbered(numbers: &[[u8; 2]]) -> England {
    let bytes = std::fs::read(assets_path().join("TEAM/TEAM.008")).expect("England's team file");
    let file = TeamFileFormat::import(&bytes).expect("a team file");
    let image = |index: usize| -> Vec<u8> {
        MatchTeam::from_team(&file.teams[index % file.teams.len()])
            .image()
            .to_vec()
    };
    let selected = (0..SELECTED)
        .map(|index| {
            let mut image = image(index);
            let ordinal = u8::try_from(index).expect("160");
            let number = numbers.get(index).copied().unwrap_or([0, ordinal]);
            image[TEAM_ID].copy_from_slice(&number);
            image[GLOBAL_NUMBER].copy_from_slice(&u16::from(ordinal).to_le_bytes());
            image[CONTROLS] = CONTROLS_BY_TURN[index % CONTROLS_BY_TURN.len()];
            image[MATCH_FLAGS] |= SIMULATED_FLAG;
            normalised_name(&mut image);
            MatchTeam::selected(image.as_slice().try_into().expect("a record"))
        })
        .collect();
    let mut career = image(0);
    career.extend_from_slice(&image(1)[PLAYERS..]);
    career[TEAM_ID].copy_from_slice(&CAREER_TEAM);
    career[GLOBAL_NUMBER].copy_from_slice(&CAREER_NUMBER.to_le_bytes());
    career[CONTROLS] = CAREER_CONTROLS;
    career[MATCH_FLAGS] |= SIMULATED_FLAG;
    normalised_name(&mut career);
    let line_up = CareerLineUp {
        order: std::array::from_fn(|place| {
            u8::try_from((place + CAREER_SHIFT) % CAREER_LINE_UP).expect("32")
        }),
        players: CAREER_PLAYERS,
    };
    let career: [u8; CAREER_RECORD_LEN] = career.try_into().expect("the career club's record");
    England {
        squads: Squads {
            selected,
            career: Some(MatchTeam::career(&career, line_up)),
        },
        numbers: numbers.to_vec(),
    }
}

/// A squad's valued players' prices.
fn prices(team: &MatchTeam) -> Prices {
    std::array::from_fn(|slot| team.image()[PLAYERS + slot * PLAYER_LEN + PRICE])
}

/// A squad's name, as the oracle writes it.
fn name(team: &MatchTeam) -> Vec<u8> {
    let field = &team.image()[NAME];
    let len = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(NAME_ROOM);
    field[..len].to_vec()
}

/// The core's view of a squad of `identity`.
fn loaded(identity: i32, team: &MatchTeam) -> LoadedTeam {
    LoadedTeam {
        identity,
        controls: TeamControls::try_from(team.controls()).expect("a control"),
        prices: prices(team),
    }
}

impl England {
    /// The career club.
    fn career(&self) -> &MatchTeam {
        self.squads.career.as_ref().expect("a career club")
    }

    /// The selected teams' and the career club's prices and controls, as the oracle's
    /// competitions write them.
    pub(crate) fn selected_teams(&self) -> (Vec<SelectedTeam>, SelectedTeam) {
        let view = |team: &MatchTeam| SelectedTeam {
            prices: prices(team),
            controls: team.controls(),
        };
        (
            self.squads.selected.iter().map(view).collect(),
            view(self.career()),
        )
    }

    /// The selected teams' prices.
    pub(crate) fn prices(&self) -> Vec<Prices> {
        self.squads.selected.iter().map(prices).collect()
    }

    /// The career club's prices.
    pub(crate) fn career_prices(&self) -> Prices {
        prices(self.career())
    }

    /// The teams' names and numbers, as the oracle's leagues write them.
    pub(crate) fn names(&self) -> Teams {
        let number = |team: &MatchTeam| team.global_number().cast_unsigned();
        Teams {
            selected: self
                .squads
                .selected
                .iter()
                .map(|team| (name(team), number(team)))
                .collect(),
            career: (name(self.career()), number(self.career())),
        }
    }

    /// The loader the stub is: the career club for its country, else the selected team the
    /// number is assigned to, else the one of its ordinal.
    pub(crate) fn loader(&self) -> Loader {
        let selected: Vec<LoadedTeam> = self
            .squads
            .selected
            .iter()
            .enumerate()
            .map(|(index, team)| loaded(i32::try_from(index).expect("160"), team))
            .collect();
        let career = loaded(-1, self.career());
        let numbers = self.numbers.clone();
        Box::new(move |team| {
            let [country, ordinal] = team;
            Some(if country == CAREER_TEAM[0] {
                career
            } else {
                let assigned = numbers.iter().position(|&number| number == team);
                selected[assigned.unwrap_or(usize::from(ordinal) % SELECTED)]
            })
        })
    }
}

/// The session's rules a competition's index varies.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Variant {
    /// The kind of game.
    game: GameType,
    /// Whether a world opens a computer club's cards, its injuries.
    computer: (bool, bool),
    /// Whether a match's statuses apply.
    statuses_apply: bool,
    /// The day's season and seed.
    day: (u16, u8),
}

/// The kinds of game by turns.
const GAMES: [GameType; 4] = [
    GameType::Competition,
    GameType::Competition,
    GameType::Season,
    GameType::Career,
];

/// The computer clubs' statuses a world opens, by turns.
const COMPUTER: [(bool, bool); 4] = [(false, false), (true, true), (true, false), (false, true)];

/// The turns of a match's statuses not applied: every third competition.
const STATUSES_TURNS: usize = 3;

/// The session of competition `index`.
pub(crate) fn variant(index: usize) -> Variant {
    let small = u8::try_from(index % 256).expect("a byte");
    Variant {
        game: GAMES[index % GAMES.len()],
        computer: COMPUTER[index / GAMES.len() % COMPUTER.len()],
        statuses_apply: index % STATUSES_TURNS != STATUSES_TURNS - 1,
        day: (SEASON + u16::from(small), small.wrapping_mul(37)),
    }
}

/// `game`'s `_gameType` (a competition's a DIY one's).
fn game_type_code(game: GameType) -> u16 {
    match game {
        GameType::Competition => 1,
        GameType::Season => 3,
        GameType::Career => 4,
    }
}

impl Variant {
    /// The core's session over `tactics` and their `orders` (its month the competition's).
    pub(crate) fn session<'a>(
        &self,
        tactics: &'a [TeamTactics],
        orders: &'a [[u8; ELEVEN]],
    ) -> Session<'a> {
        let (cards, injuries) = self.computer;
        let (season, seed) = self.day;
        Session {
            tactics,
            orders,
            rules: SimRules {
                status: StatusRules {
                    mode: 0,
                    training: false,
                    no_injuries_cards: false,
                },
                game: self.game,
                computer: ComputerStatuses { cards, injuries },
            },
            statuses_apply: self.statuses_apply,
            national_selector: false,
            day: Day {
                season,
                seed,
                phase: 0,
            },
        }
    }

    /// Upstream's session of `england`'s squads over the tactics `records`.
    pub(crate) fn oracle_session(
        &self,
        england: &England,
        records: &[[u8; TACTICS_LEN]],
    ) -> SimSession {
        let career = england.career();
        let line_up = career.career_line_up().expect("the career club's line-up");
        let (season, seed) = self.day;
        SimSession {
            selected: england
                .squads
                .selected
                .iter()
                .map(|team| team.image().to_vec())
                .collect(),
            career: Some(Side {
                record: career.image().to_vec(),
                career: Some((line_up.order, line_up.players)),
            }),
            tactics: records.to_vec(),
            status: Status {
                mode: 0,
                training: false,
                testers_rule: false,
            },
            game_type: game_type_code(self.game),
            computer: self.computer,
            statuses_apply: self.statuses_apply,
            national_selector: false,
            season,
            seed,
        }
    }
}

/// The first byte where `ours` and `theirs` differ, as a message.
fn differs(ours: &[u8], theirs: &[u8], what: &str) -> Result<(), String> {
    if ours == theirs {
        return Ok(());
    }
    let offset = ours
        .iter()
        .zip(theirs)
        .position(|(mine, upstream)| mine != upstream)
        .unwrap_or(ours.len().min(theirs.len()));
    Err(format!(
        "{what}: byte {offset}: {:?} != {:?}",
        ours.get(offset),
        theirs.get(offset)
    ))
}

/// The port's squads and upstream's alike.
pub(crate) fn squads_alike(oracle: &mut Oracle, squads: &Squads) -> Result<(), String> {
    let career_len = squads.career.as_ref().map(|team| team.image().len());
    let (selected, career) = oracle.squads(squads.selected.len(), career_len);
    for (index, (ours, theirs)) in squads.selected.iter().zip(&selected).enumerate() {
        differs(ours.image(), theirs, &format!("squad {index}"))?;
    }
    if let (Some(ours), Some(theirs)) = (&squads.career, &career) {
        differs(ours.image(), theirs, "the career club")?;
    }
    Ok(())
}

/// The port's second generator and upstream's alike.
pub(crate) fn generators_alike(oracle: &mut Oracle, rng: &Rng) -> Result<(), String> {
    let RandState {
        seed,
        xor_index,
        xor_key,
    } = oracle.rand2_state();
    let ours = (
        rng.secondary.seed,
        rng.secondary.xor_index,
        rng.secondary.xor_key,
    );
    if ours == (seed, xor_index, xor_key) {
        Ok(())
    } else {
        Err(format!(
            "the second generators: {ours:?} != {:?}",
            (seed, xor_index, xor_key)
        ))
    }
}

/// A source asked through, the parts asked recorded.
pub(crate) struct Recorded<'a, S> {
    /// The source.
    pub(crate) source: &'a mut S,
    /// The parts asked.
    pub(crate) taken: Taken,
}

impl<'a, S> Recorded<'a, S> {
    /// `source`, nothing asked yet.
    pub(crate) fn new(source: &'a mut S) -> Self {
        Self {
            source,
            taken: Taken::default(),
        }
    }
}

/// The simulation's mode flag for a neutral ground (upstream's 0xFF).
const NEUTRAL: u16 = 0xFF;

impl<S: MatchResults> MatchResults for Recorded<'_, S> {
    fn normal_time(
        &mut self,
        competition: &RunningCompetition,
        sides: Sides,
        setup: &MatchSetup,
        rng: &mut Rng,
    ) -> Result<Score, SimError> {
        self.taken.normal = true;
        self.taken.venue = Some(match setup.venue {
            Venue::Home => 0,
            Venue::Neutral => NEUTRAL,
        });
        self.source.normal_time(competition, sides, setup, rng)
    }

    fn extra_time(&mut self, sides: Sides, rng: &mut Rng) -> Result<Score, SimError> {
        self.taken.extra = true;
        self.source.extra_time(sides, rng)
    }

    fn penalties(&mut self, sides: Sides, rng: &mut Rng) -> Result<Score, SimError> {
        self.taken.penalties = true;
        self.source.penalties(sides, rng)
    }
}
