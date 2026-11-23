//! The pack's knockouts at the season end (upstream src/rules/sr_gplay.c `sr_knockouts`, MIT:
//! Davide Lorigliola's, src/rules/LICENSE-MIT; M5 #9a3), compiled as upstream builds it: seasons
//! one after another over one library state, the league's stage tables, its promedio history
//! and the SEASON WINNERS page kept between them as the caller keeps them, the host answering
//! from the season's tables.

use crate::Oracle;
use crate::ffi;
use crate::rules_library::{CLUBS, LeagueSetup, RulesTables};

/// The stage tables a bank holds (`SR_STAGE_TABLES`).
pub const STAGE_TABLES: usize = 4;

/// The clubs a promedio history holds (`SR_PROMEDIO_CLUBS`).
pub const PROMEDIO_CLUBS: usize = 48;

/// The SEASON WINNERS page's entries (`SR_WINNERS_MAX`).
pub const WINNERS: usize = 40;

/// The groups' placings the state's figures hold (`ps`).
pub const PLACINGS: usize = 64;

/// A row (`sr_row`): team, glob, won, gf, ga, pts.
pub type RowSetup = [u16; 6];

/// A result (`sr_result`): kind, div, team, glob.
pub type ResultSetup = (u8, u8, [u16; 2], [u16; 2]);

/// A group table's row (`sr_gs`): c, pts, gd, gf.
pub type FiguresSetup = [i32; 4];

/// A promedio history's club (`sr_promedio_club`): glob, points, matches.
pub type PromedioClubSetup = (u16, [u16; 2], [u8; 2]);

/// A league's stage tables (`sr_stage_bank`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankSetup {
    /// `used`.
    pub used: u8,
    /// `key`.
    pub key: u8,
    /// `fresh`.
    pub fresh: u8,
    /// `n`.
    pub counts: [u8; STAGE_TABLES],
    /// `r`.
    pub tables: Box<[[RowSetup; CLUBS]; STAGE_TABLES]>,
    /// `kept_div`.
    pub kept_division: u8,
    /// `n_kept`.
    pub kept_count: u8,
    /// `kept`.
    pub kept: Box<[RowSetup; CLUBS]>,
}

/// A promedio history (`sr_promedio`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromedioSetup {
    /// `init`.
    pub init: u8,
    /// `n`.
    pub count: u8,
    /// `c`: each club's global number, points and matches.
    pub clubs: Box<[PromedioClubSetup; PROMEDIO_CLUBS]>,
}

/// The SEASON WINNERS page (`sr_winners`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinnersSetup {
    /// `season`.
    pub season: u8,
    /// `n`.
    pub count: u8,
    /// `fresh`.
    pub fresh: u8,
    /// `e`: each entry's type, index and team word.
    pub entries: [(u8, u8, u16); WINNERS],
}

/// What the host answers (`sr_host`; no log).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostAnswers {
    /// `played_tie`: play-off, the clubs (either order) and the winner.
    pub ties: Vec<(i32, u16, u16, u16)>,
    /// `played_title`: play-off, champion and runner-up.
    pub titles: Vec<(i32, u16, u16)>,
    /// `stage_title`: stage, team words and global numbers.
    pub stage_titles: Vec<(i32, [u16; 2], [u16; 2])>,
}

impl HostAnswers {
    /// `played_tie`'s answer: the winner of play-off `play_off`'s tie of `a` and `b`.
    #[must_use]
    pub fn tie(&self, play_off: i32, a: u16, b: u16) -> Option<u16> {
        self.ties
            .iter()
            .find(|&&(index, x, y, _)| index == play_off && ((x, y) == (a, b) || (x, y) == (b, a)))
            .map(|&(.., winner)| winner)
    }

    /// `played_title`'s answer: play-off `play_off`'s champion and runner-up.
    #[must_use]
    pub fn title(&self, play_off: i32) -> Option<(u16, u16)> {
        self.titles
            .iter()
            .find(|&&(index, ..)| index == play_off)
            .map(|&(_, champion, runner_up)| (champion, runner_up))
    }

    /// `stage_title`'s answer: stage `stage`'s champion and runner-up.
    #[must_use]
    pub fn stage_title(&self, stage: i32) -> Option<([u16; 2], [u16; 2])> {
        self.stage_titles
            .iter()
            .find(|&&(index, ..)| index == stage)
            .map(|&(_, team, glob)| (team, glob))
    }
}

/// A season's end: its tables and the state's moves before, what `sr_season` holds for the
/// knockouts, the host, the bank as the season left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnockoutsSeason {
    /// The tables (`st->t`).
    pub tables: RulesTables,
    /// The state's moves before (`st->mv`): global number, delta, rule.
    pub moves: Vec<(u16, i8, u8)>,
    /// `mgr`.
    pub manager: u16,
    /// `tie_a`, `tie_b`, `tie_w`.
    pub tie: [u16; 3],
    /// `preview`.
    pub preview: u8,
    /// `own_pi`.
    pub own: [u8; 2],
    /// `b_in_ko`.
    pub seconds_in: u8,
    /// `base`.
    pub base: u16,
    /// `levels`: each level's first division (none: a null pointer).
    pub levels: Option<Vec<u8>>,
    /// `host` (none: a null pointer).
    pub host: Option<HostAnswers>,
    /// The bank as the season wrote it, when the run keeps one (none: as the last season end
    /// left it).
    pub bank: Option<BankSetup>,
}

/// Season ends of one league over one library state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnockoutsRun {
    /// The league.
    pub league: LeagueSetup,
    /// A bank kept (`se->bank`), zeroed at first; none a null pointer.
    pub bank: bool,
    /// The promedio history at first (none: a null pointer).
    pub promedio: Option<PromedioSetup>,
    /// The SEASON WINNERS page at first (none: a null pointer).
    pub winners: Option<WinnersSetup>,
    /// The seasons.
    pub seasons: Vec<KnockoutsSeason>,
}

/// What a season end left: its return, the state's moves, results, stage titles and groups'
/// figures, the bank, the promedio history, the SEASON WINNERS page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnockoutsAfter {
    /// The return (moves added, -1 refused).
    pub code: i32,
    /// `st->mv[..nmv]`.
    pub moves: Vec<(u16, i8, u8)>,
    /// `gp.res[..nres]`.
    pub results: Vec<ResultSetup>,
    /// `gp.sres`.
    pub stage_titles: [ResultSetup; 3],
    /// `gp.ms`.
    pub member_figures: Box<[FiguresSetup; CLUBS]>,
    /// `gp.ps`.
    pub placing_figures: Box<[FiguresSetup; PLACINGS]>,
    /// The bank, when kept.
    pub bank: Option<BankSetup>,
    /// The promedio history, when kept.
    pub promedio: Option<PromedioSetup>,
    /// The page, when kept.
    pub winners: Option<WinnersSetup>,
}

impl Oracle {
    /// `sr_knockouts` over the first `seasons` of `run`, one after another on one state: what
    /// each left.
    ///
    /// # Panics
    ///
    /// When the league's counts by place or second teams' words are fewer than it reads, or
    /// the tables past the state's.
    #[must_use]
    pub fn knockouts(&mut self, run: &KnockoutsRun, seasons: usize) -> Vec<KnockoutsAfter> {
        ffi::rules_library::knockouts(run, seasons)
    }
}
