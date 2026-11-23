//! The rules library (upstream src/rules/, MIT: Davide Lorigliola's, src/rules/LICENSE-MIT; M5
//! #9a), compiled as upstream builds it: the season's moves (`sr_final_moves`, `sr_b_teams`)
//! over a library state the oracle owns, a cup's clubs (`sr_cup_entrants`), parallel groups
//! (`sr_pg_season`), the grouped cup's season end and later rounds (`sr_cupd_*`) (#9a1); the
//! fixed tiers' brackets (`sr_italy`, `sr_out_held`, `sr_playoff_held`) and the checks of a
//! league's rules and spec (`sr_league_check`, `sr_spec_check`) (#9a2); the season's check
//! (`sr_season_check`) (#9a3; the knockouts: `knockouts`).

use crate::Oracle;
use crate::ffi;

/// The divisions the library's tables hold (`SR_DIVS`).
pub const DIVISIONS: usize = 16;

/// The clubs a division's table holds (`SR_CLUBS`).
pub const CLUBS: usize = 32;

/// The sides the grouped cup's two rounds hold (`2 * (SR_CUPD_PRE + SR_CUPD_R1)`).
pub const CUP_SIDES: usize = 210;

/// A country's clubs at most (`SR_CLUB_SLOTS`).
pub const CLUB_SLOTS: usize = 512;

/// A league's final tables as the library's state holds them (`sr_tables`): the divisions in
/// use, each division's clubs, each row (`team, glob, won, gf, ga, pts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesTables {
    /// `nd`.
    pub divisions: u8,
    /// `n`.
    pub counts: [u8; DIVISIONS],
    /// `t`.
    pub rows: Box<[[[u16; 6]; CLUBS]; DIVISIONS]>,
}

/// A league's rules (`sr_league`), its fields as the C reads them (the others 0).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LeagueSetup {
    /// `divisions`.
    pub divisions: u8,
    /// `promoted`, per division.
    pub promoted: Vec<u8>,
    /// `relegated`, per division.
    pub relegated: Vec<u8>,
    /// `italy`.
    pub italy: u8,
    /// `serie_b`.
    pub serie_b: u8,
    /// `group_first`.
    pub group_first: u8,
    /// `groups`.
    pub groups: u8,
    /// `spec` (none: a null pointer), `spec_len` its length.
    pub spec: Option<Vec<u8>>,
    /// `second`: each B team's global number less the base, its first team's (255 none).
    pub seconds: Vec<[u8; 2]>,
    /// `second_top` (255 any).
    pub second_top: u8,
    /// `second_flags`.
    pub second_flags: u8,
    /// `second_team`: each second team's team word (none: a null pointer).
    pub second_words: Option<Vec<u16>>,
    /// `promedio_seed`: each club's global number, points two seasons and one season before,
    /// then its matches.
    pub promedio_seeds: Vec<([u16; 3], [u8; 2])>,
    /// `team_file_high`.
    pub team_file_high: u8,
}

/// What the season's moves read: the tables, the play-offs' moves, the league's rules, its
/// levels, its base, the state's moves before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovesSetup {
    /// The tables (`st->t`).
    pub tables: RulesTables,
    /// The play-offs' moves (`st->mv`): global number, delta, rule.
    pub moves: Vec<(u16, i8, u8)>,
    /// The league's rules.
    pub league: LeagueSetup,
    /// The levels' first divisions (`sr_levels`), none for a null pointer.
    pub levels: Option<Vec<u8>>,
    /// `base`.
    pub base: u16,
    /// `st->want` before.
    pub wants: [[i8; CLUBS]; DIVISIONS],
}

/// What a cup's clubs read (`sr_cup_entrants`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CupEntrantsSetup {
    /// The quotas: division, clubs, byes.
    pub quotas: Vec<[u8; 3]>,
    /// `n`, the entrants in all.
    pub entrants: i32,
    /// `slot`, a club's each.
    pub slots: Vec<u16>,
    /// `division`, a club's each (at least as many as the slots).
    pub divisions: Vec<u8>,
    /// `place`, by slot (none: a null pointer).
    pub places: Option<Vec<u8>>,
    /// `level`, by division.
    pub levels: Vec<u8>,
    /// `career_slot`.
    pub career_slot: i32,
}

/// What a season end's parallel groups read (`sr_pg_season`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupsSetup {
    /// Each level: its first division, its groups, their sizes.
    pub levels: Vec<(u8, u8, [u8; 9])>,
    /// Each club: its base, its areas.
    pub clubs: Vec<(u8, [u8; 4])>,
    /// `old_off`.
    pub old: Vec<i8>,
    /// `new_off` before.
    pub new: Vec<i8>,
    /// `entering` (none: a null pointer).
    pub entering: Option<Vec<u8>>,
}

/// A step of the grouped cup's season end (`sr_cupd_keep_place`, `sr_cupd_note_arrival`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CupEndStep {
    /// `sr_cupd_keep_place(e, first, groups, division, slot, place, clubs)`.
    Keep {
        /// `first`.
        first: u8,
        /// `groups`.
        groups: u8,
        /// `division`.
        division: u16,
        /// `slot`.
        slot: u16,
        /// `place`.
        place: i32,
        /// `clubs`.
        clubs: i32,
    },
    /// `sr_cupd_note_arrival(e, slot)`.
    Arrival(u16),
}

/// The grouped cup's season end (`sr_cupd_end`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CupEnd {
    /// `last_div`.
    pub last_division: Box<[u8; CLUB_SLOTS]>,
    /// `place`.
    pub place: Box<[u8; CLUB_SLOTS]>,
    /// `arrival`.
    pub arrival: [u8; CLUB_SLOTS / 8],
}

/// What the grouped cup's later rounds read (`sr_cupd_make`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CupRoundsSetup {
    /// The season end (none: a null pointer).
    pub end: Option<CupEnd>,
    /// `season`.
    pub season: u32,
    /// `rounds`.
    pub rounds: u8,
    /// `first`.
    pub first: u8,
    /// `groups`.
    pub groups: u8,
    /// `division`, by slot (`nslots` its length).
    pub divisions: Vec<u8>,
    /// `word`, by slot.
    pub words: Vec<u16>,
}

/// What `sr_cupd_make` leaves: its return, the preliminary ties, every side (the call's
/// buffer filled with [`UNWRITTEN`] first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CupRounds {
    /// The return: the level's clubs, or -1.
    pub clubs: i32,
    /// `np`.
    pub preliminary: u8,
    /// `side`.
    pub sides: Box<[u16; CUP_SIDES]>,
}

/// A side the call did not write.
pub const UNWRITTEN: u16 = 0xEEEE;

/// What the fixed tiers' brackets read (`sr_italy`): the tables, the league's rules, the
/// season's results (`po_win`, `out_win`, `cpo_win`, `cup_winner`) and `test_held`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedTiersSetup {
    /// The tables (`st->t`).
    pub tables: RulesTables,
    /// The league's rules.
    pub league: LeagueSetup,
    /// `po_win`, `out_win`, `cpo_win`, `cup_winner`.
    pub winners: [u16; 4],
    /// `test_held`.
    pub test_held: u8,
}

/// What the season's check reads (`sr_season`), each none for a null pointer: its stage
/// tables' counts, its kept table's count and division (`n`, `n_kept`, `kept_div`); its promedio
/// history's and its SEASON WINNERS page's counts; its levels' count (their first divisions
/// all 0).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeasonCheckSetup {
    /// The bank's `n`, `n_kept` and `kept_div`.
    pub bank: Option<([u8; 4], u8, u8)>,
    /// The promedio history's `n`.
    pub promedio: Option<u8>,
    /// The page's `n`.
    pub winners: Option<u8>,
    /// The levels' `levels`.
    pub levels: Option<u8>,
}

/// What a check found (`sr_check_error` and the return): the code (0 sound), the play-off (-1
/// none), the byte, the field's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    /// The return, `SR_CHECK_*`.
    pub code: i32,
    /// `pidx`.
    pub play_off: i32,
    /// `offset`.
    pub offset: u32,
    /// `field`, none for a null pointer.
    pub field: Option<String>,
}

impl Oracle {
    /// `sr_final_moves` of `setup`: the state's wants after.
    #[must_use]
    pub fn final_moves(&mut self, setup: &MovesSetup) -> [[i8; CLUBS]; DIVISIONS] {
        ffi::rules_library::moves(setup, true)
    }

    /// `sr_b_teams` of `setup`: the state's wants after.
    #[must_use]
    pub fn second_team_moves(&mut self, setup: &MovesSetup) -> [[i8; CLUBS]; DIVISIONS] {
        ffi::rules_library::moves(setup, false)
    }

    /// `sr_cup_entrants` of `setup`: the slots it wrote (its return many).
    #[must_use]
    pub fn cup_entrants(&mut self, setup: &CupEntrantsSetup) -> Vec<u16> {
        ffi::rules_library::cup_entrants(setup)
    }

    /// `sr_pg_season` of `setup`: its return and the offsets after.
    #[must_use]
    pub fn parallel_groups(&mut self, setup: &GroupsSetup) -> (i32, Vec<i8>) {
        ffi::rules_library::parallel_groups(setup)
    }

    /// The grouped cup's season end: reset (`sr_cupd_end_reset`), then `steps`.
    #[must_use]
    pub fn cup_end(&mut self, steps: &[CupEndStep]) -> CupEnd {
        ffi::rules_library::cup_end(steps)
    }

    /// `sr_cupd_make` of `setup`.
    #[must_use]
    pub fn cup_rounds(&mut self, setup: &CupRoundsSetup) -> CupRounds {
        ffi::rules_library::cup_rounds(setup)
    }

    /// `sr_italy` of `setup`: its return and the moves it made (global number, delta, rule).
    #[must_use]
    pub fn fixed_tiers(&mut self, setup: &FixedTiersSetup) -> (i32, Vec<(u16, i8, u8)>) {
        ffi::rules_library::fixed_tiers(setup)
    }

    /// `sr_out_held(pts16, pts17)` and `sr_playoff_held(pts3, pts4)` of `points`.
    #[must_use]
    pub fn held(&mut self, points: [u16; 2]) -> (i32, i32) {
        ffi::rules_library::held(points)
    }

    /// `sr_spec_check(spec, len, divisions, err)` of `spec` (its length `len`).
    #[must_use]
    pub fn spec_check(&mut self, spec: &[u8], divisions: i32) -> CheckOutcome {
        ffi::rules_library::spec_check(spec, divisions)
    }

    /// `sr_league_check(lg, err)` of `league`.
    #[must_use]
    pub fn league_check(&mut self, league: &LeagueSetup) -> CheckOutcome {
        ffi::rules_library::league_check(league)
    }

    /// `sr_season_check` of `setup`.
    #[must_use]
    pub fn season_check(&mut self, setup: &SeasonCheckSetup) -> CheckOutcome {
        ffi::rules_library::season_check(setup)
    }
}
