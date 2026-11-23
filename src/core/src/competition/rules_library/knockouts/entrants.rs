//! A play-off's entrants as clubs of the tables (upstream src/rules/sr_gplay.c `ent_code`,
//! `resolve_entrants`, and what they read): by place (second teams that may not go up passed
//! over, but in a play-out), a stage table's place, the stage titles' finalists, two title
//! deciders' seeds, the previous play-off's loser, a club by team word.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::play::{ABSENT, Club, NONE, Play, club_at, place, tier};
use super::{ENTRANT, ENTRANTS, HEADER, TITLES};
use crate::competition::rules_library::season::{KnockoutResult, NO_CLUB};
use crate::competition::rules_library::table::{DIVISIONS, RulesError};

/// The entrant types: by place, from a stage table, the previous play-off's loser, a stage
/// title's finalist, a kept stage's title decider's seed, a national title play-off's seed, a
/// club by team word.
pub(super) const BY_PLACE: u8 = 0;
/// See [`BY_PLACE`].
const STAGE: u8 = 3;
/// See [`BY_PLACE`].
const PREVIOUS_LOSER: u8 = 4;
/// See [`BY_PLACE`].
const FINALIST: u8 = 5;
/// See [`BY_PLACE`].
const SPLIT_SEED: u8 = 6;
/// See [`BY_PLACE`].
const TITLE_SEED: u8 = 7;
/// See [`BY_PLACE`].
pub(super) const BY_TEAM: u8 = 8;

/// The play-out's kind: its entrants by place, second teams and all.
pub(super) const PLAY_OUT: u8 = 4;

/// The kept tables (`GS`): the frozen one and a second, never filled.
const KEPT_TABLES: i32 = 2;

/// The first of the bank's tables as a stage number (`SS_BASE`), and how many.
const BANK_FIRST: i32 = 4;
/// See [`BANK_FIRST`].
const BANK_TABLES: i32 = 4;

/// The bank's tables a title decider reads: the opening, the closing, the annual, the
/// alternative closing (stage numbers).
const OPENING: i32 = 4;
/// See [`OPENING`].
const CLOSING: i32 = 5;
/// See [`OPENING`].
const ANNUAL: i32 = 6;
/// See [`OPENING`].
const ALTERNATIVE: i32 = 7;

/// The stage titles a division keeps.
pub(super) const STAGE_TITLES: i32 = 3;

/// The seeds of a title decider (1 to 4).
const SEEDS: i32 = 4;

/// The second seed, place or finalist.
const SECOND: i32 = 2;

/// The annual table's places a title seed is taken from (the top 8), and the top 2.
const TOP_EIGHT: i32 = 8;
/// See [`TOP_EIGHT`].
const TOP_TWO: i32 = 2;

/// The bits of a stage title's number that name its stage; its division the rest.
pub(super) const STAGE_BITS: i32 = 3;
/// How far a stage title's division is shifted.
pub(super) const DIVISION_SHIFT: i32 = 2;

/// How far a team word's file is shifted in it.
const FILE_SHIFT: u16 = 8;

/// A stage title emptied: no kind, no clubs (its division as it was).
pub(super) fn emptied(title: &mut KnockoutResult) {
    title.kind = None;
    title.team = [NO_CLUB; 2];
    title.glob = [NO_CLUB; 2];
}

/// The levels of the season's league (upstream `lv_of`: `sr_post.c`'s `level_of`).
fn level_of(play: &Play<'_, '_>, division: i32) -> i32 {
    play.season
        .levels
        .map_or(division, |levels| levels.level_of(division))
}

impl Play<'_, '_> {
    /// Place `at` (from 1) of stage table `stage` as division `division`'s club (upstream
    /// `stage_club`): a bank's table (4 to 7) this season's, else the division's final table; a
    /// kept table (0, 1); none past them.
    pub(super) fn stage_club(&mut self, division: i32, stage: i32, at: i32) -> Club {
        let glob = if (BANK_FIRST..BANK_FIRST + BANK_TABLES).contains(&stage) {
            let table = stage - BANK_FIRST;
            self.stages_used = true;
            let bank = self.bank();
            if bank.fresh & 1 << table == 0 {
                if division >= self.divisions() || at < 1 || at > self.count(division) {
                    return NONE;
                }
                return club_at(division, at - 1);
            }
            let table = usize::try_from(table).expect("4 tables");
            if at < 1 || at > i32::from(bank.counts[table]) || division >= self.divisions() {
                return NONE;
            }
            bank.tables[table][usize::try_from(at - 1).expect("a place")].glob
        } else {
            let kept = if stage == 0 { self.kept.len() } else { 0 };
            let kept = i32::try_from(kept).expect("32 clubs");
            if stage >= KEPT_TABLES || at < 1 || at > kept || division >= self.divisions() {
                return NONE;
            }
            self.kept[usize::try_from(at - 1).expect("a place")].glob
        };
        (0..self.count(division))
            .find(|&place| self.row_at(division, place).glob == glob)
            .map_or(NONE, |place| club_at(division, place))
    }

    /// The club of global number `glob` in division `division` (upstream `glob_club`), none for
    /// 65535 or a division past the tables.
    pub(super) fn glob_club(&self, division: i32, glob: u16) -> Club {
        if glob == NO_CLUB || division >= self.divisions() {
            return NONE;
        }
        (0..self.count(division))
            .find(|&at| self.row_at(division, at).glob == glob)
            .map_or(NONE, |at| club_at(division, at))
    }

    /// Division `division`'s stage titles (upstream `sres_of`): the career club's division's
    /// (or the top one's without it), else another's, emptied when the division changes.
    pub(super) fn titles(&mut self, division: i32) -> &mut [KnockoutResult; TITLES] {
        let manager = self.manager_division();
        if division == manager || (manager < 0 && division == 0) {
            return &mut self.state.stage_titles;
        }
        if self.state.other_division != division {
            for title in &mut self.state.other_titles {
                emptied(title);
            }
            self.state.other_division = division;
        }
        &mut self.state.other_titles
    }

    /// The stage title `number` (its stage in the low bits, its division above) decided
    /// mid-season (upstream `stage_dec`): the career club's division's, as the host says.
    pub(super) fn stage_decided(&mut self, number: i32) -> Option<([u16; 2], [u16; 2])> {
        if number >> DIVISION_SHIFT != self.manager_division() {
            return None;
        }
        self.season
            .host
            .as_deref_mut()
            .and_then(|host| host.stage_title(number & STAGE_BITS))
    }

    /// Division `division`'s stage `stage`'s champion (`which` 0) or runner-up (1) (upstream
    /// `stage_glob`): decided mid-season, else the titles kept.
    ///
    /// # Errors
    ///
    /// [`RulesError::StagePastTitles`] when the stage is none of the three (upstream reads past
    /// them).
    fn stage_glob(&mut self, division: i32, stage: i32, which: usize) -> Result<u16, RulesError> {
        if let Some((_, glob)) = self.stage_decided(stage | division << DIVISION_SHIFT) {
            return Ok(glob[which]);
        }
        let titles = self.titles(division);
        let stage = usize::try_from(stage)
            .ok()
            .filter(|&stage| stage < titles.len())
            .ok_or(RulesError::StagePastTitles(stage))?;
        Ok(titles[stage].glob[which])
    }

    /// The stage titles' finalist `at` (upstream `title_club`, entrant type 5): 1 the opening
    /// champion; the closing champion when the two differ, or for 4; else stage `at - 2`'s
    /// runner-up.
    fn title_club(&mut self, division: i32, at: i32) -> Result<Club, RulesError> {
        let opening = self.stage_glob(division, 0, 0)?;
        let closing = self.stage_glob(division, 1, 0)?;
        if at == 1 {
            return Ok(self.glob_club(division, opening));
        }
        if opening != closing || at == SEEDS {
            return Ok(self.glob_club(division, closing));
        }
        let runner_up = self.stage_glob(division, at - SECOND, 1)?;
        Ok(self.glob_club(division, runner_up))
    }

    /// Club `club`'s points in kept stage `stage` (upstream `kept_pts`), 0 when not in it.
    fn kept_points(&self, stage: i32, club: Club) -> i32 {
        let glob = self.row(club).glob;
        if stage != 0 {
            return 0;
        }
        self.kept
            .iter()
            .find(|row| row.glob == glob)
            .map_or(0, |row| i32::from(row.points))
    }

    /// Whether `a` did better than `b` over the two stages (upstream `agg_better`): their points
    /// together, then the final table's place.
    fn aggregate_better(&self, stage: i32, a: Club, b: Club) -> bool {
        let points = |club| self.kept_points(stage, club) + i32::from(self.row(club).points);
        let (x, y) = (points(a), points(b));
        if x == y { place(a) < place(b) } else { x > y }
    }

    /// A kept stage's title decider's seed `at` (upstream `split_seed`, entrant type 6: the
    /// Mod's gplay.c `split_seed`): the two stages' leaders and the final table's top two.
    fn split_seed(&mut self, division: i32, stage: i32, at: i32) -> Club {
        if division >= self.divisions()
            || self.count(division) < TOP_TWO
            || !(1..=SEEDS).contains(&at)
        {
            return NONE;
        }
        let first = self.stage_club(division, stage, 1);
        let second = self.stage_club(division, stage, SECOND);
        let (top, runner) = (club_at(division, 0), club_at(division, 1));
        if first < 0 || second < 0 {
            return NONE;
        }
        if first == top {
            return first;
        }
        if second == runner || (first == runner && second == top) {
            let x = if self.aggregate_better(stage, first, top) {
                first
            } else {
                top
            };
            let y = if x == first { top } else { first };
            return if at == 1 || at == SEEDS { x } else { y };
        }
        let x = if first == runner {
            first
        } else if top == second {
            top
        } else {
            NONE
        };
        if x >= 0 {
            let mut y = if x == first { top } else { first };
            let mut z = if x == first { second } else { runner };
            if self.aggregate_better(stage, z, y) {
                (y, z) = (z, y);
            }
            return if at == 1 || at == SEEDS {
                x
            } else if at == SECOND {
                y
            } else {
                z
            };
        }
        let mut seeds = [first, top, second, runner];
        for index in 1..seeds.len() {
            let mut at = index;
            while at > 0 && self.aggregate_better(stage, seeds[at], seeds[at - 1]) {
                seeds.swap(at, at - 1);
                at -= 1;
            }
        }
        seeds[usize::try_from(at - 1).expect("1 to 4")]
    }

    /// Club `club`'s place in the annual table (upstream `t_place`), 0 when not in it.
    fn annual_place(&mut self, division: i32, club: Club) -> i32 {
        let mut at = 1;
        while division < self.divisions() && at <= self.count(division) {
            if self.stage_club(division, ANNUAL, at) == club {
                return at;
            }
            at += 1;
        }
        0
    }

    /// A national title play-off's seed `at` (upstream `title_seed`, entrant type 7: the Mod's
    /// `title_seed`), rule 1 or the other: the opening and closing champions, then the annual
    /// table's.
    fn title_seed(&mut self, division: i32, rule: i32, at: i32) -> Club {
        if division >= self.divisions()
            || self.count(division) < SEEDS
            || !(1..=SEEDS).contains(&at)
        {
            return NONE;
        }
        let opening = self.stage_club(division, OPENING, 1);
        let closing_table = if rule == 1 { ALTERNATIVE } else { CLOSING };
        let closing = self.stage_club(division, closing_table, 1);
        let annual = self.stage_club(division, ANNUAL, 1);
        if opening < 0 || closing < 0 || annual < 0 {
            return NONE;
        }
        if opening == closing {
            return opening;
        }
        if rule == 1 {
            return if at == 1 {
                opening
            } else if at == SECOND {
                closing
            } else {
                annual
            };
        }
        let opening_place = self.annual_place(division, opening);
        let closing_place = self.annual_place(division, closing);
        let in_top = |place: i32, top: i32| place > 0 && place <= top;
        if in_top(opening_place, TOP_TWO) && in_top(closing_place, TOP_TWO) {
            return if at == 1 || at == SEEDS {
                opening
            } else {
                closing
            };
        }
        let wanted = usize::try_from(SEEDS).expect("4 seeds");
        let mut seeds = Vec::with_capacity(wanted);
        if in_top(opening_place, TOP_EIGHT) {
            seeds.push(opening);
        }
        if in_top(closing_place, TOP_EIGHT) {
            seeds.push(closing);
        }
        let mut place_at = 1;
        while seeds.len() < wanted && place_at <= self.count(division) {
            let club = self.stage_club(division, ANNUAL, place_at);
            if club >= 0 && club != opening && club != closing {
                seeds.push(club);
            }
            place_at += 1;
        }
        if seeds.len() < wanted {
            return NONE;
        }
        let x = if in_top(opening_place, TOP_TWO) {
            opening
        } else if in_top(closing_place, TOP_TWO) {
            closing
        } else {
            NONE
        };
        if x >= 0 {
            let mut y = NONE;
            for &seed in &seeds {
                if seed != x && (seed == opening || seed == closing) && y < 0 {
                    y = seed;
                }
            }
            let mut z = NONE;
            for &seed in &seeds {
                if seed != x
                    && seed != y
                    && (z < 0 || self.annual_place(division, seed) < self.annual_place(division, z))
                {
                    z = seed;
                }
            }
            if y < 0 {
                y = seeds
                    .iter()
                    .copied()
                    .find(|&seed| seed != x && seed != z)
                    .unwrap_or(NONE);
            }
            return if at == 1 || at == SEEDS {
                x
            } else if at == SECOND {
                y
            } else {
                z
            };
        }
        for index in 1..seeds.len() {
            let mut at = index;
            while at > 0
                && self.annual_place(division, seeds[at])
                    < self.annual_place(division, seeds[at - 1])
            {
                seeds.swap(at, at - 1);
                at -= 1;
            }
        }
        // Seed `at` (1 to 4) is the `at`th of the four by the annual table.
        seeds[usize::try_from(at - 1).expect("1 to 4")]
    }

    /// The club of team word `team` (upstream `team_code`), none when not in the tables.
    pub(super) fn team_club(&self, team: u16) -> Club {
        for division in 0..self.divisions().min(i32::try_from(DIVISIONS).expect("16")) {
            for at in 0..self.count(division) {
                if self.row_at(division, at).team == team {
                    return club_at(division, at);
                }
            }
        }
        NONE
    }

    /// The record's move of the club at `division`, `at` (upstream `rec_move`): up when promoted
    /// by place (not from the top), down when relegated; none without counts.
    fn record_move(&self, division: i32, at: i32) -> Result<i32, RulesError> {
        let index = usize::try_from(division).expect("a division");
        let count = |counts: &[u8]| -> Result<Option<i32>, RulesError> {
            if counts.is_empty() {
                return Ok(None);
            }
            counts
                .get(index)
                .map(|&count| Some(i32::from(count)))
                .ok_or(RulesError::CountsPastLeague(self.league.divisions))
        };
        if let Some(promoted) = count(&self.league.promoted)?
            && at < promoted
            && division > 0
        {
            return Ok(-1);
        }
        if let Some(relegated) = count(&self.league.relegated)?
            && at >= self.count(division) - relegated
        {
            return Ok(1);
        }
        Ok(0)
    }

    /// Which second team club `club` is (upstream `b_index`), by global number or team word.
    fn second_index(&self, club: Club) -> Result<Option<usize>, RulesError> {
        let row = self.row(club);
        let seconds = &self.league.second_teams;
        let base = self.season.base;
        for (index, team) in seconds.teams.iter().enumerate() {
            if row.glob == base.wrapping_add(u16::from(team.team)) {
                return Ok(Some(index));
            }
            if let Some(words) = &seconds.words {
                let word = words
                    .get(index)
                    .ok_or(RulesError::SecondTeamsPastWords(seconds.teams.len()))?;
                if row.team == *word {
                    return Ok(Some(index));
                }
            }
        }
        Ok(None)
    }

    /// Whether club `club` is passed over as an entrant by place (upstream `b_inelig`): a second
    /// team that may not play play-offs, or may not go up (above the league's highest level for
    /// them, or into or above its first team's level after the record's moves).
    pub(super) fn second_ineligible(&self, club: Club) -> Result<bool, RulesError> {
        let seconds = &self.league.second_teams;
        if seconds.teams.is_empty() || !self.season.second_teams_in_knockouts {
            return Ok(false);
        }
        let Some(index) = self.second_index(club)? else {
            return Ok(false);
        };
        if !seconds.play_offs {
            return Ok(true);
        }
        let target = level_of(self, tier(club)) - 1;
        if let Some(top) = seconds.top
            && target < i32::from(top)
        {
            return Ok(true);
        }
        let Some(first) = seconds.teams[index].first else {
            return Ok(false);
        };
        let first = self.season.base.wrapping_add(u16::from(first));
        for division in 0..self.divisions().min(i32::try_from(DIVISIONS).expect("16")) {
            for at in 0..self.count(division) {
                if self.row_at(division, at).glob == first {
                    return Ok(target <= level_of(self, division) + self.record_move(division, at)?);
                }
            }
        }
        Ok(false)
    }

    /// Division `division`'s place `at` (from 1) as a club (upstream `place_code`): in a play-off
    /// of kind `kind`, the second teams that may not go up passed over, but in a play-out.
    fn place_club(&self, division: i32, at: i32, kind: u8) -> Result<Club, RulesError> {
        if division >= self.divisions()
            || division >= i32::try_from(DIVISIONS).expect("16")
            || at < 1
            || at > self.count(division)
        {
            return Ok(NONE);
        }
        if kind == PLAY_OUT || self.league.second_teams.teams.is_empty() {
            return Ok(club_at(division, at - 1));
        }
        let mut left = at;
        for place in 0..self.count(division) {
            let club = club_at(division, place);
            if !self.second_ineligible(club)? {
                left -= 1;
                if left == 0 {
                    return Ok(club);
                }
            }
        }
        Ok(NONE)
    }

    /// A play-off's entrant `entrant` (its four bytes) as a club (upstream `ent_code`), in a
    /// play-off of kind `kind`, the previous play-off's loser `previous`.
    fn entrant_club(
        &mut self,
        entrant: &[u8],
        previous: Club,
        kind: u8,
    ) -> Result<Club, RulesError> {
        let (kind_of, division, at, extra) = (
            entrant[0],
            i32::from(entrant[1]),
            i32::from(entrant[2]),
            entrant[3],
        );
        Ok(match kind_of {
            STAGE => self.stage_club(division, i32::from(extra), at),
            PREVIOUS_LOSER => previous,
            FINALIST => self.title_club(division, at)?,
            SPLIT_SEED => self.split_seed(division, i32::from(extra), at),
            TITLE_SEED => self.title_seed(division, i32::from(extra), at),
            BY_TEAM => {
                let (file, ordinal) = (u16::from(entrant[2]), u16::from(extra));
                let word = if self.league.team_file_high {
                    file << FILE_SHIFT | ordinal
                } else {
                    file | ordinal << FILE_SHIFT
                };
                self.team_club(word)
            }
            BY_PLACE => self.place_club(division, at, kind)?,
            _ => NONE,
        })
    }

    /// A play-off's entrants (`spec`: its header, then its entrants) as clubs (upstream
    /// `resolve_entrants`) into `clubs`; at the season end, a club by team word not in the tables
    /// absent (`absent`). Whether every one is known.
    pub(super) fn resolve_entrants(
        &mut self,
        spec: &[u8],
        previous: Club,
        clubs: &mut [Club; ENTRANTS],
        absent: bool,
    ) -> Result<bool, RulesError> {
        let (kind, entrants) = (spec[0], usize::from(spec[1]));
        let mut known = true;
        for (index, club) in clubs.iter_mut().enumerate().take(entrants.min(ENTRANTS)) {
            let entrant = &spec[HEADER + ENTRANT * index..HEADER + ENTRANT * (index + 1)];
            *club = self.entrant_club(entrant, previous, kind)?;
            if *club < 0 && absent && entrant[0] == BY_TEAM {
                *club = ABSENT;
            } else if *club < 0 {
                known = false;
            }
        }
        Ok(known)
    }
}

#[cfg(test)]
mod tests {
    use super::super::Knockouts;
    use super::super::fixture::{Host, glob, league, rows, season, tables, team, with_play};
    use super::*;
    use crate::competition::rules_library::league::SecondTeam;
    use crate::competition::rules_library::season::{ResultKind, StageBank};
    use crate::competition::rules_library::table::{CLUBS, Levels, Row, Tables};

    /// A stage title of division 0: its champion's and runner-up's places.
    fn title(champion: usize, runner_up: usize) -> KnockoutResult {
        KnockoutResult {
            kind: Some(ResultKind::Stage),
            division: 0,
            team: [team(0, champion), team(0, runner_up)],
            glob: [glob(0, champion), glob(0, runner_up)],
        }
    }

    /// Division 0's rows at places `order`, first to last.
    fn table_of(order: &[usize]) -> [Row; CLUBS] {
        let all = rows(&[8]);
        let mut table = [Row::default(); CLUBS];
        for (at, &place) in order.iter().enumerate() {
            table[at] = all[0][place];
        }
        table
    }

    #[test]
    fn a_stage_table_s_place_is_the_bank_s_this_season_else_the_final_table_s() {
        let tables = tables(&[6, 4]);
        let league = league(2, None);
        let mut bank = StageBank {
            fresh: 1 << 1,
            ..StageBank::default()
        };
        bank.counts[1] = 3;
        bank.tables[1] = table_of(&[5, 4, 3]);
        let mut played = season();
        played.bank = Some(&mut bank);
        with_play(
            &tables,
            &league,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(
                    play.stage_club(0, BANK_FIRST, 2),
                    club_at(0, 1),
                    "not this season's"
                );
                assert!(play.stages_used);
                assert_eq!(play.stage_club(0, BANK_FIRST + 1, 1), club_at(0, 5));
                assert_eq!(
                    play.stage_club(0, BANK_FIRST + 1, 4),
                    NONE,
                    "past its clubs"
                );
                assert_eq!(
                    play.stage_club(1, BANK_FIRST + 1, 1),
                    NONE,
                    "not in that division"
                );
                assert_eq!(play.stage_club(0, BANK_FIRST, 0), NONE);
                assert_eq!(play.stage_club(2, BANK_FIRST, 1), NONE);
                play.kept = table_of(&[2, 0])[..2].to_vec();
                assert_eq!(play.stage_club(0, 0, 1), club_at(0, 2), "the kept table");
                assert_eq!(play.stage_club(0, 1, 1), NONE, "the second, never filled");
                assert_eq!(play.stage_club(0, KEPT_TABLES, 1), NONE);
            },
        );
        // Division 1 has clubs but is past the divisions in use: none of the kept table's.
        let mut all = [0; DIVISIONS];
        all[..2].copy_from_slice(&[6, 6]);
        let past = Tables::new(1, all, rows(&[6, 6])).expect("tables");
        with_play(
            &past,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                play.kept = vec![past.row(1, 0)];
                assert_eq!(play.stage_club(1, 0, 1), NONE);
            },
        );
    }

    #[test]
    fn a_global_number_is_its_club_in_its_division() {
        let tables = tables(&[6, 4]);
        let league = league(2, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.glob_club(1, glob(1, 3)), club_at(1, 3));
                assert_eq!(play.glob_club(0, glob(1, 3)), NONE);
                assert_eq!(play.glob_club(0, NO_CLUB), NONE);
                assert_eq!(play.glob_club(2, glob(1, 3)), NONE);
                assert_eq!(play.team_club(team(1, 2)), club_at(1, 2));
                assert_eq!(play.team_club(team(3, 0)), NONE);
            },
        );
    }

    #[test]
    fn the_career_club_s_stage_titles_are_kept_another_division_s_emptied_on_change() {
        let tables = tables(&[6, 4, 4]);
        let league = league(3, None);
        let mut state = Knockouts::default();
        let mut played = season();
        played.manager = team(1, 0);
        with_play(&tables, &league, &mut played, &mut state, |play| {
            play.titles(1)[0] = title(1, 2);
            play.titles(0)[2] = title(3, 4);
            assert_eq!(play.state.other_division, 0);
            assert_eq!(play.titles(0)[2], title(3, 4), "the same division");
            assert_eq!(play.titles(2)[2].kind, None, "emptied");
        });
        assert_eq!(state.stage_titles()[0], title(1, 2));
        let mut emptied_title = title(1, 2);
        emptied(&mut emptied_title);
        assert_eq!(emptied_title, KnockoutResult::none());
        let mut none = Knockouts::default();
        with_play(&tables, &league, &mut season(), &mut none, |play| {
            play.titles(0)[1] = title(0, 1);
        });
        assert_eq!(
            none.stage_titles()[1],
            title(0, 1),
            "no career club: the top's"
        );
    }

    #[test]
    fn a_stage_title_decided_mid_season_is_the_host_s_for_the_career_club_s_division() {
        let tables = tables(&[6, 4]);
        let league = league(2, None);
        let decided = ([team(1, 2), team(1, 3)], [glob(1, 2), glob(1, 3)]);
        let mut host = Host {
            stage: Some((1, decided.0, decided.1)),
            ..Host::default()
        };
        let mut played = season();
        played.manager = team(1, 0);
        played.host = Some(&mut host);
        with_play(
            &tables,
            &league,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                let division_1 = 1 << DIVISION_SHIFT;
                assert_eq!(play.stage_decided(1 | division_1), Some(decided));
                assert_eq!(play.stage_decided(division_1), None, "another stage");
                assert_eq!(play.stage_decided(1), None, "another division");
                assert_eq!(play.stage_glob(1, 1, 1), Ok(glob(1, 3)));
                play.titles(1)[0] = KnockoutResult {
                    glob: [glob(1, 0), glob(1, 1)],
                    ..title(0, 1)
                };
                assert_eq!(play.stage_glob(1, 0, 0), Ok(glob(1, 0)));
                assert_eq!(
                    play.stage_glob(1, STAGE_TITLES, 0),
                    Err(RulesError::StagePastTitles(3))
                );
            },
        );
    }

    #[test]
    fn the_stage_titles_finalists_are_the_champions_else_a_runner_up() {
        let tables = tables(&[6]);
        let league = league(1, None);
        let mut state = Knockouts::default();
        with_play(&tables, &league, &mut season(), &mut state, |play| {
            play.titles(0)[0] = title(2, 4);
            play.titles(0)[1] = title(3, 5);
            assert_eq!(play.title_club(0, 1), Ok(club_at(0, 2)));
            assert_eq!(
                play.title_club(0, SECOND),
                Ok(club_at(0, 3)),
                "two champions"
            );
            play.titles(0)[1] = title(2, 5);
            assert_eq!(
                play.title_club(0, SECOND),
                Ok(club_at(0, 4)),
                "stage 0's runner-up"
            );
            assert_eq!(
                play.title_club(0, 3),
                Ok(club_at(0, 5)),
                "stage 1's runner-up"
            );
            assert_eq!(
                play.title_club(0, SEEDS),
                Ok(club_at(0, 2)),
                "the closing champion"
            );
        });
    }

    #[test]
    fn a_split_title_s_seeds_are_the_stages_leaders_and_the_top_two() {
        let tables = tables(&[6]);
        let league = league(1, None);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                // The kept stage won by the final table's 1st: the champion.
                play.kept = table_of(&[0, 3])[..2].to_vec();
                assert_eq!(play.split_seed(0, 0, 2), club_at(0, 0));
                // Its 2nd first, the 1st second: the better over the two stages, then the other.
                play.kept = table_of(&[1, 0])[..2].to_vec();
                play.kept[0].points = 90;
                assert_eq!(play.kept_points(0, club_at(0, 1)), 90);
                assert_eq!(play.kept_points(1, club_at(0, 1)), 0);
                assert_eq!(play.kept_points(0, club_at(0, 4)), 0);
                assert!(play.aggregate_better(0, club_at(0, 1), club_at(0, 0)));
                assert_eq!(play.split_seed(0, 0, 1), club_at(0, 1));
                assert_eq!(play.split_seed(0, 0, SECOND), club_at(0, 0));
                assert_eq!(play.split_seed(0, 0, SEEDS), club_at(0, 1));
                assert_eq!(play.split_seed(0, 0, 0), NONE);
                assert_eq!(play.split_seed(1, 0, 1), NONE);
                play.kept.clear();
                assert_eq!(play.split_seed(0, 0, 1), NONE, "no kept table");
            },
        );
    }

    #[test]
    fn a_title_play_off_s_seeds_are_the_champions_then_the_annual_table_s() {
        let tables = tables(&[8]);
        let league = league(1, None);
        let table = |stage: i32| usize::try_from(stage - BANK_FIRST).expect("a table");
        let (opening, closing, alternative) = (table(OPENING), table(CLOSING), table(ALTERNATIVE));
        let mut bank = StageBank {
            fresh: 1 << opening | 1 << closing | 1 << alternative,
            ..StageBank::default()
        };
        bank.counts[opening] = 8;
        bank.tables[opening] = table_of(&[5, 0, 1, 2, 3, 4, 6, 7]);
        bank.counts[closing] = 8;
        bank.tables[closing] = table_of(&[6, 0, 1, 2, 3, 4, 5, 7]);
        bank.counts[alternative] = 8;
        bank.tables[alternative] = table_of(&[7, 0, 1, 2, 3, 4, 5, 6]);
        let mut played = season();
        played.bank = Some(&mut bank);
        with_play(
            &tables,
            &league,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.annual_place(0, club_at(0, 3)), 4, "the final table's");
                assert_eq!(play.annual_place(1, club_at(0, 3)), 0);
                // Rule 1: the opening and the alternative closing champions, then the annual
                // table's.
                assert_eq!(play.title_seed(0, 1, 1), club_at(0, 5));
                assert_eq!(play.title_seed(0, 1, SECOND), club_at(0, 7));
                assert_eq!(play.title_seed(0, 1, 3), club_at(0, 0));
                // Else four seeds: the champions in the top 8, then the annual table's best.
                assert_eq!(play.title_seed(0, 0, 1), club_at(0, 0));
                assert_eq!(play.title_seed(0, 0, SECOND), club_at(0, 1));
                assert_eq!(play.title_seed(0, 0, 3), club_at(0, 5));
                assert_eq!(play.title_seed(0, 0, SEEDS), club_at(0, 6));
                assert_eq!(play.title_seed(0, 0, 0), NONE);
            },
        );
    }

    #[test]
    fn the_record_s_move_is_its_place_s() {
        let tables = tables(&[6, 6]);
        let mut counted = league(2, None);
        counted.promoted = vec![1, 2];
        counted.relegated = vec![2, 0];
        with_play(
            &tables,
            &counted,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.record_move(0, 0), Ok(0), "not from the top");
                assert_eq!(play.record_move(1, 1), Ok(-1));
                assert_eq!(play.record_move(1, 2), Ok(0));
                assert_eq!(play.record_move(0, 4), Ok(1));
                assert_eq!(play.record_move(1, 5), Ok(0));
            },
        );
        let mut short = league(2, None);
        short.promoted = vec![1];
        with_play(
            &tables,
            &short,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.record_move(1, 0), Err(RulesError::CountsPastLeague(2)));
            },
        );
    }

    #[test]
    fn a_second_team_that_may_not_go_up_is_passed_over_by_place_not_in_a_play_out() {
        let tables = tables(&[6, 6]);
        let mut seconds = league(2, None);
        // Division 1's 1st: the second team of division 0's 3rd; another by word.
        seconds.second_teams.teams = vec![
            SecondTeam {
                team: u8::try_from(glob(1, 0)).expect("a byte"),
                first: Some(u8::try_from(glob(0, 2)).expect("a byte")),
            },
            SecondTeam {
                team: 0xF0,
                first: None,
            },
        ];
        seconds.second_teams.play_offs = true;
        seconds.second_teams.words = Some(vec![0, team(1, 3)]);
        let mut played = season();
        played.second_teams_in_knockouts = true;
        with_play(
            &tables,
            &seconds,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(play.second_index(club_at(1, 0)), Ok(Some(0)));
                assert_eq!(play.second_index(club_at(1, 3)), Ok(Some(1)), "by word");
                assert_eq!(play.second_index(club_at(1, 1)), Ok(None));
                assert_eq!(
                    play.second_ineligible(club_at(1, 0)),
                    Ok(true),
                    "its first team's level"
                );
                assert_eq!(
                    play.second_ineligible(club_at(1, 3)),
                    Ok(false),
                    "no first team"
                );
                assert_eq!(play.place_club(1, 1, 1), Ok(club_at(1, 1)));
                assert_eq!(play.place_club(1, 1, PLAY_OUT), Ok(club_at(1, 0)));
                assert_eq!(play.place_club(1, 7, 1), Ok(NONE));
                assert_eq!(play.place_club(2, 1, 1), Ok(NONE));
            },
        );
        seconds.second_teams.play_offs = false;
        with_play(
            &tables,
            &seconds,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(
                    play.second_ineligible(club_at(1, 3)),
                    Ok(true),
                    "no play-offs"
                );
            },
        );
        seconds.second_teams.play_offs = true;
        seconds.second_teams.top = Some(1);
        with_play(
            &tables,
            &seconds,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(
                    play.second_ineligible(club_at(1, 3)),
                    Ok(true),
                    "above the highest"
                );
            },
        );
        // One level of both divisions: division 1's club may go up to its first team's.
        let levels = Levels { first: vec![0] };
        let mut levelled = season();
        levelled.second_teams_in_knockouts = true;
        levelled.levels = Some(&levels);
        with_play(
            &tables,
            &seconds,
            &mut levelled,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(level_of(play, 1), 0);
            },
        );
        with_play(
            &tables,
            &seconds,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(level_of(play, 1), 1);
            },
        );
        with_play(
            &tables,
            &seconds,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                assert_eq!(
                    play.second_ineligible(club_at(1, 0)),
                    Ok(false),
                    "not in the knockouts"
                );
            },
        );
        seconds.second_teams.words = Some(Vec::new());
        with_play(
            &tables,
            &seconds,
            &mut played,
            &mut Knockouts::default(),
            |play| {
                assert_eq!(
                    play.second_index(club_at(1, 1)),
                    Err(RulesError::SecondTeamsPastWords(2))
                );
            },
        );
    }

    #[test]
    fn an_entrant_of_each_type_is_its_club() {
        let tables = tables(&[6, 6]);
        let mut high = league(2, None);
        let mut state = Knockouts::default();
        with_play(&tables, &high, &mut season(), &mut state, |play| {
            play.titles(0)[0] = title(4, 5);
            play.titles(0)[1] = title(3, 5);
            play.kept = table_of(&[0, 1])[..2].to_vec();
            let file = |division: u8| {
                u8::try_from(team(usize::from(division), 0) >> FILE_SHIFT).expect("a file")
            };
            assert_eq!(
                play.entrant_club(&[BY_PLACE, 1, 2, 0], NONE, 1),
                Ok(club_at(1, 1))
            );
            assert_eq!(
                play.entrant_club(&[STAGE, 0, 2, 4], NONE, 1),
                Ok(club_at(0, 1))
            );
            assert_eq!(
                play.entrant_club(&[PREVIOUS_LOSER, 0, 0, 0], club_at(1, 4), 1),
                Ok(club_at(1, 4))
            );
            assert_eq!(
                play.entrant_club(&[FINALIST, 0, 1, 0], NONE, 1),
                Ok(club_at(0, 4))
            );
            assert_eq!(
                play.entrant_club(&[SPLIT_SEED, 0, 1, 0], NONE, 1),
                Ok(club_at(0, 0))
            );
            assert_eq!(
                play.entrant_club(&[TITLE_SEED, 0, 1, 1], NONE, 1),
                Ok(club_at(0, 0))
            );
            assert_eq!(
                play.entrant_club(&[BY_TEAM, 0, file(1), 3], NONE, 1),
                Ok(club_at(1, 3))
            );
            assert_eq!(
                play.entrant_club(&[1, 0, 1, 0], NONE, 1),
                Ok(NONE),
                "no such type"
            );
        });
        high.team_file_high = false;
        with_play(&tables, &high, &mut season(), &mut state, |play| {
            // The Mod's layout: the file in the low byte.
            let word = team(1, 3);
            let (file, ordinal) = (
                u8::try_from(word >> FILE_SHIFT).expect("a byte"),
                u8::try_from(word & 0xFF).expect("a byte"),
            );
            assert_eq!(
                play.entrant_club(&[BY_TEAM, 0, ordinal, file], NONE, 1),
                Ok(club_at(1, 3))
            );
        });
    }

    #[test]
    fn a_play_off_s_entrants_are_known_or_absent_by_team_word() {
        let tables = tables(&[6, 6]);
        let league = league(2, None);
        let file = u8::try_from(team(1, 0) >> FILE_SHIFT).expect("a file");
        let mut spec = vec![1, 3, 1, 0, 0, 0];
        spec.extend([BY_PLACE, 0, 1, 0, BY_TEAM, 0, file, 2, BY_TEAM, 0, file, 30]);
        with_play(
            &tables,
            &league,
            &mut season(),
            &mut Knockouts::default(),
            |play| {
                let mut clubs = [NONE; ENTRANTS];
                assert_eq!(
                    play.resolve_entrants(&spec, NONE, &mut clubs, true),
                    Ok(true)
                );
                assert_eq!(clubs[..3], [club_at(0, 0), club_at(1, 2), ABSENT]);
                assert_eq!(
                    play.resolve_entrants(&spec, NONE, &mut clubs, false),
                    Ok(false)
                );
                spec[8] = 9;
                assert_eq!(
                    play.resolve_entrants(&spec, NONE, &mut clubs, true),
                    Ok(false),
                    "a place past the clubs"
                );
            },
        );
    }
}
