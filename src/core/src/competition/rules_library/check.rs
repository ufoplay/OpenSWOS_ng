//! Untrusted input checked once, before any rule reads it (upstream src/rules/sr_check.c
//! `sr_spec_check`, `sr_league_check`): a league's knockouts' spec (the bytes the Mod's
//! `compile_spec` makes, or a spec written by hand) and a league's rules. The rules keep the
//! Mod's fixed buffers (a play-off's entrants in 64 slots, a round's members in 128, a group's
//! rows in 32); these walks refuse every count over its buffer, every index past what exists,
//! every byte with no meaning, naming the play-off, the byte and the field.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::league::League;
use super::table::{CLUBS, DIVISIONS};

/// A play-off's entrants at most (gplay.c `GN`).
const ENTRANTS: usize = 64;

/// A play-off's rounds at most.
const ROUNDS: u8 = 16;

/// The bytes of a play-off's header, and of a round's.
const HEADER: usize = 6;

/// The bytes of an entrant.
const ENTRANT: usize = 4;

/// The kinds of play-off (1 to 8).
const KINDS: std::ops::RangeInclusive<u8> = 1..=8;

/// The promedio's kind: no entrants, no rounds.
const PROMEDIO: u8 = 7;

/// The entrant types (0 to 8).
const ENTRANT_TYPES: u8 = 8;

/// The entrant types that name a division: by place, a stage table, the title deciders' seeds,
/// the previous play-off's loser, a promedio's.
const DIVISION_TYPES: [u8; 5] = [0, 3, 5, 6, 7];

/// The entrant type by place.
const BY_PLACE: u8 = 0;

/// The entrant type from a stage table.
const STAGE: u8 = 3;

/// A stage table entrant's tables: the two kept ones, or a fixture stage's.
const KEPT_TABLES: u8 = 2;

/// The first fixture stage table.
const FIRST_STAGE_TABLE: u8 = 4;

/// The stage tables the bank holds (`SR_STAGE_TABLES`).
const STAGE_TABLES: u8 = 4;

/// The flag of a round played as a group.
const GROUP: u8 = 0x80;

/// The bits of a round's legs or meetings, its host, its pairing.
const LOW_BITS: u8 = 0x7F;

/// A group round's meetings at most.
const MEETINGS: u8 = 8;

/// A knockout round's legs (one to three).
const LEGS: std::ops::RangeInclusive<u8> = 1..=3;

/// A round's host at most.
const HOSTS: u8 = 3;

/// A round's level at most.
const LEVELS: u8 = 5;

/// A member naming the best loser of the rounds so far.
const BEST_LOSER: u8 = 0x3F;

/// The members naming an entrant (below), a group's placing (below the next), a tie's winner.
const ENTRANT_MEMBERS: u8 = 0x40;

/// See [`ENTRANT_MEMBERS`].
const PLACING_MEMBERS: u8 = 0x80;

/// The bits of a member's index.
const MEMBER_INDEX: u8 = 0x3F;

/// What a check found wrong (`SR_CHECK_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckCode {
    /// The bytes end inside a field.
    Truncated,
    /// A count over its buffer.
    Count,
    /// An index past what exists.
    Index,
    /// A byte with no meaning.
    Value,
}

/// The first fault a check found (`sr_check_error`): its code, the play-off (none outside
/// one), the byte or element, the field's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CheckError {
    /// What is wrong.
    pub code: CheckCode,
    /// The play-off's index, none outside one.
    pub play_off: Option<u32>,
    /// The byte (a spec) or the element (a state).
    pub offset: u32,
    /// The field's name.
    pub field: &'static str,
}

/// A fault at `offset` of play-off `play_off`.
fn fault(code: CheckCode, play_off: Option<u32>, offset: usize, field: &'static str) -> CheckError {
    CheckError {
        code,
        play_off,
        offset: u32::try_from(offset).expect("a spec's bytes"),
        field,
    }
}

/// A league's knockouts' `spec`, for a league of `divisions` divisions (0 or past 16: any up
/// to 16), checked (upstream `sr_spec_check`): each play-off's header (its kind; the promedio's
/// matches and loser's division; else at most 64 entrants and 1 to 16 rounds), its entrants
/// (their type; the division, the place from 1, the stage table they name), its rounds (at
/// most 32 members a group, 64 ties a knockout; 8 meetings, 1 to 3 legs; the host, the level,
/// the pairing; each member an entrant, a group's placing or a tie's winner so far; at most 64
/// placings and ties in all), up to the 0 that ends them.
///
/// # Errors
///
/// The first fault, a [`CheckError`].
pub fn spec_check(spec: &[u8], divisions: i32) -> Result<(), CheckError> {
    let divisions = match usize::try_from(divisions) {
        Ok(divisions) if divisions > 0 && divisions < DIVISIONS => divisions,
        Ok(_) | Err(_) => DIVISIONS,
    };
    let len = spec.len();
    let mut at = 0;
    for play_off in 0.. {
        let index = Some(play_off);
        if at >= len {
            return Err(fault(
                CheckCode::Truncated,
                index,
                at,
                "end of the play-offs (a 0)",
            ));
        }
        if spec[at] == 0 {
            return Ok(());
        }
        if len - at < HEADER {
            return Err(fault(CheckCode::Truncated, index, at, "play-off header"));
        }
        let header = &spec[at..at + HEADER];
        let (kind, entrants, rounds) = (header[0], usize::from(header[1]), header[2]);
        if !KINDS.contains(&kind) {
            return Err(fault(CheckCode::Value, index, at, "kind"));
        }
        if kind == PROMEDIO {
            if entrants != 0
                || rounds != 0
                || header[3] == 0
                || header[4] == 0
                || usize::from(header[4]) >= divisions
            {
                return Err(fault(
                    CheckCode::Value,
                    index,
                    at,
                    "promedio (matches, loser_div)",
                ));
            }
            at += HEADER;
            continue;
        }
        if entrants > ENTRANTS {
            return Err(fault(
                CheckCode::Count,
                index,
                at + 1,
                "entrants (at most 64)",
            ));
        }
        if !(1..=ROUNDS).contains(&rounds) {
            return Err(fault(CheckCode::Count, index, at + 2, "rounds (1..16)"));
        }
        at += HEADER;
        if len - at < ENTRANT * entrants {
            return Err(fault(CheckCode::Truncated, index, at, "entrants"));
        }
        for _ in 0..entrants {
            let entrant = &spec[at..at + ENTRANT];
            let kind = entrant[0];
            if kind > ENTRANT_TYPES {
                return Err(fault(CheckCode::Value, index, at, "entrant type"));
            }
            if DIVISION_TYPES.contains(&kind) && usize::from(entrant[1]) >= divisions {
                return Err(fault(CheckCode::Index, index, at + 1, "entrant division"));
            }
            if kind == BY_PLACE && entrant[2] < 1 {
                return Err(fault(
                    CheckCode::Index,
                    index,
                    at + 2,
                    "entrant place (from 1)",
                ));
            }
            let table = entrant[3];
            if kind == STAGE
                && !(table < KEPT_TABLES
                    || (FIRST_STAGE_TABLE..FIRST_STAGE_TABLE + STAGE_TABLES).contains(&table))
            {
                return Err(fault(
                    CheckCode::Index,
                    index,
                    at + 3,
                    "entrant stage table",
                ));
            }
            at += ENTRANT;
        }
        let (mut placings, mut ties_so_far) = (0usize, 0usize);
        for round in 0..rounds {
            if len - at < HEADER {
                return Err(fault(CheckCode::Truncated, index, at, "round header"));
            }
            let header = &spec[at..at + HEADER];
            let (ties, legs) = (usize::from(header[0]), header[1]);
            let (host, level) = (header[2] & LOW_BITS, header[3]);
            let group = legs & GROUP != 0;
            let members = if group { ties } else { 2 * ties };
            if if group { ties > CLUBS } else { ties > ENTRANTS } {
                let field = if group {
                    "group members (at most 32)"
                } else {
                    "ties (at most 64)"
                };
                return Err(fault(CheckCode::Count, index, at, field));
            }
            if if group {
                legs & LOW_BITS > MEETINGS
            } else {
                !LEGS.contains(&legs)
            } {
                let field = if group {
                    "meetings (0..8)"
                } else {
                    "legs (1..3)"
                };
                return Err(fault(CheckCode::Value, index, at + 1, field));
            }
            if host > HOSTS {
                return Err(fault(CheckCode::Value, index, at + 2, "host"));
            }
            if level > LEVELS {
                return Err(fault(CheckCode::Value, index, at + 3, "level"));
            }
            if !group && usize::from(header[4] & LOW_BITS) > 2 * ties {
                return Err(fault(
                    CheckCode::Count,
                    index,
                    at + 4,
                    "pairing (at most the round's clubs)",
                ));
            }
            at += HEADER;
            // Faithful to the original: a round's member bytes are twice its ties, a group's
            // too (of which its ties are read).
            if len - at < 2 * ties {
                return Err(fault(CheckCode::Truncated, index, at, "members"));
            }
            for offset in 0..members {
                let member = spec[at + offset];
                let named = usize::from(member & MEMBER_INDEX);
                let known = if member == BEST_LOSER {
                    round > 0 && ties_so_far > 0
                } else if member < ENTRANT_MEMBERS {
                    usize::from(member) < entrants
                } else if member < PLACING_MEMBERS {
                    named < placings
                } else {
                    named < ties_so_far
                };
                if !known {
                    return Err(fault(CheckCode::Index, index, at + offset, "member"));
                }
            }
            if group {
                if placings + ties > ENTRANTS {
                    return Err(fault(CheckCode::Count, index, at, "placings (at most 64)"));
                }
                placings += ties;
            } else {
                if ties_so_far + ties > ENTRANTS {
                    return Err(fault(
                        CheckCode::Count,
                        index,
                        at,
                        "ties of the play-off (at most 64)",
                    ));
                }
                ties_so_far += ties;
            }
            at += 2 * ties;
        }
    }
    unreachable!("the play-offs end in a 0 or a fault")
}

/// A league's rules checked (upstream `sr_league_check`): at most 16 divisions; its fixed
/// tiers' 2 to 4 groups and its upper tier within its divisions; its spec, when it has one, of
/// some bytes and sound ([`spec_check`]).
///
/// # Errors
///
/// The first fault, a [`CheckError`].
pub fn league_check(league: &League) -> Result<(), CheckError> {
    let divisions = league.divisions;
    if usize::from(divisions) > DIVISIONS {
        return Err(fault(CheckCode::Count, None, 0, "divisions (at most 16)"));
    }
    if let Some(tiers) = league.fixed_tiers
        && (tiers.lower_groups < 2
            || tiers.lower_groups > 4
            || u16::from(tiers.lower_first) + u16::from(tiers.lower_groups) > u16::from(divisions)
            || tiers.upper >= divisions)
    {
        return Err(fault(
            CheckCode::Index,
            None,
            0,
            "the fixed tiers' divisions (the upper tier, the lower tier's groups)",
        ));
    }
    match &league.spec {
        Some(spec) if spec.is_empty() => Err(fault(
            CheckCode::Count,
            None,
            0,
            "spec_len (a spec's bytes: required)",
        )),
        Some(spec) => spec_check(spec, i32::from(divisions)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::rules_library::league::FixedTiers;

    /// A play-off of kind 1: two entrants by place (division 0, places 1 and 2), one knockout
    /// round of one tie (two legs, host 0, level 0, pairing 2) between them.
    const ONE_TIE: [u8; 20] = [1, 2, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 1, 2, 0, 0, 2, 0];

    /// [`ONE_TIE`]'s members, then the end.
    fn spec() -> Vec<u8> {
        let mut spec = ONE_TIE.to_vec();
        spec.extend([0, 1, 0]);
        spec
    }

    #[test]
    fn a_sound_spec_passes() {
        assert_eq!(spec_check(&spec(), 2), Ok(()));
        // A promedio: matches, the loser's division.
        assert_eq!(spec_check(&[7, 0, 0, 38, 1, 0, 0], 2), Ok(()));
    }

    #[test]
    fn a_faulty_spec_names_its_first_fault() {
        let fault_of = |spec: &[u8], divisions| {
            spec_check(spec, divisions).map_err(|error| (error.code, error.play_off, error.offset))
        };
        assert_eq!(fault_of(&[], 0), Err((CheckCode::Truncated, Some(0), 0)));
        assert_eq!(
            fault_of(&[9, 0, 1, 0, 0, 0], 0),
            Err((CheckCode::Value, Some(0), 0))
        );
        let mut wrong = spec();
        wrong[6] = 9;
        assert_eq!(fault_of(&wrong, 2), Err((CheckCode::Value, Some(0), 6)));
        // A division past the league's two.
        let mut far = spec();
        far[7] = 2;
        assert_eq!(fault_of(&far, 2), Err((CheckCode::Index, Some(0), 7)));
        // A member naming a third entrant.
        let mut member = spec();
        member[21] = 2;
        assert_eq!(fault_of(&member, 2), Err((CheckCode::Index, Some(0), 21)));
        // No end.
        assert_eq!(
            fault_of(&spec()[..22], 2),
            Err((CheckCode::Truncated, Some(1), 22))
        );
    }

    #[test]
    fn a_league_is_checked_with_its_spec() {
        let tiers = FixedTiers {
            upper: 1,
            lower_first: 2,
            lower_groups: 3,
        };
        let league = League {
            divisions: 5,
            fixed_tiers: Some(tiers),
            spec: Some(spec()),
            ..League::default()
        };
        assert_eq!(league_check(&league), Ok(()));
        let narrow = League {
            divisions: 4,
            ..league.clone()
        };
        assert_eq!(
            league_check(&narrow).map_err(|error| error.code),
            Err(CheckCode::Index)
        );
        let empty = League {
            spec: Some(Vec::new()),
            ..league.clone()
        };
        assert_eq!(
            league_check(&empty).map_err(|error| error.code),
            Err(CheckCode::Count)
        );
        let wide = League {
            divisions: 17,
            ..League::default()
        };
        assert_eq!(
            league_check(&wide).map_err(|error| error.code),
            Err(CheckCode::Count)
        );
    }
}
