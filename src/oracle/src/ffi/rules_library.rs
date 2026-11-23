//! src/rules/'s season's moves, cups' clubs, parallel groups, grouped cup, fixed tiers' brackets
//! and checks, through src/swos_rules_*.c, over states and buffers the oracle owns.

use std::ffi::CStr;
use std::mem::MaybeUninit;

use super::bindings::swos_rules_check_c::{
    self as check, sr_league_check, sr_season_check, sr_spec_check,
};
use super::bindings::swos_rules_cupd_c::{
    sr_cupd_end, sr_cupd_end_reset, sr_cupd_keep_place, sr_cupd_make, sr_cupd_note_arrival,
    sr_cupd_rounds,
};
use super::bindings::swos_rules_cups_c::{sr_cup_entrants, sr_quota};
use super::bindings::swos_rules_dplay_c::{self as dplay, sr_italy, sr_out_held, sr_playoff_held};
use super::bindings::swos_rules_groups_c::{sr_pg_club, sr_pg_league, sr_pg_level, sr_pg_season};
use super::bindings::swos_rules_post_c::{self as post, sr_b_teams, sr_final_moves, sr_levels};
use crate::rules_library::{
    CLUB_SLOTS, CLUBS, CUP_SIDES, CheckOutcome, CupEnd, CupEndStep, CupEntrantsSetup, CupRounds,
    CupRoundsSetup, DIVISIONS, FixedTiersSetup, GroupsSetup, LeagueSetup, MovesSetup, RulesTables,
    SeasonCheckSetup, UNWRITTEN,
};

/// The moves the state holds (`SR_MOVES`).
const MOVES: usize = 48;

/// A cup's entrants at most (`SR_CUP_CLUBS`).
const CUP_CLUBS: usize = 64;

/// A zeroed C value (a plain struct of integers, arrays and pointers: zero is a valid value).
fn zeroed<T>() -> Box<T> {
    // SAFETY: the types it is used for are C structs of integers, arrays and raw pointers, for
    // which all-zero bytes are a valid value.
    unsafe { Box::new(MaybeUninit::<T>::zeroed().assume_init()) }
}

/// A unit's `sr_league` of `$setup` (a `LeagueSetup`), its arrays borrowed from it.
macro_rules! league {
    ($unit:ident, $setup:expr) => {{
        let setup: &LeagueSetup = $setup;
        let mut league = zeroed::<$unit::sr_league>();
        league.divisions = setup.divisions;
        league.promoted = setup.promoted.as_ptr();
        league.relegated = setup.relegated.as_ptr();
        league.italy = setup.italy;
        league.serie_b = setup.serie_b;
        league.group_first = setup.group_first;
        league.groups = setup.groups;
        if let Some(spec) = &setup.spec {
            league.spec = spec.as_ptr();
            league.spec_len = u32::try_from(spec.len()).expect("a spec's bytes");
        }
        league.seconds = u8::try_from(setup.seconds.len()).expect("a few second teams");
        league.second = setup.seconds.as_ptr();
        league.second_top = setup.second_top;
        league.second_flags = setup.second_flags;
        if let Some(words) = &setup.second_words {
            assert!(
                words.len() >= setup.seconds.len(),
                "a word each second team"
            );
            league.second_team = words.as_ptr();
        }
        league.team_file_high = setup.team_file_high;
        league
    }};
}

/// A unit's zeroed `sr_state` holding `$tables` (a `RulesTables`).
macro_rules! state {
    ($unit:ident, $tables:expr) => {{
        let mut state = zeroed::<$unit::sr_state>();
        tables!($unit, state, $tables);
        state
    }};
}

/// `$tables` (a `RulesTables`) written into a unit's `sr_state`.
macro_rules! tables {
    ($unit:ident, $state:expr, $tables:expr) => {{
        let tables: &RulesTables = $tables;
        assert!(
            usize::from(tables.divisions) <= DIVISIONS
                && tables
                    .counts
                    .iter()
                    .all(|&count| usize::from(count) <= CLUBS),
            "tables within the state"
        );
        let state = &mut $state;
        state.t.nd = tables.divisions;
        state.t.n = tables.counts;
        for (division, rows) in tables.rows.iter().enumerate() {
            for (place, &[team, glob, won, gf, ga, pts]) in rows.iter().enumerate() {
                state.t.t[division][place] = $unit::sr_row {
                    team,
                    glob,
                    won,
                    gf,
                    ga,
                    pts,
                };
            }
        }
    }};
}

/// The league's counts by place, as many as its divisions read.
fn counts_cover(league: &LeagueSetup) {
    let divisions = usize::from(league.divisions).min(DIVISIONS);
    assert!(
        league.promoted.len() >= divisions && league.relegated.len() >= divisions,
        "the counts the league's divisions read"
    );
}

/// `sr_final_moves` (`all`) or `sr_b_teams` of `setup`: the state's wants after.
pub(crate) fn moves(setup: &MovesSetup, all: bool) -> [[i8; CLUBS]; DIVISIONS] {
    assert!(setup.moves.len() <= MOVES, "the moves the state holds");
    counts_cover(&setup.league);
    let mut state = state!(post, &setup.tables);
    for (slot, &(glob, delta, rule)) in setup.moves.iter().enumerate() {
        state.mv[slot] = post::sr_move { glob, delta, rule };
    }
    state.nmv = i32::try_from(setup.moves.len()).expect("48 moves");
    state.want = setup.wants;
    let league = league!(post, &setup.league);
    let levels = setup.levels.as_ref().map(|first| sr_levels {
        levels: u8::try_from(first.len()).expect("a few levels"),
        first: first.as_ptr(),
    });
    let levels_ptr = levels.as_ref().map_or(std::ptr::null(), std::ptr::from_ref);
    // SAFETY: the state, the league and its arrays live through the call; the counts it reads
    // are within the arrays (checked above), its tables within their image.
    unsafe {
        if all {
            sr_final_moves(&raw mut *state, &raw const *league, levels_ptr, setup.base);
        } else {
            sr_b_teams(&raw mut *state, &raw const *league, levels_ptr, setup.base);
        }
    }
    state.want
}

/// `sr_cup_entrants` of `setup`.
pub(crate) fn cup_entrants(setup: &CupEntrantsSetup) -> Vec<u16> {
    let quotas: Vec<sr_quota> = setup
        .quotas
        .iter()
        .map(|&[division, clubs, byes]| sr_quota {
            division,
            clubs,
            byes,
        })
        .collect();
    let count = setup.slots.len();
    if count <= CLUB_SLOTS {
        assert!(setup.divisions.len() >= count, "a division each");
        let places = setup.places.as_ref().map_or(usize::MAX, Vec::len);
        assert!(
            setup.slots.iter().all(|&slot| usize::from(slot) < places),
            "a place each slot"
        );
        assert!(
            setup
                .quotas
                .iter()
                .all(|&[division, ..]| usize::from(division) < setup.levels.len()),
            "a level each quota"
        );
    }
    let mut out = [0u16; CUP_CLUBS];
    // SAFETY: every array the call reads is the setup's, within the bounds checked above; it
    // writes at most 64 slots.
    let written = unsafe {
        sr_cup_entrants(
            quotas.as_ptr(),
            i32::try_from(quotas.len()).expect("a few quotas"),
            setup.entrants,
            i32::try_from(count).expect("a country's clubs"),
            setup.slots.as_ptr(),
            setup.divisions.as_ptr(),
            setup
                .places
                .as_ref()
                .map_or(std::ptr::null(), |places| places.as_ptr()),
            setup.levels.as_ptr(),
            setup.career_slot,
            out.as_mut_ptr(),
        )
    };
    out[..usize::try_from(written).expect("64 at most")].to_vec()
}

/// `sr_pg_season` of `setup`.
pub(crate) fn parallel_groups(setup: &GroupsSetup) -> (i32, Vec<i8>) {
    let levels: Vec<sr_pg_level> = setup
        .levels
        .iter()
        .map(|&(first, groups, size)| sr_pg_level {
            first,
            groups,
            size,
        })
        .collect();
    let clubs: Vec<sr_pg_club> = setup
        .clubs
        .iter()
        .map(|&(base, area)| sr_pg_club { base, area })
        .collect();
    let count = clubs.len();
    assert!(
        setup.old.len() >= count
            && setup.new.len() >= count
            && setup
                .entering
                .as_ref()
                .is_none_or(|entering| entering.len() >= count),
        "an offset each"
    );
    let league = sr_pg_league {
        levels: u8::try_from(levels.len()).unwrap_or(u8::MAX),
        level: levels.as_ptr(),
        clubs: u16::try_from(count).unwrap_or(u16::MAX),
        club: clubs.as_ptr(),
    };
    let mut new = setup.new.clone();
    // SAFETY: the league's arrays and the offsets live through the call, an entry each club;
    // the caller checks no club stays above the chain's first group.
    let code = unsafe {
        sr_pg_season(
            &raw const league,
            setup.old.as_ptr(),
            new.as_mut_ptr(),
            setup
                .entering
                .as_ref()
                .map_or(std::ptr::null(), |entering| entering.as_ptr()),
        )
    };
    (code, new)
}

/// The image of `end`.
fn end_image(end: &sr_cupd_end) -> CupEnd {
    CupEnd {
        last_division: Box::new(end.last_div),
        place: Box::new(end.place),
        arrival: end.arrival,
    }
}

/// The grouped cup's season end after `steps`.
pub(crate) fn cup_end(steps: &[CupEndStep]) -> CupEnd {
    let mut end = zeroed::<sr_cupd_end>();
    // SAFETY: the season end is the oracle's; each call writes within it (a slot past it
    // ignored).
    unsafe {
        sr_cupd_end_reset(&raw mut *end);
        for &step in steps {
            match step {
                CupEndStep::Keep {
                    first,
                    groups,
                    division,
                    slot,
                    place,
                    clubs,
                } => sr_cupd_keep_place(&raw mut *end, first, groups, division, slot, place, clubs),
                CupEndStep::Arrival(slot) => sr_cupd_note_arrival(&raw mut *end, slot),
            }
        }
    }
    end_image(&end)
}

/// `sr_cupd_make` of `setup`.
pub(crate) fn cup_rounds(setup: &CupRoundsSetup) -> CupRounds {
    let end = setup.end.as_ref().map(|end| sr_cupd_end {
        last_div: *end.last_division,
        place: *end.place,
        arrival: end.arrival,
    });
    let mut out = zeroed::<sr_cupd_rounds>();
    out.side = [UNWRITTEN; CUP_SIDES];
    assert!(
        setup.words.len() >= setup.divisions.len().min(CLUB_SLOTS),
        "a team word each slot read"
    );
    // SAFETY: the season end, the slots' arrays and the rounds live through the call; it reads
    // the first `nslots` (at most 512) of each array, and the caller checks its preliminary
    // round takes no more sides than the level's clubs.
    let clubs = unsafe {
        sr_cupd_make(
            end.as_ref().map_or(std::ptr::null(), std::ptr::from_ref),
            setup.season,
            setup.rounds,
            setup.first,
            setup.groups,
            i32::try_from(setup.divisions.len()).expect("a few slots"),
            setup.divisions.as_ptr(),
            setup.words.as_ptr(),
            &raw mut *out,
        )
    };
    CupRounds {
        clubs,
        preliminary: out.np,
        sides: Box::new(out.side),
    }
}

/// `sr_italy` of `setup`: its return and the moves it made.
pub(crate) fn fixed_tiers(setup: &FixedTiersSetup) -> (i32, Vec<(u16, i8, u8)>) {
    counts_cover(&setup.league);
    let mut state = state!(dplay, &setup.tables);
    let league = league!(dplay, &setup.league);
    let mut season = zeroed::<dplay::sr_season>();
    let [po_win, out_win, cpo_win, cup_winner] = setup.winners;
    season.po_win = po_win;
    season.out_win = out_win;
    season.cpo_win = cpo_win;
    season.cup_winner = cup_winner;
    season.test_held = setup.test_held;
    // SAFETY: the state, the league (its arrays the setup's) and the season live through the
    // call; the caller checks the league's lower tier reaches no national round of four
    // groups (where the C writes past its tables).
    let code = unsafe { sr_italy(&raw mut *state, &raw const *league, &raw const *season) };
    let moves = state.mv[..usize::try_from(state.nmv).unwrap_or_default()]
        .iter()
        .map(|played| (played.glob, played.delta, played.rule))
        .collect();
    (code, moves)
}

/// `sr_out_held` and `sr_playoff_held` of `points`.
pub(crate) fn held([first, second]: [u16; 2]) -> (i32, i32) {
    // SAFETY: pure functions of their arguments.
    unsafe { (sr_out_held(first, second), sr_playoff_held(first, second)) }
}

/// The outcome of a check that returned `code` and wrote `error`.
fn outcome(code: i32, error: &check::sr_check_error) -> CheckOutcome {
    let field = (!error.field.is_null()).then(|| {
        // SAFETY: the check writes a pointer to one of its string literals, or none.
        unsafe { CStr::from_ptr(error.field) }
            .to_string_lossy()
            .into_owned()
    });
    CheckOutcome {
        code,
        play_off: error.pidx,
        offset: error.offset,
        field,
    }
}

/// `sr_spec_check` of `spec`.
pub(crate) fn spec_check(spec: &[u8], divisions: i32) -> CheckOutcome {
    let mut error = zeroed::<check::sr_check_error>();
    // SAFETY: the spec's bytes live through the call, its length theirs; the error is the
    // oracle's.
    let code = unsafe {
        sr_spec_check(
            spec.as_ptr(),
            u32::try_from(spec.len()).expect("a spec's bytes"),
            divisions,
            &raw mut *error,
        )
    };
    outcome(code, &error)
}

/// `sr_league_check` of `league`.
pub(crate) fn league_check(league: &LeagueSetup) -> CheckOutcome {
    let checked = league!(check, league);
    let mut error = zeroed::<check::sr_check_error>();
    // SAFETY: the league and its arrays live through the call; the error is the oracle's.
    let code = unsafe { sr_league_check(&raw const *checked, &raw mut *error) };
    outcome(code, &error)
}

/// `sr_season_check` of `setup`.
pub(crate) fn season_check(setup: &SeasonCheckSetup) -> CheckOutcome {
    let mut bank = zeroed::<check::sr_stage_bank>();
    let mut promedio = zeroed::<check::sr_promedio>();
    let mut winners = zeroed::<check::sr_winners>();
    let first = vec![0u8; setup.levels.map_or(0, usize::from)];
    let levels = check::sr_levels {
        levels: setup.levels.unwrap_or_default(),
        first: first.as_ptr(),
    };
    let mut season = zeroed::<check::sr_season>();
    if let Some((counts, kept_count, kept_division)) = setup.bank {
        bank.n = counts;
        bank.n_kept = kept_count;
        bank.kept_div = kept_division;
        season.bank = &raw mut *bank;
    }
    if let Some(count) = setup.promedio {
        promedio.n = count;
        season.promedio = &raw mut *promedio;
    }
    if let Some(count) = setup.winners {
        winners.n = count;
        season.winners = &raw mut *winners;
    }
    if setup.levels.is_some() {
        season.levels = &raw const levels;
    }
    let mut error = zeroed::<check::sr_check_error>();
    // SAFETY: the season and what it points to (the bank, the history, the page, the levels
    // and their first divisions, as many as they say) live through the call; the error is the
    // oracle's.
    let code = unsafe { sr_season_check(&raw const *season, &raw mut *error) };
    outcome(code, &error)
}

mod knockouts;

pub(crate) use knockouts::knockouts;
