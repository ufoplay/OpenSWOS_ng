//! `openswos_core::sim`'s standing, form and statuses against upstream's src/match_sim.c and
//! src/career_status.c (M5 #8b): from any league (any image, its first row's games, its teams
//! and their places mostly the league's), any team and any teams' prices, its rank by price
//! (`CalcTeamPriceRankPercentOffset`, with `FindTeamInDIY` and `GetMoreExpensiveTeamsCount`)
//! alike, a team not in its league refused as upstream halts (0x6006); from any team record and
//! any day, its form (`CalcTeamViewOffset`) alike; from any team (a selected team's record or
//! the career club's), any mode and any status rules, its available players and goalkeepers
//! (`GetNumberOfAvailablePlayers`, `GetNumberOfAvailableSubstitutes`) alike; from any team, any
//! stand-ins' table (or none), a career game or not, any kinds a computer club's records keep
//! and any state of the second generator, the status buckets a match gives its players
//! (`InitPlayerRemapTableForTeam`) alike, the record and the generator after; England's 93 teams
//! (`assets/TEAM/TEAM.008`) alike too. Cases the port refuses (where upstream reads past its
//! records or memory) are not run. Without upstream's checkout the oracle is not built and
//! these tests are not either (`tests/available.rs` says so).
#![cfg(not(oracle_unavailable))]

mod sim_setup;

use openswos_common::Importer;
use openswos_core::competition::running::{RunningCompetition, StandingsRow};
use openswos_core::competition::table::Line;
use openswos_core::sim::PLAYER_LEN;
use openswos_core::sim::{
    Availability, ComputerStatuses, Day, GameType, MatchTeam, RECORD_LEN, SimError, SimRules,
    available_keepers, available_players, form, price_rank, status_buckets,
};
use openswos_oracle::Oracle;
use openswos_oracle::cup::SelectedTeam;
use openswos_oracle::rng::RandState;
use openswos_oracle::sim::{
    BucketSetup, CAREER_LINE_UP, DayGlobals, RankSetup, Status, TACTICS_LEN, TACTICS_SLOTS,
    TeamSetup,
};
use openswos_rng::{Generator, Rng};
use openswos_team_resource::TeamFileFormat;
use openswos_test_utils::assets_path;
use proptest::collection::vec;
use proptest::prelude::*;
use sim_setup::{
    CARDS_INJURIES, CONTROLS, LAST_OF_ELEVEN, LEAGUE_TEAMS, LINE_UP, NOT_IN_LEAGUE, NOT_POSITION,
    PINNED, PLAYERS, POSITION, ROW_BYTES, SELECTED_TEAMS, TEAM_ID, core_status, core_team,
    game_type, game_type_code, league, loader, record, selected, squad_status, status, team,
};

proptest! {
    #[test]
    fn a_team_s_rank_in_its_league_is_upstream_s(
        (image, ids) in league(),
        team_record in record(RECORD_LEN, 16),
        pick in prop_oneof![9 => (0..LEAGUE_TEAMS).prop_map(Some), 1 => Just(None)],
        teams in vec(selected(), SELECTED_TEAMS),
        career_team in selected(),
        national_selector in any::<bool>(),
    ) {
        let mut team_record = team_record;
        if let Some(index) = pick {
            team_record[TEAM_ID].copy_from_slice(&ids[index]);
        }
        let competition = RunningCompetition::from_image(&image);
        let team = MatchTeam::selected(&team_record.clone().try_into().expect("a record"));
        let ours = price_rank(&competition, &team, loader(&teams, &career_team), national_selector);
        // Refused where upstream reads past the competition: not run.
        prop_assume!(
            matches!(ours, Ok(_) | Err(SimError::NotInCompetition(_))),
            "{:?}",
            ours
        );
        let theirs = Oracle::lock().price_rank(&RankSetup {
            competition: Box::new(image),
            team: team_record,
            teams,
            career_team,
            national_selector,
        });
        match ours {
            Err(_) => prop_assert_eq!(theirs, Err(NOT_IN_LEAGUE)),
            Ok(rank) => prop_assert_eq!(theirs, Ok(rank)),
        }
    }

    #[test]
    fn a_team_s_form_is_upstream_s(
        team_record in record(RECORD_LEN, 16),
        (season, seed, phase) in any::<(u16, u8, u8)>(),
    ) {
        let image: [u8; RECORD_LEN] = team_record.try_into().expect("a record");
        let ours = form(&MatchTeam::selected(&image), &Day { season, seed, phase });
        let theirs = Oracle::lock().form(&image, DayGlobals { season, seed, phase });
        let packed = u32::from(ours.first.cast_unsigned()) << 16 | u32::from(ours.second.cast_unsigned());
        prop_assert_eq!(packed, theirs);
    }

    #[test]
    fn the_available_players_and_keepers_are_upstream_s(
        (record, career) in team(),
        mode in prop_oneof![Just(-1i16), Just(0i16), Just(1i16), any::<i16>()],
        places in prop_oneof![Just(0u16), 1..=16u16, any::<u16>()],
        status in status(),
    ) {
        let team = core_team(&record, career);
        let rules = core_status(Status { mode: mode.cast_unsigned(), ..status });
        let setup = TeamSetup {
            team: record,
            career,
            tactics: vec![[0; TACTICS_LEN]; TACTICS_SLOTS],
            status,
        };
        let packed = |availability: Availability| u32::from(availability.out) << 16 | u32::from(availability.available);
        let players = available_players(&team, places, &rules);
        if let Ok(players) = players {
            let theirs = Oracle::lock().available_players(&setup, mode, places);
            prop_assert_eq!(packed(players), theirs, "the players");
        }
        let keepers = available_keepers(&team, &rules);
        if let Ok(keepers) = keepers {
            let theirs = Oracle::lock().available_keepers(&setup, mode);
            prop_assert_eq!(packed(keepers), theirs, "the keepers");
        }
    }

    #[test]
    fn a_match_s_status_buckets_are_upstream_s(
        (record, career) in team(),
        stand_ins in proptest::option::of(vec(prop_oneof![3 => Just(0xFFu8), 1 => 0..32u8, 1 => any::<u8>()], CAREER_LINE_UP)),
        game in game_type(),
        computer in any::<(bool, bool)>(),
        status in status(),
        computer_team in prop_oneof![3 => Just(false), 1 => Just(true)],
        rand in any::<(u8, u8, u8)>(),
        fixed in prop_oneof![1 => Just(None), 1 => proptest::sample::select(&PINNED[..]).prop_map(Some)],
        squad in proptest::option::of(vec(squad_status(), 32)),
        last_keeper in any::<bool>(),
    ) {
        let mut record = record;
        if computer_team {
            record[CONTROLS] = 1;
        }
        if last_keeper {
            let slot = match career {
                Some((order, _)) => order[LAST_OF_ELEVEN],
                None => record[LINE_UP + LAST_OF_ELEVEN],
            };
            let at = PLAYERS + usize::from(slot) * PLAYER_LEN + POSITION;
            if at < record.len() {
                record[at] &= NOT_POSITION;
            }
        }
        if let Some(statuses) = squad {
            for (slot, status) in statuses.into_iter().enumerate() {
                let at = PLAYERS + slot * PLAYER_LEN + CARDS_INJURIES;
                if at < record.len() {
                    record[at] = status;
                }
            }
        }
        let stand_ins: Option<[u8; CAREER_LINE_UP]> = stand_ins.map(|table| table.try_into().expect("32"));
        let mut team = core_team(&record, career);
        let (seed, xor_index, xor_key) = rand;
        let mut rng = Rng::default();
        rng.secondary = Generator { seed, xor_index, xor_key };
        rng.fixed = fixed;
        let rules = SimRules {
            status: core_status(status),
            game,
            computer: ComputerStatuses { cards: computer.0, injuries: computer.1 },
        };
        let ours = status_buckets(&mut team, stand_ins.as_ref(), &rules, &mut rng);
        prop_assume!(ours.is_ok(), "{:?}", ours);
        let mut oracle = Oracle::lock();
        oracle.set_rand2_state(RandState { seed, xor_index, xor_key });
        oracle.set_fixed_random(fixed);
        let setup = TeamSetup {
            team: record,
            career,
            tactics: vec![[0; TACTICS_LEN]; TACTICS_SLOTS],
            status,
        };
        let after = oracle.status_buckets(&setup, &BucketSetup { stand_ins, game_type: game_type_code(game), computer });
        oracle.set_fixed_random(None);
        prop_assert_eq!(team.image(), &after[..]);
        let state = oracle.rand2_state();
        prop_assert_eq!(
            (rng.secondary.seed, rng.secondary.xor_index, rng.secondary.xor_key),
            (state.seed, state.xor_index, state.xor_key)
        );
    }
}

/// The first row's games a real league has played here: enough for its ranks to count.
const GAMES: u16 = 10;

#[test]
fn a_real_team_file_s_teams_stand_and_take_their_buckets_as_upstream_s() {
    let bytes = std::fs::read(assets_path().join("TEAM/TEAM.008")).expect("England's team file");
    let file = TeamFileFormat::import(&bytes).expect("a team file");
    // The league: England's first 20 teams, in order; the loader gives each its own prices.
    let mut competition = RunningCompetition::default();
    competition.set_num_teams(i16::try_from(LEAGUE_TEAMS).expect("20"));
    let mut teams = vec![
        SelectedTeam {
            prices: [0; 16],
            controls: 0
        };
        SELECTED_TEAMS
    ];
    for (index, team) in file.teams.iter().take(LEAGUE_TEAMS).enumerate() {
        let id = team.id.to_bytes();
        competition.set_league_team(index, id);
        competition.set_team_entry_offset(index, u16::try_from(index).expect("20") * ROW_BYTES);
        let prices = MatchTeam::from_team(team);
        teams[usize::from(id[1]) % SELECTED_TEAMS].prices =
            std::array::from_fn(|slot| prices.price(slot).cast_unsigned());
    }
    competition.set_row(
        0,
        StandingsRow {
            team: 0,
            line: Line {
                played: GAMES,
                won: 0,
                drawn: 0,
                lost: 0,
                goals_for: 0,
                goals_against: 0,
                points: 0,
            },
        },
    );
    let career_team = SelectedTeam {
        prices: [0; 16],
        controls: 0,
    };
    let status = Status {
        mode: 1,
        training: false,
        testers_rule: false,
    };
    let rules = SimRules {
        status: core_status(status),
        game: GameType::Career,
        computer: ComputerStatuses {
            cards: true,
            injuries: true,
        },
    };
    let mut oracle = Oracle::lock();
    for (index, real) in file.teams.iter().take(LEAGUE_TEAMS).enumerate() {
        let team = MatchTeam::from_team(real);
        let ours = price_rank(&competition, &team, loader(&teams, &career_team), false);
        let theirs = oracle.price_rank(&RankSetup {
            competition: Box::new(*competition.image()),
            team: team.image().to_vec(),
            teams: teams.clone(),
            career_team,
            national_selector: false,
        });
        assert_eq!(
            ours.map_err(|_| NOT_IN_LEAGUE),
            theirs,
            "team {index}'s rank"
        );
        let image: [u8; RECORD_LEN] = team.image().try_into().expect("a record");
        let day = Day {
            season: 1996,
            seed: u8::try_from(index).expect("20"),
            phase: 3,
        };
        let ours = form(&team, &day);
        let theirs = oracle.form(
            &image,
            DayGlobals {
                season: day.season,
                seed: day.seed,
                phase: day.phase,
            },
        );
        assert_eq!(
            u32::from(ours.first.cast_unsigned()) << 16 | u32::from(ours.second.cast_unsigned()),
            theirs,
            "team {index}'s form"
        );
        for seed in 0..=u8::MAX {
            let mut ours = team.clone();
            let mut rng = Rng::default();
            rng.secondary = Generator {
                seed,
                xor_index: 0,
                xor_key: 0,
            };
            status_buckets(&mut ours, None, &rules, &mut rng)
                .unwrap_or_else(|error| panic!("team {index}: {error}"));
            oracle.set_rand2_state(RandState {
                seed,
                xor_index: 0,
                xor_key: 0,
            });
            let after = oracle.status_buckets(
                &TeamSetup {
                    team: image.to_vec(),
                    career: None,
                    tactics: vec![[0; TACTICS_LEN]; TACTICS_SLOTS],
                    status,
                },
                &BucketSetup {
                    stand_ins: None,
                    game_type: game_type_code(GameType::Career),
                    computer: (true, true),
                },
            );
            assert_eq!(ours.image(), &after[..], "team {index}, seed {seed}");
        }
    }
}
