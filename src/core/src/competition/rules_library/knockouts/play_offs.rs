//! A season end's play-offs in their spec's order (upstream src/rules/sr_gplay.c `gp_season`,
//! `wn_add`): each its own stream of draws seeded from its entrants, its rounds of groups and
//! ties, then its outcome (the moves, a result, a stage title) and its SEASON WINNERS entries.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::entrants::{DIVISION_SHIFT, PLAY_OUT, STAGE_BITS, STAGE_TITLES, emptied};
use super::play::{ABSENT, Club, DIVISIONS_I32, HOST_NEUTRAL, NONE, Play, tier};
use super::promedio::{ANNUAL, ANNUAL_FRESH};
use super::{DRAWN_POT, ENTRANT, ENTRANTS, GROUP, HEADER, LOW_BITS, PAIR};
use crate::competition::rules_library::moves::MOVES;
use crate::competition::rules_library::season::{
    KnockoutResult, NO_CLUB, ResultKind, WINNERS, WinnersEntry,
};
use crate::competition::rules_library::table::{CLUBS, RulesError};

/// The play-off kinds played at the season end: promotion, title, continental, promedio,
/// stage title (and the play-out, [`PLAY_OUT`]).
const PROMOTION: u8 = 1;
/// See [`PROMOTION`].
const TITLE: u8 = 5;
/// See [`PROMOTION`].
const CONTINENTAL: u8 = 6;
/// See [`PROMOTION`].
const PROMEDIO: u8 = 7;
/// See [`PROMOTION`].
const STAGE: u8 = 8;

/// A play-off's flags: the better record hosts (`by_rec`); the next season's supercup, not
/// played now (`PO_SUPER`); its placings taken past the second teams (`PO_SKIP2`).
const BY_RECORD: u8 = 2;
/// See [`BY_RECORD`].
const SUPERCUP: u8 = 4;
/// See [`BY_RECORD`].
const SKIP_SECONDS: u8 = 0x80;

/// How far a play-off's flags hold its final group's winners, and their bits.
const WINNERS_SHIFT: u8 = 4;
/// See [`WINNERS_SHIFT`].
const WINNERS_BITS: u8 = 7;

/// A round's host's flag: a club drawn against itself goes through (`HOST_SKIP_SAME`); a
/// round's pairing's: a draw.
const SKIP_SAME: u8 = 0x80;
/// See [`SKIP_SAME`].
const DRAW: u8 = 0x80;

/// The legs of a best of three.
const SERIES: u8 = 3;

/// A member code: the previous round's best placed loser (`BEST_LOSER`); below it an
/// entrant; from the next a group's placing, then a tie's winner, then a tie's loser.
const BEST_LOSER: u8 = 0x3F;
/// See [`BEST_LOSER`].
const PLACINGS: u8 = 0x40;
/// See [`BEST_LOSER`].
const WINNERS_OF: u8 = 0x80;
/// See [`BEST_LOSER`].
const LOSERS_OF: u8 = 0xC0;
/// The bits of a member code's index.
const MEMBER_INDEX: u8 = 0x3F;

/// The loser's division of the reserve (`RESERVE`): out of every division.
const RESERVE: u8 = 0xFE;

/// The seed's multiplier over the tables and over a play-off's entrants, and how far a club's
/// points are shifted in each.
const SEED_MULTIPLIER: u32 = 31;
/// See [`SEED_MULTIPLIER`].
const POINTS_SHIFT: u32 = 9;
/// See [`SEED_MULTIPLIER`].
const ENTRANT_POINTS_SHIFT: u32 = 16;
/// A play-off's own seed's multiplier, by its number from 1 (2^32 over the golden ratio).
const PLAY_OFF_SEED: u32 = 0x9E37_79B9;
/// An entrant not known in a play-off's seed.
const UNKNOWN_SEED: u32 = 0xFFFF;

/// The SEASON WINNERS page's entry types: the division the career club played in, a
/// division's 1st, a play-off's winner; a title play-off's last ties' winners (with the
/// play-off's number in the low bits).
const PLAYED_IN: u8 = b'M';
/// See [`PLAYED_IN`].
const LEADER: u8 = b'L';
/// See [`PLAYED_IN`].
const WINNER: u8 = b'K';
/// See [`PLAYED_IN`].
const LAST_TIES: u8 = 0x80;

/// The last ties of a title play-off the page keeps (the MLS conference finals and final).
const LAST_TIES_KEPT: usize = 3;

/// A play-off's header.
#[derive(Debug, Clone, Copy)]
struct Header {
    /// Its kind.
    kind: u8,
    /// Its entrants.
    entrants: usize,
    /// Its rounds.
    rounds: u8,
    /// The winners' division (a promedio's matches counted; a stage title's number).
    winner_division: u8,
    /// The losers' division.
    loser_division: u8,
    /// Its flags.
    flags: u8,
}

impl Header {
    /// The header a play-off's spec starts with.
    fn of(spec: &[u8]) -> Self {
        let [
            kind,
            entrants,
            rounds,
            winner_division,
            loser_division,
            flags,
        ] = <[u8; HEADER]>::try_from(&spec[..HEADER]).expect("a checked spec's header");
        Self {
            kind,
            entrants: usize::from(entrants),
            rounds,
            winner_division,
            loser_division,
            flags,
        }
    }

    /// A promotion's final group's clubs that go up: its flags' count, else 1.
    fn group_winners(self) -> usize {
        match self.flags >> WINNERS_SHIFT & WINNERS_BITS {
            0 => 1,
            winners => usize::from(winners),
        }
    }
}

/// A play-off's rounds so far (upstream `gp_season`'s locals). A tie or a group not played is
/// none: the C's locals there are never written.
struct Rounds {
    /// The entrants' clubs (`club`).
    clubs: [Club; ENTRANTS],
    /// The play-off is played (`ok`).
    ok: bool,
    /// The ties' winners (`W`).
    wins: [Option<Club>; ENTRANTS],
    /// The ties' losers (`L`).
    losses: [Option<Club>; ENTRANTS],
    /// The ties so far, played or not (`nt`).
    ties: usize,
    /// The last tie played (`last`), none after a group.
    last: Option<usize>,
    /// The last knockout round's first tie (`lastr`).
    last_round: usize,
    /// Its ties (`nlast`).
    last_ties: usize,
    /// The groups' placings (`P`).
    placings: [Option<Club>; ENTRANTS],
    /// The placings so far, played or not (`np`).
    placed: usize,
    /// The last group played's first placing (`glast`), none after a tie.
    last_group: Option<usize>,
    /// A knockout round came after a group (`kog`).
    after_group: bool,
}

impl Rounds {
    /// No round yet, of entrants `clubs`.
    fn new(clubs: [Club; ENTRANTS], ok: bool) -> Self {
        Self {
            clubs,
            ok,
            wins: [None; ENTRANTS],
            losses: [None; ENTRANTS],
            ties: 0,
            last: None,
            last_round: 0,
            last_ties: 0,
            placings: [None; ENTRANTS],
            placed: 0,
            last_group: None,
            after_group: false,
        }
    }

    /// Tie `tie`'s winner and loser, of a play-off played: each of its ties is (at most 64,
    /// the spec's check).
    fn tie(&self, tie: usize) -> (Club, Club) {
        let played = "a played play-off's tie";
        (
            self.wins[tie].expect(played),
            self.losses[tie].expect(played),
        )
    }
}

impl Play<'_, '_> {
    /// An entry of the SEASON WINNERS page (upstream `wn_add`): only while it is written, of a
    /// team, at most 40.
    fn winner_entry(&mut self, kind: u8, index: i32, team: u16) {
        if !self.winners_on || team == NO_CLUB {
            return;
        }
        let Some(winners) = self.season.winners.as_deref_mut() else {
            return;
        };
        let count = usize::from(winners.count);
        if count >= WINNERS {
            return;
        }
        let [index, ..] = index.to_le_bytes();
        winners.entries[count] = WinnersEntry { kind, index, team };
        winners.count += 1;
    }

    /// A round not played, read: none, noted.
    fn unplayed(&mut self) -> Club {
        self.read_unplayed = true;
        NONE
    }

    /// A round not played, read where its value `counts`: none, noted then.
    fn unplayed_where(&mut self, counts: bool) -> Club {
        if counts { self.unplayed() } else { NONE }
    }

    /// Placing `at` of the play-off's groups: none for a group not played (noted).
    ///
    /// # Errors
    ///
    /// [`RulesError::PlacingPastGroups`] past the 64 (upstream reads past its placings).
    fn placing(&mut self, rounds: &Rounds, at: usize) -> Result<Club, RulesError> {
        match rounds.placings.get(at) {
            Some(Some(club)) => Ok(*club),
            Some(None) => Ok(self.unplayed()),
            None => Err(RulesError::PlacingPastGroups(at)),
        }
    }

    /// The season end's play-offs (upstream `gp_season`): the draws seeded from the tables,
    /// the results and the stage titles emptied, the SEASON WINNERS page begun, each play-off
    /// in order; the bank's stage tables spent when one was read.
    pub(super) fn play_offs(&mut self) -> Result<(), RulesError> {
        let divisions = self.divisions().min(DIVISIONS_I32);
        let mut seed: u32 = 0;
        for division in 0..divisions {
            for at in 0..self.count(division) {
                let row = self.row_at(division, at);
                seed = seed
                    .wrapping_mul(SEED_MULTIPLIER)
                    .wrapping_add(u32::from(row.glob))
                    .wrapping_add(u32::from(row.points) << POINTS_SHIFT);
            }
        }
        self.rng = seed;
        self.state.result_count = 0;
        for title in &mut self.state.stage_titles {
            emptied(title);
        }
        self.state.other_division = NONE;
        self.stages_used = false;
        self.open_page(divisions);
        let league = self.league;
        if let Some(spec) = &league.spec {
            let (mut previous, mut at, mut index) = (NONE, 0, 0);
            while spec[at] != 0 {
                at += self.play_off(&spec[at..], index, &mut previous)?;
                index += 1;
            }
        }
        self.winners_on = false;
        if self.stages_used
            && !self.season.preview
            && let Some(bank) = self.season.bank.as_deref_mut()
        {
            bank.fresh = 0;
        }
        Ok(())
    }

    /// The SEASON WINNERS page's start: written for the career club's league (a preview's
    /// entries appended); its division and each division's 1st (the top's by the annual table
    /// when it is this season's), the page emptied first.
    fn open_page(&mut self, divisions: i32) {
        let manager = self.season.manager;
        let mut played_in = None;
        if manager != 0 {
            for division in 0..divisions {
                for at in 0..self.count(division) {
                    if self.row_at(division, at).team == manager {
                        played_in = Some(division);
                    }
                }
            }
        }
        self.winners_on = played_in.is_some() || self.season.preview;
        if self.season.preview || self.season.winners.is_none() {
            return;
        }
        let Some(played_in) = played_in else {
            return;
        };
        let bank = self.bank();
        let annual = (bank.fresh & ANNUAL_FRESH != 0
            && i32::from(bank.counts[ANNUAL]) == self.count(0))
        .then_some(bank.tables[ANNUAL][0].team);
        if let Some(winners) = self.season.winners.as_deref_mut() {
            winners.count = 0;
            winners.fresh = 1;
        }
        self.winner_entry(PLAYED_IN, played_in, manager);
        for division in 0..divisions {
            if self.count(division) != 0 {
                let leader = match annual {
                    Some(team) if division == 0 => team,
                    Some(_) | None => self.row_at(division, 0).team,
                };
                self.winner_entry(LEADER, division, leader);
            }
        }
    }

    /// Play-off `index` (`spec`: from its header), after a play-off whose last tie's loser was
    /// `previous` (then this one's): the bytes it takes. A promedio's relegation; else, played
    /// unless a stage title decided mid-season, the career club's own in a preview, a
    /// supercup or an entrant not known: its rounds, its outcome, its page's entries.
    fn play_off(
        &mut self,
        spec: &[u8],
        index: i32,
        previous: &mut Club,
    ) -> Result<usize, RulesError> {
        let header = Header::of(spec);
        let mut at = HEADER + ENTRANT * header.entrants;
        self.by_record = header.flags & BY_RECORD != 0;
        if header.kind == PROMEDIO {
            if !self.season.preview {
                self.promedio(header.winner_division, header.loser_division);
            }
            return Ok(at);
        }
        let mut ok = matches!(
            header.kind,
            PROMOTION | PLAY_OUT | TITLE | CONTINENTAL | STAGE
        );
        let mut decided = if header.kind == STAGE {
            self.stage_decided(i32::from(header.winner_division))
        } else {
            None
        };
        if decided.is_some() {
            ok = false;
        }
        let own = self.season.own_play_offs;
        if self.season.preview && own.iter().any(|&own| i32::from(own) == index + 1) {
            ok = false;
        }
        if header.flags & SUPERCUP != 0 {
            ok = false;
            decided = None;
        }
        self.play_off = index;
        let mut clubs = [NONE; ENTRANTS];
        if !self.resolve_entrants(spec, *previous, &mut clubs, true)? {
            ok = false;
        }
        self.rng = self.entrants_seed(index, header.kind, &clubs[..header.entrants]);
        if header.flags & DRAWN_POT != 0 {
            let drawn: Vec<Club> = self
                .pot_permutation(spec)
                .into_iter()
                .map(|entrant| clubs[entrant])
                .collect();
            clubs[..drawn.len()].copy_from_slice(&drawn);
        }
        let mut rounds = Rounds::new(clubs, ok);
        for _ in 0..header.rounds {
            at += self.round(&mut rounds, header, &spec[at..])?;
        }
        let played_title = self.outcome(&rounds, header, index, decided)?;
        self.page(&rounds, header, index, played_title)?;
        *previous = match rounds.last {
            Some(tie) if rounds.ok => rounds.tie(tie).1,
            Some(_) | None => NONE,
        };
        Ok(at)
    }

    /// A play-off's own seed: its number and kind, then its entrants' teams and points.
    fn entrants_seed(&self, index: i32, kind: u8, clubs: &[Club]) -> u32 {
        let first = PLAY_OFF_SEED
            .wrapping_mul(index.cast_unsigned().wrapping_add(1))
            .wrapping_add(u32::from(kind));
        clubs.iter().fold(first, |seed, &club| {
            let entrant = if club >= 0 {
                let row = self.row(club);
                u32::from(row.team).wrapping_add(u32::from(row.points) << ENTRANT_POINTS_SHIFT)
            } else {
                UNKNOWN_SEED
            };
            seed.wrapping_mul(SEED_MULTIPLIER).wrapping_add(entrant)
        })
    }

    /// A round of play-off `play_off` (`spec`: from its header): its members, then a group or
    /// its ties; a member not known stops the play-off (an absent club leaves a group). The
    /// bytes it takes.
    fn round(
        &mut self,
        rounds: &mut Rounds,
        play_off: Header,
        spec: &[u8],
    ) -> Result<usize, RulesError> {
        let ties = usize::from(spec[0]);
        let group = spec[1] & GROUP != 0;
        let codes = &spec[HEADER..HEADER + if group { ties } else { PAIR * ties }];
        let mut members = Vec::with_capacity(codes.len());
        for (at, &code) in codes.iter().enumerate() {
            let club = self.member(rounds, play_off, group, code, at)?;
            if club < 0 && !(club == ABSENT && group) {
                rounds.ok = false;
            }
            members.push(club);
        }
        if group {
            self.group_round(rounds, &members, spec);
        } else {
            self.knockout_round(rounds, members, spec)?;
        }
        // Faithful to the original: a group's member bytes are twice its members too.
        Ok(HEADER + PAIR * ties)
    }

    /// Member `at` of a round (a group's or not) by its code: the previous round's best placed
    /// loser, an entrant, a group's placing (its figures the member's), a tie's winner or loser;
    /// none when not known.
    ///
    /// A round not played is read only where its value counts: in a group (an absent member
    /// is left out), a placing's figures, a best loser by record (upstream reads the rows).
    /// Elsewhere the play-off is not played and upstream's value goes unread.
    fn member(
        &mut self,
        rounds: &Rounds,
        play_off: Header,
        group: bool,
        code: u8,
        at: usize,
    ) -> Result<Club, RulesError> {
        let named = usize::from(code & MEMBER_INDEX);
        if code == BEST_LOSER {
            let mut best = NONE;
            for tie in rounds.last_round..(rounds.last_round + rounds.last_ties).min(rounds.ties) {
                let loser = match rounds.losses[tie] {
                    Some(loser) => loser,
                    None => self.unplayed_where(group || self.by_record),
                };
                if loser >= 0 && (best < 0 || self.better(loser, best)) {
                    best = loser;
                }
            }
            return Ok(best);
        }
        if code < PLACINGS {
            let entrant = usize::from(code);
            return Ok(if entrant < play_off.entrants {
                rounds.clubs[entrant]
            } else {
                NONE
            });
        }
        if code < WINNERS_OF {
            let found = if play_off.flags & SKIP_SECONDS != 0 {
                self.eligible_placing(rounds, named)?
            } else {
                (named < rounds.placed).then_some(named)
            };
            let Some(found) = found else {
                return Ok(NONE);
            };
            let club = match rounds.placings[found] {
                Some(club) => club,
                None => self.unplayed_where(group || at < CLUBS),
            };
            if club >= 0 && at < CLUBS {
                self.state.member_figures[at] = self.state.placing_figures[found];
            }
            return Ok(club);
        }
        if named >= rounds.ties {
            return Ok(NONE);
        }
        let tie = if code < LOSERS_OF {
            rounds.wins[named]
        } else {
            rounds.losses[named]
        };
        Ok(match tie {
            Some(club) => club,
            None => self.unplayed_where(group),
        })
    }

    /// The `nth` placing (from 0) that is no second team kept from going up (upstream's
    /// 'skip_second'), if any.
    fn eligible_placing(
        &mut self,
        rounds: &Rounds,
        nth: usize,
    ) -> Result<Option<usize>, RulesError> {
        let mut left = nth;
        for at in 0..rounds.placed {
            let club = self.placing(rounds, at)?;
            if club < 0 || self.second_ineligible(club)? {
                continue;
            }
            if left == 0 {
                return Ok(Some(at));
            }
            left -= 1;
        }
        Ok(None)
    }

    /// A group round (`spec`: from its header) of `members`: the absent left out (their
    /// figures too); played while the play-off is, its placings after the play-off's so far.
    fn group_round(&mut self, rounds: &mut Rounds, members: &[Club], spec: &[u8]) {
        let ties = usize::from(spec[0]);
        let (meetings, host) = (spec[1] & LOW_BITS, spec[2] & LOW_BITS);
        let bonuses = &spec[HEADER + ties..HEADER + PAIR * ties];
        let mut present = Vec::with_capacity(CLUBS);
        let mut bonus = Vec::with_capacity(CLUBS);
        for (at, (&club, &points)) in members.iter().zip(bonuses).enumerate().take(CLUBS) {
            if club != ABSENT {
                self.state.member_figures[present.len()] = self.state.member_figures[at];
                present.push(club);
                bonus.push(points);
            }
        }
        if rounds.ok && rounds.placed + present.len() <= ENTRANTS {
            let mut order = [NONE; CLUBS];
            let first = rounds.placed;
            self.group(
                &present,
                meetings,
                host == HOST_NEUTRAL,
                &bonus,
                &mut order,
                first,
            );
            for (placing, &club) in rounds.placings[first..]
                .iter_mut()
                .zip(&order[..present.len()])
            {
                *placing = Some(club);
            }
            rounds.last_group = Some(first);
        }
        rounds.placed += present.len();
        rounds.last = None;
    }

    /// A knockout round (`spec`: from its header) of `members`, played while the play-off is:
    /// paired (drawn, or best v worst), then each tie: a club drawn against itself through,
    /// clubs too far apart on points the better placed through, a best of three, a tie; the
    /// career club's as played.
    fn knockout_round(
        &mut self,
        rounds: &mut Rounds,
        mut members: Vec<Club>,
        spec: &[u8],
    ) -> Result<(), RulesError> {
        let ties = usize::from(spec[0]);
        let (legs, host, skip) = (spec[1], spec[2] & LOW_BITS, spec[2] & SKIP_SAME != 0);
        let (level, pairing, gap) = (spec[3], spec[4], i32::from(spec[5]));
        if rounds.ok && pairing & DRAW != 0 {
            let drawn = usize::from(pairing & LOW_BITS).min(PAIR * ties);
            for at in (1..drawn).rev() {
                let other = self.below(u32::try_from(at + 1).expect("128 members"));
                members.swap(at, usize::try_from(other).expect("a member"));
            }
        } else if rounds.ok && pairing != 0 {
            let paired = usize::from(pairing).min(PAIR * ties);
            self.best_against_worst(&mut members[..paired])?;
        }
        rounds.last_round = rounds.ties;
        rounds.last_ties = ties;
        if rounds.last_group.is_some() {
            rounds.after_group = true;
        }
        for &[a, b] in members.as_chunks::<PAIR>().0.iter().take(ties) {
            if rounds.ok && rounds.ties < ENTRANTS {
                let difference = i32::from(self.row(a).points) - i32::from(self.row(b).points);
                let simulated = if a == b && skip {
                    a
                } else if gap != 0 && difference.abs() >= gap {
                    if self.better(a, b) { a } else { b }
                } else if legs == SERIES {
                    self.series(a, b, level)
                } else {
                    self.tie(a, b, legs, host, level)
                };
                let winner = self.played(a, b, simulated);
                rounds.wins[rounds.ties] = Some(winner);
                rounds.losses[rounds.ties] = Some(if winner == a { b } else { a });
                rounds.last = Some(rounds.ties);
                rounds.last_group = None;
            }
            rounds.ties += 1;
        }
        Ok(())
    }

    /// Best v worst: `members` sorted best placed first, then paired 1st v last, 2nd v second
    /// last...
    ///
    /// # Errors
    ///
    /// [`RulesError::OddPairing`] for an odd count (upstream pairs its last with a club it
    /// never wrote).
    fn best_against_worst(&self, members: &mut [Club]) -> Result<(), RulesError> {
        if !members.len().is_multiple_of(PAIR) {
            return Err(RulesError::OddPairing(members.len()));
        }
        for at in 1..members.len() {
            let club = members[at];
            let mut to = at;
            while to > 0 && self.better(club, members[to - 1]) {
                members[to] = members[to - 1];
                to -= 1;
            }
            members[to] = club;
        }
        let sorted = members.to_vec();
        for (pair, [first, second]) in members.as_chunks_mut::<PAIR>().0.iter_mut().enumerate() {
            *first = sorted[sorted.len() - 1 - pair];
            *second = sorted[pair];
        }
        Ok(())
    }

    /// A play-off's outcome: a stage title's champion and runner-up; a title the host played;
    /// played, a final group's 1st (and 2nd) or the last tie's winner (and loser) as a title
    /// or continental result, a promotion's moves (a final group's top, else the last tie's
    /// winner, to the winners' division; its other entrants from there to the losers'), a
    /// play-out's (the last round's losers down). The title's champion the host played.
    fn outcome(
        &mut self,
        rounds: &Rounds,
        play_off: Header,
        index: i32,
        decided: Option<([u16; 2], [u16; 2])>,
    ) -> Result<Option<u16>, RulesError> {
        let Header {
            kind,
            winner_division,
            loser_division,
            ..
        } = play_off;
        if kind == STAGE {
            self.stage_title(rounds, winner_division, decided);
            return Ok(None);
        }
        if kind == TITLE
            && let Some(champion) = self.played_title(index)
        {
            return Ok(Some(champion));
        }
        if !rounds.ok {
            return Ok(None);
        }
        let winners = i32::from(winner_division);
        let losers = if loser_division == RESERVE {
            self.divisions()
        } else {
            i32::from(loser_division)
        };
        let to = |club: Club, up: bool| {
            if up {
                winners
            } else if tier(club) == winners {
                losers
            } else {
                tier(club)
            }
        };
        let entrants = &rounds.clubs[..play_off.entrants];
        match (kind, rounds.last_group, rounds.last) {
            (TITLE | CONTINENTAL, Some(group), _) if !rounds.after_group => {
                let champion = self.placing(rounds, group)?;
                let runner_up = if rounds.placed - group > 1 {
                    self.placing(rounds, group + 1)?
                } else {
                    NONE
                };
                if champion >= 0 {
                    self.result(result_kind(kind), champion, runner_up);
                }
            }
            (TITLE | CONTINENTAL, _, Some(tie)) => {
                let (winner, loser) = rounds.tie(tie);
                let loser = if loser == winner { NONE } else { loser };
                self.result(result_kind(kind), winner, loser);
            }
            (PROMOTION, Some(group), _) => {
                for &club in entrants {
                    let mut up = false;
                    for winner in 0..play_off.group_winners() {
                        up |= self.placing(rounds, group + winner)? == club;
                    }
                    self.move_entrant(club, to(club, up) - tier(club))?;
                }
            }
            (PLAY_OUT, _, Some(_)) => {
                let last = rounds.last_round..(rounds.last_round + rounds.last_ties).min(ENTRANTS);
                for tie in last {
                    let (_, loser) = rounds.tie(tie);
                    self.move_club(loser, losers - tier(loser));
                }
            }
            (PROMOTION, None, Some(tie)) => {
                let (winner, _) = rounds.tie(tie);
                for &club in entrants {
                    self.move_entrant(club, to(club, club == winner) - tier(club))?;
                }
            }
            _ => {}
        }
        Ok(None)
    }

    /// An entrant's move: an absent club's only when it stays or the moves are full.
    ///
    /// # Errors
    ///
    /// [`RulesError::AbsentMoved`] when an absent club moves (upstream moves the row before
    /// its tables: division 0 by its numbering, place -2).
    fn move_entrant(&mut self, club: Club, delta: i32) -> Result<(), RulesError> {
        if club != ABSENT {
            self.move_club(club, delta);
            return Ok(());
        }
        if delta == 0 || self.moves.len() >= MOVES {
            return Ok(());
        }
        Err(RulesError::AbsentMoved)
    }

    /// Stage title `number`'s champion and runner-up (its stage in the low bits, none past the
    /// three; its division above): decided mid-season, else the play-off's last tie's when
    /// played.
    fn stage_title(&mut self, rounds: &Rounds, number: u8, decided: Option<([u16; 2], [u16; 2])>) {
        let number = i32::from(number);
        let stage = number & STAGE_BITS;
        if stage >= STAGE_TITLES {
            return;
        }
        let division = number >> DIVISION_SHIFT;
        let won = match (decided, rounds.last) {
            (Some((team, glob)), _) => Some(KnockoutResult {
                kind: Some(ResultKind::Stage),
                division: u8::try_from(division).expect("a byte's division"),
                team,
                glob,
            }),
            (None, Some(tie)) if rounds.ok => {
                let (winner, loser) = rounds.tie(tie);
                let won = self.row(winner);
                let (team, glob) = if loser == winner {
                    (NO_CLUB, NO_CLUB)
                } else {
                    let lost = self.row(loser);
                    (lost.team, lost.glob)
                };
                Some(KnockoutResult {
                    kind: Some(ResultKind::Stage),
                    division: u8::try_from(tier(winner)).expect("16 divisions"),
                    team: [won.team, team],
                    glob: [won.glob, glob],
                })
            }
            (None, Some(_) | None) => None,
        };
        let titles = self.titles(division);
        if let Some(won) = won {
            titles[usize::try_from(stage).expect("3 stages")] = won;
        }
    }

    /// Title play-off `index` as the host played it (a group contest's final): its champion's
    /// team word, its result recorded, when the host says and its champion is in the tables.
    fn played_title(&mut self, index: i32) -> Option<u16> {
        let (champion, runner_up) = self.season.host.as_deref_mut()?.played_title(index)?;
        let club = self.team_club(champion);
        if club < 0 {
            return None;
        }
        let loser = self.team_club(runner_up);
        self.result(ResultKind::Title, club, loser);
        Some(champion)
    }

    /// A play-off's SEASON WINNERS entries: a stage title's champion; the title the host
    /// played; a play-off played (not a play-out), its final group's 1st or its last tie's
    /// winner; a title play-off's last three ties' winners.
    fn page(
        &mut self,
        rounds: &Rounds,
        play_off: Header,
        index: i32,
        played_title: Option<u16>,
    ) -> Result<(), RulesError> {
        let kind = play_off.kind;
        if kind == STAGE {
            let number = i32::from(play_off.winner_division);
            let stage = number & STAGE_BITS;
            let team = if stage < STAGE_TITLES {
                let titles = self.titles(number >> DIVISION_SHIFT);
                let title = titles[usize::try_from(stage).expect("3 stages")];
                if title.kind.is_some() {
                    title.team[0]
                } else {
                    NO_CLUB
                }
            } else {
                NO_CLUB
            };
            self.winner_entry(WINNER, index, team);
        } else if let Some(champion) = played_title {
            let team = if champion == 0 { NO_CLUB } else { champion };
            self.winner_entry(WINNER, index, team);
        } else if rounds.ok && kind != PLAY_OUT {
            // A group sets no last tie and a tie no last group: upstream's choice between them
            // (`kog`) never sees both.
            let club = match (rounds.last_group, rounds.last) {
                (Some(group), _) => self.placing(rounds, group)?,
                (None, Some(tie)) => rounds.tie(tie).0,
                (None, None) => NONE,
            };
            if club >= 0 {
                let team = self.row(club).team;
                self.winner_entry(WINNER, index, team);
            }
        }
        if rounds.ok && kind == TITLE && rounds.last_group.is_none() {
            let [number, ..] = index.to_le_bytes();
            let first = rounds.ties.saturating_sub(LAST_TIES_KEPT);
            for tie in first..rounds.ties.min(ENTRANTS) {
                let team = self.row(rounds.tie(tie).0).team;
                let tie = i32::try_from(tie).expect("64 ties");
                self.winner_entry(LAST_TIES | number & LOW_BITS, tie, team);
            }
        }
        Ok(())
    }
}

/// The result's kind of a title or continental play-off.
fn result_kind(kind: u8) -> ResultKind {
    if kind == TITLE {
        ResultKind::Title
    } else {
        ResultKind::Continental
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{Host, glob, league, season, tables, team, with_play};
    use super::super::play::{LEVEL_BETTER, club_at};
    use super::super::{GroupFigures, Knockouts, KnockoutsOutcome};
    use super::*;
    use crate::competition::rules_library::moves::{Move, MoveRule};
    use crate::competition::rules_library::season::{KnockoutSeason, PlayedTie, Promedio, Winners};
    use crate::competition::rules_library::table::Tables;

    /// The tables: three divisions of six clubs.
    const COUNTS: [u8; 3] = [6, 6, 6];

    /// Entrant by place: division `division`'s `place` (from 1).
    fn place(division: u8, place: u8) -> [u8; ENTRANT] {
        [0, division, place, 0]
    }

    /// A play-off of `kind` between `winners` and `losers` divisions with `flags`: its entrants
    /// and rounds.
    fn play_off(
        kind: u8,
        divisions: [u8; 2],
        flags: u8,
        entrants: &[[u8; ENTRANT]],
        rounds: &[Vec<u8>],
    ) -> Vec<u8> {
        let count = |len: usize| u8::try_from(len).expect("a few");
        let mut bytes = vec![
            kind,
            count(entrants.len()),
            count(rounds.len()),
            divisions[0],
            divisions[1],
            flags,
        ];
        bytes.extend(entrants.iter().flatten());
        bytes.extend(rounds.iter().flatten());
        bytes
    }

    /// A knockout round of single matches (`legs`), its members, the better placed through
    /// when `gap` points apart (0 never).
    fn knockout(members: &[u8], legs: u8, pairing: u8, gap: u8) -> Vec<u8> {
        let ties = u8::try_from(members.len() / PAIR).expect("a few");
        let mut bytes = vec![ties, legs, 0, LEVEL_BETTER, pairing, gap];
        bytes.extend(members);
        bytes
    }

    /// A group round of `members`, `meetings` each, no bonus.
    fn group(members: &[u8], meetings: u8) -> Vec<u8> {
        let mut bytes = vec![
            u8::try_from(members.len()).expect("32"),
            GROUP | meetings,
            0,
            0,
            0,
            0,
        ];
        bytes.extend(members);
        bytes.extend(vec![0; members.len()]);
        bytes
    }

    /// A spec of `play_offs`.
    fn spec(play_offs: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes: Vec<u8> = play_offs.iter().flatten().copied().collect();
        bytes.push(0);
        bytes
    }

    /// A move of `glob` by `delta`.
    fn moved(glob: u16, delta: i8) -> Move {
        Move {
            glob,
            delta,
            rule: MoveRule::Knockouts,
        }
    }

    /// The season end of `spec` over [`COUNTS`] in `season`: its outcome and moves.
    fn end(
        state: &mut Knockouts,
        spec: Vec<u8>,
        season: &mut KnockoutSeason<'_>,
    ) -> (Result<KnockoutsOutcome, RulesError>, Vec<Move>) {
        let tables: Tables = tables(&COUNTS);
        let league = league(3, Some(spec));
        let mut moves = Vec::new();
        let outcome = state.season(&tables, &league, season, &mut moves);
        (outcome, moves)
    }

    #[test]
    fn a_promotion_s_winner_goes_up_its_other_entrant_down() {
        let promotion = spec(&[play_off(
            PROMOTION,
            [0, 1],
            0,
            &[place(0, 6), place(1, 1)],
            &[knockout(&[0, 1], 1, 0, 0)],
        )]);
        for (winner, expected) in [
            (
                team(1, 0),
                vec![moved(glob(0, 5), 1), moved(glob(1, 0), -1)],
            ),
            (team(0, 5), vec![]),
        ] {
            let mut played = season();
            played.played = PlayedTie {
                a: team(0, 5),
                b: team(1, 0),
                winner,
            };
            let (outcome, moves) = end(&mut Knockouts::default(), promotion.clone(), &mut played);
            assert_eq!(
                outcome,
                Ok(KnockoutsOutcome {
                    moves: expected.len(),
                    read_unplayed: false
                })
            );
            assert_eq!(moves, expected);
        }
    }

    #[test]
    fn a_play_out_s_last_round_s_losers_go_down() {
        // The better placed through, 3 points a place apart: (0, 3) and (0, 2) stay.
        let play_out = spec(&[play_off(
            PLAY_OUT,
            [0, 1],
            0,
            &[place(0, 4), place(0, 5), place(0, 3), place(0, 6)],
            &[knockout(&[0, 1, 2, 3], 1, 0, 1)],
        )]);
        let (outcome, moves) = end(&mut Knockouts::default(), play_out, &mut season());
        assert_eq!(outcome.map(|done| done.moves), Ok(2));
        assert_eq!(moves, [moved(glob(0, 4), 1), moved(glob(0, 5), 1)]);
    }

    #[test]
    fn a_final_group_promotes_its_top_and_its_reserve_leaves_every_division() {
        let entrants = [place(0, 6), place(1, 1), place(1, 2)];
        let clubs = [club_at(0, 5), club_at(1, 0), club_at(1, 1)];
        for (winners, flags) in [(2, 2 << WINNERS_SHIFT), (1, 0)] {
            let promotion = spec(&[play_off(
                PROMOTION,
                [0, RESERVE],
                flags,
                &entrants,
                &[group(&[0, 1, 2], 2)],
            )]);
            let mut state = Knockouts::default();
            let (outcome, moves) = end(&mut state, promotion, &mut season());
            let (_, placings) = state.figures();
            let top: Vec<Club> = placings[..winners].iter().map(|row| row.club).collect();
            // The top to division 0; division 0's other club to the reserve (division 3).
            let expected: Vec<Move> = clubs
                .iter()
                .filter_map(|&club| {
                    let to = if top.contains(&club) {
                        0
                    } else if tier(club) == 0 {
                        3
                    } else {
                        tier(club)
                    };
                    let delta = i8::try_from(to - tier(club)).expect("a few");
                    (delta != 0).then(|| moved(u16::try_from(club).expect("a club"), delta))
                })
                .collect();
            assert_eq!(moves, expected);
            assert_eq!(outcome.map(|done| done.moves), Ok(expected.len()));
        }
    }

    #[test]
    fn a_title_s_result_is_its_final_group_s_1st_and_2nd_or_its_final_s() {
        let mut state = Knockouts::default();
        let mut page = Winners::default();
        let mut played = season();
        played.manager = team(0, 0);
        played.winners = Some(&mut page);
        let title = spec(&[play_off(
            TITLE,
            [0, 0],
            0,
            &[place(0, 1), place(0, 2), place(0, 3)],
            &[group(&[0, 1, 2], 1)],
        )]);
        let (outcome, moves) = end(&mut state, title, &mut played);
        assert_eq!(outcome.map(|done| done.moves), Ok(0));
        assert!(moves.is_empty());
        let (_, placings) = state.figures();
        let (first, second) = (placings[0].club, placings[1].club);
        let result = state.results()[0];
        assert_eq!(result.kind, Some(ResultKind::Title));
        let word = |club: Club| {
            team(
                usize::try_from(tier(club)).expect("0"),
                usize::try_from(club).expect("a club"),
            )
        };
        assert_eq!(result.team, [word(first), word(second)]);
        // The page: the career club's division, the three 1sts, the play-off's winner.
        let entries: Vec<(u8, u8, u16)> = page.entries[..usize::from(page.count)]
            .iter()
            .map(|entry| (entry.kind, entry.index, entry.team))
            .collect();
        assert_eq!(
            entries,
            [
                (PLAYED_IN, 0, team(0, 0)),
                (LEADER, 0, team(0, 0)),
                (LEADER, 1, team(1, 0)),
                (LEADER, 2, team(2, 0)),
                (WINNER, 0, word(first))
            ]
        );
        assert_eq!(page.fresh, 1);
    }

    #[test]
    fn a_continental_final_s_result_and_a_title_s_last_three_ties_on_the_page() {
        let mut state = Knockouts::default();
        let mut page = Winners::default();
        let mut played = season();
        played.manager = team(1, 0);
        played.winners = Some(&mut page);
        played.played = PlayedTie {
            a: team(0, 1),
            b: team(0, 0),
            winner: team(0, 1),
        };
        let continental = play_off(
            CONTINENTAL,
            [0, 0],
            0,
            &[place(0, 1), place(0, 2)],
            &[knockout(&[0, 1], 2, 0, 0)],
        );
        // Semi-finals and a final, the better placed through.
        let title = play_off(
            TITLE,
            [0, 0],
            0,
            &[place(1, 1), place(1, 4), place(1, 2), place(1, 3)],
            &[
                knockout(&[0, 1, 2, 3], 1, 0, 1),
                knockout(&[WINNERS_OF, WINNERS_OF | 1], 1, 0, 1),
            ],
        );
        let (outcome, _) = end(&mut state, spec(&[continental, title]), &mut played);
        assert!(outcome.is_ok());
        assert_eq!(state.results()[0].kind, Some(ResultKind::Continental));
        assert_eq!(state.results()[0].team, [team(0, 1), team(0, 0)]);
        assert_eq!(state.results()[1].team, [team(1, 0), team(1, 1)]);
        let entries: Vec<(u8, u8, u16)> = page.entries[..usize::from(page.count)]
            .iter()
            .map(|entry| (entry.kind, entry.index, entry.team))
            .collect();
        assert_eq!(
            entries[4..],
            [
                (WINNER, 0, team(0, 1)),
                (WINNER, 1, team(1, 0)),
                (LAST_TIES | 1, 0, team(1, 0)),
                (LAST_TIES | 1, 1, team(1, 1)),
                (LAST_TIES | 1, 2, team(1, 0))
            ]
        );
    }

    #[test]
    fn a_stage_title_is_its_final_s_or_decided_mid_season() {
        let stage = |number: u8| {
            spec(&[play_off(
                STAGE,
                [number, 0],
                0,
                &[place(0, 3), place(0, 1)],
                &[knockout(&[0, 1], 1, 0, 1)],
            )])
        };
        let mut state = Knockouts::default();
        let (outcome, _) = end(&mut state, stage(1), &mut season());
        assert!(outcome.is_ok());
        assert_eq!(
            state.stage_titles()[1],
            KnockoutResult {
                kind: Some(ResultKind::Stage),
                division: 0,
                team: [team(0, 0), team(0, 2)],
                glob: [glob(0, 0), glob(0, 2)],
            }
        );
        // Decided mid-season: the host's, nothing played.
        let decided = ([team(0, 4), team(0, 5)], [glob(0, 4), glob(0, 5)]);
        let mut host = Host {
            stage: Some((1, decided.0, decided.1)),
            ..Host::default()
        };
        let mut played = season();
        played.manager = team(0, 3);
        played.host = Some(&mut host);
        let mut state = Knockouts::default();
        let (outcome, _) = end(&mut state, stage(1), &mut played);
        assert!(outcome.is_ok());
        assert_eq!(
            (state.stage_titles()[1].team, state.stage_titles()[1].glob),
            decided
        );
        // Past the three: none.
        let mut state = Knockouts::default();
        assert!(
            end(&mut state, stage(STAGE_TITLES_U8), &mut season())
                .0
                .is_ok()
        );
        assert!(
            state
                .stage_titles()
                .iter()
                .all(|title| title.kind.is_none())
        );
    }

    /// A stage title's number past the three.
    const STAGE_TITLES_U8: u8 = 3;

    #[test]
    fn a_play_off_stopped_after_its_first_round_decides_no_stage_title() {
        // The final's second club absent: the play-off stops after its semi-final.
        let file = u8::try_from(team(0, 0) >> u8::BITS).expect("a file");
        let entrants = [place(0, 3), place(0, 1), [BY_TEAM_ENTRANT, 0, file, 30]];
        let rounds = [
            knockout(&[0, 1], 1, 0, 1),
            knockout(&[WINNERS_OF, 2], 1, 0, 1),
        ];
        let mut state = Knockouts::default();
        let stopped = spec(&[play_off(STAGE, [1, 0], 0, &entrants, &rounds)]);
        assert!(end(&mut state, stopped, &mut season()).0.is_ok());
        assert_eq!(state.stage_titles()[1].kind, None);
    }

    #[test]
    fn an_absent_group_member_s_figures_leave_with_it() {
        // A ranking round of an absent club and two others: their own figures, moved up.
        let file = u8::try_from(team(0, 0) >> u8::BITS).expect("a file");
        let entrants = [[BY_TEAM_ENTRANT, 0, file, 30], place(0, 5), place(0, 6)];
        let ranking = spec(&[play_off(
            TITLE,
            [0, 0],
            0,
            &entrants,
            &[group(&[0, 1, 2], 0)],
        )]);
        let mut state = Knockouts::default();
        let figures = |points| GroupFigures {
            club: NONE,
            points,
            goal_difference: 0,
            goals: 0,
        };
        state.member_figures[..3].copy_from_slice(&[figures(1), figures(9), figures(5)]);
        assert!(end(&mut state, ranking, &mut season()).0.is_ok());
        assert_eq!(state.results()[0].team, [team(0, 4), team(0, 5)]);
        assert_eq!(state.figures().0[0].points, 9);
    }

    #[test]
    fn a_title_the_host_played_is_its_result() {
        let mut host = Host {
            title: Some((0, team(2, 3), team(2, 1))),
            ..Host::default()
        };
        let mut played = season();
        played.host = Some(&mut host);
        let mut state = Knockouts::default();
        let title = spec(&[play_off(
            TITLE,
            [0, 0],
            0,
            &[place(2, 1), place(2, 2)],
            &[knockout(&[0, 1], 1, 0, 0)],
        )]);
        let (outcome, _) = end(&mut state, title, &mut played);
        assert!(outcome.is_ok());
        assert_eq!(state.results().len(), 1);
        assert_eq!(state.results()[0].team, [team(2, 3), team(2, 1)]);
    }

    #[test]
    fn the_promedio_relegates_but_in_the_preview() {
        let promedio = spec(&[vec![PROMEDIO, 0, 0, 30, 1, 0]]);
        let mut history = Promedio::default();
        let mut played = season();
        played.promedio = Some(&mut history);
        let (outcome, moves) = end(&mut Knockouts::default(), promedio.clone(), &mut played);
        assert_eq!(outcome.map(|done| done.moves), Ok(2));
        assert_eq!(moves, [moved(glob(0, 5), 1), moved(glob(0, 4), 1)]);
        let mut history = Promedio::default();
        let mut preview = season();
        preview.promedio = Some(&mut history);
        preview.preview = true;
        let (outcome, _) = end(&mut Knockouts::default(), promedio, &mut preview);
        assert_eq!(outcome.map(|done| done.moves), Ok(0));
    }

    #[test]
    fn a_play_off_follows_the_previous_one_s_loser_and_some_are_not_played() {
        let first = play_off(
            PROMOTION,
            [0, 1],
            0,
            &[place(0, 6), place(1, 1)],
            &[knockout(&[0, 1], 1, 0, 1)],
        );
        // Its loser, division 1's 1st, against division 1's 2nd: the better placed through.
        let second = play_off(
            PROMOTION,
            [1, 2],
            0,
            &[[PREVIOUS_LOSER_ENTRANT, 0, 0, 0], place(2, 1)],
            &[knockout(&[0, 1], 1, 0, 1)],
        );
        let (outcome, moves) = end(
            &mut Knockouts::default(),
            spec(&[first.clone(), second.clone()]),
            &mut season(),
        );
        assert!(outcome.is_ok());
        assert_eq!(moves, []);
        let mut supercup = second.clone();
        supercup[5] = SUPERCUP;
        let mut preview = season();
        preview.preview = true;
        preview.own_play_offs = [2, 0];
        for (play_offs, season) in [
            (spec(&[first.clone(), supercup]), &mut season()),
            (spec(&[first, second]), &mut preview),
        ] {
            let (outcome, _) = end(&mut Knockouts::default(), play_offs, season);
            assert_eq!(
                outcome,
                Ok(KnockoutsOutcome {
                    moves: 0,
                    read_unplayed: false
                })
            );
        }
    }

    #[test]
    fn a_play_off_not_played_reads_its_rounds_only_where_they_count() {
        // A group then a final of its placings, not played: the placings' figures count.
        let grouped = play_off(
            TITLE,
            [0, 0],
            SUPERCUP,
            &[place(0, 1), place(0, 2)],
            &[
                group(&[0, 1], 1),
                knockout(&[PLACINGS, PLACINGS | 1], 1, 0, 0),
            ],
        );
        let (outcome, _) = end(&mut Knockouts::default(), spec(&[grouped]), &mut season());
        assert_eq!(outcome.map(|done| done.read_unplayed), Ok(true));
        // Knockout rounds only: the winners not played go unread.
        let knocked = play_off(
            TITLE,
            [0, 0],
            SUPERCUP,
            &[place(0, 1), place(0, 2), place(0, 3), place(0, 4)],
            &[
                knockout(&[0, 1, 2, 3], 1, 0, 0),
                knockout(
                    &[WINNERS_OF, WINNERS_OF | 1, BEST_LOSER, LOSERS_OF],
                    1,
                    0,
                    0,
                ),
            ],
        );
        let (outcome, _) = end(&mut Knockouts::default(), spec(&[knocked]), &mut season());
        assert_eq!(outcome.map(|done| done.read_unplayed), Ok(false));
    }

    #[test]
    fn upstream_s_reads_past_its_memory_are_refused() {
        let odd = spec(&[play_off(
            TITLE,
            [0, 0],
            0,
            &[place(0, 1), place(0, 2), place(0, 3), place(0, 4)],
            &[knockout(&[0, 1, 2, 3], 1, 3, 0)],
        )]);
        assert_eq!(
            end(&mut Knockouts::default(), odd, &mut season()).0,
            Err(RulesError::OddPairing(3))
        );
        let file = u8::try_from(team(0, 0) >> u8::BITS).expect("a file");
        let absent = spec(&[play_off(
            PROMOTION,
            [0, 1],
            0,
            &[place(1, 1), place(1, 2), [BY_TEAM_ENTRANT, 0, file, 30]],
            &[knockout(&[0, 1], 1, 0, 0)],
        )]);
        assert_eq!(
            end(&mut Knockouts::default(), absent, &mut season()).0,
            Err(RulesError::AbsentMoved)
        );
        // 62 placings, then a final group of two promoting seven.
        let past = spec(&[play_off(
            PROMOTION,
            [0, 1],
            7 << WINNERS_SHIFT,
            &[place(0, 1), place(1, 1)],
            &[group(&[0; 32], 0), group(&[1; 30], 0), group(&[0, 1], 0)],
        )]);
        assert_eq!(
            end(&mut Knockouts::default(), past, &mut season()).0,
            Err(RulesError::PlacingPastGroups(64))
        );
        let mut unsound = spec(&[play_off(
            TITLE,
            [0, 0],
            0,
            &[place(0, 1), place(0, 2)],
            &[knockout(&[0, 1], 1, 0, 0)],
        )]);
        unsound.pop();
        assert!(matches!(
            end(&mut Knockouts::default(), unsound, &mut season()).0,
            Err(RulesError::Check(_))
        ));
    }

    /// The entrant types of a club by team word, of the previous play-off's loser.
    const BY_TEAM_ENTRANT: u8 = 8;
    /// See [`BY_TEAM_ENTRANT`].
    const PREVIOUS_LOSER_ENTRANT: u8 = 4;

    #[test]
    fn best_v_worst_pairs_the_sorted_1st_with_the_last() {
        let tables = tables(&COUNTS);
        let league = league(3, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                let mut members = [club_at(1, 0), club_at(0, 3), club_at(0, 5), club_at(0, 1)];
                assert_eq!(play.best_against_worst(&mut members), Ok(()));
                assert_eq!(
                    members,
                    [club_at(1, 0), club_at(0, 1), club_at(0, 5), club_at(0, 3)]
                );
                assert_eq!(
                    play.best_against_worst(&mut members[..3]),
                    Err(RulesError::OddPairing(3))
                );
            },
        );
    }

    #[test]
    fn a_play_off_s_header_rounds_seed_and_page_entries() {
        let header = Header::of(&[TITLE, 3, 2, 1, RESERVE, 3 << WINNERS_SHIFT | BY_RECORD]);
        assert_eq!(
            (
                header.kind,
                header.entrants,
                header.rounds,
                header.winner_division,
                header.loser_division
            ),
            (TITLE, 3, 2, 1, RESERVE)
        );
        assert_eq!(header.group_winners(), 3);
        assert_eq!(Header::of(&[1, 0, 0, 0, 0, BY_RECORD]).group_winners(), 1);
        assert_eq!(
            (result_kind(TITLE), result_kind(CONTINENTAL)),
            (ResultKind::Title, ResultKind::Continental)
        );
        let mut rounds = Rounds::new([NONE; ENTRANTS], true);
        rounds.wins[0] = Some(4);
        rounds.losses[0] = Some(5);
        assert_eq!(rounds.tie(0), (4, 5));
        let tables = tables(&COUNTS);
        let league = league(3, None);
        let mut page = Winners::default();
        let mut played = season();
        played.winners = Some(&mut page);
        with_play(
            &tables,
            &league,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                let clubs = [club_at(0, 1), ABSENT];
                let row = play.row(clubs[0]);
                let first = PLAY_OFF_SEED.wrapping_mul(3).wrapping_add(u32::from(TITLE));
                let expected = first
                    .wrapping_mul(SEED_MULTIPLIER)
                    .wrapping_add(
                        u32::from(row.team) + (u32::from(row.points) << ENTRANT_POINTS_SHIFT),
                    )
                    .wrapping_mul(SEED_MULTIPLIER)
                    .wrapping_add(UNKNOWN_SEED);
                assert_eq!(play.entrants_seed(2, TITLE, &clubs), expected);
                play.winner_entry(WINNER, 0, team(0, 0));
                assert_eq!(
                    play.season.winners.as_deref().map(|page| page.count),
                    Some(0),
                    "not written"
                );
                play.winners_on = true;
                play.winner_entry(WINNER, 0x105, NO_CLUB);
                for _ in 0..=WINNERS {
                    play.winner_entry(WINNER, 0x105, team(0, 0));
                }
                assert_eq!(play.placing(&rounds, 1), Ok(NONE));
                assert!(play.read_unplayed);
                assert_eq!(
                    play.placing(&rounds, ENTRANTS),
                    Err(RulesError::PlacingPastGroups(ENTRANTS))
                );
                assert_eq!(play.unplayed_where(false), NONE);
                assert_eq!(play.move_entrant(ABSENT, 0), Ok(()));
                assert_eq!(play.move_entrant(ABSENT, 1), Err(RulesError::AbsentMoved));
                assert_eq!(play.move_entrant(club_at(0, 1), 1), Ok(()));
                assert_eq!(play.moves.len(), 1);
            },
        );
        assert_eq!(usize::from(page.count), WINNERS);
        assert_eq!(
            page.entries[0],
            WinnersEntry {
                kind: WINNER,
                index: 5,
                team: team(0, 0)
            }
        );
    }
}
