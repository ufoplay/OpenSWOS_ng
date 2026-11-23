//! A league's knockouts at the season end (upstream src/rules/sr_gplay.c `sr_knockouts`, the
//! Mod's tools/gplay.c `gp_season`, whole), ported as the Mod wrote it: its generator, seeds,
//! tie model and order. Every kind of play-off (promotion, play-out, placement, title,
//! continental, promedio, stage title) and entrant (a place, a stage table's place, the stage
//! titles' finalists, the title deciders' seeds, the previous play-off's loser, a club by team
//! word), group rounds, the drawn pot; beside the moves, its results (a title's champion, a
//! continental play-off's two, a stage title's champion and runner-up) and the SEASON WINNERS
//! page. What the career club played itself comes from the season's host. A tie is decided by
//! the clubs' strengths: the place in their division and the tier, the home side 10% up.
//!
//! The state the season ends keep ([`Knockouts`]: the results, the stage titles, the groups'
//! figures) is the caller's, as upstream's library state is.
//!
//! Where a play-off not played reads a round it did not play (its placings, ties or best losers:
//! the C's locals, never written) and the value counts (a group's member, a placing's figures, a
//! row read, a final group's placing), the port reads none and says so
//! ([`KnockoutsOutcome::read_unplayed`]): the C's result there is its stack's.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

mod entrants;
mod groups;
mod play;
mod play_offs;
mod promedio;

use self::play::{NONE, Play};
use super::check::league_check;
use super::league::League;
use super::moves::Move;
use super::season::{KnockoutResult, KnockoutSeason, StageBank, season_check};
use super::table::{CLUBS, RulesError, Tables};

/// The bytes of a play-off's header, and of a round's.
const HEADER: usize = 6;

/// The bytes of a play-off's entrant.
const ENTRANT: usize = 4;

/// The play-off entrants a spec holds at most (`GN`), its ties and its groups' placings too.
const ENTRANTS: usize = 64;

/// The stage titles a division keeps (`sres`, `srl`).
const TITLES: usize = 3;

/// A tie's clubs; a knockout round's member bytes are as many per tie.
const PAIR: usize = 2;

/// A round's legs' flag of a group (its meetings in the low bits).
const GROUP: u8 = 0x80;

/// A play-off's flag of the drawn pot.
const DRAWN_POT: u8 = 8;

/// The bits under a flag's high bit (a round's host, a group's meetings, a draw's count).
const LOW_BITS: u8 = 0x7F;

/// A group table's row (`sr_gs`): its club, points, goal difference and goals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GroupFigures {
    /// The club (division times 32 plus its place).
    pub club: i32,
    /// The points.
    pub points: i32,
    /// The goal difference.
    pub goal_difference: i32,
    /// The goals.
    pub goals: i32,
}

/// The state a league's season ends keep (upstream `sr_state.gp`'s parts that outlive one): the
/// last season end's results, the career club's division's stage titles, another division's,
/// the groups' figures. A new one is the library's reset state (`sr_reset`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Knockouts {
    /// The results (`res`).
    results: [KnockoutResult; play::RESULTS],
    /// The results decided (`nres`).
    result_count: usize,
    /// The career club's division's stage titles (`sres`).
    stage_titles: [KnockoutResult; TITLES],
    /// Another division's (`srl`).
    other_titles: [KnockoutResult; TITLES],
    /// That division (`srl_div`).
    other_division: i32,
    /// A round's members' figures (`ms`).
    member_figures: [GroupFigures; CLUBS],
    /// The groups' placings' figures (`ps`).
    placing_figures: [GroupFigures; ENTRANTS],
}

impl Default for Knockouts {
    fn default() -> Self {
        Self {
            results: [KnockoutResult::default(); play::RESULTS],
            result_count: 0,
            stage_titles: [KnockoutResult::default(); TITLES],
            other_titles: [KnockoutResult::default(); TITLES],
            other_division: 0,
            member_figures: [GroupFigures::default(); CLUBS],
            placing_figures: [GroupFigures::default(); ENTRANTS],
        }
    }
}

/// What a season end's knockouts did: the moves they added, whether a play-off not played read
/// a round it did not play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnockoutsOutcome {
    /// The moves added.
    pub moves: usize,
    /// A round not played was read (the C reads its locals, never written, there).
    pub read_unplayed: bool,
}

impl Knockouts {
    /// The last season end's results: titles' champions, continental places.
    #[must_use]
    pub fn results(&self) -> &[KnockoutResult] {
        &self.results[..self.result_count]
    }

    /// The career club's division's stage titles (the top division's without it).
    #[must_use]
    pub fn stage_titles(&self) -> &[KnockoutResult; TITLES] {
        &self.stage_titles
    }

    /// The groups' figures the state keeps: a round's members', the placings'.
    #[must_use]
    pub fn figures(&self) -> (&[GroupFigures; CLUBS], &[GroupFigures; ENTRANTS]) {
        (&self.member_figures, &self.placing_figures)
    }

    /// The season end's knockouts of `league` over `tables` (upstream `sr_knockouts`): its
    /// spec's play-offs in order, each club's move appended to `moves` (at most 48 in all), the
    /// results, the stage titles and the SEASON WINNERS page kept; a 'keep' stage's frozen table
    /// read from the season's bank when its division is the league's.
    ///
    /// # Errors
    ///
    /// [`RulesError::Check`] when the league's rules or the season's state are not sound
    /// (upstream's refusal, nothing played); [`RulesError`] where the C reads past its memory (a
    /// stage title past the three, an odd pairing's last club, an absent club moved, counts by
    /// place or second teams' words fewer than read).
    pub fn season(
        &mut self,
        tables: &Tables,
        league: &League,
        season: &mut KnockoutSeason<'_>,
        moves: &mut Vec<Move>,
    ) -> Result<KnockoutsOutcome, RulesError> {
        league_check(league).map_err(RulesError::Check)?;
        season_check(season).map_err(RulesError::Check)?;
        let before = moves.len();
        let kept = season
            .bank
            .as_deref()
            .filter(|bank| bank.kept_count != 0 && bank.kept_division < league.divisions)
            .map(|bank| bank.kept[..usize::from(bank.kept_count)].to_vec())
            .unwrap_or_default();
        let mut play = Play {
            tables,
            league,
            season,
            state: self,
            moves,
            no_bank: StageBank::default(),
            kept,
            rng: 0,
            by_record: false,
            play_off: 0,
            stages_used: false,
            winners_on: false,
            read_unplayed: false,
        };
        play.state.other_division = NONE;
        play.play_offs()?;
        let read_unplayed = play.read_unplayed;
        Ok(KnockoutsOutcome {
            moves: moves.len() - before,
            read_unplayed,
        })
    }
}

/// The knockouts' unit tests' tables, league, season, host and play.
#[cfg(test)]
pub(super) mod fixture {
    use super::Knockouts;
    use super::play::Play;
    use crate::competition::rules_library::league::{League, SecondTeams};
    use crate::competition::rules_library::season::{
        KnockoutHost, KnockoutSeason, PlayedTie, StageBank,
    };
    use crate::competition::rules_library::table::{CLUBS, DIVISIONS, Row, Tables};

    /// A division's team file: the first, then one a division.
    const FILE: u16 = 10;

    /// The top's points; each place down `STEP` fewer.
    const TOP: u16 = 60;
    /// See [`TOP`].
    const STEP: u16 = 3;

    /// A row's wins and goals.
    const WON: u16 = 10;
    /// See [`WON`].
    const GOALS_FOR: u16 = 30;
    /// See [`WON`].
    const GOALS_AGAINST: u16 = 20;

    /// The team word of division `division`'s club `place` (its file in the high byte).
    pub(super) fn team(division: usize, place: usize) -> u16 {
        let file = FILE + u16::try_from(division).expect("16");
        file << u8::BITS | u16::try_from(place).expect("32")
    }

    /// The global number of division `division`'s club `place`.
    pub(super) fn glob(division: usize, place: usize) -> u16 {
        u16::try_from(division * CLUBS + place).expect("512")
    }

    /// The rows of `counts` clubs a division: team words and global numbers by division and
    /// place, points falling by 3 a place, the same record.
    pub(super) fn rows(counts: &[u8]) -> [[Row; CLUBS]; DIVISIONS] {
        let mut rows = [[Row::default(); CLUBS]; DIVISIONS];
        for (division, (&count, table)) in counts.iter().zip(&mut rows).enumerate() {
            for (place, row) in table.iter_mut().enumerate().take(usize::from(count)) {
                let points = TOP - STEP * u16::try_from(place).expect("32");
                *row = Row {
                    team: team(division, place),
                    glob: glob(division, place),
                    won: WON,
                    goals_for: GOALS_FOR,
                    goals_against: GOALS_AGAINST,
                    points,
                };
            }
        }
        rows
    }

    /// Tables of `rows`, `counts` clubs a division.
    pub(super) fn tables_of(counts: &[u8], rows: [[Row; CLUBS]; DIVISIONS]) -> Tables {
        let mut all = [0; DIVISIONS];
        all[..counts.len()].copy_from_slice(counts);
        Tables::new(u8::try_from(counts.len()).expect("16"), all, rows).expect("tables")
    }

    /// Tables of `counts` clubs a division ([`rows`]).
    pub(super) fn tables(counts: &[u8]) -> Tables {
        tables_of(counts, rows(counts))
    }

    /// A league of `divisions` and `spec`: no counts by place, no second teams, team words
    /// with their file in the high byte.
    pub(super) fn league(divisions: u8, spec: Option<Vec<u8>>) -> League {
        League {
            divisions,
            promoted: Vec::new(),
            relegated: Vec::new(),
            second_teams: SecondTeams {
                teams: Vec::new(),
                top: None,
                play_offs: false,
                leave_lowest: false,
                words: None,
            },
            fixed_tiers: None,
            spec,
            promedio_seeds: Vec::new(),
            team_file_high: true,
        }
    }

    /// A season of nothing.
    pub(super) fn season<'a>() -> KnockoutSeason<'a> {
        KnockoutSeason {
            manager: 0,
            played: PlayedTie::default(),
            bank: None,
            promedio: None,
            winners: None,
            host: None,
            preview: false,
            own_play_offs: [0; 2],
            second_teams_in_knockouts: false,
            base: 0,
            levels: None,
        }
    }

    /// A host answering from fixed answers.
    #[derive(Debug, Default)]
    pub(super) struct Host {
        /// A tie of a play-off played: its play-off, clubs and winner.
        pub(super) tie: Option<(i32, u16, u16, u16)>,
        /// A title played: its play-off, champion and runner-up.
        pub(super) title: Option<(i32, u16, u16)>,
        /// A stage title decided: its stage, team words, global numbers.
        pub(super) stage: Option<(i32, [u16; 2], [u16; 2])>,
    }

    impl KnockoutHost for Host {
        fn played_tie(&mut self, play_off: i32, a: u16, b: u16) -> Option<u16> {
            self.tie
                .filter(|&(index, x, y, _)| index == play_off && (x, y) == (a, b))
                .map(|(.., winner)| winner)
        }

        fn played_title(&mut self, play_off: i32) -> Option<(u16, u16)> {
            self.title
                .filter(|&(index, ..)| index == play_off)
                .map(|(_, champion, runner_up)| (champion, runner_up))
        }

        fn stage_title(&mut self, stage: i32) -> Option<([u16; 2], [u16; 2])> {
            self.stage
                .filter(|&(index, ..)| index == stage)
                .map(|(_, team, glob)| (team, glob))
        }
    }

    /// `f` over a play of `tables`, `league` and `season` from state `state`, no moves before.
    pub(super) fn with_play<'a, R>(
        tables: &Tables,
        league: &League,
        season: &mut KnockoutSeason<'a>,
        state: &mut Knockouts,
        f: impl FnOnce(&mut Play<'_, 'a>) -> R,
    ) -> R {
        let mut moves = Vec::new();
        let mut play = Play {
            tables,
            league,
            season,
            state,
            moves: &mut moves,
            no_bank: StageBank::default(),
            kept: Vec::new(),
            rng: 0,
            by_record: false,
            play_off: 0,
            stages_used: false,
            winners_on: false,
            read_unplayed: false,
        };
        f(&mut play)
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{glob, league, rows, season, tables};
    use super::*;
    use crate::competition::rules_library::moves::MoveRule;
    use crate::competition::rules_library::season::{PlayedTie, Winners};

    /// A promotion play-off of a stage table's 1st (stage table `table` of division 0) and
    /// division 1's 1st, the season's played tie its winner.
    fn promotion(table: u8) -> Vec<u8> {
        vec![
            1, 2, 1, 0, 1, 0, 3, 0, 1, table, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0,
        ]
    }

    #[test]
    fn a_new_state_is_zeroed() {
        let state = Knockouts::default();
        assert!(state.results().is_empty());
        assert_eq!(state.stage_titles(), &[KnockoutResult::default(); TITLES]);
        assert_eq!(state.figures().0[0], GroupFigures::default());
        assert_eq!(state.figures().1.len(), ENTRANTS);
    }

    #[test]
    fn an_unsound_league_or_season_is_refused_nothing_played() {
        let tables = tables(&[4, 4]);
        let mut state = Knockouts::default();
        let mut moves = vec![Move {
            glob: 1,
            delta: 1,
            rule: MoveRule::FixedTiers,
        }];
        let unsound = league(2, Some(vec![1]));
        let refused = state.season(&tables, &unsound, &mut season(), &mut moves);
        assert!(matches!(refused, Err(RulesError::Check(_))));
        let mut full = Winners {
            count: 41,
            ..Winners::default()
        };
        let mut damaged = season();
        damaged.winners = Some(&mut full);
        let refused = state.season(&tables, &league(2, None), &mut damaged, &mut moves);
        assert!(matches!(refused, Err(RulesError::Check(_))));
        assert_eq!(moves.len(), 1);
        assert_eq!(state, Knockouts::default());
    }

    #[test]
    fn the_kept_table_is_the_bank_s_when_its_division_is_the_league_s() {
        let tables = tables(&[4, 4]);
        let mut kept = StageBank {
            kept_count: 2,
            ..StageBank::default()
        };
        // The kept table's 1st: division 0's 3rd.
        kept.kept[0] = rows(&[4])[0][2];
        kept.kept[1] = rows(&[4])[0][0];
        for (division, moves_made) in [(0, 2), (2, 0)] {
            kept.kept_division = division;
            let mut played = season();
            played.bank = Some(&mut kept);
            played.played = PlayedTie {
                a: tables.row(0, 2).team,
                b: tables.row(1, 0).team,
                winner: tables.row(1, 0).team,
            };
            let mut moves = vec![Move {
                glob: 9,
                delta: 1,
                rule: MoveRule::FixedTiers,
            }];
            let outcome = Knockouts::default().season(
                &tables,
                &league(2, Some(promotion(0))),
                &mut played,
                &mut moves,
            );
            assert_eq!(
                outcome,
                Ok(KnockoutsOutcome {
                    moves: moves_made,
                    read_unplayed: false
                }),
                "{division}"
            );
            assert_eq!(moves.len(), 1 + moves_made);
            if moves_made > 0 {
                assert_eq!(moves[1].glob, glob(0, 2));
            }
        }
    }
}
