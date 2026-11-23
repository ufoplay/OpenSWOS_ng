//! `openswos_core::sim` against upstream's src/match_sim.c (M5 #8a): the engine's tables are
//! upstream's compiled ones; any team record, every field the port reads is the one the C
//! struct reads; from any team (a selected team's record or the career club's, its line-up),
//! any tactics and any status rules, a player's price for his place (`GetPlayerPrice`), his
//! match value (`UpdatePlayerPositionAndReturnRole`) and the team's strength
//! (`ComputeOldPlayerSkillForViewResult`) alike, the record after byte for byte; any nearness
//! (`CountNearnessFactor`) and any availability (`IsPlayerInjured`) alike; every team of a
//! real team file (England's 93, `assets/TEAM/TEAM.008`), as a selected team and as the career
//! club, its values and strength alike with upstream's compiled tactics. Cases the port refuses
//! (where upstream reads past its records, tables or memory) are not run. Without upstream's
//! checkout the oracle is not built and these tests are not either (`tests/available.rs` says
//! so).
#![cfg(not(oracle_unavailable))]

mod sim_setup;

use openswos_common::{ByteReader, Importer};
use openswos_core::sim::tables::{
    AREA_DISTANCES, AREA_PONDERS, POSITION_WEIGHTS, PRICE_RISES, SKILL_PONDERS, SPOT_AREAS,
};
use openswos_core::sim::{
    CAREER_RECORD_LEN, MatchTeam, RECORD_LEN, match_value, nearness, player_price, strength,
    unavailable,
};
use openswos_oracle::Oracle;
use openswos_oracle::sim::{CAREER_LINE_UP, Status, TeamSetup};
use openswos_tac_resource::PlayerSpots;
use openswos_team_resource::TeamFileFormat;
use openswos_test_utils::assets_path;
use proptest::collection::vec;
use proptest::prelude::*;
use sim_setup::{
    LINE_UP, LINE_UP_LEN, VALUES_DIRTY, core_status, core_tactics, core_team, record, spot, status,
    tactics, team,
};

#[test]
fn the_engine_s_tables_are_upstream_s() {
    let tables = Oracle::lock().price_tables();
    assert_eq!(tables.position_weights, POSITION_WEIGHTS.concat());
    assert_eq!(
        &tables.area_distances[..AREA_DISTANCES.len() * AREA_DISTANCES[0].len()],
        AREA_DISTANCES.concat()
    );
    assert_eq!(tables.area_ponders, AREA_PONDERS);
    let spot_areas: Vec<i8> = SPOT_AREAS
        .concat()
        .into_iter()
        .map(u8::cast_signed)
        .collect();
    assert_eq!(tables.spot_areas, spot_areas);
    assert_eq!(tables.skill_ponders, SKILL_PONDERS);
    assert_eq!(tables.price_rises, PRICE_RISES);
}

proptest! {
    #[test]
    fn every_field_is_where_upstream_s_compiler_puts_it(bytes in vec(any::<u8>(), RECORD_LEN)) {
        let image: [u8; RECORD_LEN] = bytes.try_into().expect("a record's bytes");
        let c = Oracle::lock().team_fields(&image);
        let ours = MatchTeam::selected(&image);
        prop_assert_eq!(ours.image(), &image[..]);
        prop_assert_eq!(
            [
                ours.country(),
                ours.ordinal(),
                ours.controls(),
                ours.own_goals(),
                ours.match_flags(),
                ours.tactics(),
                ours.league(),
                ours.values_dirty(),
            ],
            c.bytes
        );
        prop_assert_eq!(ours.global_number(), c.global_number);
        for place in 0..LINE_UP_LEN {
            prop_assert_eq!(ours.player_number(place), c.line_up[place]);
        }
        for (slot, player) in c.players.iter().enumerate() {
            prop_assert_eq!(
                [
                    ours.nationality(slot),
                    ours.match_value(slot),
                    ours.shirt(slot),
                    ours.price(slot),
                    ours.form(slot),
                ],
                player.signed
            );
            prop_assert_eq!([ours.position_and_face(slot), ours.cards_injuries(slot)], player.flags);
            prop_assert_eq!(ours.skills(slot).map(u8::cast_signed), player.skills);
            prop_assert_eq!([0, 1, 2, 3].map(|column| ours.goals(slot, column)), player.goals);
        }
    }

    #[test]
    fn a_player_s_price_for_his_place_is_upstream_s(
        record in record(RECORD_LEN, 16),
        slot in 0..16u16,
        number in 0..=11u16,
        records in tactics(),
    ) {
        let team = MatchTeam::selected(&record.clone().try_into().expect("a record"));
        let ours = player_price(&team, usize::from(slot), usize::from(number), &core_tactics(&records));
        // Refused where upstream reads past its tables: not run.
        prop_assume!(ours.is_ok(), "{:?}", ours);
        let setup = TeamSetup {
            team: record,
            career: None,
            tactics: records,
            status: Status { mode: 0, training: false, testers_rule: false },
        };
        let theirs = Oracle::lock().player_price(&setup, slot, number);
        prop_assert_eq!(ours, Ok(theirs.answer));
    }

    #[test]
    fn a_player_s_match_value_is_upstream_s(
        (record, career) in team(),
        place in 0..=20u16,
        records in tactics(),
        status in status(),
    ) {
        let mut team = core_team(&record, career);
        let ours = match_value(
            &mut team,
            usize::from(place),
            &core_tactics(&records),
            &core_status(status),
        );
        prop_assume!(ours.is_ok(), "{:?}", ours);
        let value = ours.expect("reckoned");
        let setup = TeamSetup { team: record, career, tactics: records, status };
        let theirs = Oracle::lock().match_value(&setup, place);
        prop_assert_eq!(u32::from(value.role) << 16 | u32::from(value.value), theirs.answer);
        prop_assert_eq!(team.image(), &theirs.team[..]);
    }

    #[test]
    fn a_team_s_strength_is_upstream_s(
        (record, career) in team(),
        substitutes in prop_oneof![9 => 0..=5u16, 1 => any::<u16>()],
        records in tactics(),
        status in status(),
    ) {
        let mut team = core_team(&record, career);
        let ours = strength(&mut team, substitutes, &core_tactics(&records), &core_status(status));
        prop_assume!(ours.is_ok(), "{:?}", ours);
        let setup = TeamSetup { team: record, career, tactics: records, status };
        let theirs = Oracle::lock().team_strength(&setup, substitutes);
        let (strength, current) = theirs.answer;
        prop_assert_eq!(ours, Ok(strength));
        prop_assert_eq!(current, strength, "the strength upstream keeps");
        prop_assert_eq!(team.image(), &theirs.team[..]);
    }

    #[test]
    fn the_nearness_is_upstream_s(
        area in 0..35u16,
        ball in 0..35u16,
        price in any::<u16>(),
        spots in vec(spot(), 35),
    ) {
        let spots: [u8; 35] = spots.try_into().expect("35 spots");
        let mut bytes = ByteReader::new(&spots);
        let player = PlayerSpots(std::array::from_fn(|_| {
            bytes.read_array::<5>().expect("a row").map(Into::into)
        }));
        let ours = nearness(usize::from(area), usize::from(ball), price, &player);
        prop_assume!(ours.is_ok(), "{:?}", ours);
        prop_assert_eq!(ours, Ok(Oracle::lock().nearness(area, ball, price, &spots)));
    }

    #[test]
    fn a_player_s_availability_is_upstream_s(cards_injuries in any::<u8>(), status in status()) {
        let theirs = Oracle::lock().injured(cards_injuries, status);
        prop_assert_eq!(unavailable(cards_injuries, &core_status(status)), theirs != 0);
    }
}

/// The substitutes a real team's strength counts here (the most upstream's competitions
/// field).
const BENCH: u16 = 5;

#[test]
fn a_real_team_file_s_teams_play_at_upstream_s_values() {
    let bytes = std::fs::read(assets_path().join("TEAM/TEAM.008")).expect("England's team file");
    let file = TeamFileFormat::import(&bytes).expect("a team file");
    let mut oracle = Oracle::lock();
    let records = oracle.compiled_tactics();
    let tactics = core_tactics(&records);
    let status = Status {
        mode: 1,
        training: false,
        testers_rule: false,
    };
    for (index, team) in file.teams.iter().enumerate() {
        let selected = MatchTeam::from_team(team);
        // As the career club: its record and 16 empty reserves, its line-up its record's.
        let mut career_record = selected.image().to_vec();
        career_record.resize(CAREER_RECORD_LEN, 0);
        let mut order = [0u8; CAREER_LINE_UP];
        for (place, slot) in order.iter_mut().enumerate() {
            *slot = u8::try_from(place).expect("32");
        }
        order[..LINE_UP_LEN].copy_from_slice(&selected.image()[LINE_UP..LINE_UP + LINE_UP_LEN]);
        let career = Some((order, 16));
        for (record, career) in [(selected.image().to_vec(), None), (career_record, career)] {
            for dirty in [0u8, 1] {
                let mut record = record.clone();
                record[VALUES_DIRTY] = dirty;
                let setup = TeamSetup {
                    team: record.clone(),
                    career,
                    tactics: records.clone(),
                    status,
                };
                for place in 0..LINE_UP_LEN {
                    let mut ours = core_team(&record, career);
                    let value = match_value(&mut ours, place, &tactics, &core_status(status))
                        .unwrap_or_else(|error| panic!("team {index}, place {place}: {error}"));
                    let theirs = oracle.match_value(&setup, u16::try_from(place).expect("16"));
                    assert_eq!(
                        u32::from(value.role) << 16 | u32::from(value.value),
                        theirs.answer,
                        "team {index}, place {place}"
                    );
                    assert_eq!(
                        ours.image(),
                        &theirs.team[..],
                        "team {index}, place {place}"
                    );
                }
                let mut ours = core_team(&record, career);
                let ours_strength = strength(&mut ours, BENCH, &tactics, &core_status(status))
                    .unwrap_or_else(|error| panic!("team {index}: {error}"));
                let theirs = oracle.team_strength(&setup, BENCH);
                assert_eq!(ours_strength, theirs.answer.0, "team {index}'s strength");
                assert_eq!(ours.image(), &theirs.team[..], "team {index}");
            }
        }
    }
}
