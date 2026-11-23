//! A competition under way (docs/PLAN.md M5 #4): upstream's `DIYCompetitionEx`, the record the
//! engine plays a league, a cup or a tournament in.
//!
//! Upstream gives many of its regions two meanings (unions: a league's and a cup's view of the
//! match data, the shuffle order reused as a cup bracket's scratch, the standings rows a world's
//! league phase and split reuse), and its code runs past arrays' declared ends into the fields
//! after them (a league of more than 14 clubs writes `leagueTeamIds` on into
//! `competitionFormat`). So the state is upstream's image itself, held whole, and each field is
//! a typed accessor at its offset: an index past an array's end reads and writes the bytes
//! after it, as the C does. The image is upstream's 64-bit build's (include/game_types.h; the
//! oracle checks every offset against the C), so the units that play a competition are
//! checked against the C on the same bytes, and a save's image is written back as read.

use super::table::Line;

/// The bytes of a competition's image in upstream's 64-bit build (`sizeof(DIYCompetitionEx)`,
/// 0xA50).
pub const IMAGE_LEN: usize = 2640;

/// The rows of the standings (`leagueStandings[64]`).
pub const STANDINGS_ROWS: usize = 64;

/// The offset of the first standings row.
const STANDINGS: usize = 1488;

/// The offset of the league's places by standing (`teamEntryOffsets`).
const TEAM_ENTRY_OFFSETS: usize = 118;

/// The offset of the league's teams (`leagueTeamIds`).
const LEAGUE_TEAMS: usize = 310;

/// The offset of the season's months (the league section's `phaseStartMonths`).
pub const PHASE_MONTHS: usize = 470;

/// The bytes of a standings row (`DIYTeamStats`).
const ROW_LEN: usize = 18;

/// The offset of the competition's name buffer (`competitionName`).
pub(super) const NAME: usize = 4;

/// The bytes of the name buffer.
pub const NAME_LEN: usize = 36;

/// The offset of a cup's penalties, round by round (cup view `roundPenalties[6]`): the last of
/// its rounds' rules.
pub const CUP_ROUND_PENALTIES: usize = 134;

/// A team as two bytes in the image: its country, then its ordinal (upstream writes team ids
/// big-endian: `{country, ordinal}`).
pub type TeamBytes = [u8; 2];

/// A team's standings row (upstream `DIYTeamStats`): which team, and its line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct StandingsRow {
    /// The team (`teamSelOffset`): -1 the career club, otherwise its index among the
    /// competition's selected teams.
    pub team: i32,
    /// Its line.
    pub line: Line,
}

/// A value an image holds: its bytes, little-endian as upstream's build lays them out.
pub trait Field: Sized {
    /// Its bytes.
    const LEN: usize;
    /// The value `bytes` hold.
    fn decode(bytes: &[u8]) -> Self;
    /// `self` into `bytes`.
    fn encode(self, bytes: &mut [u8]);
}

/// Implements [`Field`] for an integer, little-endian.
macro_rules! integer_field {
    ($($ty:ty),+) => {
        $(impl Field for $ty {
            const LEN: usize = std::mem::size_of::<$ty>();

            fn decode(bytes: &[u8]) -> Self {
                Self::from_le_bytes(bytes.try_into().expect("a field's bytes"))
            }

            fn encode(self, bytes: &mut [u8]) {
                bytes.copy_from_slice(&self.to_le_bytes());
            }
        })+
    };
}

integer_field!(u8, i8, u16, i16, i32);

impl Field for TeamBytes {
    const LEN: usize = 2;

    fn decode(bytes: &[u8]) -> Self {
        bytes.try_into().expect("a team's two bytes")
    }

    fn encode(self, bytes: &mut [u8]) {
        bytes.copy_from_slice(&self);
    }
}

/// Generates a getter and a setter for each field at its offset in the image.
macro_rules! fields {
    ($($(#[$doc:meta])* $get:ident, $set:ident: $ty:ty = $offset:expr;)*) => {
        $(
            $(#[$doc])*
            #[must_use]
            pub fn $get(&self) -> $ty {
                self.get($offset)
            }

            $(#[$doc])*
            pub fn $set(&mut self, value: $ty) {
                self.set($offset, value);
            }
        )*
    };
}

/// Generates an indexed getter and setter for each array at its offset: an index past the
/// array's end reaches the bytes after it, as upstream's code does.
macro_rules! arrays {
    ($($(#[$doc:meta])* $get:ident, $set:ident: $ty:ty = $offset:expr;)*) => {
        $(
            $(#[$doc])*
            ///
            /// # Panics
            ///
            /// When `index` reaches past the image.
            #[must_use]
            pub fn $get(&self, index: usize) -> $ty {
                self.get($offset + index * <$ty as Field>::LEN)
            }

            $(#[$doc])*
            ///
            /// # Panics
            ///
            /// When `index` reaches past the image.
            pub fn $set(&mut self, index: usize, value: $ty) {
                self.set($offset + index * <$ty as Field>::LEN, value);
            }
        )*
    };
}

/// Generates the accessors of upstream's competition header (`DIYCompetition`, 104 bytes at the
/// image's start: its name pointer, an address, at 40; 4 bytes of padding after
/// `playEachTeam`), which a competition under way and a tournament's state both begin with.
macro_rules! header_fields {
    () => {
        /// The competition's name buffer (`competitionName[36]`): its text, then 0s.
        #[must_use]
        pub fn name(&self) -> [u8; NAME_LEN] {
            self.bytes(NAME)
        }

        /// Sets the name buffer.
        pub fn set_name(&mut self, name: [u8; NAME_LEN]) {
            self.image[NAME..NAME + NAME_LEN].copy_from_slice(&name);
        }

        fields! {
            /// The competition's type (`competitionType`): 0 a league, 1 a cup, 2 a tournament.
            competition_type, set_competition_type: i16 = 0;
            /// In use, a cup's (`isActive`); over, a slot's (`competitionOver`).
            is_active, set_is_active: u16 = 2;
            /// The slot's cache key byte (`compSlot`).
            comp_slot, set_comp_slot: u8 = 48;
            /// The slot's index for cache matching (`slotId`).
            slot_id, set_slot_id: u8 = 49;
            /// The competition's id (`field_2E`, which the code reads as its `compId`).
            comp_id, set_comp_id: u8 = 50;
            /// The month or phase for scheduling (`viewRandomPhase`).
            view_random_phase, set_view_random_phase: u8 = 51;
            /// The teams' average skill, a cup's (`avgSkill`); a trophy's bonus, a slot's
            /// (`trophyBonusValue`).
            avg_skill, set_avg_skill: u8 = 52;
            /// The strength tier (`strengthTier`).
            strength_tier, set_strength_tier: u8 = 53;
            /// The teams (`numTeams`).
            num_teams, set_num_teams: i16 = 54;
            /// The pitch type or the season (`pitchTypeOrSeason`).
            pitch_type_or_season, set_pitch_type_or_season: i16 = 56;
            /// The pitch type (`pitchType`).
            pitch_type, set_pitch_type: i16 = 58;
            /// The first month (`startMonth`).
            start_month, set_start_month: i8 = 60;
            /// The last month (`endMonth`).
            end_month, set_end_month: i8 = 61;
            /// The result code, a cup's (`resultCode`).
            result_code, set_result_code: i16 = 62;
            /// The winner, a slot's (`winnerTeam`): the same bytes as the result code.
            winner_team, set_winner_team: u16 = 62;
            /// The substitutes, a cup's (`substitutesCount`); the squad view's parameter, a slot's
            /// (`squadViewParam`).
            substitutes_count, set_substitutes_count: i16 = 64;
            /// Slot state A (`slotStateA`).
            slot_state_a, set_slot_state_a: i16 = 66;
            /// Slot state B (`slotStateB`).
            slot_state_b, set_slot_state_b: i16 = 68;
            /// Slot state C (`slotStateC`).
            slot_state_c, set_slot_state_c: i16 = 70;
            /// 1 once a match has begun (`matchStarted`).
            match_started, set_match_started: i16 = 72;
            /// 1 when the career club takes part (`careerTeamInvolved`).
            career_team_involved, set_career_team_involved: i16 = 74;
            /// 0 no groups, 1 groups, 2 groups and a final (`hasGroups`).
            has_groups, set_has_groups: i16 = 76;
            /// The cup menu's state (`menuState`).
            menu_state, set_menu_state: i16 = 78;
            /// The round iterator, a cup's (`roundIterator`); finished, a slot's
            /// (`compFinished`).
            round_iterator, set_round_iterator: i16 = 80;
            /// The teams in the current stage (`numTeamsInStage`).
            num_teams_in_stage, set_num_teams_in_stage: i16 = 82;
            /// The match pairs a round (`matchPairsPerRound`): a league's groups.
            match_pairs_per_round, set_match_pairs_per_round: i16 = 84;
            /// The matches of a round (`totalMatchesInRound`): a league group's teams.
            total_matches_in_round, set_total_matches_in_round: i16 = 86;
            /// The round complete, a cup's (`roundComplete`); the lowest division, a league's
            /// (`minDivisionIndex`).
            round_complete, set_round_complete: i16 = 88;
            /// The competition finished, a cup's (`competitionFinished`); the highest division's
            /// offset, a league's (`maxDivisionOffset`).
            competition_finished, set_competition_finished: i16 = 90;
            /// The round's index, a cup's (`currentRoundIndex`); the promotions, a league's
            /// (`promotionCount`).
            current_round_index, set_current_round_index: i16 = 92;
            /// The rounds, a cup's (`numRounds`); the relegations, a league's
            /// (`relegationCount`).
            num_rounds, set_num_rounds: i16 = 94;
            /// The sub-round counter (`subRoundCounter`).
            sub_round_counter, set_sub_round_counter: i16 = 96;
            /// Each team plays the others this many times (`playEachTeam`).
            play_each_team, set_play_each_team: i16 = 98;
        }
    };
}

pub(crate) use {arrays, fields, header_fields};

/// A competition under way: upstream's image of it, read and written field by field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunningCompetition {
    /// The image.
    image: Box<[u8; IMAGE_LEN]>,
}

impl Default for RunningCompetition {
    /// A cleared one: every byte 0.
    fn default() -> Self {
        Self {
            image: Box::new([0; IMAGE_LEN]),
        }
    }
}

impl RunningCompetition {
    /// The competition `image` holds.
    #[must_use]
    pub fn from_image(image: &[u8; IMAGE_LEN]) -> Self {
        Self {
            image: Box::new(*image),
        }
    }

    /// Its image.
    #[must_use]
    pub fn image(&self) -> &[u8; IMAGE_LEN] {
        &self.image
    }

    /// The value at `offset`.
    fn get<T: Field>(&self, offset: usize) -> T {
        T::decode(&self.image[offset..offset + T::LEN])
    }

    /// Writes `value` at `offset`.
    fn set<T: Field>(&mut self, offset: usize, value: T) {
        value.encode(&mut self.image[offset..offset + T::LEN]);
    }

    /// The bytes from `offset` on, `N` of them.
    fn bytes<const N: usize>(&self, offset: usize) -> [u8; N] {
        self.image[offset..offset + N].try_into().expect("N bytes")
    }

    /// The offset of word `field` of standings row `index`'s line.
    fn line_word(index: usize, field: usize) -> usize {
        STANDINGS + index * ROW_LEN + <i32 as Field>::LEN + field * <u16 as Field>::LEN
    }

    /// Whether place `place` of the league's order by standing lies within the image.
    #[must_use]
    pub fn has_team_entry(place: usize) -> bool {
        TEAM_ENTRY_OFFSETS + (place + 1) * <u16 as Field>::LEN <= IMAGE_LEN
    }

    /// Whether the league's team `index` lies within the image.
    #[must_use]
    pub fn has_league_team(index: usize) -> bool {
        LEAGUE_TEAMS + (index + 1) * <TeamBytes as Field>::LEN <= IMAGE_LEN
    }

    /// Standings row `index`.
    ///
    /// # Panics
    ///
    /// When `index` is past the 64 rows.
    #[must_use]
    pub fn row(&self, index: usize) -> StandingsRow {
        assert!(index < STANDINGS_ROWS, "64 standings rows");
        let word = |field: usize| self.get::<u16>(Self::line_word(index, field));
        StandingsRow {
            team: self.get(STANDINGS + index * ROW_LEN),
            line: Line {
                played: word(0),
                won: word(1),
                drawn: word(2),
                lost: word(3),
                goals_for: word(4),
                goals_against: word(5),
                points: word(6),
            },
        }
    }

    /// Sets standings row `index`.
    ///
    /// # Panics
    ///
    /// When `index` is past the 64 rows.
    pub fn set_row(&mut self, index: usize, row: StandingsRow) {
        assert!(index < STANDINGS_ROWS, "64 standings rows");
        self.set(STANDINGS + index * ROW_LEN, row.team);
        let line = row.line;
        let words = [
            line.played,
            line.won,
            line.drawn,
            line.lost,
            line.goals_for,
            line.goals_against,
            line.points,
        ];
        for (field, word) in words.into_iter().enumerate() {
            self.set(Self::line_word(index, field), word);
        }
    }

    header_fields!();

    // After the header.
    fields! {
        /// The fewest substitutes (`minSubstitutes`; also the league's cycle flag, which
        /// swaps home and away).
        min_substitutes, set_min_substitutes: u16 = 104;
        /// A win's points (`pointsForAWin`).
        points_for_win, set_points_for_win: u16 = 106;
        /// The league's fewest substitutes (league view `leagueMinSubstitutes`).
        league_min_substitutes, set_league_min_substitutes: u16 = 108;
        /// The league's most substitutes (`leagueMaxSubstitutes`).
        league_max_substitutes, set_league_max_substitutes: u16 = 110;
        /// Groups, a DIY league's (`hasGroupsDiy`).
        has_groups_diy, set_has_groups_diy: u16 = 112;
        /// The matches of a section (`totalMatchesPerSection`).
        total_matches_per_section, set_total_matches_per_section: u16 = 114;
        /// The matches of the section left (`totalMatchesPerSectionLive`).
        total_matches_per_section_live, set_total_matches_per_section_live: u16 = 116;
        /// The current match, a cup's (cup view `currentMatchIndex`): the league's first
        /// bytes.
        cup_current_match_index, set_cup_current_match_index: u16 = 108;
        /// The competition's format (`competitionFormat`, both views).
        competition_format, set_competition_format: u16 = 338;
        /// The round's format (`roundFormat`, both views).
        round_format, set_round_format: u16 = 340;
        /// Matches played (`matchesPlayedCount`).
        matches_played_count, set_matches_played_count: u16 = 454;
        /// The current slot (`leagueCurrentSlot`; also the section's progression flag).
        league_current_slot, set_league_current_slot: u16 = 456;
        /// The match progress (`leagueMatchProgress`; also the phase's index).
        league_match_progress, set_league_match_progress: u16 = 458;
        /// The matches of the season (`leagueTotalMatches`).
        league_total_matches, set_league_total_matches: u16 = 460;
        /// The section's half groups (`halfGroupCount`).
        half_group_count, set_half_group_count: u16 = 462;
        /// The section's matches a half (`matchesPerHalf`).
        matches_per_half, set_matches_per_half: u16 = 464;
        /// The half groups counted (`halfGroupsCounter`).
        half_groups_counter, set_half_groups_counter: u16 = 466;
        /// The section's matches played (`sectionMatchesProgress`).
        section_matches_progress, set_section_matches_progress: u16 = 468;
        /// A cup's second-leg counter (bracket scratch `secondLegCounter`).
        second_leg_counter, set_second_leg_counter: u16 = 822;
        /// A cup's group section flag (`groupSectionFlag`).
        group_section_flag, set_group_section_flag: u16 = 824;
        /// A cup's matches counted (`matchCount`).
        match_count, set_match_count: u16 = 826;
        /// A cup's matches (`totalMatches`).
        total_matches, set_total_matches: u16 = 828;
        /// A cup's current month's index (`curMonthIdx`).
        cur_month_idx, set_cur_month_idx: u16 = 830;
    }

    arrays! {
        /// The league's places by standing (league view `teamEntryOffsets[64]`): each a
        /// standings row's offset, 18 a row.
        team_entry_offset, set_team_entry_offset: u16 = TEAM_ENTRY_OFFSETS;
        /// The league's shuffle indices and their pad (`shuffleAndPad[64]`).
        shuffle_and_pad, set_shuffle_and_pad: u8 = 246;
        /// The league's teams (`leagueTeamIds[14]`, both views; a league of more than 14 runs
        /// on into `competitionFormat` and past the match data, as upstream's does).
        league_team, set_league_team: TeamBytes = LEAGUE_TEAMS;
        /// A cup's legs, round by round (cup view `roundLegs[6]`).
        cup_round_legs, set_cup_round_legs: u16 = 110;
        /// A cup's extra time, round by round (`roundExtraTime[6]`).
        cup_round_extra_time, set_cup_round_extra_time: u16 = 122;
        /// A cup's penalties, round by round (`roundPenalties[6]`).
        cup_round_penalties, set_cup_round_penalties: u16 = CUP_ROUND_PENALTIES;
        /// A cup's round results (`cupRoundResults[100]`): each match's flags, by match.
        cup_round_result, set_cup_round_result: u8 = 146;
        /// A cup's scores, by match: the home side's goals in the high byte (the round
        /// results' words from their 64th byte, `cupRoundResults + 0x40`).
        cup_round_score, set_cup_round_score: u16 = 210;
        /// A cup's second-leg aggregates, packed goals by match (`cupAggregateScores[48]`).
        cup_aggregate_score, set_cup_aggregate_score: u16 = 342;
        /// A league's qualified teams, by division (`qualifiedTeamsByDivision[8]`).
        qualified_team_by_division, set_qualified_team_by_division: u16 = 438;
        /// Each phase's first month (section `phaseStartMonths[128]`; a split league's days'
        /// weeks, `splitWeeks[48]`, the same bytes).
        phase_start_month, set_phase_start_month: u8 = 470;
        /// A cup's shoot-outs, by match: the home side's penalties in the high byte (the
        /// season's months, as words).
        cup_penalty_score, set_cup_penalty_score: u16 = 470;
        /// A cup's teams (`cupTeamIds[64]`).
        cup_team, set_cup_team: TeamBytes = 598;
        /// The teams' grid pairings (`teamGridPairings[64]`).
        grid_pairing, set_grid_pairing: u8 = 726;
        /// The shuffle order (`teamShuffleOrder[698]`, whose bytes past its first 32 a cup's
        /// bracket uses as its scratch).
        shuffle_order, set_shuffle_order: u8 = 790;
        /// A cup bracket's month progression (bracket scratch `progTable[12]`).
        prog_table, set_prog_table: u8 = 832;
        /// Each cup team's identity (`teamOffsetTable[64]`): -1 the career club, otherwise its
        /// index among the selected teams.
        team_offset, set_team_offset: i32 = 844;
        /// The bracket scratch's tail (`scratchTail[0x184]`).
        scratch_tail, set_scratch_tail: u8 = 1100;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_image_reads_back_as_written() {
        let image: [u8; IMAGE_LEN] =
            std::array::from_fn(|index| u8::try_from(index % 251).expect("a byte"));
        let competition = RunningCompetition::from_image(&image);
        assert_eq!(competition.image(), &image);
        assert_eq!(RunningCompetition::default().image(), &[0; IMAGE_LEN]);
    }

    #[test]
    fn a_field_is_its_bytes_at_its_offset_little_endian() {
        let mut competition = RunningCompetition::default();
        competition.set_num_teams(-2);
        assert_eq!(&competition.image()[54..56], &[0xFE, 0xFF]);
        assert_eq!(competition.num_teams(), -2);
        competition.set_result_code(-1);
        assert_eq!(
            competition.winner_team(),
            u16::MAX,
            "two names of the same bytes"
        );
        competition.set_comp_id(22);
        assert_eq!(competition.image()[50], 22);
        competition.set_team_offset(1, -1);
        assert_eq!(&competition.image()[848..852], &[0xFF; 4]);
        let mut name = [0; 36];
        name[..3].copy_from_slice(b"CUP");
        competition.set_name(name);
        assert_eq!(&competition.image()[4..8], b"CUP\0");
        assert_eq!(competition.name(), name);
    }

    #[test]
    fn two_views_share_their_bytes() {
        let mut competition = RunningCompetition::default();
        competition.set_league_min_substitutes(7);
        assert_eq!(competition.cup_current_match_index(), 7);
        // The league's places (from byte 118) run over the cup's rounds (legs from 110, extra
        // time from 122): place 2 is round 0's extra time.
        competition.set_team_entry_offset(2, 0x1234);
        assert_eq!(competition.cup_round_extra_time(0), 0x1234);
    }

    #[test]
    fn an_index_past_an_array_s_end_reaches_the_next_field() {
        let mut competition = RunningCompetition::default();
        // League team 14 is past `leagueTeamIds[14]`: it is `competitionFormat`.
        competition.set_league_team(14, [0x01, 0x02]);
        assert_eq!(competition.competition_format(), 0x0201);
        assert_eq!(competition.league_team(14), [0x01, 0x02]);
    }

    #[test]
    fn a_standings_row_is_its_team_and_line() {
        let mut competition = RunningCompetition::default();
        let row = StandingsRow {
            team: -1,
            line: Line {
                played: 2,
                won: 1,
                drawn: 1,
                lost: 0,
                goals_for: 3,
                goals_against: 1,
                points: 4,
            },
        };
        competition.set_row(63, row);
        assert_eq!(competition.row(63), row);
        assert_eq!(
            &competition.image()[IMAGE_LEN - 2..],
            &[4, 0],
            "the last row's points"
        );
        assert_eq!(
            &competition.image()[STANDINGS + 63 * ROW_LEN..][..4],
            &[0xFF; 4]
        );
    }
}
