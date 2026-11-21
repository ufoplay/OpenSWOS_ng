//! Round robins (docs/PLAN.md M5 #1): the order a league, a tournament's group or a cup played
//! as a league plays its matches in. The world states them (`rules/fixtures.json`): each
//! size's, and those some competitions play instead of their size's. The engine has none of its
//! own (upstream `_fixtureSchedulePtrTable`, `_euroChampFixtureData`, diy_competition.c
//! `getCurrentFixtureTeamPtrs`).

use std::collections::BTreeMap;

use crate::world::CompetitionRef;

/// A match of a round robin: its two clubs, by their places among the clubs that play it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pair {
    /// The club at home.
    pub home: u8,
    /// The club away.
    pub away: u8,
}

/// A round robin: every pair of its clubs once, in the order they are played. A second cycle
/// plays the pairs again, each the other way round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    /// Its clubs.
    teams: u8,
    /// Whether every club plays at most once a match day.
    rounds: bool,
    /// Its pairs.
    pairs: Vec<Pair>,
}

impl Schedule {
    /// The round robin of `teams` clubs that plays `pairs`, real rounds if `rounds`. The world
    /// checks it before it makes one.
    pub(crate) fn new(teams: u8, rounds: bool, pairs: Vec<Pair>) -> Self {
        Self {
            teams,
            rounds,
            pairs,
        }
    }

    /// Its clubs.
    #[must_use]
    pub fn teams(&self) -> u8 {
        self.teams
    }

    /// Whether it plays in real rounds, every club at most once a match day of
    /// [`Schedule::match_day_pairs`] pairs: a career league of this size can be played
    /// (upstream `FixtureRounds_Real`; the original's odd tables are not, and its career
    /// league refuses them, panic 0x1313).
    #[must_use]
    pub fn rounds(&self) -> bool {
        self.rounds
    }

    /// Its pairs, in playing order.
    #[must_use]
    pub fn pairs(&self) -> &[Pair] {
        &self.pairs
    }

    /// The pairs a match day counts: half its clubs, rounded down (an odd number of clubs
    /// leaves one at rest).
    #[must_use]
    pub fn match_day_pairs(&self) -> usize {
        usize::from(self.teams / 2)
    }
}

/// The world's round robins: one of each size it plays, and those some competitions play
/// instead of their size's.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schedules {
    /// Each size's, by its clubs.
    sizes: BTreeMap<u8, Schedule>,
    /// The competitions' own.
    own: BTreeMap<CompetitionRef, Schedule>,
}

impl Schedules {
    /// The round robins `sizes`, and the competitions' `own`. The world checks them before it
    /// makes them.
    pub(crate) fn new(
        sizes: BTreeMap<u8, Schedule>,
        own: BTreeMap<CompetitionRef, Schedule>,
    ) -> Self {
        Self { sizes, own }
    }

    /// The round robin of `teams` clubs, if the world has one.
    #[must_use]
    pub fn of_size(&self, teams: u8) -> Option<&Schedule> {
        self.sizes.get(&teams)
    }

    /// The round robin `competition` plays with `teams` clubs (upstream
    /// `getCurrentFixtureTeamPtrs`): its own when the world gives it one, else its size's.
    /// Its own is of its groups' size: none for another number of clubs.
    #[must_use]
    pub fn of(&self, competition: CompetitionRef, teams: u8) -> Option<&Schedule> {
        match self.own.get(&competition) {
            Some(own) => (own.teams == teams).then_some(own),
            None => self.of_size(teams),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{CupRef, LeagueRef};

    fn pair(home: u8, away: u8) -> Pair {
        Pair { home, away }
    }

    fn schedules() -> Schedules {
        let three = Schedule::new(3, false, vec![pair(0, 1), pair(2, 0), pair(1, 2)]);
        let own = Schedule::new(2, true, vec![pair(1, 0)]);
        let two = Schedule::new(2, true, vec![pair(0, 1)]);
        Schedules::new(
            BTreeMap::from([(2, two), (3, three)]),
            BTreeMap::from([(CompetitionRef::Cup(CupRef(0)), own)]),
        )
    }

    #[test]
    fn a_schedule_is_its_pairs() {
        let schedules = schedules();
        let three = schedules.of_size(3).expect("three");
        assert_eq!(
            (three.teams(), three.rounds(), three.match_day_pairs()),
            (3, false, 1)
        );
        assert_eq!(three.pairs().get(1), Some(&pair(2, 0)));
        assert!(schedules.of_size(4).is_none());
    }

    #[test]
    fn a_competition_plays_its_own_else_its_size_s() {
        let schedules = schedules();
        let cup = CompetitionRef::Cup(CupRef(0));
        let league = CompetitionRef::League(LeagueRef(0));
        assert_eq!(
            schedules.of(cup, 2).map(Schedule::pairs),
            Some(&[pair(1, 0)][..]),
            "its own"
        );
        assert_eq!(
            schedules.of(league, 2).map(Schedule::pairs),
            Some(&[pair(0, 1)][..]),
            "its size's"
        );
        assert!(schedules.of(cup, 3).is_none(), "its own is of 2 clubs");
        assert!(schedules.of(league, 5).is_none(), "no size of 5");
    }
}
