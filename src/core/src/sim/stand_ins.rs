//! The stand-ins of a simulated match (docs/PLAN.md M5 #8c; upstream src/match_sim.c
//! `BuildAvailablePlayersRemapAndMask`): a few of the bench's fit players, picked by the
//! team's substitutes, each standing in for a place of the eleven his position may take, by the
//! formation's defensive order. A stand-in may score a place's goal, or take its status.

use openswos_rng::Rng;

use super::tables::{ROLE_SLOTS, STAND_IN_CANDIDATES, STAND_IN_PICKS};
use super::team::{CAREER_LINE_UP, MatchTeam};
use super::value::SimError;

/// No stand-in for a record (upstream's table filled with 0xFF).
pub const NONE: u8 = 0xFF;

/// How far a player's position is shifted in `positionAndFace`.
const POSITION_SHIFT: u8 = 5;

/// The goalkeeper's position.
const GOALKEEPER: u8 = 0;

/// A player's status bucket (`cardsInjuries` bits 7-5).
const STATUS_BUCKET: u8 = 0xE0;

/// A player's injuries (bits 4-2).
const INJURY_BITS: u8 = 0x1C;

/// The first injury that keeps a stand-in out.
const SERIOUS_INJURY: u8 = 0x08;

/// A player's cards (bits 1-0).
const CARD_BITS: u8 = 0x03;

/// The bits of a draw a pick count is drawn by (of eight).
const PICK_DRAW: u16 = 0x07;

/// The bits of a draw the candidates' row is drawn by (four of eight).
const CANDIDATES_DRAW: u16 = 0x18;

/// The candidates of one count of substitutes.
const CANDIDATE_ROW: usize = 32;

/// The draw under which a goalkeeper is taken as a stand-in.
const KEEPER_ODDS: u16 = 4;

/// The bits of a draw a role's slot is drawn by.
const SLOT_DRAW: u16 = 0x07;

/// A match's stand-ins for a team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StandIns {
    /// Each record's stand-in's record, [`NONE`] for none.
    pub table: [u8; CAREER_LINE_UP],
    /// The stand-ins picked.
    pub count: u16,
    /// The records picked, a bit each.
    pub mask: u32,
}

/// A player's position (`positionAndFace` bits 7-5).
fn role(position_and_face: u8) -> u8 {
    position_and_face >> POSITION_SHIFT
}

/// The record line-up place `place` fields (a byte), checked within the line-up.
fn place_record(team: &MatchTeam, place: usize) -> Result<u8, SimError> {
    team.line_up(place)
        .map(i8::cast_unsigned)
        .ok_or(SimError::LineUpPastOrder(place))
}

/// The stand-ins' entry of record `record`.
fn entry(table: &[u8; CAREER_LINE_UP], record: u8) -> Result<u8, SimError> {
    table
        .get(usize::from(record))
        .copied()
        .ok_or(SimError::StandInPastTable(record.cast_signed()))
}

/// The record of the place of the eleven the formation's slot `slot` puts, for `team`.
fn slot_record(team: &MatchTeam, orders: &[[u8; 11]], slot: u8) -> Result<u8, SimError> {
    let tactics = team.tactics().cast_signed();
    let order = usize::try_from(tactics)
        .ok()
        .and_then(|index| orders.get(index))
        .ok_or(SimError::TacticsPastTable(tactics))?;
    place_record(team, usize::from(order[usize::from(slot)]))
}

/// `team`'s stand-ins for a simulated match (upstream `BuildAvailablePlayersRemapAndMask`), by
/// its `substitutes` (fewest, most), the formations' `orders` and `mode` (the competition's
/// track: 0, a player's serious injuries keep him off the bench; else his cards), drawn by
/// `rng`: a count of stand-ins by the fewest substitutes; candidates' places by the most, until
/// one of none; each a fit player, a goalkeeper only a draw in 64 (another draw then, the
/// count not spent), standing in for a place his position may take (a second draw when that
/// place has a stand-in already).
///
/// # Errors
///
/// [`SimError`] when the substitutes are past upstream's tables, a place is past the line-up,
/// names no record of the team or of the stand-ins' table, or the tactics are past the
/// formations.
pub fn stand_ins(
    team: &MatchTeam,
    (fewest, most): (u16, u16),
    orders: &[[u8; 11]],
    mode: i16,
    rng: &mut Rng,
) -> Result<StandIns, SimError> {
    let mut stand_ins = StandIns {
        table: [NONE; CAREER_LINE_UP],
        count: 0,
        mask: 0,
    };
    let draw = rng.rand2();
    let picks = STAND_IN_PICKS
        .get(usize::from(fewest))
        .ok_or(SimError::SubstitutesPastTable(fewest))?[usize::from(draw & PICK_DRAW)];
    if picks == 0 {
        return Ok(stand_ins);
    }
    let draw = rng.rand2();
    // Faithful to the original: the row of the most substitutes from 1.
    let row = usize::from(most)
        .checked_sub(1)
        .map(|row| row * CANDIDATE_ROW + usize::from(draw & CANDIDATES_DRAW))
        .ok_or(SimError::SubstitutesPastTable(most))?;
    let mut left = i16::from(picks.cast_signed()) - 1;
    let mut next = row;
    while left >= 0 {
        let candidate = *STAND_IN_CANDIDATES
            .get(next)
            .ok_or(SimError::SubstitutesPastTable(most))?;
        next += 1;
        left -= 1;
        if candidate == 0 {
            break;
        }
        let record = place_record(team, usize::from(candidate))?;
        let slot = usize::from(record);
        if slot >= team.records() {
            return Err(SimError::PlayerPastTeam(record.cast_signed()));
        }
        let status = team.cards_injuries(slot);
        if status & STATUS_BUCKET != 0 {
            continue;
        }
        let out = if mode == 0 {
            status & INJURY_BITS >= SERIOUS_INJURY
        } else {
            status & CARD_BITS != 0
        };
        if out {
            continue;
        }
        let position = role(team.position_and_face(slot));
        if position == GOALKEEPER && rng.rand2() >= KEEPER_ODDS {
            left += 1;
            continue;
        }
        let slots = &ROLE_SLOTS[usize::from(position)];
        let mut target = slot_record(team, orders, slots[usize::from(rng.rand2() & SLOT_DRAW)])?;
        if entry(&stand_ins.table, target)?.cast_signed() >= 0 {
            target = slot_record(team, orders, slots[usize::from(rng.rand2() & SLOT_DRAW)])?;
            if entry(&stand_ins.table, target)?.cast_signed() >= 0 {
                continue;
            }
        }
        stand_ins.table[usize::from(target)] = record;
        stand_ins.mask |= 1 << record;
        stand_ins.count += 1;
    }
    Ok(stand_ins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::team::{PLAYERS, RECORD_LEN};

    /// A selected team fielding its records in order, every player a striker, its tactics 0.
    fn team() -> MatchTeam {
        let mut team = MatchTeam::selected(&[0; RECORD_LEN]);
        for slot in 0..PLAYERS {
            team.set_player_number(slot, u8::try_from(slot).expect("16"));
            team.set_position_and_face(slot, 7 << POSITION_SHIFT);
        }
        team
    }

    #[test]
    fn no_substitutes_pick_no_stand_in() {
        let mut rng = Rng::default();
        rng.fixed = Some(0);
        let stand_ins = stand_ins(&team(), (0, 1), &[[0; 11]], 0, &mut rng).expect("drawn");
        assert_eq!(stand_ins.count, 0);
        assert!(stand_ins.table.iter().all(|&entry| entry == NONE));
    }

    #[test]
    fn substitutes_past_the_tables_are_refused() {
        let mut rng = Rng::default();
        rng.fixed = Some(0);
        assert_eq!(
            stand_ins(&team(), (6, 1), &[[0; 11]], 0, &mut rng),
            Err(SimError::SubstitutesPastTable(6))
        );
    }
}
