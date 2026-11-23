//! The rules library (docs/PLAN.md M5 #9a): the season-end and cup rules OpenSWOS took from the
//! library it shared with the SWOS 96/97 Mod (upstream src/rules/: `swos-competition-engine` at
//! its tag v1, `87b6527`, with OpenSWOS's changes since). Its input is a league's final tables
//! and its rules; its output, each club's move. No engine: the calendars, contests and saves are
//! the game's.
//!
//! - The season's moves ([`final_moves`], [`second_team_moves`]): each club's one-level move by
//!   its record's place and the play-offs, then the second teams' rules.
//! - A cup's clubs from the second season on ([`cup_entrants`]), by its real list's quotas.
//! - Parallel groups at the season end ([`parallel_groups`]): each move into a level of groups,
//!   by the clubs' areas.
//! - A cup of a level of grouped divisions, a later season's first two rounds
//!   ([`grouped_cup_rounds`]).
//! - A league's rules ([`League`]) and their check ([`league_check`], [`spec_check`]: the
//!   knockouts' spec).
//! - The fixed tiers' brackets ([`fixed_tier_brackets`], M5 #9a2): an upper tier's play-off
//!   and play-out, a lower tier of groups' play-off.
//!
//! The pack's knockouts and the career club's contests come with M5 #9a3 and #9a4.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. The library's code is under the MIT licence,
//! kept with its port in `LICENSE-MIT` beside this module (upstream src/rules/LICENSE-MIT); the
//! port, part of OpenSWOS_ng, is GPL-3.0-or-later like the rest of the game.

mod check;
mod cup_entrants;
mod fixed_tiers;
mod generator;
mod grouped_cup;
mod knockouts;
mod league;
mod moves;
mod parallel_groups;
mod season;
mod table;

pub use check::{CheckCode, CheckError, league_check, spec_check};
pub use cup_entrants::{CLUB_SLOTS, CUP_CLUBS, CupClubs, Quota, cup_entrants};
pub use fixed_tiers::{FixedTiersSeason, fixed_tier_brackets, play_off_held, play_out_held};
pub use grouped_cup::{
    FIRST_ROUND_TIES, GroupedCup, GroupedCupEnd, GroupedCupRounds, PRELIMINARY_TIES, SIDES,
    grouped_cup_rounds,
};
pub use knockouts::{GroupFigures, Knockouts, KnockoutsOutcome};
pub use league::{FixedTiers, League, SecondTeam, SecondTeams};
pub use moves::{MOVES, Move, MoveRule, Wants, final_moves, second_team_moves};
pub use parallel_groups::{
    GROUP_LEVELS, GROUPS, GroupClub, GroupLevel, GroupMoves, PARALLEL, parallel_groups,
};
pub use season::{
    KnockoutHost, KnockoutResult, KnockoutSeason, NO_CLUB, PROMEDIO_CLUBS, PlayedTie, Promedio,
    PromedioClub, PromedioSeed, ResultKind, STAGE_TABLES, StageBank, WINNERS, Winners,
    WinnersEntry, season_check,
};
pub use table::{CLUBS, DIVISIONS, Levels, Row, RulesError, Tables};
