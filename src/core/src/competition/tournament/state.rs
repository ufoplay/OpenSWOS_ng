//! A tournament's state (docs/PLAN.md M5 #4d1): upstream's `DIYTournamentState`
//! (include/game_types.h), the record a tournament keeps its stages in while the competition
//! under way plays the current one. Held as upstream's image, each field a typed accessor at
//! its offset, as [`RunningCompetition`](super::super::running::RunningCompetition) is: its
//! header is the same `DIYCompetition`, with the same accessors, and an index past an array's
//! end reaches the bytes after it, as the C does. The image is upstream's 64-bit build's (the
//! oracle checks every offset against the C).

use crate::competition::running::{
    Field, NAME, NAME_LEN, TeamBytes, arrays, fields, header_fields,
};

/// The bytes of a tournament's state in upstream's 64-bit build (`sizeof(DIYTournamentState)`:
/// its fields end at 0x45E, and the header's pointer aligns it to 8).
pub const STATE_LEN: usize = 1120;

/// The stages a tournament holds (`slots[6]`).
pub const STAGES: usize = 6;

/// The teams a tournament holds (`teamIds[64]`, `groupMap[64]`, `teamIndexOrder[64]`).
pub const TEAMS: usize = 64;

/// The offset of the team ids (`teamIds`).
pub(super) const TEAM_IDS: usize = 154;

/// The offset of the first stage's slot (`slots`).
const SLOTS: usize = 346;

/// The bytes of a stage's slot (`DIYTournamentSlot`, 0x76).
const SLOT_LEN: usize = 118;

/// The offset of a slot's months (`progressionTable[90]`).
const SLOT_MONTHS: usize = 28;

/// Generates an indexed getter and setter for each field of a stage's slot, by the stage: a
/// stage past the six reaches the bytes after them, as upstream's code does.
macro_rules! slot_fields {
    ($($(#[$doc:meta])* $get:ident, $set:ident: $ty:ty = $offset:literal;)*) => {
        $(
            $(#[$doc])*
            ///
            /// # Panics
            ///
            /// When `stage` reaches past the image.
            #[must_use]
            pub fn $get(&self, stage: usize) -> $ty {
                self.get(SLOTS + stage * SLOT_LEN + $offset)
            }

            $(#[$doc])*
            ///
            /// # Panics
            ///
            /// When `stage` reaches past the image.
            pub fn $set(&mut self, stage: usize, value: $ty) {
                self.set(SLOTS + stage * SLOT_LEN + $offset, value);
            }
        )*
    };
}

/// A tournament's state: upstream's image of it, read and written field by field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TournamentState {
    /// The image.
    image: Box<[u8; STATE_LEN]>,
}

impl Default for TournamentState {
    /// A cleared one: every byte 0.
    fn default() -> Self {
        Self {
            image: Box::new([0; STATE_LEN]),
        }
    }
}

impl TournamentState {
    /// The state `image` holds.
    #[must_use]
    pub fn from_image(image: &[u8; STATE_LEN]) -> Self {
        Self {
            image: Box::new(*image),
        }
    }

    /// Its image.
    #[must_use]
    pub fn image(&self) -> &[u8; STATE_LEN] {
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

    header_fields!();

    // After the header.
    fields! {
        /// A league's warnings (`leagueWarningsEnabled`, the bytes a competition under way calls
        /// `minSubstitutes`).
        league_warnings_enabled, set_league_warnings_enabled: u16 = 104;
    }

    arrays! {
        /// The teams (`teamIds[64]`): each its country, then its ordinal.
        team, set_team: TeamBytes = 154;
        /// Each team's group, by its index (`groupMap[64]`).
        group, set_group: u8 = 282;
        /// The teams' order, by index (`teamIndexOrder[64]`): the seeds' and the draws'.
        team_index, set_team_index: u8 = 1054;
    }

    slot_fields! {
        /// The first slot's: whether a stage's bracket is doubled (`_pad_00`, written as a word
        /// on the first slot).
        slot_pad, set_slot_pad: u16 = 0;
        /// 1 when the stage's bracket is doubled (`doubleCount`).
        slot_double_count, set_slot_double_count: u16 = 2;
        /// Nonzero for a stage of groups, 0 a knockout (`isLeagueFormat`).
        slot_is_league_format, set_slot_is_league_format: u16 = 4;
        /// 0 when the stage's draw is a shuffle, otherwise seeded by value (`seedingType`).
        slot_seeding_type, set_slot_seeding_type: u16 = 6;
        /// The stage's teams (`numTeams`).
        slot_num_teams, set_slot_num_teams: u16 = 8;
        /// The teams that go through, the next stage's (`divisionFactor`).
        slot_division_factor, set_slot_division_factor: u16 = 10;
        /// How many times each team of a group plays the others (`playEachTeam`).
        slot_play_each_team, set_slot_play_each_team: u16 = 12;
        /// A win's points, a group stage's (`initConst168`).
        slot_points_for_win, set_slot_points_for_win: u16 = 14;
        /// The groups (`groups`): 0 a knockout.
        slot_groups, set_slot_groups: u16 = 16;
        /// The teams a group (`teamsPerGroup`).
        slot_teams_per_group, set_slot_teams_per_group: u16 = 18;
        /// A knockout's legs (`formatFlag16E`).
        slot_legs, set_slot_legs: u16 = 20;
        /// A knockout's extra time (`initConst170`).
        slot_extra_time, set_slot_extra_time: u16 = 22;
        /// A knockout's penalties (`formatFlag172`).
        slot_penalties, set_slot_penalties: u16 = 24;
        /// The stage's matches a team (`matchesPerSlot`).
        slot_matches, set_slot_matches: u16 = 26;
    }

    /// The offset of `stage`'s month `index`.
    fn month_offset(stage: usize, index: usize) -> usize {
        SLOTS + stage * SLOT_LEN + SLOT_MONTHS + index
    }

    /// Whether `stage`'s month `index` lies within the image (a stage's months run on into the
    /// next slot, as upstream's do).
    #[must_use]
    pub fn has_month(stage: usize, index: usize) -> bool {
        Self::month_offset(stage, index) < STATE_LEN
    }

    /// The month of `stage`'s match `index` (`progressionTable[90]`).
    ///
    /// # Panics
    ///
    /// When the month reaches past the image.
    #[must_use]
    pub fn slot_month(&self, stage: usize, index: usize) -> u8 {
        self.get(Self::month_offset(stage, index))
    }

    /// Sets the month of `stage`'s match `index`.
    ///
    /// # Panics
    ///
    /// When the month reaches past the image.
    pub fn set_slot_month(&mut self, stage: usize, index: usize, month: u8) {
        self.set(Self::month_offset(stage, index), month);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_image_reads_back_as_written() {
        let image: [u8; STATE_LEN] =
            std::array::from_fn(|index| u8::try_from(index % 251).expect("a byte"));
        let state = TournamentState::from_image(&image);
        assert_eq!(state.image(), &image);
        assert_eq!(TournamentState::default().image(), &[0; STATE_LEN]);
    }

    #[test]
    fn the_header_is_a_competition_s() {
        let mut state = TournamentState::default();
        state.set_num_teams(24);
        assert_eq!(&state.image()[54..56], &[24, 0]);
        state.set_total_matches_in_round(5);
        assert_eq!(&state.image()[86..88], &[5, 0]);
        let mut name = [0; NAME_LEN];
        name[..3].copy_from_slice(b"CUP");
        state.set_name(name);
        assert_eq!(&state.image()[4..8], b"CUP\0");
        state.set_league_warnings_enabled(0x0201);
        assert_eq!(&state.image()[104..106], &[1, 2]);
    }

    #[test]
    fn a_stage_s_fields_are_at_its_slot() {
        let mut state = TournamentState::default();
        state.set_slot_num_teams(1, 16);
        assert_eq!(&state.image()[SLOTS + SLOT_LEN + 8..][..2], &[16, 0]);
        state.set_slot_matches(5, 0x0302);
        assert_eq!(&state.image()[SLOTS + 5 * SLOT_LEN + 26..][..2], &[2, 3]);
        state.set_slot_month(0, 0, 0x38);
        assert_eq!(state.image()[SLOTS + SLOT_MONTHS], 0x38);
        assert_eq!(state.slot_month(0, 0), 0x38);
    }

    #[test]
    fn an_index_past_an_array_s_end_reaches_the_next_field() {
        let mut state = TournamentState::default();
        // Team 64 is past `teamIds[64]`: the group map's first two bytes.
        state.set_team(TEAMS, [7, 9]);
        assert_eq!((state.group(0), state.group(1)), (7, 9));
        // Month 90 of a stage is the next slot's first bytes.
        state.set_slot_month(0, 90, 1);
        assert_eq!(state.slot_pad(1), 1);
        // The seventh slot is the team order.
        state.set_slot_pad(STAGES, 0x0302);
        assert_eq!((state.team_index(0), state.team_index(1)), (2, 3));
        assert!(TournamentState::has_month(5, 0));
        assert!(!TournamentState::has_month(5, STATE_LEN));
    }
}
