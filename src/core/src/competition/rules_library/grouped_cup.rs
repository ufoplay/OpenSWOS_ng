//! A cup of a level of grouped divisions: a later season's first two rounds (upstream
//! src/rules/sr_cupd.c; the SWOS 96/97 Mod's tools/gplay.c `cd_rank`, `cd_season`, `cd_make`).
//! The first season plays the real draw (the world's table). From the second, the level's clubs
//! enter by last season's end: those relegated into it and the pool's new clubs first, then by
//! their place from the bottom of their group; then a preliminary round and a first round,
//! each tie paired within a group where it can be. The draws are the Mod's generator, seeded
//! from the season.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::cup_entrants::CLUB_SLOTS;
use super::generator::below;
use super::table::{DIVISIONS, RulesError};

/// A generated preliminary round's ties at most (`SR_CUPD_PRE`).
pub const PRELIMINARY_TIES: usize = 41;

/// A generated first round's ties at most (`SR_CUPD_R1`).
pub const FIRST_ROUND_TIES: usize = 64;

/// The sides the two rounds hold.
pub const SIDES: usize = 2 * (PRELIMINARY_TIES + FIRST_ROUND_TIES);

/// A slot in no table.
const NO_DIVISION: u8 = 0xFF;

/// The bits of a slot's arrival flags.
const FLAG_BITS: usize = 8;

/// The first place byte a club's place from the bottom counts from.
const BOTTOM: u8 = 2;

/// An entrant's order when last season's end does not know it.
const UNKNOWN: u16 = 0x40;

/// An entrant's order when it came into the level from above or from the pool.
const ARRIVED: u16 = 1;

/// The fewest rounds the cup may have, and the most (a first round of 64 ties).
const ROUNDS: std::ops::RangeInclusive<u8> = 2..=8;

/// The draws an entrant's key is broken by.
const KEY_DRAW: u32 = 256;

/// How far an entrant's order is shifted in its key.
const ORDER_SHIFT: u16 = 8;

/// The sides' group of none.
const NO_GROUP: u8 = 0xFF;

/// A preliminary tie as a first round's side: its index, then a file no team has.
const TIE_SIDE: u16 = 0xFF;

/// The multiplier the season seeds the Mod's generator by.
const SEASON_SEED: u32 = 104_729;

/// What the season adds to its seed.
const SEED_OFFSET: u32 = 17;

/// What last season's end tells the cup's entry order (`sr_cupd_end`): each slot's division
/// (255 in no table) and, in the cup's level, its place from the bottom of its group plus 2
/// (0 none); the slots that took a pool club.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupedCupEnd {
    /// Each slot's division last season.
    pub last_division: [u8; CLUB_SLOTS],
    /// Each slot's place from the bottom plus 2, 0 none.
    pub place: [u8; CLUB_SLOTS],
    /// The slots that took a pool club, a bit each.
    pub arrival: [u8; CLUB_SLOTS / FLAG_BITS],
}

impl Default for GroupedCupEnd {
    /// A season end that knows nothing (upstream `sr_cupd_end_reset`).
    fn default() -> Self {
        Self {
            last_division: [NO_DIVISION; CLUB_SLOTS],
            place: [0; CLUB_SLOTS],
            arrival: [0; CLUB_SLOTS / FLAG_BITS],
        }
    }
}

/// The cup's level: its rounds, its first division and its groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupedCup {
    /// Its rounds (2 to 8).
    pub rounds: u8,
    /// Its level's first division.
    pub first: u8,
    /// Its level's groups.
    pub groups: u8,
}

impl GroupedCupEnd {
    /// A club's final place (`place`, 0 the top, of `clubs`) in `division` (upstream
    /// `sr_cupd_keep_place`), in the cup's level by `cup`'s first division and groups. A slot
    /// past a country's is ignored.
    pub fn keep_place(
        &mut self,
        cup: GroupedCup,
        division: u16,
        slot: u16,
        place: i32,
        clubs: i32,
    ) {
        let slot = usize::from(slot);
        if slot >= CLUB_SLOTS {
            return;
        }
        let [low, _] = division.to_le_bytes();
        self.last_division[slot] = low;
        let first = u16::from(cup.first);
        if division >= first && division < first + u16::from(cup.groups) {
            let [place, ..] = (i32::from(BOTTOM) + (clubs - 1 - place)).to_le_bytes();
            self.place[slot] = place;
        }
    }

    /// A slot that took a pool club (upstream `sr_cupd_note_arrival`); past a country's,
    /// ignored.
    pub fn note_arrival(&mut self, slot: u16) {
        let slot = usize::from(slot);
        if slot < CLUB_SLOTS {
            self.arrival[slot / FLAG_BITS] |= 1 << (slot % FLAG_BITS);
        }
    }

    /// Whether slot `slot` took a pool club.
    fn arrived(&self, slot: usize) -> bool {
        self.arrival[slot / FLAG_BITS] & 1 << (slot % FLAG_BITS) != 0
    }
}

/// The rounds made: the preliminary ties, then the first round's; tie `t`'s sides `2t` and
/// `2t + 1`, each a team word, 0 a bye, or a preliminary tie's winner (`t << 8 | 255`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupedCupRounds {
    /// The level's clubs this season.
    pub clubs: usize,
    /// The preliminary ties.
    pub preliminary: u8,
    /// The sides written, the preliminary round's first.
    pub sides: Vec<u16>,
}

/// `sides`' ties written into `out` (upstream `cd_pair`): shuffled, each paired with the next
/// side of its group, else the next side, a last one alone given a bye.
fn pair(state: &mut u32, sides: &mut [(u16, u8)], out: &mut Vec<u16>) {
    for index in (1..sides.len()).rev() {
        let bound = u32::try_from(index + 1).expect("256 sides");
        let other = usize::try_from(below(state, bound)).expect("a side");
        sides.swap(index, other);
    }
    let mut used = vec![false; sides.len()];
    for index in 0..sides.len() {
        if used[index] {
            continue;
        }
        let (side, group) = sides[index];
        let partner = (index + 1..sides.len())
            .find(|&other| !used[other] && sides[other].1 != NO_GROUP && sides[other].1 == group)
            .or_else(|| (index + 1..sides.len()).find(|&other| !used[other]));
        used[index] = true;
        out.push(side);
        match partner {
            Some(other) => {
                used[other] = true;
                out.push(sides[other].0);
            }
            None => out.push(0),
        }
    }
}

/// A later season's first two rounds of `cup` (upstream `sr_cupd_make`): the level's clubs this
/// season (`divisions`, by slot: 255 none) in entry order, by last season's `end` (none: a random
/// order): the relegated and the pool's new clubs first, then from the bottom of each group, a
/// draw breaking ties; the first ones, beyond twice the first round's ties, two by two into the
/// preliminary round (41 ties at most); its winners, then the others, into the first round,
/// byes filling it; each round paired within the groups where it can be. The draws are seeded
/// from `season` (the Mod's season byte, the year from 1900); each slot's side is its team word
/// (`words`). Up to 512 slots are read.
///
/// # Errors
///
/// [`RulesError::CupPastLimits`] when the rounds are outside 2 to 8 or the level past the
/// league's divisions (upstream's refusal, nothing made); [`RulesError`] when a slot has no team
/// word, or the preliminary round would take more sides than the level's clubs (upstream
/// reads entries it never wrote).
pub fn grouped_cup_rounds(
    end: Option<&GroupedCupEnd>,
    season: u32,
    cup: GroupedCup,
    divisions: &[u8],
    words: &[u16],
) -> Result<GroupedCupRounds, RulesError> {
    if !ROUNDS.contains(&cup.rounds) || usize::from(cup.first) + usize::from(cup.groups) > DIVISIONS
    {
        return Err(RulesError::CupPastLimits {
            rounds: cup.rounds,
            first: cup.first,
            groups: cup.groups,
        });
    }
    let slots = divisions.len().min(CLUB_SLOTS);
    let mut state = season.wrapping_mul(SEASON_SEED).wrapping_add(SEED_OFFSET);
    let (first, groups) = (cup.first, cup.groups);
    // Each entrant: its key, its slot, its group.
    let mut entrants: Vec<(u16, u16, u8)> = Vec::new();
    for (slot, &division) in divisions.iter().enumerate().take(slots) {
        if division == NO_DIVISION || division < first || division - first >= groups {
            continue;
        }
        let order = end.map_or(UNKNOWN, |end| {
            let last = end.last_division[slot];
            if end.arrived(slot) {
                ARRIVED
            } else if last == NO_DIVISION {
                UNKNOWN
            } else if last < first || last - first >= groups {
                ARRIVED
            } else if end.place[slot] >= BOTTOM {
                u16::from(end.place[slot])
            } else {
                UNKNOWN
            }
        });
        let draw = u16::try_from(below(&mut state, KEY_DRAW)).expect("a byte");
        let slot = u16::try_from(slot).expect("512 slots");
        entrants.push((order << ORDER_SHIFT | draw, slot, division - first));
    }
    // Faithful to the original: an insertion sort, equal keys kept in their order.
    entrants.sort_by_key(|&(key, ..)| key);
    let clubs = entrants.len();
    let first_round = 1usize << (cup.rounds - 2);
    let preliminary = clubs.saturating_sub(2 * first_round).min(PRELIMINARY_TIES);
    if 2 * preliminary > clubs {
        return Err(RulesError::PreliminaryPastClubs(preliminary));
    }
    // The clubs the rounds take: the preliminary round's, then the first round's up to its
    // sides (the others are not read).
    let taken = (clubs - 2 * preliminary).min((2 * first_round).saturating_sub(preliminary));
    let drawn = &entrants[..2 * preliminary + taken];
    if let Some(&(_, slot, _)) = drawn
        .iter()
        .find(|&&(_, slot, _)| usize::from(slot) >= words.len())
    {
        return Err(RulesError::SlotPastWords(slot));
    }
    let word = |slot: u16| words[usize::from(slot)];
    let mut sides = Vec::with_capacity(SIDES);
    let mut round: Vec<(u16, u8)> = entrants[..2 * preliminary]
        .iter()
        .map(|&(_, slot, group)| (word(slot), group))
        .collect();
    pair(&mut state, &mut round, &mut sides);
    let tie_groups: Vec<u8> = (0..preliminary)
        .map(|tie| {
            entrants[..2 * preliminary]
                .iter()
                .rev()
                .find(|&&(_, slot, _)| word(slot) == sides[2 * tie])
                .map_or(NO_GROUP, |&(.., group)| group)
        })
        .collect();
    let mut round: Vec<(u16, u8)> = (0..preliminary)
        .map(|tie| {
            let index = u16::try_from(tie).expect("41 ties");
            (index << ORDER_SHIFT | TIE_SIDE, tie_groups[tie])
        })
        .collect();
    for &(_, slot, group) in &drawn[2 * preliminary..] {
        round.push((word(slot), group));
    }
    round.resize(round.len().max(2 * first_round), (0, NO_GROUP));
    pair(&mut state, &mut round, &mut sides);
    Ok(GroupedCupRounds {
        clubs,
        preliminary: u8::try_from(preliminary).expect("41 ties"),
        sides,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cup of rounds `rounds` over divisions 4 and 5.
    const CUP: GroupedCup = GroupedCup {
        rounds: 2,
        first: 4,
        groups: 2,
    };

    #[test]
    fn a_season_end_keeps_places_from_the_bottom_and_arrivals() {
        let mut end = GroupedCupEnd::default();
        end.keep_place(CUP, 5, 7, 0, 18);
        end.keep_place(CUP, 2, 8, 3, 18);
        let past = u16::try_from(CLUB_SLOTS).expect("512");
        end.keep_place(CUP, 5, past, 0, 18);
        end.note_arrival(9);
        end.note_arrival(past);
        assert_eq!((end.last_division[7], end.place[7]), (5, 2 + 17));
        assert_eq!((end.last_division[8], end.place[8]), (2, 0));
        assert!(end.arrived(9) && !end.arrived(7));
    }

    #[test]
    fn the_rounds_take_every_club_of_the_level() {
        // Six clubs of the level, 3 rounds (a first round of two ties): four go to two
        // preliminary ties; the first round's four sides are their winners and the two others.
        let cup = GroupedCup { rounds: 3, ..CUP };
        let divisions = [4, 5, 4, 5, 4, 5, 0, NO_DIVISION];
        let words: Vec<u16> = (0..8).map(|slot| 0x0100 + slot).collect();
        let rounds = grouped_cup_rounds(None, 126, cup, &divisions, &words).expect("made");
        assert_eq!((rounds.clubs, rounds.preliminary), (6, 2));
        assert_eq!(rounds.sides.len(), 4 + 4);
        let mut drawn: Vec<u16> = rounds.sides[..4].to_vec();
        drawn.extend(
            rounds.sides[4..]
                .iter()
                .filter(|&&side| side & 0xFF != 0xFF),
        );
        drawn.sort_unstable();
        assert_eq!(drawn, [0x100, 0x101, 0x102, 0x103, 0x104, 0x105]);
        // The same season, the same draw.
        assert_eq!(
            grouped_cup_rounds(None, 126, cup, &divisions, &words),
            Ok(rounds)
        );
    }

    #[test]
    fn what_the_cup_cannot_be_is_refused() {
        let past = GroupedCup { rounds: 9, ..CUP };
        assert!(matches!(
            grouped_cup_rounds(None, 0, past, &[], &[]),
            Err(RulesError::CupPastLimits { .. })
        ));
        let wide = GroupedCup { first: 15, ..CUP };
        assert!(grouped_cup_rounds(None, 0, wide, &[], &[]).is_err());
        assert_eq!(
            grouped_cup_rounds(None, 0, CUP, &[4], &[]),
            Err(RulesError::SlotPastWords(0))
        );
        // 43 clubs, rounds 2: a preliminary round of 41 ties, 82 sides of 43.
        let crowded = [4u8; 43];
        let words = [1u16; 43];
        assert_eq!(
            grouped_cup_rounds(None, 0, CUP, &crowded, &words),
            Err(RulesError::PreliminaryPastClubs(41))
        );
    }
}
