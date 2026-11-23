//! src/diy_competition.c's league play over `_diyCompetitionStart`: the next fixture, the
//! progress, a match's result recorded, the qualifiers, the season's moves; the teams named
//! and numbered in `_selectedTeams` and `_careerTeam`.

use super::bindings::diy_competition_c::{self as diy, TeamFileEx};
use super::bindings::{swos_amiga_data_imports_c as data, swos_data_bulk_imports_c as bulk};
use super::draw::loaded;
use super::league::image;
use super::running::IMAGE_LEN;
use super::stubs;
use super::tournament::{STATE_LEN, read, write};
use crate::play::{MovesCounted, ResultSetup, Teams};

/// The bytes of a standings row: a team's record is its row's offset.
const ROW_BYTES: usize = 18;

/// The bytes of a team's name field (`TeamFile.teamName`).
const NAME_BYTES: usize = 17;

/// The elements of the array `_array` points at.
fn len_of<T, const N: usize>(_array: *const [T; N]) -> usize {
    N
}

/// `image` written into `_diyCompetitionStart`.
fn write_competition(image: &[u8; IMAGE_LEN]) {
    // SAFETY: a plain write of the struct's size to a global the lock's holder owns.
    unsafe {
        (&raw mut diy::_diyCompetitionStart)
            .cast::<u8>()
            .copy_from_nonoverlapping(image.as_ptr(), IMAGE_LEN);
    }
}

/// The address of standings row `row`'s team record (the competition's start, 18 bytes a row).
fn record_of(row: usize) -> *mut u8 {
    (&raw mut diy::_diyCompetitionStart)
        .cast::<u8>()
        .wrapping_add(row * ROW_BYTES)
}

/// The row of the team record at `address`.
fn row_of(address: *const u8) -> usize {
    let base = (&raw const diy::_diyCompetitionStart).cast::<u8>();
    (address.addr() - base.addr()) / ROW_BYTES
}

/// `team`'s name and global number set.
fn set_team(team: *mut TeamFileEx, name: &[u8], number: u16) {
    // SAFETY: plain writes to a team record of a global the lock's holder owns, the name
    // within its field, ended by a 0.
    unsafe {
        let field = (&raw mut (*team).team.teamName).cast::<u8>();
        field.write_bytes(0, NAME_BYTES);
        field.copy_from_nonoverlapping(name.as_ptr(), name.len().min(NAME_BYTES - 1));
        (*team).team.globalTeamNumber = number.cast_signed();
    }
}

/// The teams' names and numbers set.
pub(super) fn set_teams(teams: &Teams) {
    for (ordinal, (name, number)) in teams.selected.iter().enumerate() {
        set_team(loaded(ordinal), name, *number);
    }
    let (name, number) = &teams.career;
    set_team((&raw mut diy::_careerTeam).cast(), name, *number);
}

/// `DIY_GetCurrentFixtureTeamPtrs` of `image`: the two sides' rows.
pub(crate) fn fixture(before: &[u8; IMAGE_LEN]) -> (usize, usize) {
    write_competition(before);
    // SAFETY: the function reads the global just written and upstream's round robins; it
    // writes the two pointers, read by value after it.
    unsafe {
        diy::DIY_GetCurrentFixtureTeamPtrs();
        (
            row_of(diy::_fixtureTeam1CompPtr),
            row_of(diy::_fixtureTeam2CompPtr),
        )
    }
}

/// `DIY_AdvanceLeagueProgressAfterMatch` of `image` after rows `home` and `away`'s match.
pub(crate) fn advance(before: &[u8; IMAGE_LEN], home: usize, away: usize) -> Box<[u8; IMAGE_LEN]> {
    write_competition(before);
    // SAFETY: the two records are rows of the global just written.
    unsafe { diy::DIY_AdvanceLeagueProgressAfterMatch(record_of(home), record_of(away)) };
    image()
}

/// `DIY_PlayOrSimulateLeagueFixture` of `setup`, simulated (selector 0), its result cached.
pub(crate) fn play_result(setup: &ResultSetup) -> (Box<[u8; IMAGE_LEN]>, Box<[u8; STATE_LEN]>) {
    set_teams(&setup.teams);
    write_competition(&setup.competition);
    write(&setup.state);
    let (home, away) = setup.goals;
    // SAFETY: plain writes to globals the lock's holder owns: the cached result the match
    // takes (a signed word: 0 and up), the career's flags the stubs own.
    unsafe {
        data::_cachedViewResultTeam1Goals = [home];
        data::_cachedViewResultTeam2Goals = [away];
        stubs::_gameType = 0;
        stubs::_careerLoadedFlag = 0;
    }
    // SAFETY: the function reads and writes the globals just set; its externs are the compiled
    // data's and the stubs.
    super::halting(|| unsafe {
        diy::DIY_PlayOrSimulateLeagueFixture(0, 0);
    })
    .expect("a league match does not halt");
    (image(), read())
}

/// `DIY_UpdateLeagueQualifiers` of `image`, the tournament's state `state`, the teams `teams`.
pub(crate) fn update_qualifiers(
    image_before: &[u8; IMAGE_LEN],
    state: &[u8; STATE_LEN],
    teams: &Teams,
) -> Box<[u8; IMAGE_LEN]> {
    set_teams(teams);
    write_competition(image_before);
    write(state);
    // SAFETY: the function reads and writes the globals just set.
    unsafe { diy::DIY_UpdateLeagueQualifiers() };
    image()
}

/// `DIY_UpdatePromotionRelegationScratchCounts` of `image`, the teams `teams`: each team's
/// change in the scratch, from 0, and the career club's flags.
pub(crate) fn season_moves(image_before: &[u8; IMAGE_LEN], teams: &Teams) -> MovesCounted {
    set_teams(teams);
    write_competition(image_before);
    let numbers: Vec<u16> = teams.selected.iter().map(|&(_, number)| number).collect();
    let career = teams.career.1;
    // SAFETY: plain writes and reads of globals the lock's holder owns, each number within the
    // scratch (`GLOBAL_TEAM_NUMBERS` bytes); the function reads the image just written.
    unsafe {
        let scratch = (&raw mut bulk::_diyLeagueScratchBuffer).cast::<u8>();
        let len = len_of(&raw const bulk::_diyLeagueScratchBuffer);
        for &number in numbers.iter().chain(std::iter::once(&career)) {
            assert!(usize::from(number) < len, "a number of the scratch");
            scratch.add(usize::from(number)).write(0);
        }
        stubs::_playPromotedAnimation = 0;
        stubs::_playRelegatedAnimation = 0;
        diy::DIY_UpdatePromotionRelegationScratchCounts();
        let change = |number: u16| scratch.add(usize::from(number)).read().cast_signed();
        MovesCounted {
            selected: numbers.iter().map(|&number| change(number)).collect(),
            career: change(career),
            promoted: stubs::_playPromotedAnimation != 0,
            relegated: stubs::_playRelegatedAnimation != 0,
        }
    }
}
