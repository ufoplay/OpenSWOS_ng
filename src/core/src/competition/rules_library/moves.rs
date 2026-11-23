//! The season's moves (upstream src/rules/sr_post.c `sr_final_moves`, `sr_b_teams`; the SWOS
//! 96/97 Mod's tools/gplay.c `sec_post`): each club's one-level move from its record's place,
//! the play-offs' moves on top, then the second teams' rules. A second team (a club's B team)
//! may not go up above the league's highest level for them, nor into or above its first team's
//! level after this season's moves: held, the next club of its division goes up instead; it
//! goes down when its first team comes into its level, the best placed relegated of its
//! division staying.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::league::{League, SecondTeam};
use super::table::{CLUBS, DIVISIONS, Levels, RulesError, Tables};

/// The moves the state holds (`SR_MOVES`).
pub const MOVES: usize = 48;

/// What made a play-off's move (`SR_RULE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveRule {
    /// The fixed tiers' brackets (`SR_RULE_ITALY`: dplay.c).
    FixedTiers,
    /// The pack's knockouts (`SR_RULE_KNOCKOUT`: gplay.c).
    Knockouts,
}

/// A rule's code.
const FIXED_TIERS: u8 = 1;

/// See [`FIXED_TIERS`].
const KNOCKOUTS: u8 = 2;

impl TryFrom<u8> for MoveRule {
    type Error = u8;

    fn try_from(code: u8) -> Result<Self, u8> {
        match code {
            FIXED_TIERS => Ok(Self::FixedTiers),
            KNOCKOUTS => Ok(Self::Knockouts),
            other => Err(other),
        }
    }
}

impl From<MoveRule> for u8 {
    fn from(rule: MoveRule) -> Self {
        match rule {
            MoveRule::FixedTiers => FIXED_TIERS,
            MoveRule::Knockouts => KNOCKOUTS,
        }
    }
}

/// A club's move a play-off made (`sr_move`): its global number, its division delta (one level:
/// -1 up, 1 down), the rule that made it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    /// The club's global number.
    pub glob: u16,
    /// Its division delta.
    pub delta: i8,
    /// The rule that made it.
    pub rule: MoveRule,
}

/// Each club's move, a level (`sr_state.want`, by division and place): -1 up, 1 down, 0 none.
/// The state's: what a season end does not write stays.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Wants(pub [[i8; CLUBS]; DIVISIONS]);

impl Default for Wants {
    fn default() -> Self {
        Self([[0; CLUBS]; DIVISIONS])
    }
}

/// `value`'s sign.
fn sign(value: i32) -> i8 {
    match value.cmp(&0) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// The league's rules checked against the tables' image: its divisions within its counts and
/// the tables.
fn checked(league: &League) -> Result<usize, RulesError> {
    let divisions = usize::from(league.divisions).min(DIVISIONS);
    if league.promoted.len() < divisions || league.relegated.len() < divisions {
        return Err(RulesError::CountsPastLeague(league.divisions));
    }
    Ok(divisions)
}

/// Each club's move into `wants` (upstream `sr_final_moves`): the top `promoted` of a division
/// but the first up, else the bottom `relegated` down; the play-offs' `moves` added; their
/// sign; then the second teams' rules ([`second_team_moves`], `base` the league's first global
/// number). The divisions are the league's, up to the tables' 16.
///
/// # Errors
///
/// [`RulesError`] when there are more moves than the state holds, or fewer counts by place
/// than the league's divisions.
pub fn final_moves(
    tables: &Tables,
    moves: &[Move],
    league: &League,
    levels: &Levels,
    base: u16,
    wants: &mut Wants,
) -> Result<(), RulesError> {
    if moves.len() > MOVES {
        return Err(RulesError::MovesPastState(moves.len()));
    }
    let divisions = checked(league)?;
    for division in 0..divisions {
        let clubs = tables.count(division);
        for place in 0..clubs {
            let glob = tables.row(division, place).glob;
            let promoted = usize::from(league.promoted[division]);
            let relegated = i32::from(league.relegated[division]);
            let mut want: i32 = if place < promoted && division > 0 {
                -1
            } else if i32::try_from(place).expect("32")
                >= i32::try_from(clubs).expect("32") - relegated
            {
                1
            } else {
                0
            };
            want += moves
                .iter()
                .filter(|played| played.glob == glob)
                .map(|played| i32::from(played.delta))
                .sum::<i32>();
            wants.0[division][place] = sign(want);
        }
    }
    second_team_moves(tables, wants, league, levels, base);
    Ok(())
}

/// The level club `glob` reaches after this season's moves, when it is in the tables.
fn level_after(tables: &Tables, wants: &Wants, levels: &Levels, glob: u16) -> Option<i32> {
    tables.find(glob).map(|(division, place)| {
        levels.level_of(i32::try_from(division).expect("16")) + i32::from(wants.0[division][place])
    })
}

/// Whether second team `team`, at division `division`, may not go up a level (upstream
/// `up_bad`): above the league's highest level for second teams, or into or above its first
/// team's level after this season's moves.
fn up_barred(
    tables: &Tables,
    wants: &Wants,
    (league, levels, base): (&League, &Levels, u16),
    team: SecondTeam,
    division: usize,
) -> bool {
    let target = levels.level_of(i32::try_from(division).expect("16")) - 1;
    if let Some(top) = league.second_teams.top
        && target < i32::from(top)
    {
        return true;
    }
    team.first
        .and_then(|first| level_after(tables, wants, levels, base.wrapping_add(u16::from(first))))
        .is_some_and(|first| target <= first)
}

/// Whether the club at `division`, `place` is passed over for a promotion place (upstream
/// `ineligible`): a second team that may not go up.
fn ineligible(
    tables: &Tables,
    wants: &Wants,
    rules: (&League, &Levels, u16),
    division: usize,
    place: usize,
) -> bool {
    let (league, _, base) = rules;
    let glob = tables.row(division, place).glob;
    league
        .second_teams
        .teams
        .iter()
        .find(|team| glob == base.wrapping_add(u16::from(team.team)))
        .is_some_and(|&team| {
            !league.second_teams.play_offs || up_barred(tables, wants, rules, team, division)
        })
}

/// The second teams' rules over the moves in `wants` (upstream `sr_b_teams`), `base` the
/// league's first global number: a second team coupled with its first team (that one in or
/// below its level after the moves, and it not going down) goes down, unless it is in the
/// lowest level and may not leave it, the best placed relegated of its division staying; one
/// that may not go up is held. Either way, its promotion passes to the next club of its
/// division that is not moving and may go up.
pub fn second_team_moves(
    tables: &Tables,
    wants: &mut Wants,
    league: &League,
    levels: &Levels,
    base: u16,
) {
    let rules = (league, levels, base);
    let lowest_level = levels.lowest(league.divisions);
    for &team in &league.second_teams.teams {
        let Some((division, place)) = tables.find(base.wrapping_add(u16::from(team.team))) else {
            continue;
        };
        let level = levels.level_of(i32::try_from(division).expect("16"));
        let moving = wants.0[division][place];
        let next = level + i32::from(moving);
        let first_next = team.first.and_then(|first| {
            level_after(tables, wants, levels, base.wrapping_add(u16::from(first)))
        });
        let lowest = level == lowest_level;
        let coupled = first_next.is_some_and(|first| first >= next.max(level) && next <= level);
        if coupled && (!lowest || league.second_teams.leave_lowest) {
            wants.0[division][place] = 1;
            if !lowest {
                let clubs = tables.count(division);
                if let Some(stays) =
                    (0..clubs).find(|&other| other != place && wants.0[division][other] > 0)
                {
                    wants.0[division][stays] = 0;
                }
            }
        } else if next < level && (coupled || up_barred(tables, wants, rules, team, division)) {
            wants.0[division][place] = 0;
        } else {
            continue;
        }
        if next < level {
            let clubs = tables.count(division);
            if let Some(up) = (0..clubs).find(|&other| {
                other != place
                    && wants.0[division][other] == 0
                    && !ineligible(tables, wants, rules, division, other)
            }) {
                wants.0[division][up] = moving;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::rules_library::league::SecondTeams;
    use crate::competition::rules_library::table::Row;

    /// A division of `count` clubs, global numbers from `first`.
    fn division(first: u16, count: u16) -> Vec<Row> {
        (first..first + count)
            .map(|glob| Row {
                glob,
                ..Row::default()
            })
            .collect()
    }

    /// Two divisions of 6: two up, two down.
    fn league() -> League {
        League {
            divisions: 2,
            promoted: vec![0, 2],
            relegated: vec![2, 2],
            ..League::default()
        }
    }

    #[test]
    fn a_club_moves_by_its_place_and_the_play_offs() {
        let tables = Tables::of(&[division(100, 6), division(200, 6)]).expect("tables");
        let mut wants = Wants::default();
        wants.0[2][0] = 7;
        let saved = Move {
            glob: 104,
            delta: -1,
            rule: MoveRule::Knockouts,
        };
        final_moves(
            &tables,
            &[saved],
            &league(),
            &Levels::default(),
            100,
            &mut wants,
        )
        .expect("moved");
        assert_eq!(&wants.0[0][..6], &[0, 0, 0, 0, 0, 1]);
        assert_eq!(&wants.0[1][..6], &[-1, -1, 0, 0, 1, 1]);
        assert_eq!(wants.0[2][0], 7, "past the league: the state's");
    }

    #[test]
    fn a_second_team_held_passes_its_promotion_on() {
        // Club 201 is the B team of 105, which goes down into its level: it goes down
        // instead of up, its place passing to 202; the best placed relegated, 204, stays.
        let tables =
            Tables::of(&[division(100, 6), division(200, 6), division(300, 6)]).expect("tables");
        let mut league = League {
            divisions: 3,
            promoted: vec![0, 2, 2],
            relegated: vec![2, 2, 2],
            ..League::default()
        };
        league.second_teams.teams = vec![SecondTeam {
            team: 101,
            first: Some(5),
        }];
        let mut wants = Wants::default();
        final_moves(&tables, &[], &league, &Levels::default(), 100, &mut wants).expect("moved");
        assert_eq!(&wants.0[1][..6], &[-1, 1, -1, 0, 0, 1]);
        // Not coupled but barred above level 1: held, 202 up instead.
        league.second_teams.teams[0].first = None;
        league.second_teams.top = Some(1);
        let mut wants = Wants::default();
        final_moves(&tables, &[], &league, &Levels::default(), 100, &mut wants).expect("moved");
        assert_eq!(&wants.0[1][..6], &[-1, 0, -1, 0, 1, 1]);
        // In the lowest division, unable to leave it: held instead of sent down.
        league.second_teams = SecondTeams {
            teams: vec![SecondTeam {
                team: 201,
                first: Some(105),
            }],
            ..SecondTeams::default()
        };
        let mut wants = Wants::default();
        final_moves(&tables, &[], &league, &Levels::default(), 100, &mut wants).expect("moved");
        assert_eq!(&wants.0[2][..6], &[-1, 0, -1, 0, 1, 1]);
    }

    #[test]
    fn what_the_state_cannot_hold_is_refused() {
        let tables = Tables::default();
        let played = Move {
            glob: 1,
            delta: 1,
            rule: MoveRule::FixedTiers,
        };
        assert_eq!(
            final_moves(
                &tables,
                &[played; MOVES + 1],
                &league(),
                &Levels::default(),
                0,
                &mut Wants::default()
            ),
            Err(RulesError::MovesPastState(49))
        );
        let short = League {
            promoted: vec![0],
            ..league()
        };
        assert_eq!(
            final_moves(
                &tables,
                &[],
                &short,
                &Levels::default(),
                0,
                &mut Wants::default()
            ),
            Err(RulesError::CountsPastLeague(2))
        );
        assert_eq!(MoveRule::try_from(2), Ok(MoveRule::Knockouts));
        assert_eq!(u8::from(MoveRule::FixedTiers), 1);
        assert_eq!(MoveRule::try_from(3), Err(3));
    }
}
