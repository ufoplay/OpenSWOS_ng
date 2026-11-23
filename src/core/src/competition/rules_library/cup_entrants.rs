//! A cup's clubs from the second season on (upstream src/rules/sr_cups.c `sr_cup_entrants`; the
//! SWOS 96/97 Mod's tools/gplay.c `cf_build`, 4.0.1). The first season plays the real draw (the
//! world's lists); from the second, each cup keeps its real list's shape (per division, how
//! many clubs and how many of them with a bye) and takes them from where the clubs are now: the
//! ones relegated into the division first, then those that stayed, then the promoted, then
//! those without a place; within each, by last season's place, then by slot. The career club
//! goes first when it is among them.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::table::RulesError;

/// A cup's entrants at most (`SR_CUP_CLUBS`).
pub const CUP_CLUBS: usize = 64;

/// A country's clubs at most (`SR_CLUB_SLOTS`: two team files' slots).
pub const CLUB_SLOTS: usize = 512;

/// A place's bits in a place byte (its level in the others).
const PLACE_BITS: u8 = 0x1F;

/// How far a place byte's level is shifted.
const LEVEL_SHIFT: u8 = 5;

/// A club without a place: past every place.
const NO_PLACE: u32 = 0xFF;

/// The order of a club relegated into its division, of one that stayed, of a promoted one, of
/// one without a place.
const RELEGATED: u32 = 0;

/// See [`RELEGATED`].
const STAYED: u32 = 1;

/// See [`RELEGATED`].
const PROMOTED: u32 = 2;

/// See [`RELEGATED`].
const UNPLACED: u32 = 3;

/// How far an entrant's order is shifted in its key.
const ORDER_SHIFT: u32 = 17;

/// How far its place is shifted in its key.
const PLACE_SHIFT: u32 = 9;

/// A division's share of a cup (`sr_quota`): its clubs, how many of them with a bye.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Quota {
    /// The division.
    pub division: u8,
    /// Its clubs in the cup.
    pub clubs: u8,
    /// How many of them have a bye.
    pub byes: u8,
}

/// A league's clubs as a cup takes them: each club's slot and this season's division (after
/// the moves), last season's places by slot (`place | level << 5`, 0 none; none at all when
/// they are not this league's last season), each division's level (a level of groups one).
#[derive(Debug, Clone, Copy)]
pub struct CupClubs<'a> {
    /// Each club's slot.
    pub slots: &'a [u16],
    /// Each club's division this season.
    pub divisions: &'a [u8],
    /// Last season's places, by slot.
    pub places: Option<&'a [u8]>,
    /// Each division's level.
    pub levels: &'a [u8],
}

/// This season's clubs of a cup of `quotas`, `entrants` in all, from `clubs` (upstream
/// `sr_cup_entrants`): each quota's division in turn, its clubs ordered (relegated into it,
/// stayed, promoted, without a place; then the place, the slot), the career club's slot
/// (`career_slot`, below 0 none) first; the quota's last `byes` set aside; round 1's clubs,
/// then the byes. None when the real list must be played: more clubs than a country holds, a
/// division with fewer clubs than its quota, or not `entrants` in all.
///
/// # Errors
///
/// [`RulesError`] when a club has no division, a slot no place, or a quota's division no
/// level (upstream reads past them).
pub fn cup_entrants(
    quotas: &[Quota],
    entrants: i32,
    clubs: &CupClubs<'_>,
    career_slot: i32,
) -> Result<Option<Vec<u16>>, RulesError> {
    let count = clubs.slots.len();
    if count > CLUB_SLOTS {
        return Ok(None);
    }
    if clubs.divisions.len() < count {
        return Err(RulesError::ClubsPastArrays(count));
    }
    let (mut round_one, mut byes) = (Vec::new(), Vec::new());
    for quota in quotas {
        let level = *clubs
            .levels
            .get(usize::from(quota.division))
            .ok_or(RulesError::DivisionPastLevels(quota.division))?;
        let mut candidates: Vec<(u32, u16)> = Vec::with_capacity(count);
        for (&slot, &division) in clubs.slots.iter().zip(clubs.divisions) {
            if division == quota.division {
                candidates.push((0, slot));
            }
        }
        let wanted = usize::from(quota.clubs);
        if candidates.len() < wanted {
            return Ok(None);
        }
        for (key, slot) in &mut candidates {
            let place = match clubs.places {
                Some(places) => *places
                    .get(usize::from(*slot))
                    .ok_or(RulesError::SlotPastPlaces(*slot))?,
                None => 0,
            };
            let (rank, order) = if place == 0 {
                (NO_PLACE, UNPLACED)
            } else {
                let last_level = place >> LEVEL_SHIFT;
                let order = match last_level.cmp(&level) {
                    std::cmp::Ordering::Less => RELEGATED,
                    std::cmp::Ordering::Equal => STAYED,
                    std::cmp::Ordering::Greater => PROMOTED,
                };
                (u32::from(place & PLACE_BITS), order)
            };
            *key = order << ORDER_SHIFT | rank << PLACE_SHIFT | u32::from(*slot);
        }
        // Faithful to the original: an insertion sort, equal keys kept in their order.
        candidates.sort_by_key(|&(key, _)| key);
        if let Some(at) = candidates
            .iter()
            .position(|&(_, slot)| i32::from(slot) == career_slot)
        {
            let career = candidates.remove(at);
            candidates.insert(0, career);
        }
        let byes_from = wanted.saturating_sub(usize::from(quota.byes));
        for (index, &(_, slot)) in candidates.iter().take(wanted).enumerate() {
            let list = if index >= byes_from {
                &mut byes
            } else {
                &mut round_one
            };
            if list.len() < CUP_CLUBS {
                list.push(slot);
            }
        }
    }
    let room = CUP_CLUBS - round_one.len();
    round_one.extend(byes.into_iter().take(room));
    let listed = i32::try_from(round_one.len()).expect("64 clubs");
    Ok((listed == entrants && listed != 0).then_some(round_one))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A place byte: `place` at `level`.
    fn placed(place: u8, level: u8) -> u8 {
        place | level << LEVEL_SHIFT
    }

    #[test]
    fn a_division_s_clubs_come_relegated_stayed_promoted_then_unplaced() {
        // Division 1 (level 1): slot 0 was 3rd at level 1, slot 1 1st at level 0 (relegated),
        // slot 2 2nd at level 2 (promoted), slot 3 none, slot 4 1st at level 1.
        let mut places = [0; 8];
        places[..5].copy_from_slice(&[placed(3, 1), placed(1, 0), placed(2, 2), 0, placed(1, 1)]);
        let clubs = CupClubs {
            slots: &[0, 1, 2, 3, 4, 5],
            divisions: &[1, 1, 1, 1, 1, 0],
            places: Some(&places),
            levels: &[0, 1],
        };
        let quotas = [
            Quota {
                division: 1,
                clubs: 4,
                byes: 1,
            },
            Quota {
                division: 0,
                clubs: 1,
                byes: 1,
            },
        ];
        assert_eq!(
            cup_entrants(&quotas, 5, &clubs, -1),
            Ok(Some(vec![1, 4, 0, 2, 5]))
        );
        // The career club first; the count not met: the real list.
        assert_eq!(
            cup_entrants(&quotas, 5, &clubs, 3),
            Ok(Some(vec![3, 1, 4, 0, 5]))
        );
        assert_eq!(cup_entrants(&quotas, 4, &clubs, -1), Ok(None));
        // No places: by slot.
        let unplaced = CupClubs {
            places: None,
            ..clubs
        };
        assert_eq!(
            cup_entrants(&quotas, 5, &unplaced, -1),
            Ok(Some(vec![0, 1, 2, 3, 5]))
        );
    }

    #[test]
    fn what_cannot_be_read_is_refused_and_what_cannot_be_met_is_the_real_list() {
        let quota = Quota {
            division: 2,
            clubs: 1,
            byes: 0,
        };
        let clubs = CupClubs {
            slots: &[9],
            divisions: &[2],
            places: Some(&[0; 4]),
            levels: &[0, 1, 2],
        };
        assert_eq!(
            cup_entrants(&[quota], 1, &clubs, -1),
            Err(RulesError::SlotPastPlaces(9))
        );
        let no_level = CupClubs {
            levels: &[0],
            ..clubs
        };
        assert_eq!(
            cup_entrants(&[quota], 1, &no_level, -1),
            Err(RulesError::DivisionPastLevels(2))
        );
        let too_few = Quota { clubs: 2, ..quota };
        let unplaced = CupClubs {
            places: None,
            ..clubs
        };
        assert_eq!(cup_entrants(&[too_few], 2, &unplaced, -1), Ok(None));
        let many_slots = vec![0; CLUB_SLOTS + 1];
        let many_divisions = vec![2; CLUB_SLOTS + 1];
        let crowded = CupClubs {
            slots: &many_slots,
            divisions: &many_divisions,
            ..unplaced
        };
        assert_eq!(cup_entrants(&[quota], 1, &crowded, -1), Ok(None));
    }
}
