//! src/diy_competition.c's `InitDIYCompetitionFromTableEntry`: a division of a record started
//! in `_diyCompetitionStart`; and what a cup's start shares with it (`ffi::cup`): the session,
//! upstream's lists for the call, its compiled records.

use std::ffi::{CString, c_char};

use super::bindings::competition_data_c;
use super::bindings::diy_competition_c::{
    self as diy, CompCupHeader, CompLeagueHeader, Competition, CompetitionCup, CompetitionLeague,
    Division,
};
use super::bindings::swos_amiga_ptr_imports_c as tables;
use super::draw::{loaded, set_prices};
use super::running::IMAGE_LEN;
use super::stubs::{self, LIST_END};

/// The marker a country table's cups follow (upstream's `(Competition *)-2`).
const CUPS_MARKER: isize = -2;

/// The end of a country table (upstream's `(Competition *)-1`).
const TABLE_END: isize = -1;
use crate::draw::Prices;
use crate::league::{CompiledRecord, CompiledSetup, DivisionSetup, RecordBytes};

/// The record's C struct, kept alive with the names it points at.
enum Record {
    /// A league's.
    League(Box<CompetitionLeague>),
    /// A cup's (played as a league).
    Cup(Box<CompetitionCup>),
}

impl Record {
    /// Its address.
    fn address(&mut self) -> *mut u8 {
        match self {
            Self::League(league) => (&raw mut **league).cast(),
            Self::Cup(cup) => (&raw mut **cup).cast(),
        }
    }
}

/// The header of `bytes` as the C's `Competition`.
pub(super) fn competition(bytes: [u8; 9]) -> Competition {
    let [
        comp_id,
        kind,
        team,
        begin,
        end,
        reserved1,
        playoff,
        reserved2,
        reorder,
    ] = bytes;
    Competition {
        compId: comp_id,
        type_: kind.cast_signed(),
        teamNumber: team.cast_signed(),
        beginMonth: begin.cast_signed(),
        endMonth: end.cast_signed(),
        reserved1: reserved1.cast_signed(),
        isPlayoff: playoff.cast_signed(),
        reserved2: reserved2.cast_signed(),
        seedReorderFlag: reorder.cast_signed(),
    }
}

/// `name` as a C string's pointer, its string kept in `strings`; null for none.
pub(super) fn pointer(name: Option<&Vec<u8>>, strings: &mut Vec<CString>) -> *mut c_char {
    match name {
        Some(name) => {
            let string = CString::new(name.clone()).expect("a name without 0s");
            let address = string.as_ptr().cast_mut();
            strings.push(string);
            address
        }
        None => std::ptr::null_mut(),
    }
}

/// The record `bytes` describes, its names in `strings`.
fn record(bytes: &RecordBytes, strings: &mut Vec<CString>) -> Record {
    match bytes {
        RecordBytes::League {
            header,
            divisions_count,
            rounds,
            points_for_win,
            substitutes,
            divisions,
            names,
        } => {
            // SAFETY: a record of zeroed bytes is a valid one (integers and null pointers).
            let mut league: Box<CompetitionLeague> = Box::new(unsafe { std::mem::zeroed() });
            league.header = CompLeagueHeader {
                comp: competition(*header),
                divisionsCount: *divisions_count,
                rounds: *rounds,
                pointsForWin: *points_for_win,
                subsAndMaxSubs: substitutes.cast_signed(),
            };
            for (slot, division) in league.divisions.iter_mut().zip(divisions) {
                let [teams, promoted, promoted_po, relegated, relegated_po, link] = *division;
                *slot = Division {
                    teams,
                    promoted,
                    promotedPo: promoted_po,
                    relegated,
                    relegatedPo: relegated_po,
                    qualifierLinkOffset: link,
                };
            }
            let mut pointers = league.name;
            for (slot, name) in pointers.iter_mut().zip(names) {
                *slot = pointer(name.as_ref(), strings);
            }
            league.name = pointers;
            Record::League(league)
        }
        RecordBytes::Cup {
            header,
            num_rounds_override,
            teams,
            away_goals_mode,
            substitutes,
            round_string_base,
            round_rules,
            names,
        } => {
            // SAFETY: as above.
            let mut cup: Box<CompetitionCup> = Box::new(unsafe { std::mem::zeroed() });
            cup.header = CompCupHeader {
                comp: competition(*header),
                numRoundsOverride: *num_rounds_override,
                teams: *teams,
                awayGoalsMode: *away_goals_mode,
                subsAndPointsForWin: substitutes.cast_signed(),
                roundStringBase: *round_string_base,
            };
            cup.roundRuleFlags = *round_rules;
            let mut pointers = cup.name;
            for (slot, name) in pointers.iter_mut().zip(names) {
                *slot = pointer(name.as_ref(), strings);
            }
            cup.name = pointers;
            Record::Cup(cup)
        }
    }
}

/// What a start reads besides its record: the image before, the division, the session's pitch
/// and the loaded teams' prices.
pub(super) struct Session<'a> {
    /// The competition's image before.
    pub(super) before: &'a [u8; IMAGE_LEN],
    /// The division.
    pub(super) division: u16,
    /// The session's pitch type.
    pub(super) pitch_type: u16,
    /// The selected teams' prices, by ordinal.
    pub(super) teams: &'a [Prices],
    /// The career club's prices.
    pub(super) career_team: &'a Prices,
}

/// The session's image, settings and teams set for a start.
pub(super) fn set_session(session: &Session<'_>) {
    for (ordinal, prices) in session.teams.iter().enumerate() {
        set_prices(loaded(ordinal), prices);
    }
    set_prices((&raw mut diy::_careerTeam).cast(), session.career_team);
    // SAFETY: plain writes to globals the lock's holder owns: the image over the struct, the
    // session's pitch.
    unsafe {
        (&raw mut diy::_diyCompetitionStart)
            .cast::<u8>()
            .copy_from_nonoverlapping(session.before.as_ptr(), IMAGE_LEN);
        stubs::_pitchType = session.pitch_type;
    }
}

/// The competition's image now.
pub(super) fn image() -> Box<[u8; IMAGE_LEN]> {
    let mut after = Box::new([0; IMAGE_LEN]);
    // SAFETY: a plain read of a global the lock's holder owns, its image's bytes.
    after.copy_from_slice(unsafe {
        std::slice::from_raw_parts(
            (&raw const diy::_diyCompetitionStart).cast::<u8>(),
            IMAGE_LEN,
        )
    });
    after
}

/// `InitDIYCompetitionFromTableEntry` of the compiled record `setup` names, upstream's tables
/// as they are: none when the tables have no such record.
pub(crate) fn start_compiled(setup: &CompiledSetup) -> Option<Result<Box<[u8; IMAGE_LEN]>, u16>> {
    let session = Session {
        before: &setup.before,
        division: setup.division,
        pitch_type: setup.pitch_type,
        teams: &setup.teams,
        career_team: &setup.career_team,
    };
    with_compiled(setup.record, |address| start_record(address, &session))
}

/// The record at the `slot`th entry after the country table `country`'s marker (-2) and before
/// its end (-1): none past them.
///
/// # Safety
///
/// The lock's holder owns upstream's globals.
unsafe fn country_cup(country: u8, slot: usize) -> Option<*mut u8> {
    // SAFETY: a plain read of a global the lock's holder owns; a country's table is upstream's
    // array of record pointers, ended by -1, its cups after its -2.
    unsafe {
        let tables = competition_data_c::_competitionsTable;
        let table = tables.get(usize::from(country))?.cast::<*mut u8>();
        if table.is_null() {
            return None;
        }
        let mut entry = table;
        while entry.read() as isize != CUPS_MARKER {
            if entry.read() as isize == TABLE_END {
                return None;
            }
            entry = entry.add(1);
        }
        for _ in 0..=slot {
            entry = entry.add(1);
            if entry.read() as isize == TABLE_END {
                return None;
            }
        }
        Some(entry.read())
    }
}

/// The compiled record of `comp_id` the oracle binds by name: the play-offs, and the
/// tournaments no table of records reaches (the Copa Libertadores has no compiled record in
/// upstream's SDL build: its loader builds it from the JSON).
fn bound(comp_id: u8) -> Option<*mut u8> {
    let records: [*mut u8; 9] = [
        (&raw mut competition_data_c::_div1Playoffs).cast(),
        (&raw mut competition_data_c::_div2Playoffs).cast(),
        (&raw mut competition_data_c::_div3Playoffs).cast(),
        (&raw mut competition_data_c::_dutchPlayoffs).cast(),
        (&raw mut competition_data_c::_scottishPlayoffs).cast(),
        (&raw mut competition_data_c::_spanishPlayoffs).cast(),
        (&raw mut competition_data_c::_uruguayanPlayoffs).cast(),
        (&raw mut competition_data_c::_worldCup).cast(),
        (&raw mut competition_data_c::_europeanChampionships).cast(),
    ];
    records
        .into_iter()
        // SAFETY: a plain read of a global the lock's holder owns: each record starts with its
        // `Competition` header, its `compId` first.
        .find(|&record| unsafe { record.read() } == comp_id)
}

/// `start` called on the compiled record `record` names, upstream's tables as they are: a
/// career copy made as `InitCareer` makes it, and put back after; none when the tables have no
/// such record.
pub(super) fn with_compiled<R>(
    record: CompiledRecord,
    start: impl FnOnce(*mut u8) -> R,
) -> Option<R> {
    match record {
        CompiledRecord::CountryLeague(country) => {
            // SAFETY: a plain read of a global the lock's holder owns; a country's table is
            // upstream's array of record pointers, its league first.
            let league = unsafe {
                let tables = competition_data_c::_competitionsTable;
                let table = tables.get(usize::from(country))?.cast::<*mut u8>();
                if table.is_null() {
                    return None;
                }
                table.read()
            };
            Some(start(league))
        }
        CompiledRecord::CountryCup { country, slot } => {
            // SAFETY: the lock's holder owns the globals.
            let cup = unsafe { country_cup(country, slot) }?;
            Some(start(cup))
        }
        CompiledRecord::Bound(comp_id) => bound(comp_id).map(start),
        CompiledRecord::CareerContest { comp_id, copy } => {
            // SAFETY: plain reads of a global the lock's holder owns; an entry with a copy pairs
            // two compiled records, the template starting with its base.
            let contest = unsafe { competition_data_c::_careerContests }
                .into_iter()
                .find(|contest| {
                    !contest.competitionCopy.is_null()
                        && unsafe { contest.competition.read_unaligned() }.compId == comp_id
                })?;
            let template = contest.competition.cast::<u8>();
            if !copy {
                return Some(start(template));
            }
            let target = contest.competitionCopy.cast::<u8>();
            let size = usize::from(contest.templateSize);
            // SAFETY: the copy's storage holds the template's size (career_comp_copies.c);
            // it is made as `InitCareer` makes it, and put back after the call.
            unsafe {
                let saved = std::slice::from_raw_parts(target, size).to_vec();
                target.copy_from_nonoverlapping(template, size);
                let result = start(target);
                target.copy_from_nonoverlapping(saved.as_ptr(), size);
                Some(result)
            }
        }
    }
}

/// `call` with upstream's two lists naming only `address` (when `national`, when
/// `continental`), put back after it.
pub(super) fn with_lists<R>(
    address: *mut u8,
    national: bool,
    continental: bool,
    call: impl FnOnce() -> R,
) -> R {
    let listed = |listed: bool| {
        let entry = if listed { address as usize } else { LIST_END };
        [
            entry as *mut std::ffi::c_void,
            LIST_END as *mut std::ffi::c_void,
        ]
    };
    // SAFETY: plain writes to globals the lock's holder owns, set for the call and put back
    // after it.
    unsafe {
        let (national_list, continental_list) = (
            tables::_nationalTeamCompetitionsAndQualifications1,
            tables::_europeanCupsandNationalTeamCompetitionsAndQualifications,
        );
        let mut list = national_list;
        list[..2].copy_from_slice(&listed(national));
        tables::_nationalTeamCompetitionsAndQualifications1 = list;
        let mut list = continental_list;
        list[..2].copy_from_slice(&listed(continental));
        tables::_europeanCupsandNationalTeamCompetitionsAndQualifications = list;
        let result = call();
        tables::_nationalTeamCompetitionsAndQualifications1 = national_list;
        tables::_europeanCupsandNationalTeamCompetitionsAndQualifications = continental_list;
        result
    }
}

/// `InitDIYCompetitionFromTableEntry` of the record at `address` in `session`.
fn start_record(address: *mut u8, session: &Session<'_>) -> Result<Box<[u8; IMAGE_LEN]>, u16> {
    set_session(session);
    // SAFETY: `address` is a compiled record; the function's externs are the compiled data's
    // and the stubs.
    super::halting(|| unsafe { diy::InitDIYCompetitionFromTableEntry(session.division, address) })?;
    Ok(image())
}

/// `InitDIYCompetitionFromTableEntry` of `setup`: the competition's image after, or upstream's
/// halt.
pub(crate) fn start(setup: &DivisionSetup) -> Result<Box<[u8; IMAGE_LEN]>, u16> {
    let mut strings = Vec::new();
    let mut record = record(&setup.record, &mut strings);
    let address = record.address();
    let division_names: Vec<CString> = setup
        .division_names
        .iter()
        .map(|name| CString::new(name.clone()).expect("a name without 0s"))
        .collect();
    set_session(&Session {
        before: &setup.before,
        division: setup.division,
        pitch_type: setup.pitch_type,
        teams: &setup.teams,
        career_team: &setup.career_team,
    });
    // SAFETY: plain writes to a global the lock's holder owns, the default division names, set
    // for the call and put back after it; the record and the names live through the call.
    unsafe {
        let names = tables::_divisionStringTable;
        let mut table = names;
        for (slot, name) in table.iter_mut().zip(&division_names) {
            *slot = name.as_ptr().cast_mut().cast();
        }
        tables::_divisionStringTable = table;
        let started = with_lists(address, setup.national, setup.continental, || {
            super::halting(|| diy::InitDIYCompetitionFromTableEntry(setup.division, address))
        });
        tables::_divisionStringTable = names;
        started?;
    }
    Ok(image())
}
