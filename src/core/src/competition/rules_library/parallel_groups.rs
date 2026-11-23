//! Leagues with parallel groups at the season end (upstream src/rules/sr_groups.c
//! `sr_pg_season`; the SWOS 96/97 Mod's tools/gplay.c `pg_one`, `pg_level`, `pg_room`, `pg_a2`,
//! `pg_ban`, `pg_make_room`, 4.0.5 with 4.0.6's guard). The game has moved every club one
//! division up or down the league's chain; each move becomes one level: into a single division
//! as it is; into a level of groups, the club's first choice of group, then its second, each
//! with room or room made by moving a club that may go elsewhere, else the group nearest its
//! first choice with room. The clubs that stay are counted in their groups first.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::cup_entrants::CLUB_SLOTS;
use super::table::RulesError;

/// A league's levels at most (`SR_PG_LEVELS`).
pub const GROUP_LEVELS: usize = 8;

/// A level's groups side by side at most (`SR_PG_GROUPS`).
pub const GROUPS: usize = 9;

/// The levels of groups whose areas a club carries (`SR_PG_PARALLEL`).
pub const PARALLEL: usize = 4;

/// No area at a level (an area byte), or no level of groups.
const NONE: u8 = 0xFF;

/// An area byte's first choice (its low nibble).
const NIBBLE: u8 = 0x0F;

/// How far an area byte's second choice is shifted.
const SECOND_SHIFT: u8 = 4;

/// A second choice of none (as a nibble), or of none at all.
const NO_CHOICE: u8 = 15;

/// A second choice of any group (a second team).
const ANY_GROUP: u8 = 14;

/// The bit of a second choice that bans another group.
const BAN: u8 = 8;

/// The bits of a second choice that name its group, plus one.
const CHOICE_GROUP: u8 = 7;

/// The passes room is made in: a second team, a club out of its first choice, one out of both
/// choices, one in its first choice whose second has room, one free to go anywhere.
const ROOM_PASSES: usize = 5;

/// A group no pass names.
const NOWHERE: usize = 99;

/// One level of the league's chain, top-down (`sr_pg_level`): a single division (one group) or
/// groups side by side, divisions `first` to `first + groups - 1`, each of `size` clubs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupLevel {
    /// Its first division.
    pub first: u8,
    /// Its groups.
    pub groups: u8,
    /// Each group's clubs.
    pub size: [u8; GROUPS],
}

/// A club that can play at a level of groups (`sr_pg_club`): its division base in its team file,
/// and its area byte at each level of groups, top-down: the low nibble its first choice of
/// group, the high its second plus one (0 or 15 none, 14 any group, plus 8 never another
/// group); 255 no area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupClub {
    /// Its division base.
    pub base: u8,
    /// Its area byte at each level of groups.
    pub area: [u8; PARALLEL],
}

/// A club's divisions before and after the game's moves, as offsets from its base, and whether
/// it enters the lowest level from below the chain this season end.
#[derive(Debug)]
pub struct GroupMoves<'a> {
    /// Each club's offset before the season end.
    pub old: &'a [i8],
    /// Each club's offset after the game's move: rewritten.
    pub new: &'a mut [i8],
    /// Each club's entering flag (non-zero: a slot taking a club from below the chain), none
    /// for none.
    pub entering: Option<&'a [u8]>,
}

/// Area byte `area`'s first choice.
fn first_choice(area: u8) -> usize {
    usize::from(area & NIBBLE)
}

/// Area byte `area`'s second choice (upstream `area2`): 15 none, 14 any group, else the group.
/// Faithful to 4.0.6's guard: a ban with no second choice is none.
fn second_choice(area: u8) -> usize {
    let high = area >> SECOND_SHIFT;
    usize::from(if high == 0 || high == NO_CHOICE {
        NO_CHOICE
    } else if high == ANY_GROUP {
        ANY_GROUP
    } else if high & CHOICE_GROUP == 0 {
        NO_CHOICE
    } else {
        (high & CHOICE_GROUP) - 1
    })
}

/// Whether area byte `area` bans another group (upstream `banned`).
fn banned(area: u8) -> bool {
    let high = area >> SECOND_SHIFT;
    high != 0 && high < ANY_GROUP && high & BAN != 0
}

/// The level of division `division` (upstream `level_of`): past the last level's groups, the
/// level below the chain (`levels.len()`); else the lowest level it is at or below; else the
/// top.
fn level_of(division: i32, levels: &[GroupLevel]) -> usize {
    let last = levels.last().expect("a level at least");
    if division >= i32::from(last.first) + i32::from(last.groups) {
        return levels.len();
    }
    levels
        .iter()
        .rposition(|level| division >= i32::from(level.first))
        .unwrap_or(0)
}

/// Group `group`, else the nearest one with room, the later one at equal distance (upstream
/// `with_room`); `group` itself when none has room.
fn with_room(group: usize, level: &GroupLevel, occupied: &[u8; GROUPS]) -> usize {
    let groups = usize::from(level.groups);
    for distance in 0..GROUPS {
        let later = group + distance;
        if later < groups && occupied[later] < level.size[later] {
            return later;
        }
        if let Some(earlier) = group.checked_sub(distance)
            && occupied[earlier] < level.size[earlier]
        {
            return earlier;
        }
    }
    group
}

/// The working state of a season end's placement.
struct Placing<'a> {
    /// The league's levels.
    levels: &'a [GroupLevel],
    /// Its clubs.
    clubs: &'a [GroupClub],
    /// Each club's offset after the moves.
    new: &'a mut [i8],
    /// Each level's groups' clubs placed so far.
    occupied: [[u8; GROUPS]; GROUP_LEVELS],
    /// The clubs still to place in a level of groups.
    pending: Vec<bool>,
}

impl Placing<'_> {
    /// Group `group` of level `level` is full: a club placed in it that can leave moves
    /// (upstream `make_room`), a second team first (to the group with room nearest `group`),
    /// else a club out of its first choice whose first choice has room, else one out of both
    /// choices whose second has room, else one in its first choice whose second has room,
    /// else a club free to go anywhere to the nearest group with room. Whether one moved.
    fn make_room(&mut self, me: usize, level: usize, group: usize, parallel: usize) -> bool {
        if parallel >= PARALLEL {
            return false;
        }
        let row = self.levels[level];
        let groups = usize::from(row.groups);
        for pass in 0..ROOM_PASSES {
            for (other, club) in self.clubs.iter().enumerate() {
                if other == me
                    || self.pending[other]
                    || i32::from(self.new[other]) + i32::from(club.base)
                        != i32::from(row.first) + i32::try_from(group).expect("9 groups")
                {
                    continue;
                }
                let area = club.area[parallel];
                let (first, second) = (first_choice(area), second_choice(area));
                let any = usize::from(ANY_GROUP);
                let to = if (pass == 0 && second == any)
                    || (pass == ROOM_PASSES - 1 && second < groups && !banned(area))
                {
                    with_room(group, &row, &self.occupied[level])
                } else if second != any {
                    match pass {
                        1 if first != group => first,
                        2 if first != group && second != group => second,
                        3 if first == group => second,
                        _ => NOWHERE,
                    }
                } else {
                    NOWHERE
                };
                if to == group || to >= groups || self.occupied[level][to] >= row.size[to] {
                    continue;
                }
                // Faithful to the original: byte counts, wrapping.
                self.occupied[level][to] = self.occupied[level][to].wrapping_add(1);
                self.occupied[level][group] = self.occupied[level][group].wrapping_sub(1);
                self.new[other] = offset(row.first, to, club.base);
                return true;
            }
        }
        false
    }
}

/// The offset from base `base` of group `group` of a level starting at `first`, as a signed
/// byte.
fn offset(first: u8, group: usize, base: u8) -> i8 {
    let division = i32::from(first) + i32::try_from(group).expect("9 groups") - i32::from(base);
    let [low, ..] = division.to_le_bytes();
    low.cast_signed()
}

/// The league's levels checked against the limits (upstream's own refusal): each level's index
/// among the levels of groups, none for a single division.
fn parallels(levels: &[GroupLevel], clubs: usize) -> Result<[u8; GROUP_LEVELS], RulesError> {
    if levels.is_empty() || levels.len() > GROUP_LEVELS || clubs > CLUB_SLOTS {
        return Err(RulesError::GroupsPastLimits);
    }
    let mut index = [NONE; GROUP_LEVELS];
    let mut parallel = 0;
    for (level, row) in levels.iter().enumerate() {
        if row.groups < 1 || usize::from(row.groups) > GROUPS {
            return Err(RulesError::GroupsPastLimits);
        }
        if row.groups > 1 {
            index[level] = parallel;
            parallel += 1;
        }
    }
    if usize::from(parallel) > PARALLEL {
        return Err(RulesError::GroupsPastLimits);
    }
    Ok(index)
}

/// The season end's placement of `clubs` in the league of `levels` (upstream `sr_pg_season`):
/// each club's move from `moves.old` to `moves.new` made a level's; into a level of groups,
/// placed by its choices and the room there, `moves.new` rewritten. Clubs off the chain, or
/// into the reserve below it, are left as moved.
///
/// # Errors
///
/// [`RulesError::GroupsPastLimits`] when the league is past the limits (upstream's refusal,
/// nothing changed); [`RulesError`] when a club has no offsets or entering flag, or stays above
/// the chain's first group (upstream reads or counts past them).
pub fn parallel_groups(
    levels: &[GroupLevel],
    clubs: &[GroupClub],
    moves: GroupMoves<'_>,
) -> Result<(), RulesError> {
    let parallel = parallels(levels, clubs.len())?;
    let count = clubs.len();
    if moves.old.len() < count
        || moves.new.len() < count
        || moves
            .entering
            .is_some_and(|entering| entering.len() < count)
    {
        return Err(RulesError::ClubsPastArrays(count));
    }
    let lowest = levels.last().expect("a level at least");
    let below_chain = i32::from(lowest.first) + i32::from(lowest.groups);
    let place_of = |club: usize, new: &[i8]| {
        let entering = moves.entering.is_some_and(|entering| entering[club] != 0);
        let base = i32::from(clubs[club].base);
        if entering {
            (below_chain, i32::from(lowest.first))
        } else {
            (
                i32::from(moves.old[club]) + base,
                i32::from(new[club]) + base,
            )
        }
    };
    let mut placing = Placing {
        levels,
        clubs,
        new: moves.new,
        occupied: [[0; GROUPS]; GROUP_LEVELS],
        pending: vec![false; count],
    };
    for (club, member) in clubs.iter().enumerate() {
        let (old, new) = place_of(club, placing.new);
        let from = level_of(old, levels);
        let to = match new.cmp(&old) {
            std::cmp::Ordering::Less => from.checked_sub(1),
            std::cmp::Ordering::Equal => Some(from),
            std::cmp::Ordering::Greater => Some(from + 1),
        };
        let Some(to) = to.filter(|&to| to < levels.len()) else {
            continue;
        };
        let row = levels[to];
        if to == from {
            let group = old - i32::from(row.first);
            if group < 0 {
                return Err(RulesError::ClubAboveChain(club));
            }
            if group < i32::from(row.groups) {
                let counted =
                    &mut placing.occupied[from][usize::try_from(group).expect("9 groups")];
                *counted = counted.wrapping_add(1);
            }
        } else if row.groups == 1 {
            placing.new[club] = offset(row.first, 0, member.base);
        } else {
            placing.pending[club] = true;
        }
    }
    for (club, member) in clubs.iter().enumerate() {
        if !placing.pending[club] {
            continue;
        }
        placing.pending[club] = false;
        let (old, new) = place_of(club, placing.new);
        let from = level_of(old, levels);
        let to = if new < old { from - 1 } else { from + 1 };
        let row = levels[to];
        let groups = usize::from(row.groups);
        let index = usize::from(parallel[to]);
        let area = member.area.get(index).copied().unwrap_or(NONE);
        let (first, second) = (first_choice(area), second_choice(area));
        let fallback = if first < groups {
            first
        } else if area == NONE {
            groups / 2
        } else {
            groups - 1
        };
        let mut placed = None;
        for choice in [first, second] {
            if choice >= groups {
                continue;
            }
            if placing.occupied[to][choice] < row.size[choice]
                || placing.make_room(club, to, choice, index)
            {
                placed = Some(choice);
                break;
            }
        }
        let group = placed.unwrap_or_else(|| with_room(fallback, &row, &placing.occupied[to]));
        placing.occupied[to][group] = placing.occupied[to][group].wrapping_add(1);
        placing.new[club] = offset(row.first, group, member.base);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A level of `groups` groups of `size` clubs from division `first`.
    fn level(first: u8, groups: u8, size: u8) -> GroupLevel {
        let mut sizes = [0; GROUPS];
        sizes[..usize::from(groups)].fill(size);
        GroupLevel {
            first,
            groups,
            size: sizes,
        }
    }

    /// A club of base 0 and area byte `area` at the one level of groups.
    fn club(area: u8) -> GroupClub {
        GroupClub {
            base: 0,
            area: [area, NONE, NONE, NONE],
        }
    }

    #[test]
    fn an_area_byte_reads_its_two_choices() {
        assert_eq!((first_choice(0x21), second_choice(0x21)), (1, 1));
        assert_eq!(second_choice(0x01), 15);
        assert_eq!(second_choice(0xE0), 14);
        assert_eq!(second_choice(0xF0), 15);
        // A ban: its group the low bits; a ban with none, none.
        assert_eq!((second_choice(0xA0), banned(0xA0)), (1, true));
        assert_eq!((second_choice(0x80), banned(0x80)), (15, true));
        assert!(!banned(0xE0));
    }

    #[test]
    fn a_relegated_club_goes_to_its_first_choice_else_its_second_else_the_nearest() {
        // Level 0 one division, level 1 two groups of 2 (divisions 1, 2). Clubs 0 and 1 stay
        // in group 0 (division 1); club 2 comes down from division 0 choosing group 0 then 1;
        // club 3 comes down choosing only group 0.
        let levels = [level(0, 1, 4), level(1, 2, 2)];
        let clubs = [club(0x00), club(0x00), club(0x20), club(0xF0)];
        let old = [1, 1, 0, 0];
        let mut new = [1, 1, 1, 1];
        parallel_groups(
            &levels,
            &clubs,
            GroupMoves {
                old: &old,
                new: &mut new,
                entering: None,
            },
        )
        .expect("placed");
        assert_eq!(new, [1, 1, 2, 2]);
    }

    #[test]
    fn room_is_made_by_moving_a_second_team() {
        // Group 0 full of a second team (any group) and a club; a club comes down choosing
        // only group 0: the second team moves to group 1.
        let levels = [level(0, 1, 4), level(1, 2, 2)];
        let clubs = [club(0xE0), club(0xF0), club(0xF0)];
        let old = [1, 1, 0];
        let mut new = [1, 1, 1];
        parallel_groups(
            &levels,
            &clubs,
            GroupMoves {
                old: &old,
                new: &mut new,
                entering: None,
            },
        )
        .expect("placed");
        assert_eq!(new, [2, 1, 1]);
    }

    #[test]
    fn a_club_entering_from_below_takes_the_lowest_level() {
        let levels = [level(0, 1, 4), level(1, 2, 2)];
        let clubs = [club(0x01)];
        let mut new = [5];
        parallel_groups(
            &levels,
            &clubs,
            GroupMoves {
                old: &[3],
                new: &mut new,
                entering: Some(&[1]),
            },
        )
        .expect("placed");
        assert_eq!(new, [2]);
    }

    #[test]
    fn a_league_past_the_limits_is_refused() {
        let mut new = [0];
        let moves = |new: &mut [i8; 1]| -> Result<(), RulesError> {
            let too_wide = GroupLevel {
                groups: 10,
                ..level(0, 1, 1)
            };
            parallel_groups(
                &[too_wide],
                &[club(0)],
                GroupMoves {
                    old: &[0],
                    new,
                    entering: None,
                },
            )
        };
        assert_eq!(moves(&mut new), Err(RulesError::GroupsPastLimits));
        assert_eq!(
            parallel_groups(
                &[level(2, 1, 4)],
                &[club(0)],
                GroupMoves {
                    old: &[0],
                    new: &mut [0],
                    entering: None,
                },
            ),
            Err(RulesError::ClubAboveChain(0))
        );
        assert_eq!(
            parallel_groups(
                &[],
                &[],
                GroupMoves {
                    old: &[],
                    new: &mut [],
                    entering: None
                }
            ),
            Err(RulesError::GroupsPastLimits)
        );
    }
}
