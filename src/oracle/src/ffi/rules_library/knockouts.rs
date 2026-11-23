//! src/rules/'s knockouts (`sr_knockouts`), through src/swos_rules_gplay.c, over a state, a
//! stage bank, a promedio history and a SEASON WINNERS page the oracle owns; the host's
//! callbacks answer from the season's tables.

use std::ffi::c_void;
use std::ptr;

use super::super::bindings::swos_rules_gplay_c::{self as gplay, sr_knockouts};
use super::{counts_cover, zeroed};
use crate::knockouts::{
    BankSetup, HostAnswers, KnockoutsAfter, KnockoutsRun, PromedioSetup, RowSetup, WinnersSetup,
};
// (`league!` and `tables!` read them by name.)
use crate::rules_library::{CLUBS, DIVISIONS, LeagueSetup, RulesTables};

/// The moves the state holds (`SR_MOVES`).
const MOVES: usize = 48;

/// A row of the C's.
fn row(&[team, glob, won, gf, ga, pts]: &RowSetup) -> gplay::sr_row {
    gplay::sr_row {
        team,
        glob,
        won,
        gf,
        ga,
        pts,
    }
}

/// A row of the setup's.
fn row_setup(row: &gplay::sr_row) -> RowSetup {
    [row.team, row.glob, row.won, row.gf, row.ga, row.pts]
}

/// The bank `setup` written into `bank`.
fn write_bank(bank: &mut gplay::sr_stage_bank, setup: &BankSetup) {
    bank.used = setup.used;
    bank.key = setup.key;
    bank.fresh = setup.fresh;
    bank.n = setup.counts;
    for (rows, table) in bank.r.iter_mut().zip(setup.tables.iter()) {
        for (to, from) in rows.iter_mut().zip(table) {
            *to = row(from);
        }
    }
    bank.kept_div = setup.kept_division;
    bank.n_kept = setup.kept_count;
    for (to, from) in bank.kept.iter_mut().zip(setup.kept.iter()) {
        *to = row(from);
    }
}

/// The bank as the setup's.
fn bank_setup(bank: &gplay::sr_stage_bank) -> BankSetup {
    BankSetup {
        used: bank.used,
        key: bank.key,
        fresh: bank.fresh,
        counts: bank.n,
        tables: Box::new(bank.r.map(|table| table.map(|played| row_setup(&played)))),
        kept_division: bank.kept_div,
        kept_count: bank.n_kept,
        kept: Box::new(bank.kept.map(|played| row_setup(&played))),
    }
}

/// The promedio history of `setup`.
fn promedio(setup: &PromedioSetup) -> Box<gplay::sr_promedio> {
    let mut promedio = zeroed::<gplay::sr_promedio>();
    promedio.init = setup.init;
    promedio.n = setup.count;
    for (to, &(glob, points, matches)) in promedio.c.iter_mut().zip(setup.clubs.iter()) {
        *to = gplay::sr_promedio_club {
            glob,
            p: points,
            m: matches,
        };
    }
    promedio
}

/// The promedio history as the setup's.
fn promedio_setup(promedio: &gplay::sr_promedio) -> PromedioSetup {
    PromedioSetup {
        init: promedio.init,
        count: promedio.n,
        clubs: Box::new(promedio.c.map(|club| (club.glob, club.p, club.m))),
    }
}

/// The SEASON WINNERS page of `setup`.
fn winners(setup: &WinnersSetup) -> Box<gplay::sr_winners> {
    let mut winners = zeroed::<gplay::sr_winners>();
    winners.season = setup.season;
    winners.n = setup.count;
    winners.fresh = setup.fresh;
    for (to, &(kind, idx, team)) in winners.e.iter_mut().zip(&setup.entries) {
        *to = gplay::sr_winners_entry {
            type_: kind,
            idx,
            team,
        };
    }
    winners
}

/// The page as the setup's.
fn winners_setup(winners: &gplay::sr_winners) -> WinnersSetup {
    WinnersSetup {
        season: winners.season,
        count: winners.n,
        fresh: winners.fresh,
        entries: winners.e.map(|entry| (entry.type_, entry.idx, entry.team)),
    }
}

/// A result as the setup's.
fn result(result: &gplay::sr_result) -> (u8, u8, [u16; 2], [u16; 2]) {
    (result.kind, result.div, result.team, result.glob)
}

/// The season's answers behind the host's context.
///
/// # Safety
///
/// `ctx` is a `HostAnswers` living through the call.
unsafe fn answers<'a>(ctx: *mut c_void) -> &'a HostAnswers {
    // SAFETY: as the caller says.
    unsafe { &*ctx.cast::<HostAnswers>() }
}

/// `played_tie`: the answers' winner of the tie.
unsafe extern "C-unwind" fn played_tie(
    ctx: *mut c_void,
    pidx: i32,
    a: u16,
    b: u16,
    winner: *mut u16,
) -> i32 {
    // SAFETY: the host's context is the season's answers, living through `sr_knockouts`.
    let answers = unsafe { answers(ctx) };
    answers.tie(pidx, a, b).map_or(0, |played| {
        // SAFETY: the C passes its local to write.
        unsafe { winner.write(played) };
        1
    })
}

/// `played_title`: the answers' champion and runner-up.
unsafe extern "C-unwind" fn played_title(
    ctx: *mut c_void,
    pidx: i32,
    champion: *mut u16,
    runner_up: *mut u16,
) -> i32 {
    // SAFETY: the host's context is the season's answers, living through `sr_knockouts`.
    let answers = unsafe { answers(ctx) };
    answers.title(pidx).map_or(0, |(first, second)| {
        // SAFETY: the C passes its locals to write.
        unsafe {
            champion.write(first);
            runner_up.write(second);
        }
        1
    })
}

/// `stage_title`: the answers' stage title.
unsafe extern "C-unwind" fn stage_title(
    ctx: *mut c_void,
    k: i32,
    team: *mut u16,
    glob: *mut u16,
) -> i32 {
    // SAFETY: the host's context is the season's answers, living through `sr_knockouts`.
    let answers = unsafe { answers(ctx) };
    answers.stage_title(k).map_or(0, |(teams, globs)| {
        // SAFETY: the C passes its arrays of two to write.
        unsafe {
            ptr::copy_nonoverlapping(teams.as_ptr(), team, teams.len());
            ptr::copy_nonoverlapping(globs.as_ptr(), glob, globs.len());
        }
        1
    })
}

/// `sr_knockouts` over the first `seasons` of `run`: what each left.
pub(crate) fn knockouts(run: &KnockoutsRun, seasons: usize) -> Vec<KnockoutsAfter> {
    counts_cover(&run.league);
    let mut league = league!(gplay, &run.league);
    let seeds: Vec<gplay::sr_promedio_seed> = run
        .league
        .promedio_seeds
        .iter()
        .map(
            |&([glob, points2, points1], [matches2, matches1])| gplay::sr_promedio_seed {
                glob,
                points2,
                points1,
                matches2,
                matches1,
            },
        )
        .collect();
    league.promedio_seed = seeds.as_ptr();
    league.promedio_seeds = u8::try_from(seeds.len()).expect("a few seeds");
    let mut state = zeroed::<gplay::sr_state>();
    let mut bank = run.bank.then(zeroed::<gplay::sr_stage_bank>);
    let mut promedio = run.promedio.as_ref().map(promedio);
    let mut winners = run.winners.as_ref().map(winners);
    let mut after = Vec::with_capacity(seasons);
    for season in run.seasons.iter().take(seasons) {
        tables!(gplay, *state, &season.tables);
        assert!(season.moves.len() <= MOVES, "the moves the state holds");
        state.mv = [gplay::sr_move {
            glob: 0,
            delta: 0,
            rule: 0,
        }; MOVES];
        for (to, &(glob, delta, rule)) in state.mv.iter_mut().zip(&season.moves) {
            *to = gplay::sr_move { glob, delta, rule };
        }
        state.nmv = i32::try_from(season.moves.len()).expect("48 moves");
        if let (Some(bank), Some(setup)) = (bank.as_deref_mut(), &season.bank) {
            write_bank(bank, setup);
        }
        let levels = season.levels.as_ref().map(|first| gplay::sr_levels {
            levels: u8::try_from(first.len()).expect("a few levels"),
            first: first.as_ptr(),
        });
        let host = season.host.as_ref().map(|answers| gplay::sr_host {
            ctx: ptr::from_ref(answers).cast_mut().cast(),
            played_tie: Some(played_tie),
            played_title: Some(played_title),
            stage_title: Some(stage_title),
            log: None,
        });
        let mut se = zeroed::<gplay::sr_season>();
        se.mgr = season.manager;
        [se.tie_a, se.tie_b, se.tie_w] = season.tie;
        se.bank = bank.as_deref_mut().map_or(ptr::null_mut(), ptr::from_mut);
        se.promedio = promedio
            .as_deref_mut()
            .map_or(ptr::null_mut(), ptr::from_mut);
        se.winners = winners
            .as_deref_mut()
            .map_or(ptr::null_mut(), ptr::from_mut);
        se.host = host.as_ref().map_or(ptr::null(), ptr::from_ref);
        se.preview = season.preview;
        se.own_pi = season.own;
        se.b_in_ko = season.seconds_in;
        se.base = season.base;
        se.levels = levels.as_ref().map_or(ptr::null(), ptr::from_ref);
        // SAFETY: the state, the league (its arrays the run's), the season and what it points
        // to (the bank, the history, the page, the host and its answers, the levels) live
        // through the call; the counts the league reads are within its arrays (checked above),
        // the tables within the state's. The caller runs no season where the C reads past its
        // memory or a round it did not play (the port says so).
        let code = unsafe { sr_knockouts(&raw mut *state, &raw const *league, &raw const *se) };
        let made = usize::try_from(state.nmv).unwrap_or_default().min(MOVES);
        let gp = &state.gp;
        let results = usize::try_from(gp.nres).unwrap_or_default();
        after.push(KnockoutsAfter {
            code,
            moves: state.mv[..made]
                .iter()
                .map(|made| (made.glob, made.delta, made.rule))
                .collect(),
            results: gp.res[..results.min(gp.res.len())]
                .iter()
                .map(result)
                .collect(),
            stage_titles: gp.sres.each_ref().map(result),
            member_figures: Box::new(gp.ms.map(|row| [row.c, row.pts, row.gd, row.gf])),
            placing_figures: Box::new(gp.ps.map(|row| [row.c, row.pts, row.gd, row.gf])),
            bank: bank.as_deref().map(bank_setup),
            promedio: promedio.as_deref().map(promedio_setup),
            winners: winners.as_deref().map(winners_setup),
        });
    }
    after
}
