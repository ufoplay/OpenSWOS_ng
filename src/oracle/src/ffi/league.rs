//! src/diy_competition.c's `InitDIYCompetitionFromTableEntry`: a division of a record started
//! in `_diyCompetitionStart`.

use std::ffi::{CString, c_char};

use super::bindings::diy_competition_c::{
    self as diy, CompCupHeader, CompLeagueHeader, Competition, CompetitionCup, CompetitionLeague,
    Division,
};
use super::draw::{loaded, set_prices};
use super::running::IMAGE_LEN;
use super::stubs::{self, DIVISION_NAMES, LIST_END};
use crate::league::{DivisionSetup, RecordBytes};

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
fn competition(bytes: [u8; 9]) -> Competition {
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
fn pointer(name: Option<&Vec<u8>>, strings: &mut Vec<CString>) -> *mut c_char {
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
    for (ordinal, prices) in setup.teams.iter().enumerate() {
        set_prices(loaded(ordinal), prices);
    }
    set_prices((&raw mut diy::_careerTeam).cast(), &setup.career_team);
    // SAFETY: plain writes to globals the lock's holder owns: the image over the struct, the
    // stubbed lists, table and settings; the record and the names live through the call.
    unsafe {
        let competition = &raw mut diy::_diyCompetitionStart;
        competition
            .cast::<u8>()
            .copy_from_nonoverlapping(setup.before.as_ptr(), IMAGE_LEN);
        stubs::_pitchType = setup.pitch_type;
        let mut table = [std::ptr::null(); DIVISION_NAMES];
        for (slot, name) in table.iter_mut().zip(&division_names) {
            *slot = name.as_ptr().cast();
        }
        stubs::_divisionStringTable = table;
        let listed = |listed: bool| if listed { address as usize } else { LIST_END };
        stubs::_nationalTeamCompetitionsAndQualifications1 = [listed(setup.national), LIST_END];
        stubs::_europeanCupsandNationalTeamCompetitionsAndQualifications =
            [listed(setup.continental), LIST_END];
        super::halting(|| diy::InitDIYCompetitionFromTableEntry(setup.division, address))?;
        let mut after = Box::new([0; IMAGE_LEN]);
        after.copy_from_slice(std::slice::from_raw_parts(
            competition.cast::<u8>(),
            IMAGE_LEN,
        ));
        Ok(after)
    }
}
