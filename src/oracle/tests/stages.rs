//! `openswos_core::competition::tournament::next_stage` against upstream's
//! `DIY_AdvanceToNextCompetitionSlot` (src/career_teams.c) and the stage ends and loads it calls
//! (src/diy_competition.c `DIY_BuildSeededBracketAndUpdateScores`,
//! `DIY_AdjustWorldCupBracketScores`, `DIY_CopyTeamGridFromCompToDIY`,
//! `DIY_LoadCurrentCompetitionSlot`): tournaments played through as the career plays them,
//! every match, every stage's end and every next stage alike on both sides (both images byte
//! for byte, the C's name pointers aside, and the second generators); random tournaments whose
//! stages mostly follow each other, the own draws' ids among them, and every tournament of World
//! 1996-97. Without upstream's checkout the oracle is not built and these tests are not either
//! (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

mod tournaments;

use openswos_competitions_json_resource::SymbolId;
use openswos_core::competition::cup::{
    CupFixture, CupPlayError, MatchResults, Venue, next_round, play_cup_match,
};
use openswos_core::competition::league::{PlayError, play_result};
use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition};
use openswos_core::competition::schedule::Schedule;
use openswos_core::competition::start::Pitch;
use openswos_core::competition::table::Score;
use openswos_core::competition::tournament::{
    Advance, LoadError, NextStageError, ResetNames, STAGES, STATE_LEN, StageRules, TEAMS,
    TournamentStart, TournamentState, next_stage, start_tournament,
};
use openswos_core::world::World;
use openswos_oracle::Oracle;
use openswos_oracle::cup::SelectedTeam;
use openswos_oracle::cup_play::{CachedResults, CupMatchSetup, Taken};
use openswos_oracle::play::{ResultSetup, Teams};
use openswos_oracle::rng::RandState;
use openswos_oracle::stage::StageSetup;
use openswos_oracle::tables::country;
use openswos_oracle::tournament::{TournamentBytes, TournamentSetup};
use openswos_rng::{Generator, Rng};
use proptest::collection::vec;
use proptest::prelude::*;
use tournaments::{
    DIY_CUP, DIY_LEAGUE, DIY_TOURNAMENT, LEAGUE, MIN_TOURNAMENTS, NAME_POINTER, SEASONAL,
    SELECTED_TEAMS, WORLD, core_record, own_draw, record, selected, selected_loader, start_9697,
    tournaments, world_teams,
};

/// The identity of the career club among the teams.
const CAREER_CLUB: i32 = -1;

/// The career club's global team number here.
const CAREER_NUMBER: u16 = 1000;

/// The European Championships' ids: their groups play the championships' own round robin
/// (diy_competition.c `getCurrentFixtureTeamPtrs`).
const EURO_CHAMPIONSHIPS: [u8; 2] = [21, 22];

/// The simulation's mode flag for a neutral ground (upstream's 0xFF).
const NEUTRAL: u16 = 0xFF;

/// The most matches a tournament plays here before it is called endless.
const MAX_MATCHES: usize = 5000;

/// The match from which every result is decisive, so replays end.
const DECISIVE_FROM: usize = 2000;

/// Upstream's halt when no pattern of the World Cup's own draw fits its best placed.
const NO_PATTERN: u16 = 0x8034;

/// The times a stage of groups' teams play each other here: few, so a tournament plays fast.
const PLAYS: std::ops::RangeInclusive<i8> = 1..=2;

/// A stage's kind, for [`plausible`]: groups (their count and their teams), or a knockout
/// (doubled or not).
#[derive(Debug, Clone, Copy)]
enum Kind {
    /// Groups.
    Groups(i8, i8),
    /// A knockout, doubled or not.
    Knockout(bool),
}

/// A stage's kind: groups of 2 to 6 in 1 to 8, or a knockout, doubled a time in six.
fn kind() -> impl Strategy<Value = Kind> {
    prop_oneof![
        (1..=8i8, 2..=6i8).prop_map(|(groups, per_group)| Kind::Groups(groups, per_group)),
        prop_oneof![5 => Just(false), 1 => Just(true)].prop_map(Kind::Knockout),
    ]
}

/// A stage drawn: its kind, a pick (what a stage of groups sends on, a knockout's teams, a
/// noise's teams), and whether its teams are noise (any, a time in eight).
fn drawn_stage() -> impl Strategy<Value = (Kind, u8, bool)> {
    (
        kind(),
        any::<u8>(),
        prop_oneof![7 => Just(false), 1 => Just(true)],
    )
}

/// The groups of 1 to 8 of 2 to 8 teams that hold `teams` exactly, the pick's of them.
fn groups_of(teams: i8, pick: u8) -> Option<(i8, i8)> {
    let fits: Vec<(i8, i8)> = (1..=8i8)
        .filter(|&groups| teams % groups == 0 && (2..=8).contains(&(teams / groups)))
        .map(|groups| (groups, teams / groups))
        .collect();
    (!fits.is_empty()).then(|| fits[usize::from(pick) % fits.len()])
}

/// The stages' bytes and the last stage's teams through: each stage's teams what the stage
/// before sends on (a knockout's winners, some of a stage of groups' teams), unless noise; a
/// stage of groups the groups that hold its teams (a knockout when none do).
fn plausible(stages: &[(Kind, u8, bool)]) -> ([[i8; 3]; STAGES], i8) {
    let mut rounds = [[0i8; 3]; STAGES];
    let (first, first_pick, _) = stages[0];
    let mut teams = match first {
        Kind::Groups(groups, per_group) => groups * per_group,
        Kind::Knockout(_) => 2 + i8::try_from(first_pick % 63).expect("below 63"),
    };
    let mut kind = first;
    let mut through = 0;
    for (stage, round) in rounds.iter_mut().enumerate() {
        *round = match kind {
            Kind::Groups(groups, per_group) => [teams, groups, per_group],
            Kind::Knockout(doubled) => [teams, if doubled { -1 } else { 0 }, 0],
        };
        let pick = stages[stage].1;
        through = match kind {
            Kind::Groups(..) => {
                1 + i8::try_from(u16::from(pick) % u16::from(teams.cast_unsigned().max(1)))
                    .expect("a byte")
            }
            Kind::Knockout(_) => (teams / 2).max(1),
        };
        let Some(&(next, next_pick, noise)) = stages.get(stage + 1) else {
            break;
        };
        teams = if noise {
            1 + i8::try_from(next_pick % 64).expect("below 64")
        } else {
            through
        };
        kind = match next {
            Kind::Groups(..) => groups_of(teams, next_pick)
                .map_or(Kind::Knockout(false), |(groups, per_group)| {
                    Kind::Groups(groups, per_group)
                }),
            Kind::Knockout(doubled) => Kind::Knockout(doubled),
        };
    }
    (rounds, through)
}

/// A tournament to play: [`record`]'s, its stages [`plausible`] and their plays few.
fn tournament() -> impl Strategy<Value = (TournamentBytes, Vec<Option<usize>>)> {
    (record(), vec(drawn_stage(), STAGES), PLAYS).prop_map(|((mut bytes, seeds), drawn, plays)| {
        let drawn_count = usize::try_from(bytes.round_string_base).expect("1 to 6");
        let (rounds, through) = plausible(&drawn[..drawn_count]);
        // The stages end at the final: none of a team alone after it.
        let count = rounds[..drawn_count]
            .iter()
            .position(|&[teams, ..]| teams < 2)
            .unwrap_or(drawn_count)
            .max(1);
        bytes.round_string_base = i8::try_from(count).expect("1 to 6");
        bytes.rounds = rounds;
        if count == STAGES {
            bytes.reserved = through;
        } else if count == drawn_count {
            bytes.rounds[count][0] = through;
        }
        bytes.play_each_team = plays;
        (bytes, seeds)
    })
}

/// A team's name: 1 to 16 letters.
fn team_name() -> impl Strategy<Value = Vec<u8>> {
    vec(b'A'..=b'Z', 1..=16)
}

/// The teams' names: each selected team a name and its ordinal as its number, the career club
/// too.
fn names() -> impl Strategy<Value = Teams> {
    (vec(team_name(), SELECTED_TEAMS), team_name()).prop_map(|(names, career)| Teams {
        selected: names.into_iter().zip(0..).collect(),
        career: (career, CAREER_NUMBER),
    })
}

/// The core's name of identity `identity` among `names`.
fn name_of(names: &Teams) -> impl Fn(i32) -> Option<Vec<u8>> + '_ {
    |identity| match identity {
        CAREER_CLUB => Some(names.career.0.clone()),
        _ => usize::try_from(identity)
            .ok()
            .and_then(|index| names.selected.get(index))
            .map(|(name, _)| name.clone()),
    }
}

/// Match `number`'s results, drawn from `seed`: goals 0 to 3 (decisive from
/// [`DECISIVE_FROM`]), extra time 0 to 2, a shoot-out 0 to 9.
fn results_of(seed: u64, number: usize) -> CachedResults {
    let mut state =
        seed ^ (u64::try_from(number).expect("a match") + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut next = |bound: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        u16::try_from(state % bound).expect("small")
    };
    let normal = if number >= DECISIVE_FROM {
        (1, 0)
    } else {
        (next(4), next(4))
    };
    CachedResults {
        normal,
        extra: (next(3), next(3)),
        penalties: (next(10), next(10)),
    }
}

/// A match's results as the core's source: each part, and which were asked.
struct Given {
    /// The results.
    results: CachedResults,
    /// The parts asked.
    taken: Taken,
}

impl MatchResults for Given {
    fn normal_time(&mut self, _fixture: CupFixture, venue: Venue) -> Score {
        self.taken.normal = true;
        self.taken.venue = Some(match venue {
            Venue::Home => 0,
            Venue::Neutral => NEUTRAL,
        });
        let (home, away) = self.results.normal;
        Score { home, away }
    }

    fn extra_time(&mut self, _fixture: CupFixture) -> Score {
        self.taken.extra = true;
        let (home, away) = self.results.extra;
        Score { home, away }
    }

    fn penalties(&mut self, _fixture: CupFixture) -> Score {
        self.taken.penalties = true;
        let (home, away) = self.results.penalties;
        Score { home, away }
    }
}

/// The career's copy of a stage's progress into the state before its advance (src/career_teams.c
/// `Career_UpdateHybridCompetitionProgressAndMaybeAdvanceSlot`, M7's): the same on both sides.
fn keep_stage_progress(
    state: &mut TournamentState,
    competition: &RunningCompetition,
    knockout: bool,
) {
    state.set_match_started(competition.match_started());
    state.set_career_team_involved(competition.career_team_involved());
    state.set_result_code(competition.result_code());
    if knockout {
        state.set_has_groups(competition.round_iterator());
    }
}

/// Both sides of a tournament under way: the port's state and competition, upstream's images.
struct Sides {
    /// The port's state.
    state: TournamentState,
    /// The port's competition under way.
    competition: RunningCompetition,
    /// Upstream's state.
    their_state: Box<[u8; STATE_LEN]>,
    /// Upstream's competition under way.
    their_competition: Box<[u8; IMAGE_LEN]>,
}

impl Sides {
    /// Both sides alike: the images (the name pointers aside).
    fn alike(&self, what: &str) -> Result<(), TestCaseError> {
        let mut theirs = *self.their_competition;
        theirs[NAME_POINTER].copy_from_slice(&self.competition.image()[NAME_POINTER]);
        for (offset, (&mine, &upstream)) in self
            .competition
            .image()
            .iter()
            .zip(theirs.iter())
            .enumerate()
        {
            prop_assert_eq!(mine, upstream, "{}: competition byte {}", what, offset);
        }
        let mut theirs = *self.their_state;
        theirs[NAME_POINTER].copy_from_slice(&self.state.image()[NAME_POINTER]);
        for (offset, (&mine, &upstream)) in self.state.image().iter().zip(theirs.iter()).enumerate()
        {
            prop_assert_eq!(mine, upstream, "{}: state byte {}", what, offset);
        }
        Ok(())
    }
}

/// How a tournament played through ended.
#[derive(Debug)]
enum Ended {
    /// It ended after its last stage, after this many matches.
    Finished(usize),
    /// The port refused a step (upstream runs past its memory, or reads garbage, there).
    Refused(String),
}

/// What a tournament is played with: its teams, their names, the world's rules for its stages,
/// its round robins.
struct Play<'a> {
    /// The selected teams.
    teams: &'a [SelectedTeam],
    /// The career club.
    career_team: SelectedTeam,
    /// The teams' names.
    names: &'a Teams,
    /// The rules of its stages' loads.
    rules: StageRules<'a>,
    /// The round robin of its groups of a size.
    schedule: &'a dyn Fn(u8) -> Option<Schedule>,
    /// The seed of its matches' results.
    seed: u64,
    /// Each stage's end changed on both sides before its advance: whether the teams through
    /// are made 1, whether the next stage's bracket is made doubled (none, as it is).
    stage_ends: &'a [Option<(bool, bool)>],
}

/// `state` changed for stage `stage`'s end as `change` says: its teams through 1, the next
/// stage's bracket doubled.
fn change_stage_end(
    state: &mut TournamentState,
    stage: usize,
    (through_one, doubled): (bool, bool),
) {
    if through_one {
        state.set_slot_division_factor(stage, 1);
    }
    if doubled && TournamentState::has_slot(stage + 1) {
        state.set_slot_double_count(stage + 1, 1);
    }
}

/// Whether a league's refusal is one where upstream runs past its memory or reads garbage.
fn league_refusal(error: &PlayError) -> bool {
    !matches!(error, PlayError::UnknownTeam(_))
}

/// Whether a cup's refusal is one where upstream runs past its bracket.
fn cup_refusal(error: &CupPlayError) -> bool {
    !matches!(error, CupPlayError::UnknownTeam(_))
}

/// The current stage of groups played to its end on both sides.
fn play_groups(
    oracle: &mut Oracle,
    sides: &mut Sides,
    play: &Play<'_>,
    number: &mut usize,
) -> Result<Option<Ended>, TestCaseError> {
    let size = u8::try_from(sides.competition.total_matches_in_round()).ok();
    let Some(schedule) = size.and_then(play.schedule) else {
        return Ok(Some(Ended::Refused("no round robin".to_owned())));
    };
    while sides.competition.min_substitutes() != 0 {
        prop_assert!(*number < MAX_MATCHES, "a tournament that never ends");
        let goals = results_of(play.seed, *number).normal;
        let played = play_result(
            &mut sides.competition,
            &mut sides.state,
            &schedule,
            Score {
                home: goals.0,
                away: goals.1,
            },
            name_of(play.names),
        );
        if let Err(error) = &played {
            prop_assert!(league_refusal(error), "match {}: {:?}", number, error);
            return Ok(Some(Ended::Refused(format!("{error:?}"))));
        }
        let (after, state_after) = oracle.play_result(&ResultSetup {
            competition: sides.their_competition.clone(),
            state: sides.their_state.clone(),
            teams: play.names.clone(),
            goals,
        });
        (sides.their_competition, sides.their_state) = (after, state_after);
        sides.alike(&format!("match {number}"))?;
        *number += 1;
    }
    Ok(None)
}

/// The current knockout played to its end on both sides: its matches until a round is
/// complete, then the next round unless it is finished.
fn play_knockout(
    oracle: &mut Oracle,
    sides: &mut Sides,
    play: &Play<'_>,
    rng: &mut Rng,
    number: &mut usize,
) -> Result<Option<Ended>, TestCaseError> {
    loop {
        prop_assert!(*number < MAX_MATCHES, "a tournament that never ends");
        let results = results_of(play.seed, *number);
        let mut given = Given {
            results,
            taken: Taken::default(),
        };
        let played = play_cup_match(
            &mut sides.competition,
            &mut sides.state,
            &mut given,
            selected_loader(play.teams, &play.career_team),
            rng,
        );
        if let Err(error) = &played {
            prop_assert!(cup_refusal(error), "match {}: {:?}", number, error);
            return Ok(Some(Ended::Refused(format!("{error:?}"))));
        }
        let (after, state_after, taken) = oracle.play_cup_match(&CupMatchSetup {
            competition: sides.their_competition.clone(),
            state: sides.their_state.clone(),
            teams: play.teams.to_vec(),
            career_team: play.career_team,
            results,
        });
        (sides.their_competition, sides.their_state) = (after, state_after);
        prop_assert_eq!(given.taken, taken, "match {}: the parts taken", number);
        sides.alike(&format!("match {number}"))?;
        *number += 1;
        if sides.competition.round_complete() == 0 {
            continue;
        }
        if sides.competition.competition_finished() != 0 {
            return Ok(None);
        }
        if let Err(error) = next_round(&mut sides.competition) {
            prop_assert!(cup_refusal(&error), "after match {}: {:?}", number, error);
            return Ok(Some(Ended::Refused(format!("{error:?}"))));
        }
        sides.their_competition = oracle.next_round(&sides.their_competition, &sides.their_state);
        sides.alike(&format!("after match {number}"))?;
    }
}

/// The two second generators alike.
fn generators_alike(oracle: &mut Oracle, rng: &Rng, what: &str) -> Result<(), TestCaseError> {
    let state = oracle.rand2_state();
    prop_assert_eq!(
        (
            rng.secondary.seed,
            rng.secondary.xor_index,
            rng.secondary.xor_key
        ),
        (state.seed, state.xor_index, state.xor_key),
        "{}: the second generators",
        what
    );
    Ok(())
}

/// Plays the tournament started on both `sides` to its end, as the career does: each stage's
/// matches, its progress kept, then its advance; each step alike on both sides.
fn play_through(
    oracle: &mut Oracle,
    sides: &mut Sides,
    play: &Play<'_>,
    rng: &mut Rng,
) -> Result<Ended, TestCaseError> {
    let mut number = 0;
    for stage in 0.. {
        prop_assert!(stage <= STAGES, "more stages than a tournament holds");
        let current = usize::try_from(sides.state.round_complete()).expect("a stage");
        let knockout = sides.state.slot_is_league_format(current) == 0;
        let ended = if knockout {
            play_knockout(oracle, sides, play, rng, &mut number)?
        } else {
            play_groups(oracle, sides, play, &mut number)?
        };
        if let Some(ended) = ended {
            return Ok(ended);
        }
        keep_stage_progress(&mut sides.state, &sides.competition, knockout);
        let mut their_state = TournamentState::from_image(&sides.their_state);
        let their_competition = RunningCompetition::from_image(&sides.their_competition);
        keep_stage_progress(&mut their_state, &their_competition, knockout);
        if let Some(change) = play.stage_ends.get(current).copied().flatten() {
            change_stage_end(&mut sides.state, current, change);
            change_stage_end(&mut their_state, current, change);
        }
        let advanced = next_stage(
            &mut sides.state,
            &mut sides.competition,
            &play.rules,
            selected_loader(play.teams, &play.career_team),
            name_of(play.names),
            rng,
        );
        let what = format!("stage {current}'s end");
        // Refused where upstream runs past its memory: not run, unless upstream halts there.
        let halts = matches!(
            advanced,
            Err(NextStageError::Load(LoadError::NoBestPlacedPattern(_)))
        );
        if let Err(error) = &advanced
            && !halts
        {
            return Ok(Ended::Refused(format!("{what}: {error:?}")));
        }
        let upstream = oracle.next_stage(&StageSetup {
            state: Box::new(*their_state.image()),
            competition: sides.their_competition.clone(),
            teams: play.teams.to_vec(),
            career_team: play.career_team,
            names: play.names.clone(),
        });
        match (advanced, upstream) {
            (Err(_), Err(halt)) => {
                prop_assert_eq!(halt, NO_PATTERN, "{}", what);
                return Ok(Ended::Refused("no pattern".to_owned()));
            }
            (Err(error), Ok(_)) => {
                return Err(TestCaseError::fail(format!(
                    "{what}: the port refuses ({error:?}), upstream goes on"
                )));
            }
            (Ok(advance), Err(halt)) => {
                return Err(TestCaseError::fail(format!(
                    "{what}: upstream halts ({halt:#06X}), the port {advance:?}"
                )));
            }
            (Ok(advance), Ok((state, competition))) => {
                (sides.their_state, sides.their_competition) = (state, competition);
                sides.alike(&what)?;
                generators_alike(oracle, rng, &what)?;
                match advance {
                    Advance::Ended => return Ok(Ended::Finished(number)),
                    Advance::Loaded => {}
                    Advance::Over => {
                        return Err(TestCaseError::fail(format!("{what}: over before")));
                    }
                }
            }
        }
    }
    unreachable!("the stages are bounded")
}

/// The round robin upstream plays groups of `teams` with in a competition of `comp_id`: the
/// European Championships' own, else its size's.
fn schedule(comp_id: u8, teams: u8) -> Option<Schedule> {
    let world = &*WORLD;
    let found = if EURO_CHAMPIONSHIPS.contains(&comp_id) {
        let euro = world
            .competition(&SymbolId("_europeanChampionships".to_owned()))
            .expect("the European Championships");
        world.schedules().of(euro, teams)
    } else {
        world.schedules().of_size(teams)
    };
    found.cloned()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn a_tournament_plays_through_as_upstream_plays_it(
        (mut record, seeds) in tournament(),
        teams in vec(selected(), SELECTED_TEAMS),
        career_team in selected(),
        names in names(),
        rand in any::<(u8, u8, u8)>(),
        seed in any::<u64>(),
        stage_ends in vec(prop_oneof![5 => Just(None), 1 => any::<(bool, bool)>().prop_map(Some)], STAGES),
    ) {
        // The state's teams: the selected teams in order, so every team differs.
        let mut before = TournamentState::default();
        for index in 0..TEAMS {
            before.set_team(index, [0, u8::try_from(index).expect("64")]);
        }
        record.seeds = seeds
            .iter()
            .map(|&pick| pick.map_or([0, 0], |index| before.team(index)))
            .collect();
        let core_record = core_record(&record);
        let adjective = country(record.header[2]).expect("built").map(|entry| entry.adjective);
        let name = record.names[0].clone().unwrap_or_else(|| LEAGUE.to_vec());
        let start = TournamentStart {
            record: &core_record,
            name: &name,
            prefix: adjective.as_deref(),
            reset_names: ResetNames { tournament: DIY_TOURNAMENT, league: DIY_LEAGUE, cup: DIY_CUP },
            continental: false,
            own_draw: own_draw(record.header[0]),
            odd_groups: true,
            pitch: Pitch::Seasonal,
        };
        let (rand_seed, xor_index, xor_key) = rand;
        let mut rng = Rng::default();
        rng.secondary = Generator { seed: rand_seed, xor_index, xor_key };
        let mut state = before.clone();
        let mut competition = RunningCompetition::default();
        let started = start_tournament(
            &mut state,
            &mut competition,
            &start,
            selected_loader(&teams, &career_team),
            &mut rng,
        );
        // Refused where upstream runs past its memory: not run (the start's check).
        prop_assume!(started.is_ok(), "{:?}", started);
        let mut oracle = Oracle::lock();
        oracle.set_rand2_state(RandState { seed: rand_seed, xor_index, xor_key });
        let (their_state, their_competition) = oracle
            .start_tournament(&TournamentSetup {
                state: Box::new(*before.image()),
                competition: Box::new([0; IMAGE_LEN]),
                record: record.clone(),
                national: false,
                continental: false,
                odd_groups: true,
                pitch_type: SEASONAL,
                teams: teams.clone(),
                career_team,
            })
            .expect("started as the port starts");
        let mut sides = Sides { state, competition, their_state, their_competition };
        sides.alike("started")?;
        let comp_id = record.header[0];
        let round_robin = |teams: u8| schedule(comp_id, teams);
        let play = Play {
            teams: &teams,
            career_team,
            names: &names,
            rules: start.stage_rules(),
            schedule: &round_robin,
            seed,
            stage_ends: &stage_ends,
        };
        play_through(&mut oracle, &mut sides, &play, &mut rng)?;
    }
}

/// The names of World 1996-97's check: `TEAM` and the ordinal.
fn world_names() -> Teams {
    Teams {
        selected: (0..SELECTED_TEAMS)
            .map(|ordinal| {
                (
                    format!("TEAM {ordinal:03}").into_bytes(),
                    u16::try_from(ordinal).expect("160"),
                )
            })
            .collect(),
        career: (b"CAREER".to_vec(), CAREER_NUMBER),
    }
}

#[test]
fn world_1996_97s_tournaments_play_through_as_upstream_s() {
    let world: &World = &WORLD;
    let (teams, career_team) = world_teams();
    let names = world_names();
    let mut oracle = Oracle::lock();
    let mut finished = 0;
    for (index, tournament) in tournaments(world).iter().enumerate() {
        let both = start_9697(&mut oracle, world, tournament, &teams, career_team);
        let id = world.competition_id(tournament.competition).as_str();
        let Ok((their_state, their_competition)) = both.upstream else {
            // Odd groups in the original's world: upstream halts, the port refuses (the
            // start's check).
            continue;
        };
        both.result
            .unwrap_or_else(|error| panic!("{id}: {error:?}"));
        let start = both.resolved.start(Pitch::Seasonal);
        let mut sides = Sides {
            state: both.state,
            competition: both.competition,
            their_state,
            their_competition,
        };
        let competition = tournament.competition;
        let round_robin = |size: u8| world.schedules().of(competition, size).cloned();
        let play = Play {
            teams: &teams,
            career_team,
            names: &names,
            rules: start.stage_rules(),
            schedule: &round_robin,
            seed: u64::try_from(index).expect("a few"),
            stage_ends: &[],
        };
        let mut rng = both.rng;
        let ended = play_through(&mut oracle, &mut sides, &play, &mut rng)
            .unwrap_or_else(|error| panic!("{id} ({:?}): {error}", tournament.source));
        match ended {
            Ended::Finished(matches) => {
                assert!(matches > 0, "{id}: no match played");
                finished += 1;
            }
            Ended::Refused(why) => panic!("{id} ({:?}): refused: {why}", tournament.source),
        }
    }
    assert!(finished >= MIN_TOURNAMENTS, "{finished} played through");
}
