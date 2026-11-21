//! League tables (docs/PLAN.md M5 #2): each team's line (upstream `DIYTeamStats`), a result
//! recorded into the lines, and a table's order (upstream diy_competition.c
//! `DIY_SortLeagueTableByStandings`). Every world orders an ordinary table so; a world's league
//! phase and split leagues order theirs by their own data (#9: `LeaguePhase_Sort`,
//! `LeagueSplit_Sort`).

/// The points of a draw, to each side (upstream diy_competition.c
/// `DIY_PlayOrSimulateLeagueFixture`: the competition's record states a win's, not a draw's).
const DRAW_POINTS: u16 = 1;

/// A team's line in a league table (upstream `DIYTeamStats`): its counts, 16-bit words that
/// wrap as the original's do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Line {
    /// Its games (`gamesPlayed`).
    pub played: u16,
    /// Its wins.
    pub won: u16,
    /// Its draws.
    pub drawn: u16,
    /// Its losses.
    pub lost: u16,
    /// The goals it scored.
    pub goals_for: u16,
    /// The goals it let in.
    pub goals_against: u16,
    /// Its points.
    pub points: u16,
}

impl Line {
    /// Its goal difference as the original reckons it: in signed 16-bit words, wrapping.
    #[must_use]
    pub fn goal_difference(&self) -> i16 {
        self.goals_for
            .cast_signed()
            .wrapping_sub(self.goals_against.cast_signed())
    }

    /// One more game (upstream `DIY_AdvanceLeagueProgressAfterMatch`, which counts it with
    /// the league's progress).
    pub fn count_game(&mut self) {
        self.played = self.played.wrapping_add(1);
    }
}

/// A match's score: each side's goals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Score {
    /// The goals of the side at home.
    pub home: u16,
    /// The goals of the side away.
    pub away: u16,
}

/// Records `score` into the lines of the match's sides, `home` and `away` (upstream
/// `DIY_PlayOrSimulateLeagueFixture`): a win, its side `points_for_win` (the competition's); a
/// draw, a point to each; the goals both ways. The games are counted apart, with the league's
/// progress ([`Line::count_game`]).
pub fn record(home: &mut Line, away: &mut Line, score: Score, points_for_win: u16) {
    if score.home == score.away {
        for line in [&mut *home, &mut *away] {
            line.drawn = line.drawn.wrapping_add(1);
            line.points = line.points.wrapping_add(DRAW_POINTS);
        }
    } else {
        let (winner, loser) = if score.home > score.away {
            (&mut *home, &mut *away)
        } else {
            (&mut *away, &mut *home)
        };
        winner.won = winner.won.wrapping_add(1);
        loser.lost = loser.lost.wrapping_add(1);
        winner.points = winner.points.wrapping_add(points_for_win);
    }
    home.goals_for = home.goals_for.wrapping_add(score.home);
    home.goals_against = home.goals_against.wrapping_add(score.away);
    away.goals_for = away.goals_for.wrapping_add(score.away);
    away.goals_against = away.goals_against.wrapping_add(score.home);
}

/// Orders each group of `places` (`per_group` places a group, each place a team's index into
/// `lines` and `names`), as upstream's table sort orders a group: each place in turn, from the
/// top, takes every line from the places below it that is above its own (by exchange, so
/// faithful to the original, not a stable sort: equal lines may swap). A line is above another
/// with more points; then a better goal difference; then more goals; then a name earlier byte
/// by byte, a name that ends first earlier. Places past the last whole group are left.
///
/// # Panics
///
/// When a place is no index of `lines` and `names`.
pub fn sort_groups(places: &mut [usize], per_group: usize, lines: &[Line], names: &[&[u8]]) {
    if per_group == 0 {
        return;
    }
    for group in places.chunks_exact_mut(per_group) {
        for holder in 0..group.len() {
            for challenger in holder + 1..group.len() {
                let (top, other) = (group[holder], group[challenger]);
                if above((&lines[other], names[other]), (&lines[top], names[top])) {
                    group.swap(holder, challenger);
                }
            }
        }
    }
}

/// Whether `challenger`'s line is above `holder`'s, each with its team's name.
fn above(challenger: (&Line, &[u8]), holder: (&Line, &[u8])) -> bool {
    let ((line, name), (other, other_name)) = (challenger, holder);
    if line.points != other.points {
        return line.points > other.points;
    }
    if line.goal_difference() != other.goal_difference() {
        return line.goal_difference() > other.goal_difference();
    }
    if line.goals_for != other.goals_for {
        return line.goals_for > other.goals_for;
    }
    // Faithful to the original: a name is its bytes up to the first 0, as upstream's string.
    until_nul(name) < until_nul(other_name)
}

/// `name` up to its first 0 byte.
fn until_nul(name: &[u8]) -> &[u8] {
    name.split(|&byte| byte == 0).next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three points a win, as most records state.
    const THREE: u16 = 3;

    fn line(points: u16, goals_for: u16, goals_against: u16) -> Line {
        Line {
            points,
            goals_for,
            goals_against,
            ..Line::default()
        }
    }

    #[test]
    fn a_result_goes_into_both_lines() {
        let (mut home, mut away) = (Line::default(), Line::default());
        record(&mut home, &mut away, Score { home: 2, away: 1 }, THREE);
        assert_eq!(
            (home.won, home.points, home.goals_for, home.goals_against),
            (1, 3, 2, 1)
        );
        assert_eq!((away.lost, away.points, away.goals_for), (1, 0, 1));
        record(&mut home, &mut away, Score { home: 0, away: 3 }, 2);
        assert_eq!(
            (home.lost, away.won, away.points),
            (1, 1, 2),
            "2 points a win"
        );
        record(&mut home, &mut away, Score { home: 1, away: 1 }, THREE);
        assert_eq!(
            (home.drawn, away.drawn, home.points, away.points),
            (1, 1, 4, 3)
        );
        assert_eq!((home.played, away.played), (0, 0), "counted apart");
        home.count_game();
        assert_eq!(home.played, 1);
    }

    #[test]
    fn the_counts_wrap_as_the_original_s() {
        let mut home = Line {
            points: u16::MAX,
            goals_for: u16::MAX,
            ..Line::default()
        };
        let mut away = Line::default();
        record(&mut home, &mut away, Score { home: 1, away: 0 }, THREE);
        assert_eq!((home.points, home.goals_for), (2, 0));
        // 40000 - 0 is -25536 in a signed word.
        assert_eq!(line(0, 40_000, 0).goal_difference(), -25_536);
        assert_eq!(line(0, 1, 3).goal_difference(), -2);
    }

    #[test]
    fn points_then_goal_difference_then_goals_then_names() {
        let lines = [
            line(3, 1, 1),
            line(4, 0, 9),
            line(3, 3, 1),
            line(3, 4, 2),
            line(3, 4, 2),
        ];
        let names: [&[u8]; 5] = [b"A", b"B", b"C", b"E", b"D"];
        let mut places = [0, 1, 2, 3, 4];
        sort_groups(&mut places, 5, &lines, &names);
        // B on points; D and E above C on goals, D before E by name; A last on difference.
        assert_eq!(places, [1, 4, 3, 2, 0]);
    }

    #[test]
    fn a_name_that_ends_first_is_earlier() {
        let lines = [Line::default(); 3];
        let names: [&[u8]; 3] = [b"ABC", b"AB", b"AB\0Z"];
        let mut places = [0, 1, 2];
        sort_groups(&mut places, 3, &lines, &names);
        assert_eq!(
            places,
            [1, 2, 0],
            "AB, then AB (its bytes after 0 ignored), then ABC"
        );
    }

    #[test]
    fn equal_lines_may_swap_as_the_exchange_moves_them() {
        // Two equal lines (same name) below a better one: the exchange leaves them reversed.
        let lines = [Line::default(), Line::default(), line(3, 0, 0)];
        let names: [&[u8]; 3] = [b"X", b"X", b"Y"];
        let mut places = [0, 1, 2];
        sort_groups(&mut places, 3, &lines, &names);
        assert_eq!(places, [2, 1, 0]);
    }

    #[test]
    fn each_group_is_ordered_alone() {
        let lines = [
            line(0, 0, 0),
            line(3, 0, 0),
            line(1, 0, 0),
            line(6, 0, 0),
            line(9, 0, 0),
        ];
        let names: [&[u8]; 5] = [b"A", b"B", b"C", b"D", b"E"];
        let mut places = [0, 1, 2, 3, 4];
        sort_groups(&mut places, 2, &lines, &names);
        assert_eq!(places, [1, 0, 3, 2, 4], "two groups of two, the fifth left");
        let mut untouched = [0, 1];
        sort_groups(&mut untouched, 0, &lines, &names);
        assert_eq!(untouched, [0, 1]);
    }
}
