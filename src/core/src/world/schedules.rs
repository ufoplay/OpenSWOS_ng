//! The world's round robins (`rules/fixtures.json`), checked: each a round robin of its size,
//! a size once, real rounds where it says so; the competitions that play their own named, each
//! once, of their groups' size. Whether every division has the round robin it plays joins with
//! the leagues (M5): a World 2026-27 league in zones plays its own pairs (upstream
//! league_split.c: the MLS's and Argentina's 30 clubs, two zones each).

use std::collections::{BTreeMap, BTreeSet};

use openswos_competitions_json_resource::CompetitionType;
use openswos_world_rules_resource::{Fixtures, Schedule as ScheduleFile};

use super::competitions::{CompetitionRef, Cup, League, Resolver, Tournament};
use super::rules::at;
use crate::competition::schedule::{Pair, Schedule, Schedules};
use crate::validation::{Field, Location, Problem, Problems};

/// The fewest clubs a round robin takes.
const MIN_CLUBS: u8 = 2;

/// The records of a world's competitions, which state the sizes they play.
pub(super) struct Records<'a> {
    /// Its leagues: each division's clubs.
    pub(super) leagues: &'a [League],
    /// Its cups: which play as leagues.
    pub(super) cups: &'a [Cup],
    /// Its tournaments: each group stage's groups' clubs.
    pub(super) tournaments: &'a [Tournament],
}

/// The round robins of `fixtures`, or `None` with the problems reported.
pub(super) fn schedules(
    fixtures: &Fixtures,
    records: &Records<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<Schedules> {
    let sizes = sizes(fixtures, resolver.problems);
    let own = own(fixtures, records, resolver);
    Some(Schedules::new(sizes?, own?))
}

/// Each size's round robin, a size once.
fn sizes(fixtures: &Fixtures, problems: &mut Problems) -> Option<BTreeMap<u8, Schedule>> {
    let field = Field::key("schedules");
    let mut sizes = BTreeMap::new();
    let mut given_at = BTreeMap::new();
    let mut valid = true;
    for (index, file) in fixtures.schedules.iter().enumerate() {
        let field = field.index(index);
        let teams_at = at::<Fixtures>(&field.then("teams"));
        if let Some(first) = given_at.insert(file.teams, teams_at.clone()) {
            problems.push(
                teams_at,
                Problem::RepeatedSchedule {
                    teams: file.teams,
                    first,
                },
            );
            valid = false;
        }
        match schedule(file, &field, problems) {
            Some(schedule) => {
                sizes.insert(file.teams, schedule);
            }
            None => valid = false,
        }
    }
    valid.then_some(sizes)
}

/// The competitions' own round robins: each competition once, of its groups' size.
fn own(
    fixtures: &Fixtures,
    records: &Records<'_>,
    resolver: &mut Resolver<'_>,
) -> Option<BTreeMap<CompetitionRef, Schedule>> {
    let mut own = BTreeMap::new();
    let mut named_at = BTreeMap::new();
    let mut valid = true;
    for (index, file) in fixtures.competition_schedules.iter().enumerate() {
        let field = Field::key("competition_schedules").index(index);
        let schedule = schedule(&file.schedule, &field.then("schedule"), resolver.problems);
        for (named, id) in file.competitions.iter().enumerate() {
            let location = at::<Fixtures>(&field.then("competitions").index(named));
            let Some(competition) = resolver.competition(id, || location.clone()) else {
                valid = false;
                continue;
            };
            if let Some(first) = named_at.insert(competition, location.clone()) {
                resolver
                    .problems
                    .push(location, Problem::RepeatedCompetition { first });
                valid = false;
                continue;
            }
            let teams = file.schedule.teams;
            let other_size = match plays(competition, records) {
                Plays::Knockout => {
                    resolver.problems.push(location, Problem::NoRoundRobin);
                    valid = false;
                    continue;
                }
                Plays::Sizes(stated) => stated.into_iter().find(|&size| size != teams),
                Plays::Drawn => None,
            };
            if let Some(size) = other_size {
                resolver
                    .problems
                    .push(location, Problem::ScheduleSize { teams, size });
                valid = false;
            } else if let Some(schedule) = &schedule {
                own.insert(competition, schedule.clone());
            }
        }
        valid &= schedule.is_some();
    }
    valid.then_some(own)
}

/// `file` checked, at `field` of `fixtures.json`: two clubs or more, every pair of them once,
/// real rounds where it says so.
fn schedule(file: &ScheduleFile, field: &Field, problems: &mut Problems) -> Option<Schedule> {
    let teams = file.teams;
    if teams < MIN_CLUBS {
        problems.push(
            at::<Fixtures>(&field.then("teams")),
            Problem::TooFewClubs(teams),
        );
        return None;
    }
    let pairs_field = field.then("pairs");
    let mut first_at: BTreeMap<(u8, u8), Location> = BTreeMap::new();
    let mut valid = true;
    for (index, &[home, away]) in file.pairs.iter().enumerate() {
        let location = at::<Fixtures>(&pairs_field.index(index));
        if home == away || home.max(away) >= teams {
            problems.push(location, Problem::NoPair { home, away, teams });
            valid = false;
        } else if let Some(first) = first_at.get(&(home.min(away), home.max(away))) {
            let first = first.clone();
            problems.push(location, Problem::RepeatedPair { home, away, first });
            valid = false;
        } else {
            first_at.insert((home.min(away), home.max(away)), location);
        }
    }
    let all = usize::from(teams) * usize::from(teams - 1) / 2;
    if valid && first_at.len() < all {
        problems.push(
            at::<Fixtures>(&pairs_field),
            Problem::MissingPairs {
                teams,
                missing: all - first_at.len(),
            },
        );
        valid = false;
    }
    let pairs: Vec<Pair> = file
        .pairs
        .iter()
        .map(|&[home, away]| Pair { home, away })
        .collect();
    let schedule = Schedule::new(teams, file.rounds, pairs);
    if valid
        && schedule.rounds()
        && let Some((day, club)) = twice_in_a_day(&schedule)
    {
        problems.push(
            at::<Fixtures>(&field.then("rounds")),
            Problem::NotRounds { day, club },
        );
        valid = false;
    }
    valid.then_some(schedule)
}

/// The first match day (from 1) of `schedule` with a club twice, and the club.
fn twice_in_a_day(schedule: &Schedule) -> Option<(usize, u8)> {
    schedule
        .pairs()
        .chunks(schedule.match_day_pairs())
        .enumerate()
        .find_map(|(day, pairs)| {
            let mut seen = BTreeSet::new();
            pairs
                .iter()
                .flat_map(|pair| [pair.home, pair.away])
                .find(|&club| !seen.insert(club))
                .map(|club| (day + 1, club))
        })
}

/// The round robins a competition plays.
enum Plays {
    /// Of the sizes its record states: a league's divisions, a tournament's group stages. A
    /// group of fewer than two clubs plays none (a tournament's stages past its final hold its
    /// winner, one group of one club).
    Sizes(Vec<u8>),
    /// Of sizes set when its groups are drawn: a cup played as a league (the qualifying
    /// groups, friendlies and tours `cups.json` holds; the record's `teams` is no group's).
    Drawn,
    /// None: a knockout.
    Knockout,
}

/// The round robins `competition` plays, by its record.
fn plays(competition: CompetitionRef, records: &Records<'_>) -> Plays {
    let stated = |sizes: Vec<u8>| {
        Plays::Sizes(
            sizes
                .into_iter()
                .filter(|&size| size >= MIN_CLUBS)
                .collect(),
        )
    };
    match competition {
        CompetitionRef::League(league) => stated(
            records.leagues[league.0]
                .record
                .divisions
                .iter()
                .map(|division| division.teams.byte())
                .collect(),
        ),
        CompetitionRef::Cup(cup) => match records.cups[cup.0].record.comp.kind {
            CompetitionType::League => Plays::Drawn,
            CompetitionType::Cup | CompetitionType::Tournament => Plays::Knockout,
        },
        CompetitionRef::Tournament(tournament) => stated(
            records.tournaments[tournament.0]
                .record
                .rounds
                .iter()
                .filter(|round| round.group_count.byte() != 0)
                .map(|round| round.team_group_count.byte())
                .collect(),
        ),
        CompetitionRef::Playoff(_) => Plays::Knockout,
    }
}

#[cfg(test)]
mod tests {
    use openswos_competitions_json_resource::SymbolId;
    use openswos_world_rules_resource::CompetitionSchedule;

    use crate::World;
    use crate::fixture::{fixtures, package};

    use super::*;

    /// The problems of the small world with its `fixtures.json` changed by `change`.
    fn problems(change: impl FnOnce(&mut Fixtures)) -> String {
        let mut package = package();
        let mut fixtures = fixtures();
        change(&mut fixtures);
        package.rules.fixtures = Some(fixtures);
        World::from_package(&package)
            .expect_err("invalid")
            .to_string()
    }

    fn schedule(teams: u8, rounds: bool, pairs: Vec<[u8; 2]>) -> ScheduleFile {
        ScheduleFile {
            teams,
            rounds,
            pairs,
        }
    }

    #[test]
    fn a_round_robin_is_every_pair_once() {
        assert_eq!(
            problems(|fixtures| fixtures.schedules.push(schedule(1, false, Vec::new()))),
            "rules/fixtures.json: schedules[2].teams: a round robin of 1 clubs: it takes two or more"
        );
        assert_eq!(
            problems(|fixtures| fixtures.schedules[1].pairs = vec![[1, 1], [0, 3], [0, 1]]),
            "rules/fixtures.json: schedules[1].pairs[0]: [1, 1] is no pair of 3 clubs\n\
             rules/fixtures.json: schedules[1].pairs[1]: [0, 3] is no pair of 3 clubs"
        );
        assert_eq!(
            problems(|fixtures| fixtures.schedules[1].pairs = vec![[1, 2], [2, 1], [0, 1]]),
            "rules/fixtures.json: schedules[1].pairs[1]: [2, 1] is played again (first at \
             rules/fixtures.json: schedules[1].pairs[0])"
        );
        assert_eq!(
            problems(|fixtures| fixtures.schedules[1]
                .pairs
                .pop()
                .map(drop)
                .unwrap_or_default()),
            "rules/fixtures.json: schedules[1].pairs: 1 pairs of 3 clubs are not played"
        );
        assert_eq!(
            problems(|fixtures| fixtures.schedules.push(schedule(2, true, vec![[0, 1]]))),
            "rules/fixtures.json: schedules[2].teams: a round robin of 2 clubs again (first at \
             rules/fixtures.json: schedules[0].teams)"
        );
    }

    #[test]
    fn real_rounds_have_every_club_once_a_day() {
        let four = |rounds| {
            schedule(
                4,
                rounds,
                vec![[0, 1], [1, 2], [2, 3], [3, 0], [0, 2], [1, 3]],
            )
        };
        assert_eq!(
            problems(|fixtures| fixtures.schedules.push(four(true))),
            "rules/fixtures.json: schedules[2].rounds: match day 1 has club 1 twice: no real rounds"
        );
        let mut package = package();
        let mut fixtures = fixtures();
        fixtures.schedules.push(four(false));
        package.rules.fixtures = Some(fixtures);
        let world = World::from_package(&package).expect("not said to be rounds");
        assert!(
            world
                .schedules()
                .of_size(4)
                .is_some_and(|four| !four.rounds())
        );
    }

    #[test]
    fn a_competition_s_own_round_robin_is_of_its_size() {
        let named = |id: &str, teams: u8| CompetitionSchedule {
            competitions: vec![SymbolId(id.to_owned())],
            schedule: if teams == 2 {
                schedule(2, true, vec![[0, 1]])
            } else {
                schedule(3, false, vec![[0, 1], [1, 2], [2, 0]])
            },
        };
        assert_eq!(
            problems(|fixtures| fixtures.competition_schedules.push(named("_nowhere", 2))),
            "rules/fixtures.json: competition_schedules[1].competitions[0]: no league, cup, \
             tournament or play-off is called _nowhere"
        );
        assert_eq!(
            problems(|fixtures| fixtures
                .competition_schedules
                .push(named("_albanianCup", 2))),
            "rules/fixtures.json: competition_schedules[1].competitions[0]: the competition is a \
             knockout: it plays no round robin"
        );
        assert_eq!(
            problems(|fixtures| fixtures
                .competition_schedules
                .push(named("_albanianLeague", 3))),
            "rules/fixtures.json: competition_schedules[1].competitions[0]: a round robin of 3 \
             clubs, where the competition plays 2"
        );
        // The friendly's groups are drawn: its own round robin of any size.
        let mut package = package();
        let mut fixtures = fixtures();
        fixtures.competition_schedules[0] = named("_friendly", 3);
        package.rules.fixtures = Some(fixtures);
        let world = World::from_package(&package).expect("a drawn size");
        let friendly = world
            .competition(&SymbolId("_friendly".to_owned()))
            .expect("the friendly");
        assert!(world.schedules().of(friendly, 3).is_some());
        assert_eq!(
            problems(|fixtures| fixtures.competition_schedules.push(named("_friendly", 2))),
            "rules/fixtures.json: competition_schedules[1].competitions[0]: the competition has a \
             round robin of its own already (at rules/fixtures.json: \
             competition_schedules[0].competitions[0])"
        );
    }
}
