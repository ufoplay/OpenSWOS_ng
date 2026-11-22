//! src/diy_competition.c's `InitDIYCupFromRecord`: a knockout's record started in
//! `_diyCompetitionStart`.

use std::ffi::CString;

use super::bindings::diy_competition_c::{
    self as diy, CompCupHeader, CompetitionCup, Playoffs, TeamFileEx,
};
use super::draw::loaded;
use super::league::{Session, competition, image, pointer, set_session, with_compiled, with_lists};
use super::running::IMAGE_LEN;
use crate::cup::{CompiledCupSetup, CupSetup, KnockoutBytes, Layout, SelectedTeam};
use crate::draw::Prices;

/// The record's C struct, kept alive with the names it points at.
enum Record {
    /// A cup's.
    Cup(Box<CompetitionCup>),
    /// A play-off's.
    Playoff(Box<Playoffs>),
}

impl Record {
    /// Its address.
    fn address(&mut self) -> *mut u8 {
        match self {
            Self::Cup(cup) => (&raw mut **cup).cast(),
            Self::Playoff(playoff) => (&raw mut **playoff).cast(),
        }
    }
}

/// The record `bytes` describes, its names in `strings`.
fn record(bytes: &KnockoutBytes, strings: &mut Vec<CString>) -> Record {
    let header = CompCupHeader {
        comp: competition(bytes.header),
        numRoundsOverride: bytes.num_rounds_override,
        teams: bytes.teams,
        awayGoalsMode: bytes.away_goals_mode,
        subsAndPointsForWin: bytes.substitutes.cast_signed(),
        roundStringBase: bytes.round_string_base,
    };
    let names = |slots: &mut [*mut std::ffi::c_char; 2], strings: &mut Vec<CString>| {
        for (slot, name) in slots.iter_mut().zip(&bytes.names) {
            *slot = pointer(name.as_ref(), strings);
        }
    };
    match bytes.layout {
        Layout::Cup => {
            // SAFETY: a record of zeroed bytes is a valid one (integers and null pointers).
            let mut cup: Box<CompetitionCup> = Box::new(unsafe { std::mem::zeroed() });
            cup.header = header;
            for (slot, &rule) in cup.roundRuleFlags.iter_mut().zip(&bytes.round_rules) {
                *slot = rule;
            }
            let mut pointers = cup.name;
            names(&mut pointers, strings);
            cup.name = pointers;
            Record::Cup(cup)
        }
        Layout::Playoff => {
            // SAFETY: as above.
            let mut playoff: Box<Playoffs> = Box::new(unsafe { std::mem::zeroed() });
            playoff.comp = header.comp;
            playoff.numRoundsOverride = header.numRoundsOverride;
            playoff.teams = header.teams;
            playoff.awayGoalsMode = header.awayGoalsMode;
            playoff.subsAndMaxSubs = header.subsAndPointsForWin;
            playoff.roundStringBase = header.roundStringBase;
            for (slot, &rule) in playoff.roundRuleFlags.iter_mut().zip(&bytes.round_rules) {
                *slot = rule;
            }
            let mut pointers = playoff.name;
            names(&mut pointers, strings);
            playoff.name = pointers;
            Record::Playoff(playoff)
        }
    }
}

/// The session set for a start: its image, pitch and teams, and who controls each team.
pub(super) fn set_cup_session(
    before: &[u8; IMAGE_LEN],
    pitch_type: u16,
    teams: &[SelectedTeam],
    career_team: &SelectedTeam,
) {
    let prices: Vec<Prices> = teams.iter().map(|team| team.prices).collect();
    set_session(&Session {
        before,
        division: 0,
        pitch_type,
        teams: &prices,
        career_team: &career_team.prices,
    });
    let set_controls = |team: *mut TeamFileEx, controls: u8| {
        // SAFETY: a plain write to a team record of a global the lock's holder owns.
        unsafe { (*team).team.teamControls = controls };
    };
    for (ordinal, team) in teams.iter().enumerate() {
        set_controls(loaded(ordinal), team.controls);
    }
    set_controls((&raw mut diy::_careerTeam).cast(), career_team.controls);
}

/// `InitDIYCupFromRecord` of the record at `address`.
fn start_record(address: *mut u8) -> Result<Box<[u8; IMAGE_LEN]>, u16> {
    // SAFETY: `address` is a knockout's record; the function's externs are the compiled data's
    // and the stubs.
    super::halting(|| unsafe { diy::InitDIYCupFromRecord(address) })?;
    Ok(image())
}

/// `InitDIYCupFromRecord` of `setup`: the competition's image after, or upstream's halt.
pub(crate) fn start(setup: &CupSetup) -> Result<Box<[u8; IMAGE_LEN]>, u16> {
    let mut strings = Vec::new();
    let mut record = record(&setup.record, &mut strings);
    let address = record.address();
    set_cup_session(
        &setup.before,
        setup.pitch_type,
        &setup.teams,
        &setup.career_team,
    );
    with_lists(address, setup.national, setup.continental, || {
        start_record(address)
    })
}

/// `InitDIYCupFromRecord` of the compiled record `setup` names, upstream's tables as they are:
/// none when the tables have no such record.
pub(crate) fn start_compiled(
    setup: &CompiledCupSetup,
) -> Option<Result<Box<[u8; IMAGE_LEN]>, u16>> {
    set_cup_session(
        &setup.before,
        setup.pitch_type,
        &setup.teams,
        &setup.career_team,
    );
    with_compiled(setup.record, start_record)
}
