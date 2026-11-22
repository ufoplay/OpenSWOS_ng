//! `DIYTournamentState` read through bindgen's declaration of it, and the functions of
//! src/diy_competition.c that write it.

use std::ffi::CString;

use super::bindings::diy_competition_c::{
    self as diy, CompTournamentHeader, CompetitionTournament, DIYTournamentState, Round, TeamId,
};
use super::cup::set_cup_session;
use super::league::{Session, competition, image, pointer, set_session, with_compiled, with_lists};
use super::running::{IMAGE_LEN, header};
use super::stubs::{self, WORLD_2627, WORLD_9697};
use crate::tournament::{
    CompiledTournamentSetup, RecalculateSetup, SlotFields, Started, TournamentBytes,
    TournamentFields, TournamentSetup,
};

/// The bytes of the struct in upstream's 64-bit build.
pub const STATE_LEN: usize = 1120;

/// `image` written into `_diyFileBufferCopy`.
pub(super) fn write(image: &[u8; STATE_LEN]) {
    assert_eq!(
        size_of::<DIYTournamentState>(),
        STATE_LEN,
        "the 64-bit build's struct"
    );
    // SAFETY: the image is the struct's size, written over the global the lock's holder owns
    // (its name pointer is never followed).
    unsafe {
        (&raw mut diy::_diyFileBufferCopy)
            .cast::<u8>()
            .copy_from_nonoverlapping(image.as_ptr(), STATE_LEN);
    }
}

/// `_diyFileBufferCopy`'s image now.
pub(super) fn read() -> Box<[u8; STATE_LEN]> {
    let mut after = Box::new([0; STATE_LEN]);
    // SAFETY: a plain read of a global the lock's holder owns, its image's bytes.
    after.copy_from_slice(unsafe {
        std::slice::from_raw_parts((&raw const diy::_diyFileBufferCopy).cast::<u8>(), STATE_LEN)
    });
    after
}

/// `image` written into `_diyFileBufferCopy`, every field read back.
pub(crate) fn fields(image: &[u8; STATE_LEN]) -> TournamentFields {
    write(image);
    // SAFETY: a read by value of the global just written; the slots are packed, each field
    // copied out of the copy.
    let state = unsafe { (&raw const diy::_diyFileBufferCopy).read_unaligned() };
    TournamentFields {
        header: header(&state.header),
        league_warnings_enabled: state.leagueWarningsEnabled,
        team_ids: state.teamIds,
        group_map: state.groupMap,
        slots: state
            .slots
            .iter()
            .map(|&slot| SlotFields {
                pad: slot._pad_00,
                words: [
                    slot.doubleCount,
                    slot.isLeagueFormat,
                    slot.seedingType,
                    slot.numTeams,
                    slot.divisionFactor,
                    slot.playEachTeam,
                    slot.initConst168,
                    slot.groups,
                    slot.teamsPerGroup,
                    slot.formatFlag16E,
                    slot.initConst170,
                    slot.formatFlag172,
                    slot.matchesPerSlot,
                ],
                months: slot.progressionTable,
            })
            .collect(),
        team_index_order: state.teamIndexOrder,
    }
}

/// `InitDIYTournament` from the state `before`.
pub(crate) fn reset(before: &[u8; STATE_LEN]) -> Box<[u8; STATE_LEN]> {
    write(before);
    // SAFETY: the function writes the global just set; its externs are the compiled data's and
    // the stubs.
    unsafe { diy::InitDIYTournament() };
    read()
}

/// `DIYTournament_RecalculateMatchesPerTeam` of `setup`.
pub(crate) fn recalculate(
    setup: &RecalculateSetup,
) -> (Box<[u8; STATE_LEN]>, Box<[u8; IMAGE_LEN]>) {
    set_session(&Session {
        before: &setup.competition,
        division: 0,
        pitch_type: 0,
        teams: &setup.teams,
        career_team: &setup.career_team,
    });
    write(&setup.state);
    // SAFETY: the function reads and writes the globals just set; its externs are the compiled
    // data's and the stubs.
    unsafe { diy::DIYTournament_RecalculateMatchesPerTeam() };
    (read(), image())
}

/// The record `bytes` describes, its names in `strings`.
fn record(bytes: &TournamentBytes, strings: &mut Vec<CString>) -> Box<CompetitionTournament> {
    // SAFETY: a record of zeroed bytes is a valid one (integers and null pointers).
    let mut record: Box<CompetitionTournament> = Box::new(unsafe { std::mem::zeroed() });
    record.header = CompTournamentHeader {
        comp: competition(bytes.header),
        seedingType: bytes.seeding_type,
        menuState: bytes.menu_state,
        playEachTeam: bytes.play_each_team,
        awayGoalsMode: bytes.away_goals_mode,
        subsAndPointsForWin: bytes.substitutes.cast_signed(),
        roundStringBase: bytes.round_string_base,
        rounds: bytes.rounds.map(|[teams, groups, per_group]| Round {
            teamCount: teams,
            groupCount: groups,
            teamGroupCount: per_group,
        }),
        reserved: bytes.reserved,
    };
    record.roundRuleFlags = bytes.round_rules;
    let mut names = record.name;
    for (slot, name) in names.iter_mut().zip(&bytes.names) {
        *slot = pointer(name.as_ref(), strings);
    }
    record.name = names;
    let mut seeds = record.teamSeedList;
    for (slot, &[country, ordinal]) in seeds.iter_mut().zip(&bytes.seeds) {
        *slot = TeamId {
            countryNumber: country,
            teamOrdinal: ordinal,
        };
    }
    record.teamSeedList = seeds;
    record
}

/// `InitDIYLeagueFromRecord` of the record at `address`, the world `World_Current` answers
/// `world`: the state and the competition after, or upstream's halt.
fn start_record(address: *mut u8, world: u32) -> Result<Started, u16> {
    // SAFETY: plain writes to a global the lock's holder owns, put back after the call;
    // `address` is a tournament's record; the function's externs are the compiled data's and
    // the stubs.
    let started = unsafe {
        let before = stubs::WORLD;
        stubs::WORLD = world;
        let started = super::halting(|| diy::InitDIYLeagueFromRecord(address));
        stubs::WORLD = before;
        started
    };
    started?;
    Ok((read(), image()))
}

/// `InitDIYLeagueFromRecord` of `setup`.
pub(crate) fn start(setup: &TournamentSetup) -> Result<Started, u16> {
    let mut strings = Vec::new();
    let mut record = record(&setup.record, &mut strings);
    let address = (&raw mut *record).cast::<u8>();
    set_cup_session(
        &setup.competition,
        setup.pitch_type,
        &setup.teams,
        &setup.career_team,
    );
    write(&setup.state);
    let world = if setup.odd_groups {
        WORLD_2627
    } else {
        WORLD_9697
    };
    with_lists(address, setup.national, setup.continental, || {
        start_record(address, world)
    })
}

/// `InitDIYLeagueFromRecord` of the compiled record `setup` names, upstream's tables as they
/// are, the original's world: none when the tables have no such record.
pub(crate) fn start_compiled(setup: &CompiledTournamentSetup) -> Option<Result<Started, u16>> {
    set_cup_session(
        &setup.competition,
        setup.pitch_type,
        &setup.teams,
        &setup.career_team,
    );
    write(&setup.state);
    with_compiled(setup.record, |address| start_record(address, WORLD_9697))
}
