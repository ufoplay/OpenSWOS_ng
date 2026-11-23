//! src/match_sim.c's player values, team strength, standing, form and status buckets, and
//! src/career_status.c's availability, over `_selectedTeams[0]` or `_careerTeam`, the session's
//! tactics table pointed at the setup's records.

use super::bindings::career_status_c::{self as status, PlayerFile};
use super::bindings::career_team_scalars_c as scalars;
use super::bindings::diy_competition_c::{self as diy, TeamFileEx};
use super::bindings::match_sim_c::{self as sim, TeamFile};
use super::bindings::swos_amiga_data_imports_c as data;
use super::bindings::swos_data_bulk_imports_c as bulk;
use super::bindings::tactics_data_c::{self as tactics, TeamTactics};
use super::bindings::test_rules_c as rules;
use super::cup::set_cup_session;
use super::stubs;
use crate::sim::{
    BucketSetup, DayGlobals, PlayerFields, PriceTables, RankSetup, Reckoned, Status, TACTICS_LEN,
    TACTICS_SLOTS, TeamFields, TeamSetup,
};

/// The bytes before a player's record in the team's (`sizeof(TeamFile)`).
const HEADER_LEN: usize = 76;

/// The bytes of a player's record.
const PLAYER_LEN: usize = 38;

/// The tactics records a call points the table at.
static mut TACTICS: [[u8; TACTICS_LEN]; TACTICS_SLOTS] = [[0; TACTICS_LEN]; TACTICS_SLOTS];

/// The table as compiled, kept while a call points it at the setup's records.
static mut COMPILED: [*mut TeamTactics; TACTICS_SLOTS] = [std::ptr::null_mut(); TACTICS_SLOTS];

/// The team's record in place: `_careerTeam` for the career club, else `_selectedTeams[0]`.
fn team_address(setup: &TeamSetup) -> *mut u8 {
    if setup.career.is_some() {
        (&raw mut diy::_careerTeam).cast()
    } else {
        (&raw mut diy::_selectedTeams).cast()
    }
}

/// `setup` written: the record, the career club's line-up, the tactics, the status.
fn set(setup: &TeamSetup) -> *mut u8 {
    let team = team_address(setup);
    assert_eq!(setup.tactics.len(), TACTICS_SLOTS, "a slot's tactics each");
    // SAFETY: plain writes to globals the lock's holder owns, each within its storage (the
    // career club's 1,292 bytes, a selected team's 684); the table's slots point at the
    // oracle's own records, which outlive the call.
    unsafe {
        team.copy_from_nonoverlapping(setup.team.as_ptr(), setup.team.len());
        if let Some((line_up, players)) = setup.career {
            sim::_currentMatchPlayers = line_up;
            scalars::_careerNumPlayers = players;
        }
        COMPILED = tactics::_teamTacticsTable;
        let records = &raw mut TACTICS;
        for (slot, record) in setup.tactics.iter().enumerate() {
            (*records)[slot] = *record;
            tactics::_teamTacticsTable[slot] = (&raw mut (*records)[slot]).cast::<TeamTactics>();
        }
        data::_trainingGame = [u16::from(setup.status.training)];
        data::_inEditTactics = [setup.status.mode];
        rules::TestRules_Force(i32::from(setup.status.testers_rule));
    }
    team
}

/// The record after a call, the tactics table as compiled again.
fn team_after(setup: &TeamSetup) -> Vec<u8> {
    let mut after = vec![0; setup.team.len()];
    // SAFETY: a plain read of the record just written and used; a plain write of the table
    // kept by `set`.
    unsafe {
        team_address(setup).copy_to_nonoverlapping(after.as_mut_ptr(), after.len());
        tactics::_teamTacticsTable = COMPILED;
    }
    after
}

/// The tactics records the compiled table points at.
pub(crate) fn compiled_tactics() -> Vec<[u8; TACTICS_LEN]> {
    // SAFETY: each slot points at a 370-byte record of upstream's.
    unsafe {
        (&raw const tactics::_teamTacticsTable)
            .read()
            .iter()
            .map(|&record| record.cast::<[u8; TACTICS_LEN]>().read())
            .collect()
    }
}

/// `GetPlayerPrice` of record `slot` at line-up place `number`.
pub(crate) fn player_price(setup: &TeamSetup, slot: u16, number: u16) -> Reckoned<i16> {
    let team = set(setup);
    assert!(
        HEADER_LEN + (usize::from(slot) + 1) * PLAYER_LEN <= setup.team.len(),
        "a record of the team"
    );
    // SAFETY: the player's record is the team's (`team + slot * 38`, as the C's callers pass
    // it), the tactics and tables the call reads are set.
    let answer = unsafe {
        sim::GetPlayerPrice(
            team.cast::<TeamFile>(),
            team.add(usize::from(slot) * PLAYER_LEN)
                .cast::<sim::PlayerFile>(),
            number,
        )
    };
    Reckoned {
        answer,
        team: team_after(setup),
    }
}

/// `UpdatePlayerPositionAndReturnRole` of line-up place `place`.
pub(crate) fn match_value(setup: &TeamSetup, place: u16) -> Reckoned<u32> {
    let team = set(setup);
    // SAFETY: the team is set; the caller checks the place's record is within it.
    let answer =
        unsafe { sim::UpdatePlayerPositionAndReturnRole(team.cast::<TeamFileEx>().cast(), place) };
    Reckoned {
        answer,
        team: team_after(setup),
    }
}

/// `ComputeOldPlayerSkillForViewResult`, `substitutes` counted.
pub(crate) fn strength(setup: &TeamSetup, substitutes: u16) -> Reckoned<(u16, u16)> {
    let team = set(setup);
    // SAFETY: the team is set; the caller checks every place's record is within it.
    unsafe {
        let strength = sim::ComputeOldPlayerSkillForViewResult(team.cast(), substitutes);
        Reckoned {
            answer: (strength, sim::_teamSkillCurrent),
            team: team_after(setup),
        }
    }
}

/// `CountNearnessFactor` of a player's 35 spots.
pub(crate) fn nearness(area: u16, ball: u16, price: u16, spots: &[u8; 35]) -> u16 {
    assert!(area < 35 && ball < 35, "areas of the pitch");
    // SAFETY: the distances' row and the spots are within their arrays; the map is upstream's.
    unsafe {
        let distances = (&raw const tactics::_quadrantDistanceFactor).cast::<u8>();
        diy::CountNearnessFactor(
            area,
            ball,
            price,
            distances.add(usize::from(area) * 35),
            spots.as_ptr(),
            (&raw const tactics::_quadrantCoordinatesToBallQuadrant).cast(),
        )
    }
}

/// `IsPlayerInjured` of a player of `cards_injuries`.
pub(crate) fn injured(cards_injuries: u8, status_rules: Status) -> u16 {
    let mut team = [0u8; HEADER_LEN + PLAYER_LEN];
    team[HEADER_LEN + 27] = cards_injuries;
    // SAFETY: plain writes to globals the lock's holder owns; the function reads the player's
    // record two `PlayerFile`s past the pointer, within the buffer.
    unsafe {
        data::_trainingGame = [u16::from(status_rules.training)];
        rules::TestRules_Force(i32::from(status_rules.testers_rule));
        status::IsPlayerInjured(
            team.as_mut_ptr().cast::<PlayerFile>(),
            status_rules.mode.cast_signed(),
        )
    }
}

/// `CalcTeamPriceRankPercentOffset` of `setup`.
pub(crate) fn price_rank(setup: &RankSetup) -> Result<i16, u16> {
    set_cup_session(&setup.competition, 0, &setup.teams, &setup.career_team);
    let mut team = setup.team.clone();
    // SAFETY: a plain write to a global the lock's holder owns.
    unsafe { stubs::_isNationalSelector = u16::from(setup.national_selector) };
    let mut rank = 0;
    // SAFETY: the competition is set; the team's record is the test's buffer, of which the
    // function reads the header; its loader is the stub.
    super::halting(|| unsafe {
        rank = sim::CalcTeamPriceRankPercentOffset(
            (&raw mut diy::_diyCompetitionStart).cast(),
            team.as_mut_ptr().cast::<TeamFile>(),
        );
    })?;
    Ok(rank)
}

/// `CalcTeamViewOffset` of the selected team `record` on `day`.
pub(crate) fn form(record: &[u8; 684], day: DayGlobals) -> u32 {
    let team = (&raw mut diy::_selectedTeams).cast::<u8>();
    // SAFETY: plain writes to globals the lock's holder owns; the function reads the record
    // just written and those globals.
    unsafe {
        team.copy_from_nonoverlapping(record.as_ptr(), record.len());
        stubs::_currentSeasonShowing = day.season;
        stubs::_newRandomSeedCopy = day.seed;
        stubs::_viewRandomPhase = day.phase;
        sim::CalcTeamViewOffset(team.cast())
    }
}

/// `GetNumberOfAvailablePlayers` of `setup`'s team.
pub(crate) fn available_players(setup: &TeamSetup, mode: i16, places: u16) -> u32 {
    let team = set(setup);
    // SAFETY: the team is set; the caller checks every place's record is within it.
    let answer = unsafe { status::GetNumberOfAvailablePlayers(team.cast(), mode, places) };
    team_after(setup);
    answer
}

/// `GetNumberOfAvailableSubstitutes` of `setup`'s team.
pub(crate) fn available_keepers(setup: &TeamSetup, mode: i16) -> u32 {
    let team = set(setup);
    // SAFETY: the team is set; the caller checks every place's record is within it.
    let answer = unsafe { status::GetNumberOfAvailableSubstitutes(team.cast(), mode) };
    team_after(setup);
    answer
}

/// `InitPlayerRemapTableForTeam` of `setup`'s team and `buckets`.
pub(crate) fn status_buckets(setup: &TeamSetup, buckets: &BucketSetup) -> Vec<u8> {
    let team = set(setup);
    let mut stand_ins = buckets.stand_ins;
    // SAFETY: plain writes to globals the lock's holder owns; the team is set, the stand-ins'
    // table is the call's buffer or a null pointer; the caller checks every record is within
    // the team and the table.
    unsafe {
        stubs::_gameType = buckets.game_type;
        stubs::COMPUTER_STATUSES = buckets.computer;
        let table = stand_ins
            .as_mut()
            .map_or(std::ptr::null_mut(), |table| table.as_mut_ptr());
        sim::InitPlayerRemapTableForTeam(team.cast(), table);
    }
    team_after(setup)
}

/// A selected team's record `image`, read through bindgen's `TeamFileEx`.
pub(crate) fn team_fields(image: &[u8; 684]) -> TeamFields {
    // SAFETY: a plain write of the struct's size to a global the lock's holder owns, then
    // reads of its fields by value (the union's bytes as an array).
    unsafe {
        let team = (&raw mut diy::_selectedTeams).cast::<TeamFileEx>();
        team.cast::<u8>()
            .copy_from_nonoverlapping(image.as_ptr(), image.len());
        let record = team.read_unaligned();
        let header = record.team;
        TeamFields {
            bytes: [
                header.countryNumber,
                header.teamOrdinal,
                header.teamControls,
                header.writesZeroHere,
                header.andWith0xFE,
                header.tactics,
                header.league,
                header.playerValuesDirty,
            ],
            global_number: header.globalTeamNumber,
            line_up: header.playerNumbers,
            players: record
                .players
                .iter()
                .map(|player| PlayerFields {
                    signed: [
                        player.nationality,
                        player.cachedMatchValue,
                        player.shirtNumber,
                        player.price,
                        player.form,
                    ],
                    flags: [player.positionAndFace, player.cardsInjuries],
                    skills: [
                        player.passing,
                        player.velocityHeading,
                        player.tacklingControl,
                        player.speedFinishing,
                    ],
                    goals: player.__bindgen_anon_1.goalStats,
                })
                .collect(),
        }
    }
}

/// The engine's tables, as compiled.
pub(crate) fn price_tables() -> PriceTables {
    // SAFETY: plain reads of upstream's arrays, of their declared lengths.
    unsafe {
        PriceTables {
            position_weights: (&raw const tactics::_skillsPerQuadrant).read().to_vec(),
            area_distances: (&raw const tactics::_quadrantDistanceFactor)
                .read()
                .to_vec(),
            area_ponders: (&raw const tactics::_priceSkillPonders).read().to_vec(),
            spot_areas: (&raw const tactics::_quadrantCoordinatesToBallQuadrant)
                .read()
                .to_vec(),
            skill_ponders: (&raw const bulk::_skillValuePonders).read().to_vec(),
            price_rises: (&raw const bulk::_plPricesIncreaseTable).read().to_vec(),
        }
    }
}
