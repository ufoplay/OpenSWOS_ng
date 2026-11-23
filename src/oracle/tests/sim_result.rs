//! `openswos_core::sim`'s simulated result against upstream's src/match_sim.c (M5 #8c): from any
//! tactics, each formation's defensive order (src/menus.c `InitializeTacticsPositions`) alike;
//! from any league, any two sides (selected teams' records or the career club's, its line-up),
//! any setup (venue, substitutes, goals' column), any tactics, status rules, game, computer
//! statuses, career's answers, day and state of the second generator (pinned now and then), a
//! simulated match (`CalculateViewResult`), its extra time (`ExtraTimeSecondPhaseScoring`) and
//! its shoot-out (`PenaltyShootoutScorerPair`) alike: the goals, both records after byte for
//! byte, the scorers, the generator after; a side not in its league refused as upstream halts
//! (0x6006); England's teams of the real team file (`assets/TEAM/TEAM.008`) in a league of their
//! 20, each match over many seeds, with upstream's compiled tactics. Cases the port refuses
//! (where upstream reads past its records, tables or memory, or divides by 0) are not run.
//! Without upstream's checkout the oracle is not built and these tests are not either
//! (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

mod sim_setup;

use openswos_common::Importer;
use openswos_core::competition::results::{MatchSetup, Venue};
use openswos_core::competition::running::RunningCompetition;
use openswos_core::sim::{
    CAREER_RECORD_LEN, CareerLineUp, ComputerStatuses, Day, GameType, MatchTeam, PLAYER_LEN,
    PLAYERS as PLAYERS_HELD, RECORD_LEN, Session, SimError, SimRules, StatusRules, countdown,
    extra_time, formation_orders, penalties, simulate,
};
use openswos_oracle::Oracle;
use openswos_oracle::cup::SelectedTeam;
use openswos_oracle::rng::RandState;
use openswos_oracle::sim::{CAREER_LINE_UP, DayGlobals, Status, TACTICS_LEN, TACTICS_SLOTS};
use openswos_oracle::sim_result::{CountdownSetup, Outcome, ResultSetup, Side};
use openswos_rng::{Generator, Rng};
use openswos_team_resource::TeamFileFormat;
use openswos_test_utils::assets_path;
use proptest::collection::vec;
use proptest::prelude::*;
use sim_setup::{
    CARDS_INJURIES, CONTROLS, LEAGUE_TEAMS, LINE_UP, NOT_IN_LEAGUE, PINNED, PLAYERS, PRICE,
    ROW_BYTES, SELECTED_TEAMS, SPOTS, TACTICS, TEAM_ID, core_status, core_tactics, game_type,
    game_type_code, game_type_of, league, loader, record, selected, squad_status, status, tactics,
    tactics_record, team,
};

/// The mode flag of a neutral ground (upstream's 0xFF).
const NEUTRAL: u16 = 0xFF;

/// The first row's games a real league has played here: enough for its ranks to count.
const GAMES: u16 = 10;

/// The seeds a real match is simulated over.
const SEEDS: u8 = 32;

/// The places of a selected team's line-up.
const LINE_UP_LEN: usize = 16;

/// The formations a team's tactics may be (their defensive orders': upstream's 18).
const FORMATIONS: u8 = 18;

/// The highest price step.
const MAX_PRICE: u8 = 49;

/// The first substitute's line-up place: the stand-ins' first candidate.
const FIRST_SUBSTITUTE: usize = 11;

/// The offset of a player's position in his record.
const POSITION: usize = 26;

/// A position byte's looks and flags (its position, bits 7-5, cleared: a goalkeeper).
const NOT_POSITION: u8 = 0x1F;

/// A record whose line-up names its records, whose tactics are a formation, whose prices are
/// steps and whose players are mostly fit: `record`'s, cleaned; its first substitute a
/// goalkeeper when `bench_keeper` says so (the stand-ins' keeper odds then drawn).
fn cleaned(
    mut record: Vec<u8>,
    places: &[u8],
    (tactics, bench_keeper): (u8, bool),
    players: &[(u8, u8)],
) -> Vec<u8> {
    record[LINE_UP..LINE_UP + LINE_UP_LEN].copy_from_slice(places);
    record[TACTICS] = tactics;
    for (slot, &(price, status)) in players.iter().enumerate() {
        let at = PLAYERS + slot * PLAYER_LEN;
        if at + PLAYER_LEN <= record.len() {
            record[at + PRICE] = price;
            record[at + CARDS_INJURIES] = status;
        }
    }
    if bench_keeper {
        let at = PLAYERS + usize::from(places[FIRST_SUBSTITUTE]) * PLAYER_LEN + POSITION;
        record[at] &= NOT_POSITION;
    }
    record
}

/// A selected team's record: [`record`]'s, cleaned (its line-up's places its 16 records).
fn clean_record() -> impl Strategy<Value = Vec<u8>> {
    let records = u8::try_from(PLAYERS_HELD).expect("16");
    (
        record(RECORD_LEN, records),
        vec(0..records, LINE_UP_LEN),
        (0..FORMATIONS, any::<bool>()),
        vec((0..=MAX_PRICE, squad_status()), usize::from(records)),
    )
        .prop_map(|(record, places, tactics, players)| cleaned(record, &places, tactics, &players))
}

/// A side: a selected team's clean record, or the career club's (its line-up's places its 32
/// records); now and then [`team`]'s, noise and all.
fn side() -> impl Strategy<Value = (Vec<u8>, Option<([u8; CAREER_LINE_UP], u16)>)> {
    let career_records = u8::try_from(CAREER_LINE_UP).expect("32");
    let career = (
        record(CAREER_RECORD_LEN, career_records),
        vec(0..career_records, LINE_UP_LEN),
        (0..FORMATIONS, any::<bool>()),
        vec((0..=MAX_PRICE, squad_status()), CAREER_LINE_UP),
        vec(0..career_records, CAREER_LINE_UP),
        0..=u16::from(career_records),
    )
        .prop_map(|(record, places, tactics, prices, order, players)| {
            let record = cleaned(record, &places, tactics, &prices);
            (
                record,
                Some((order.try_into().expect("32 places"), players)),
            )
        });
    prop_oneof![
        5 => clean_record().prop_map(|record| (record, None)),
        2 => career,
        1 => team(),
    ]
}

/// A spot within the pitch grid.
fn grid_spot() -> impl Strategy<Value = u8> {
    (0..=14u8, 0..=15u8).prop_map(|(row, column)| row << 4 | column)
}

/// A session's tactics: their spots within the pitch grid mostly; now and then [`tactics`]'s.
fn session_tactics() -> impl Strategy<Value = Vec<[u8; TACTICS_LEN]>> {
    let clean = vec(
        (tactics_record(), vec(grid_spot(), SPOTS.len())).prop_map(|(mut record, spots)| {
            record[SPOTS].copy_from_slice(&spots);
            record
        }),
        TACTICS_SLOTS,
    );
    prop_oneof![7 => clean, 1 => tactics()]
}

/// The core's team of a side.
fn core_side(side: &Side) -> MatchTeam {
    match side.career {
        Some((order, players)) => MatchTeam::career(
            side.record
                .as_slice()
                .try_into()
                .expect("the career club's record"),
            CareerLineUp { order, players },
        ),
        None => MatchTeam::selected(side.record.as_slice().try_into().expect("a record")),
    }
}

/// The port's outcome of `setup`, or its refusal.
fn ours(
    setup: &ResultSetup,
    fixed: Option<u8>,
    rand: (u8, u8, u8),
) -> Result<(Outcome, Rng), SimError> {
    let competition = RunningCompetition::from_image(&setup.competition);
    let records = core_tactics(&setup.tactics);
    let orders = formation_orders(&records);
    let session = Session {
        tactics: &records,
        orders: &orders,
        rules: SimRules {
            status: core_status(setup.status),
            game: game_type_of(setup.game_type),
            computer: ComputerStatuses {
                cards: setup.computer.0,
                injuries: setup.computer.1,
            },
        },
        statuses_apply: setup.statuses_apply,
        national_selector: setup.national_selector,
        day: Day {
            season: setup.day.season,
            seed: setup.day.seed,
            phase: setup.day.phase,
        },
    };
    let match_setup = MatchSetup {
        venue: if setup.mode == 0 {
            Venue::Home
        } else {
            Venue::Neutral
        },
        substitutes: setup.substitutes,
        column: setup.column,
    };
    let (seed, xor_index, xor_key) = rand;
    let mut rng = Rng::default();
    rng.secondary = Generator {
        seed,
        xor_index,
        xor_key,
    };
    rng.fixed = fixed;
    let [mut home, mut away] = [core_side(&setup.sides[0]), core_side(&setup.sides[1])];
    let (score, mut simulated) = simulate(
        &competition,
        &match_setup,
        [&mut home, &mut away],
        loader(&setup.teams, &setup.career_team),
        &session,
        &mut rng,
    )?;
    let (with_extra_time, with_penalties) = setup.then;
    let extra = if with_extra_time {
        let extra = extra_time(&mut simulated, [&mut home, &mut away], &records, &mut rng)?;
        Some((extra.home, extra.away))
    } else {
        None
    };
    let shoot_out = with_penalties.then(|| {
        let shoot_out = penalties(&mut rng);
        (shoot_out.home.cast_signed(), shoot_out.away)
    });
    Ok((
        Outcome {
            goals: (score.home, score.away),
            extra_time: extra,
            penalties: shoot_out,
            records: [home.image().to_vec(), away.image().to_vec()],
            scorers: simulated.scorers,
        },
        rng,
    ))
}

/// Upstream's outcome of `setup`, its generators set as `rand` and `fixed` say, and its second
/// generator after.
fn theirs(
    oracle: &mut Oracle,
    setup: &ResultSetup,
    fixed: Option<u8>,
    (seed, xor_index, xor_key): (u8, u8, u8),
) -> (Result<Outcome, u16>, RandState) {
    oracle.set_rand2_state(RandState {
        seed,
        xor_index,
        xor_key,
    });
    oracle.set_fixed_random(fixed);
    let outcome = oracle.simulate(setup);
    oracle.set_fixed_random(None);
    (outcome, oracle.rand2_state())
}

/// Both sides' outcomes alike.
fn alike(
    (ours, rng): &(Outcome, Rng),
    (theirs, state): &(Outcome, RandState),
    what: &str,
) -> Result<(), TestCaseError> {
    prop_assert_eq!(ours.goals, theirs.goals, "{}: the goals", what);
    prop_assert_eq!(ours.extra_time, theirs.extra_time, "{}: extra time", what);
    prop_assert_eq!(ours.penalties, theirs.penalties, "{}: the shoot-out", what);
    for side in 0..2 {
        for (offset, (&mine, &upstream)) in ours.records[side]
            .iter()
            .zip(&theirs.records[side])
            .enumerate()
        {
            prop_assert_eq!(mine, upstream, "{}: side {}'s byte {}", what, side, offset);
        }
    }
    prop_assert_eq!(ours.scorers, theirs.scorers, "{}: the scorers", what);
    prop_assert_eq!(
        (
            rng.secondary.seed,
            rng.secondary.xor_index,
            rng.secondary.xor_key
        ),
        (state.seed, state.xor_index, state.xor_key),
        "{}: the second generator",
        what
    );
    Ok(())
}

proptest! {
    #[test]
    fn the_formations_orders_are_upstream_s(records in tactics()) {
        let ours = formation_orders(&core_tactics(&records));
        prop_assert_eq!(ours, Oracle::lock().defensive_orders(&records));
    }

    #[test]
    fn a_match_s_countdown_is_upstream_s(
        (home, home_career) in side(),
        away in prop_oneof![7 => clean_record(), 1 => record(RECORD_LEN, 16)],
        swapped in any::<bool>(),
        computer_sides in any::<(bool, bool)>(),
        mode in prop_oneof![2 => Just(0u16), 2 => Just(1u16), 1 => any::<u16>()],
        training in prop_oneof![3 => Just(false), 1 => Just(true)],
        game in game_type(),
        computer in any::<(bool, bool)>(),
        rand in any::<(u8, u8, u8)>(),
        fixed in prop_oneof![3 => Just(None), 1 => proptest::sample::select(&PINNED[..]).prop_map(Some)],
    ) {
        let mut sides = [
            Side { record: home, career: home_career },
            Side { record: away, career: None },
        ];
        if swapped {
            sides.swap(0, 1);
        }
        for (side, computer_side) in sides.iter_mut().zip([computer_sides.0, computer_sides.1]) {
            if computer_side {
                side.record[CONTROLS] = 1;
            }
        }
        let (mut home, mut away) = (core_side(&sides[0]), core_side(&sides[1]));
        let rules = SimRules {
            status: StatusRules { mode: mode.cast_signed(), training, no_injuries_cards: false },
            game,
            computer: ComputerStatuses { cards: computer.0, injuries: computer.1 },
        };
        let (seed, xor_index, xor_key) = rand;
        let mut rng = Rng::default();
        rng.secondary = Generator { seed, xor_index, xor_key };
        rng.fixed = fixed;
        let ours = countdown([&mut home, &mut away], &rules, &mut rng);
        // Refused where upstream reads past its records: not run.
        prop_assume!(ours.is_ok(), "{:?}", ours);
        let mut oracle = Oracle::lock();
        oracle.set_rand2_state(RandState { seed, xor_index, xor_key });
        oracle.set_fixed_random(fixed);
        let setup = CountdownSetup { mode, training, game_type: game_type_code(game), computer };
        let after = oracle.countdown(&sides, &setup);
        oracle.set_fixed_random(None);
        prop_assert_eq!(home.image(), &after[0][..], "the home side");
        prop_assert_eq!(away.image(), &after[1][..], "the away side");
        let state = oracle.rand2_state();
        prop_assert_eq!(
            (rng.secondary.seed, rng.secondary.xor_index, rng.secondary.xor_key),
            (state.seed, state.xor_index, state.xor_key),
            "the second generators"
        );
    }

    #[test]
    fn a_simulated_match_is_upstream_s(
        (image, ids) in league(),
        (home, home_career) in side(),
        away_record in prop_oneof![7 => clean_record(), 1 => record(RECORD_LEN, 16)],
        picks in (proptest::option::of(0..LEAGUE_TEAMS), proptest::option::of(0..LEAGUE_TEAMS)),
        mode in prop_oneof![Just(0u16), Just(NEUTRAL)],
        substitutes in prop_oneof![19 => (0..=5u16, 1..=5u16), 1 => any::<(u16, u16)>()],
        column in prop_oneof![19 => 0..=3u16, 1 => any::<u16>()],
        records in session_tactics(),
        teams in vec(selected(), SELECTED_TEAMS),
        career_team in selected(),
        status in status(),
        flags in (game_type(), any::<(bool, bool)>(), any::<bool>(), any::<bool>()),
        day in any::<(u16, u8, u8)>(),
        then in any::<(bool, bool)>(),
        rand in any::<(u8, u8, u8)>(),
        fixed in prop_oneof![3 => Just(None), 1 => proptest::sample::select(&PINNED[..]).prop_map(Some)],
    ) {
        let (game, computer, statuses_apply, national_selector) = flags;
        // The away side a selected team: one career club at most.
        let mut sides = [
            Side { record: home, career: home_career },
            Side { record: away_record, career: None },
        ];
        for (side, pick) in sides.iter_mut().zip([picks.0, picks.1]) {
            if let Some(index) = pick {
                side.record[TEAM_ID].copy_from_slice(&ids[index]);
            }
        }
        // The career club playing, the loader gives its own record's prices, as upstream's
        // finds them in `_careerTeam` where the call writes it.
        let mut career_team = career_team;
        if sides[0].career.is_some() {
            career_team.prices = std::array::from_fn(|slot| sides[0].record[PLAYERS + slot * PLAYER_LEN + PRICE]);
        }
        let (season, seed, phase) = day;
        let setup = ResultSetup {
            competition: Box::new(image),
            mode,
            substitutes,
            column,
            sides,
            tactics: records,
            teams,
            career_team,
            status,
            game_type: game_type_code(game),
            computer,
            statuses_apply,
            national_selector,
            day: DayGlobals { season, seed, phase },
            then,
        };
        let ours = ours(&setup, fixed, rand);
        // Refused where upstream reads past its records, tables or memory: not run.
        prop_assume!(
            matches!(ours, Ok(_) | Err(SimError::NotInCompetition(_))),
            "{:?}",
            ours.as_ref().err()
        );
        let mut oracle = Oracle::lock();
        let (upstream, state) = theirs(&mut oracle, &setup, fixed, rand);
        match ours {
            Err(_) => prop_assert_eq!(upstream.err(), Some(NOT_IN_LEAGUE)),
            Ok(ours) => {
                let upstream = upstream.map_err(|halt| TestCaseError::fail(format!("upstream halts: {halt:#06X}")))?;
                alike(&ours, &(upstream, state), "the match")?;
            }
        }
    }
}

#[test]
fn a_real_team_file_s_teams_play_their_simulated_matches_as_upstream_s() {
    let bytes = std::fs::read(assets_path().join("TEAM/TEAM.008")).expect("England's team file");
    let file = TeamFileFormat::import(&bytes).expect("a team file");
    let mut oracle = Oracle::lock();
    let records = oracle.compiled_tactics();
    // The league: England's first 20 teams, in order, the loader giving each its own prices.
    let mut competition = RunningCompetition::default();
    competition.set_num_teams(i16::try_from(LEAGUE_TEAMS).expect("20"));
    competition.set_substitutes_count(1);
    let mut teams = vec![
        SelectedTeam {
            prices: [0; 16],
            controls: 0,
        };
        SELECTED_TEAMS
    ];
    let league: Vec<MatchTeam> = file
        .teams
        .iter()
        .take(LEAGUE_TEAMS)
        .map(MatchTeam::from_team)
        .collect();
    for (index, team) in league.iter().enumerate() {
        let id = [team.country(), team.ordinal()];
        competition.set_league_team(index, id);
        competition.set_team_entry_offset(index, u16::try_from(index).expect("20") * ROW_BYTES);
        teams[usize::from(id[1]) % SELECTED_TEAMS].prices =
            std::array::from_fn(|slot| team.price(slot).cast_unsigned());
    }
    let mut first = competition.row(0);
    first.line.played = GAMES;
    competition.set_row(0, first);
    let mut played = 0;
    for (index, pair) in league.windows(2).enumerate() {
        for seed in 0..SEEDS {
            let setup = ResultSetup {
                competition: Box::new(*competition.image()),
                mode: if seed % 2 == 0 { 0 } else { NEUTRAL },
                substitutes: (3, 5),
                column: 0,
                sides: [
                    Side {
                        record: pair[0].image().to_vec(),
                        career: None,
                    },
                    Side {
                        record: pair[1].image().to_vec(),
                        career: None,
                    },
                ],
                tactics: records.clone(),
                teams: teams.clone(),
                career_team: SelectedTeam {
                    prices: [0; 16],
                    controls: 0,
                },
                status: Status {
                    mode: 1,
                    training: false,
                    testers_rule: false,
                },
                game_type: game_type_code(GameType::Career),
                computer: (false, false),
                statuses_apply: true,
                national_selector: false,
                day: DayGlobals {
                    season: 1996,
                    seed,
                    phase: u8::try_from(index).expect("20"),
                },
                then: (true, true),
            };
            let rand = (seed, seed.wrapping_mul(7), seed.wrapping_add(3));
            let ours = ours(&setup, None, rand)
                .unwrap_or_else(|error| panic!("match {index}, seed {seed}: {error}"));
            let (upstream, state) = theirs(&mut oracle, &setup, None, rand);
            let upstream = upstream.expect("upstream plays it");
            alike(
                &ours,
                &(upstream, state),
                &format!("match {index}, seed {seed}"),
            )
            .unwrap_or_else(|error| panic!("{error}"));
            played += 1;
        }
    }
    assert_eq!(played, (LEAGUE_TEAMS - 1) * usize::from(SEEDS));
}
