//! A league's final tables as the rules library reads them, its levels, and why the library
//! refuses an input (upstream src/rules/swosrules.h `sr_row`, `sr_tables`, `sr_levels`).
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use thiserror::Error;

use super::check::CheckError;

/// The divisions a league's tables hold (`SR_DIVS`).
pub const DIVISIONS: usize = 16;

/// The clubs a division's table holds (`SR_CLUBS`).
pub const CLUBS: usize = 32;

/// A club in a final table (`sr_row`): its team word (its team file and ordinal, laid out as
/// the game lays them), its global number (the league table's index), its record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Row {
    /// The team word.
    pub team: u16,
    /// The global number.
    pub glob: u16,
    /// The matches won.
    pub won: u16,
    /// The goals scored.
    pub goals_for: u16,
    /// The goals conceded.
    pub goals_against: u16,
    /// The points.
    pub points: u16,
}

/// A league's final tables (`sr_tables`): how many divisions it has, each division's count of
/// clubs and its rows in place order (row 0 the top). The image is whole, as the caller's
/// state holds it: the counts and rows past the divisions in use are read where the C reads
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tables {
    /// The divisions in use (`nd`).
    divisions: u8,
    /// Each division's clubs (`n`).
    counts: [u8; DIVISIONS],
    /// Each division's rows (`t`).
    rows: [[Row; CLUBS]; DIVISIONS],
}

impl Default for Tables {
    fn default() -> Self {
        Self {
            divisions: 0,
            counts: [0; DIVISIONS],
            rows: [[Row::default(); CLUBS]; DIVISIONS],
        }
    }
}

impl Tables {
    /// The tables of `divisions` divisions, each division's `counts` clubs in its `rows`.
    ///
    /// # Errors
    ///
    /// [`RulesError`] when more divisions or clubs are stated than the tables hold (upstream
    /// reads past them).
    pub fn new(
        divisions: u8,
        counts: [u8; DIVISIONS],
        rows: [[Row; CLUBS]; DIVISIONS],
    ) -> Result<Self, RulesError> {
        if usize::from(divisions) > DIVISIONS {
            return Err(RulesError::DivisionsPastTables(divisions));
        }
        if let Some((division, &clubs)) = counts
            .iter()
            .enumerate()
            .find(|&(_, &clubs)| usize::from(clubs) > CLUBS)
        {
            return Err(RulesError::ClubsPastTable { division, clubs });
        }
        Ok(Self {
            divisions,
            counts,
            rows,
        })
    }

    /// The tables of `tables`, a division's rows each, best first.
    ///
    /// # Errors
    ///
    /// [`RulesError`] when there are more divisions or clubs than the tables hold.
    pub fn of(tables: &[Vec<Row>]) -> Result<Self, RulesError> {
        let divisions =
            u8::try_from(tables.len()).map_err(|_| RulesError::DivisionsPastTables(u8::MAX))?;
        if usize::from(divisions) > DIVISIONS {
            return Err(RulesError::DivisionsPastTables(divisions));
        }
        let mut counts = [0; DIVISIONS];
        let mut rows = [[Row::default(); CLUBS]; DIVISIONS];
        for (division, table) in tables.iter().enumerate() {
            let clubs = u8::try_from(table.len()).unwrap_or(u8::MAX);
            if table.len() > CLUBS {
                return Err(RulesError::ClubsPastTable { division, clubs });
            }
            counts[division] = clubs;
            rows[division][..table.len()].copy_from_slice(table);
        }
        Self::new(divisions, counts, rows)
    }

    /// The divisions in use.
    #[must_use]
    pub fn divisions(&self) -> usize {
        usize::from(self.divisions)
    }

    /// Division `division`'s clubs (any of the 16: past the divisions in use, the image's).
    #[must_use]
    pub fn count(&self, division: usize) -> usize {
        usize::from(self.counts[division])
    }

    /// Division `division`'s row of place `place`.
    #[must_use]
    pub fn row(&self, division: usize, place: usize) -> Row {
        self.rows[division][place]
    }

    /// Division `division`'s table: its clubs' rows, best first.
    #[must_use]
    pub fn table(&self, division: usize) -> &[Row] {
        &self.rows[division][..self.count(division)]
    }

    /// Where the club of global number `glob` stands (upstream `find_club`): the first place of
    /// the divisions in use that holds it.
    #[must_use]
    pub fn find(&self, glob: u16) -> Option<(usize, usize)> {
        (0..self.divisions()).find_map(|division| {
            self.table(division)
                .iter()
                .position(|row| row.glob == glob)
                .map(|place| (division, place))
        })
    }
}

/// A league's levels when some are groups of parallel divisions (`sr_levels`): where each
/// level starts. None: a level per division.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Levels {
    /// Each level's first division, top-down.
    pub first: Vec<u8>,
}

impl Levels {
    /// The level of division `division` (upstream `level_of`): the lowest level whose first
    /// division it is at or below, else the top; the division itself with no levels.
    #[must_use]
    pub fn level_of(&self, division: i32) -> i32 {
        if self.first.is_empty() {
            return division;
        }
        self.first
            .iter()
            .rposition(|&first| division >= i32::from(first))
            .map_or(0, |level| i32::try_from(level).expect("a few levels"))
    }

    /// The lowest level, a league of `divisions` divisions.
    #[must_use]
    pub fn lowest(&self, divisions: u8) -> i32 {
        if self.first.is_empty() {
            i32::from(divisions) - 1
        } else {
            i32::try_from(self.first.len()).expect("a few levels") - 1
        }
    }
}

/// Why the rules library refuses an input: upstream's own refusals, and the inputs where the
/// C reads or writes past its memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RulesError {
    /// More divisions than the tables hold (upstream reads past them).
    #[error("{0} divisions, more than the tables' 16")]
    DivisionsPastTables(u8),
    /// More clubs in a division than its table holds (upstream reads past it).
    #[error("division {division}'s {clubs} clubs, more than a table's 32")]
    ClubsPastTable {
        /// The division.
        division: usize,
        /// Its clubs.
        clubs: u8,
    },
    /// More moves than the state holds (upstream reads past them).
    #[error("{0} moves, more than the state's 48")]
    MovesPastState(usize),
    /// A league whose promoted or relegated counts are fewer than its divisions (upstream
    /// reads past them).
    #[error("the league's {0} divisions, more than its counts by place")]
    CountsPastLeague(u8),
    /// Fewer divisions, offsets or entering flags than clubs (upstream reads past them).
    #[error("{0} clubs, more than their divisions or offsets")]
    ClubsPastArrays(usize),
    /// A club's slot past the places (upstream reads past them).
    #[error("slot {0} past the places")]
    SlotPastPlaces(u16),
    /// A slot past the team words (upstream reads past them).
    #[error("slot {0} past the team words")]
    SlotPastWords(u16),
    /// A cup's division past the levels (upstream reads past them).
    #[error("division {0} past the levels")]
    DivisionPastLevels(u8),
    /// A league past the parallel groups' limits: up to 8 levels, 9 groups a level, 4 levels
    /// of groups, 512 clubs (upstream refuses it, nothing changed).
    #[error("a league past the parallel groups' limits")]
    GroupsPastLimits,
    /// A club above the league's chain (upstream counts it before its first group).
    #[error("club {0} is above the league's chain")]
    ClubAboveChain(usize),
    /// A cup's rounds outside 2 to 8, or its divisions past the league's (upstream refuses it,
    /// nothing made).
    #[error("a cup of {rounds} rounds over divisions {first} to {first}+{groups}")]
    CupPastLimits {
        /// Its rounds.
        rounds: u8,
        /// Its first division.
        first: u8,
        /// Its groups.
        groups: u8,
    },
    /// A preliminary round with more sides than the cup's clubs (upstream reads entries it
    /// never wrote).
    #[error("a preliminary round of {0} ties, more sides than clubs")]
    PreliminaryPastClubs(usize),
    /// A league's rules that are not sound (upstream's refusal, nothing played).
    #[error("the league's rules: {} at byte {} ({:?})", .0.field, .0.offset, .0.code)]
    Check(CheckError),
    /// A lower tier of more groups than its national rounds' seeds hold (upstream writes past
    /// its tables of 5).
    #[error("a lower tier of {0} groups, past its national rounds' tables")]
    GroupsPastBracket(u8),
    /// A stage title past a division's three (upstream reads past its stage titles).
    #[error("stage title {0}, past a division's three")]
    StagePastTitles(i32),
    /// A league with fewer second teams' words than its second teams (upstream reads past
    /// them).
    #[error("{0} second teams, more than their words")]
    SecondTeamsPastWords(usize),
    /// A knockout round paired best v worst over an odd count (upstream pairs its last club
    /// with one it never wrote).
    #[error("a best v worst pairing of {0} clubs, an odd count")]
    OddPairing(usize),
    /// An absent entrant moved (upstream moves the row before its tables).
    #[error("an absent entrant moved")]
    AbsentMoved,
    /// A final group's placing past the 64 (upstream reads past its placings).
    #[error("placing {0}, past a play-off's 64")]
    PlacingPastGroups(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row of global number `glob`.
    fn row(glob: u16) -> Row {
        Row {
            glob,
            ..Row::default()
        }
    }

    #[test]
    fn a_club_is_found_in_the_divisions_in_use() {
        let tables = Tables::of(&[vec![row(5), row(6)], vec![row(7)]]).expect("tables");
        assert_eq!(tables.find(7), Some((1, 0)));
        assert_eq!(tables.find(9), None);
        assert_eq!(
            (tables.divisions(), tables.count(0), tables.table(1)),
            (2, 2, &[row(7)][..])
        );
    }

    #[test]
    fn tables_past_their_image_are_refused() {
        let rows = [[Row::default(); CLUBS]; DIVISIONS];
        assert_eq!(
            Tables::new(17, [0; DIVISIONS], rows),
            Err(RulesError::DivisionsPastTables(17))
        );
        let mut counts = [0; DIVISIONS];
        counts[15] = 33;
        assert_eq!(
            Tables::new(1, counts, rows),
            Err(RulesError::ClubsPastTable {
                division: 15,
                clubs: 33
            })
        );
        assert!(Tables::of(&[vec![Row::default(); 33]]).is_err());
    }

    #[test]
    fn a_level_is_the_lowest_one_a_division_reaches() {
        let levels = Levels {
            first: vec![0, 1, 2, 5],
        };
        assert_eq!(
            [0, 1, 2, 4, 5, 9].map(|division| levels.level_of(division)),
            [0, 1, 2, 2, 3, 3]
        );
        assert_eq!(levels.level_of(-1), 0);
        assert_eq!(Levels::default().level_of(7), 7);
        assert_eq!((levels.lowest(9), Levels::default().lowest(9)), (3, 8));
    }
}
