//! A team in a match (docs/PLAN.md M5 #8a): upstream's team record as a match reads and writes
//! it (`TeamFileEx`, include/game_types.h: a 76-byte `TeamFile` header, then each player's
//! 38-byte `PlayerFile`), held as its image, each field a typed accessor at its offset, as
//! [`RunningCompetition`](crate::competition::running::RunningCompetition) is: an index past an
//! array's end reaches the bytes after it, as the C does. A selected team's record is 684 bytes
//! (16 players); the career club's 1,292 (its 16 reserves after them), and its line-up is its
//! own (`_currentMatchPlayers`), not its record's.

use openswos_common::ByteWriter;
use openswos_team_resource::{
    PLAYER_RECORD_LEN, PLAYERS_PER_TEAM, TEAM_HEADER_LEN, TEAM_RECORD_LEN, Team,
};

use crate::competition::running::{Field, arrays, fields};

/// The bytes of a selected team's record (`sizeof(TeamFileEx)`).
pub const RECORD_LEN: usize = TEAM_RECORD_LEN;

/// The bytes of the career club's record (`CAREER_TEAM_BUF_SIZE`: its 16 reserves after the
/// record's players).
pub const CAREER_RECORD_LEN: usize = RECORD_LEN + PLAYER_LEN * PLAYERS;

/// The players a selected team's record holds.
pub const PLAYERS: usize = PLAYERS_PER_TEAM;

/// The places of the career club's line-up (`_currentMatchPlayers[32]`).
pub const CAREER_LINE_UP: usize = 32;

/// The offset of the first player's record (`sizeof(TeamFile)`).
const PLAYERS_OFFSET: usize = TEAM_HEADER_LEN;

/// The bytes of a player's record (`PLAYERFILE_SIZE`).
pub const PLAYER_LEN: usize = PLAYER_RECORD_LEN;

/// The offset of the line-up (`playerNumbers[16]`).
const LINE_UP: usize = 60;

/// The offset of a player's goals (`goalStats[4]`).
const GOAL_STATS: usize = 33;

/// The career club's line-up (upstream `_currentMatchPlayers`, `_careerNumPlayers`): each place
/// the player record it fields, and how many it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CareerLineUp {
    /// Each place's player record.
    pub order: [u8; CAREER_LINE_UP],
    /// The players the club holds.
    pub players: u16,
}

/// Generates an indexed getter and setter for each field of a player's record, by the record's
/// slot: a slot past the team's records reaches the bytes after them.
macro_rules! player_fields {
    ($($(#[$doc:meta])* $get:ident, $set:ident: $ty:ty = $offset:literal;)*) => {
        $(
            $(#[$doc])*
            ///
            /// # Panics
            ///
            /// When `slot` reaches past the image.
            #[must_use]
            pub fn $get(&self, slot: usize) -> $ty {
                self.get(PLAYERS_OFFSET + slot * PLAYER_LEN + $offset)
            }

            $(#[$doc])*
            ///
            /// # Panics
            ///
            /// When `slot` reaches past the image.
            pub fn $set(&mut self, slot: usize, value: $ty) {
                self.set(PLAYERS_OFFSET + slot * PLAYER_LEN + $offset, value);
            }
        )*
    };
}

/// A team in a match: upstream's image of its record, read and written field by field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchTeam {
    /// The image: a selected team's record, or the career club's.
    image: Box<[u8]>,
    /// The career club's line-up: none for a selected team.
    career: Option<CareerLineUp>,
}

impl MatchTeam {
    /// The selected team `image` holds.
    #[must_use]
    pub fn selected(image: &[u8; RECORD_LEN]) -> Self {
        Self {
            image: Box::new(*image),
            career: None,
        }
    }

    /// The selected team `team`: its record as a team file holds it, its global number in the
    /// host's order, as upstream loads it (src/teams.c).
    #[must_use]
    pub fn from_team(team: &Team) -> Self {
        let mut writer = ByteWriter::with_capacity(RECORD_LEN);
        team.write(&mut writer);
        let image: [u8; RECORD_LEN] = writer
            .into_bytes()
            .try_into()
            .expect("a team record's bytes");
        let mut selected = Self::selected(&image);
        selected.set_global_number(team.global_number.0);
        selected
    }

    /// The career club `image` holds, fielding `line_up`.
    #[must_use]
    pub fn career(image: &[u8; CAREER_RECORD_LEN], line_up: CareerLineUp) -> Self {
        Self {
            image: Box::new(*image),
            career: Some(line_up),
        }
    }

    /// Its image.
    #[must_use]
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    /// The career club's line-up: none for a selected team.
    #[must_use]
    pub fn career_line_up(&self) -> Option<&CareerLineUp> {
        self.career.as_ref()
    }

    /// Whether it is the career club (upstream compares the team's address with `_careerTeam`).
    #[must_use]
    pub fn is_career(&self) -> bool {
        self.career.is_some()
    }

    /// The player records its image holds.
    #[must_use]
    pub fn records(&self) -> usize {
        (self.image.len() - PLAYERS_OFFSET) / PLAYER_LEN
    }

    /// The record line-up place `place` fields (`playerNumbers`, the career club's
    /// `_currentMatchPlayers`), as the signed byte upstream reads: `None` past the line-up (the
    /// career club's 32; a selected team's within its image).
    #[must_use]
    pub fn line_up(&self, place: usize) -> Option<i8> {
        match &self.career {
            Some(line_up) => line_up.order.get(place).map(|&slot| slot.cast_signed()),
            None => (LINE_UP + place < self.image.len())
                .then(|| self.player_number(place).cast_signed()),
        }
    }

    /// The value at `offset`.
    fn get<T: Field>(&self, offset: usize) -> T {
        T::decode(&self.image[offset..offset + T::LEN])
    }

    /// Writes `value` at `offset`.
    fn set<T: Field>(&mut self, offset: usize, value: T) {
        value.encode(&mut self.image[offset..offset + T::LEN]);
    }

    fields! {
        /// The team file's country (`countryNumber`).
        country, set_country: u8 = 0;
        /// Its place in the file (`teamOrdinal`).
        ordinal, set_ordinal: u8 = 1;
        /// Its number among all the teams (`globalTeamNumber`).
        global_number, set_global_number: i16 = 2;
        /// Who controls it (`teamControls`: 1 the computer).
        controls, set_controls: u8 = 4;
        /// The goals it scored against itself in a simulated match (`writesZeroHere`).
        own_goals, set_own_goals: u8 = 22;
        /// A match's flags (`andWith0xFE`: bit 0 cleared before a simulated result).
        match_flags, set_match_flags: u8 = 23;
        /// Its tactics, by their place in the session's table (`tactics`).
        tactics, set_tactics: u8 = 24;
        /// Its division (`league`).
        league, set_league: u8 = 25;
        /// 1 when its players' match values must be reckoned again, 0 when they are kept
        /// (`playerValuesDirty`).
        values_dirty, set_values_dirty: u8 = 59;
    }

    arrays! {
        /// The record each line-up place fields (`playerNumbers[16]`).
        player_number, set_player_number: u8 = LINE_UP;
    }

    player_fields! {
        /// A player's nationality (`nationality`).
        nationality, set_nationality: i8 = 0;
        /// A player's match value, as last reckoned (`cachedMatchValue`).
        match_value, set_match_value: i8 = 1;
        /// A player's shirt number (`shirtNumber`).
        shirt, set_shirt: i8 = 2;
        /// A player's position (bits 7-5: 0 the goalkeeper to 7 the striker), looks (bits 4-3)
        /// and flags (bit 1 on loan) (`positionAndFace`).
        position_and_face, set_position_and_face: u8 = 26;
        /// A player's cards (bits 1-0), injuries (bits 4-2) and status bucket (bits 7-5)
        /// (`cardsInjuries`).
        cards_injuries, set_cards_injuries: u8 = 27;
        /// A player's passing, in the low nibble (`passing`).
        passing, set_passing: u8 = 28;
        /// A player's shooting power and heading, a nibble each (`velocityHeading`).
        velocity_heading, set_velocity_heading: u8 = 29;
        /// A player's tackling and ball control, a nibble each (`tacklingControl`).
        tackling_control, set_tackling_control: u8 = 30;
        /// A player's speed and finishing, a nibble each (`speedFinishing`).
        speed_finishing, set_speed_finishing: u8 = 31;
        /// A player's price, by its step (`price`).
        price, set_price: i8 = 32;
        /// A player's form (`form`).
        form, set_form: i8 = 37;
    }

    /// A player's goals in competition column `column` (`goalStats[4]`: a column past the four
    /// reaches the bytes after them).
    ///
    /// # Panics
    ///
    /// When the goals reach past the image.
    #[must_use]
    pub fn goals(&self, slot: usize, column: usize) -> u8 {
        self.get(PLAYERS_OFFSET + slot * PLAYER_LEN + GOAL_STATS + column)
    }

    /// Whether a player's goals in competition column `column` lie within the image.
    #[must_use]
    pub fn has_goals(&self, slot: usize, column: usize) -> bool {
        PLAYERS_OFFSET + slot * PLAYER_LEN + GOAL_STATS + column < self.image.len()
    }

    /// Sets a player's goals in competition column `column`.
    ///
    /// # Panics
    ///
    /// When the goals reach past the image.
    pub fn set_goals(&mut self, slot: usize, column: usize, goals: u8) {
        self.set(
            PLAYERS_OFFSET + slot * PLAYER_LEN + GOAL_STATS + column,
            goals,
        );
    }

    /// A player's four skill bytes (passing, shooting power and heading, tackling and control,
    /// speed and finishing), in their order.
    #[must_use]
    pub fn skills(&self, slot: usize) -> [u8; 4] {
        [
            self.passing(slot),
            self.velocity_heading(slot),
            self.tackling_control(slot),
            self.speed_finishing(slot),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An image whose byte `n` is `n` modulo 251.
    fn image<const N: usize>() -> [u8; N] {
        std::array::from_fn(|index| u8::try_from(index % 251).expect("a byte"))
    }

    #[test]
    fn the_fields_are_at_their_offsets() {
        let team = MatchTeam::selected(&image::<RECORD_LEN>());
        assert_eq!((team.country(), team.ordinal()), (0, 1));
        assert_eq!(team.global_number(), i16::from_le_bytes([2, 3]));
        assert_eq!((team.tactics(), team.values_dirty()), (24, 59));
        assert_eq!(team.player_number(0), 60);
        // The second player's record starts at 76 + 38.
        assert_eq!(team.match_value(1), 115);
        assert_eq!(team.position_and_face(1), 140);
        assert_eq!(team.skills(1), [142, 143, 144, 145]);
        assert_eq!(team.goals(1, 3), 150);
        assert_eq!(team.records(), PLAYERS);
        assert!(!team.is_career());
    }

    #[test]
    fn an_index_past_an_array_reaches_the_next_field() {
        let mut team = MatchTeam::selected(&[0; RECORD_LEN]);
        // Line-up place 16 is past `playerNumbers[16]`: the first player's nationality.
        team.set_player_number(16, 0xF0);
        assert_eq!(team.nationality(0), -16);
        assert_eq!(team.line_up(16), Some(-16));
        // Goal column 4 is past `goalStats[4]`: the player's form.
        team.set_goals(0, 4, 9);
        assert_eq!(team.form(0), 9);
        assert_eq!(team.line_up(RECORD_LEN), None);
    }

    #[test]
    fn the_career_club_fields_its_own_line_up_and_its_reserves() {
        let mut order = [0; CAREER_LINE_UP];
        order[0] = 20;
        let team = MatchTeam::career(
            &image::<CAREER_RECORD_LEN>(),
            CareerLineUp { order, players: 21 },
        );
        assert!(team.is_career());
        assert_eq!(team.records(), 2 * PLAYERS);
        assert_eq!(team.line_up(0), Some(20));
        assert_eq!(team.line_up(CAREER_LINE_UP), None);
        assert_eq!(
            team.career_line_up().map(|line_up| line_up.players),
            Some(21)
        );
    }
}
