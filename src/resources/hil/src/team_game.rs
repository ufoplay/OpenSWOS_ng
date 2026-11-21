//! A team in a match (`TeamGame`; upstream docs/TESTS.md §2, scripts/rgd_schema.py): kits,
//! the marked player, the name, the players' match data and names. The game's record is
//! 1018 bytes ([`TeamGame`]): little-endian in the DOS game, big-endian in OpenSWOS's own
//! files (a `.SRP`'s match). The DOS game keeps a 686-byte tail after it, 1704 bytes in all
//! ([`DosTeamGame`]).
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 2 × 10 | the primary and secondary kits (shirt type, shirt, stripes, shorts, socks) |
//! | 20 | 2 | the marked player |
//! | 22 | 17 | the name |
//! | 39 | 3 | a byte, the own goals, a byte |
//! | 42 | 16 × 61 | the players ([`DosPlayerGame`]) |
//! | 1018 | 686 | the DOS game's tail, kept as bytes ([`DosTeamGame`]) |

use openswos_common::{ByteReader, ByteWriter, FixedName, ReadError};

/// Bytes of a team's match record.
pub const TEAM_GAME_LEN: usize = 1018;

/// Bytes of the DOS game's team record, its tail included.
pub const DOS_TEAM_GAME_LEN: usize = TEAM_GAME_LEN + TEAM_GAME_TAIL_LEN;

/// The byte order of a team record's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WordOrder {
    /// Little-endian: the DOS game's.
    Little,
    /// Big-endian: OpenSWOS's files.
    Big,
}

/// Bytes of a player's match record.
pub const PLAYER_GAME_LEN: usize = 61;

/// The players of a team's match record.
pub const TEAM_GAME_PLAYERS: usize = 16;

/// Bytes of the team's name.
pub const TEAM_GAME_NAME_LEN: usize = 17;

/// Bytes of the DOS game's tail, after the players (swos-port `unknownTail`).
pub const TEAM_GAME_TAIL_LEN: usize = 686;

/// Bytes of a player's short name.
pub const SHORT_NAME_LEN: usize = 15;

/// Bytes of a player's full name.
pub const FULL_NAME_LEN: usize = 23;

/// A kit's colours, as words.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DosKit {
    /// The shirt's pattern.
    pub shirt_type: i16,
    /// The shirt's colour.
    pub shirt_colour: i16,
    /// The stripes' colour.
    pub stripes_colour: i16,
    /// The shorts' colour.
    pub shorts_colour: i16,
    /// The socks' colour.
    pub socks_colour: i16,
}

fn read_word(reader: &mut ByteReader<'_>, order: WordOrder) -> Result<i16, ReadError> {
    match order {
        WordOrder::Little => reader.read_i16_le(),
        WordOrder::Big => reader.read_i16_be(),
    }
}

fn write_word(writer: &mut ByteWriter, order: WordOrder, word: i16) {
    match order {
        WordOrder::Little => writer.write_i16_le(word),
        WordOrder::Big => writer.write_i16_be(word),
    }
}

impl DosKit {
    fn read(reader: &mut ByteReader<'_>, order: WordOrder) -> Result<Self, ReadError> {
        Ok(Self {
            shirt_type: read_word(reader, order)?,
            shirt_colour: read_word(reader, order)?,
            stripes_colour: read_word(reader, order)?,
            shorts_colour: read_word(reader, order)?,
            socks_colour: read_word(reader, order)?,
        })
    }

    fn write(&self, writer: &mut ByteWriter, order: WordOrder) {
        let Self {
            shirt_type,
            shirt_colour,
            stripes_colour,
            shorts_colour,
            socks_colour,
        } = self;
        for word in [
            shirt_type,
            shirt_colour,
            stripes_colour,
            shorts_colour,
            socks_colour,
        ] {
            write_word(writer, order, *word);
        }
    }
}

/// A player's match record (swos-port `PlayerGame`, 61 bytes).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DosPlayerGame {
    /// Skipped (not playing).
    pub skip: i8,
    /// The player's index.
    pub player_index: i8,
    /// Goals scored.
    pub goals_scored: i8,
    /// The shirt number.
    pub shirt_number: i8,
    /// The position.
    pub position: i8,
    /// The face.
    pub face: i8,
    /// Injured.
    pub is_injured: i8,
    /// Byte 7.
    pub field_7: i8,
    /// Byte 8.
    pub field_8: i8,
    /// The cards before the match.
    pub previous_cards: i8,
    /// Yellow cards.
    pub num_yellow_cards: i8,
    /// Byte 11.
    pub field_b: i8,
    /// The short name.
    pub short_name: FixedName<SHORT_NAME_LEN>,
    /// Passing.
    pub passing: i8,
    /// Shooting.
    pub shooting: i8,
    /// Heading.
    pub heading: i8,
    /// Tackling.
    pub tackling: i8,
    /// Ball control.
    pub ball_control: i8,
    /// Speed.
    pub speed: i8,
    /// Finishing.
    pub finishing: i8,
    /// The goalkeeper's skill.
    pub goalie_skill: i8,
    /// The injuries' bits.
    pub injuries_bitfield: i8,
    /// The half played.
    pub half_played: i8,
    /// The face, again.
    pub face2: i8,
    /// The full name.
    pub full_name: FixedName<FULL_NAME_LEN>,
}

impl DosPlayerGame {
    /// Reads a player's match record (all bytes: no word order).
    ///
    /// # Errors
    ///
    /// When the data is too short.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, ReadError> {
        let [
            skip,
            player_index,
            goals_scored,
            shirt_number,
            position,
            face,
            is_injured,
            field_7,
            field_8,
            previous_cards,
            num_yellow_cards,
            field_b,
        ] = reader.read_array()?.map(u8::cast_signed);
        let short_name = FixedName::from_bytes(reader.read_array()?);
        let [
            passing,
            shooting,
            heading,
            tackling,
            ball_control,
            speed,
            finishing,
            goalie_skill,
            injuries_bitfield,
            half_played,
            face2,
        ] = reader.read_array()?.map(u8::cast_signed);
        Ok(Self {
            skip,
            player_index,
            goals_scored,
            shirt_number,
            position,
            face,
            is_injured,
            field_7,
            field_8,
            previous_cards,
            num_yellow_cards,
            field_b,
            short_name,
            passing,
            shooting,
            heading,
            tackling,
            ball_control,
            speed,
            finishing,
            goalie_skill,
            injuries_bitfield,
            half_played,
            face2,
            full_name: FixedName::from_bytes(reader.read_array()?),
        })
    }

    /// Writes the player's match record.
    pub fn write(&self, writer: &mut ByteWriter) {
        let Self {
            skip,
            player_index,
            goals_scored,
            shirt_number,
            position,
            face,
            is_injured,
            field_7,
            field_8,
            previous_cards,
            num_yellow_cards,
            field_b,
            short_name,
            passing,
            shooting,
            heading,
            tackling,
            ball_control,
            speed,
            finishing,
            goalie_skill,
            injuries_bitfield,
            half_played,
            face2,
            full_name,
        } = self;
        writer.write_bytes(
            &[
                *skip,
                *player_index,
                *goals_scored,
                *shirt_number,
                *position,
                *face,
                *is_injured,
                *field_7,
                *field_8,
                *previous_cards,
                *num_yellow_cards,
                *field_b,
            ]
            .map(i8::cast_unsigned),
        );
        writer.write_bytes(short_name.as_bytes());
        writer.write_bytes(
            &[
                *passing,
                *shooting,
                *heading,
                *tackling,
                *ball_control,
                *speed,
                *finishing,
                *goalie_skill,
                *injuries_bitfield,
                *half_played,
                *face2,
            ]
            .map(i8::cast_unsigned),
        );
        writer.write_bytes(full_name.as_bytes());
    }
}

/// A team's match record (`TeamGame`, 1018 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TeamGame {
    /// The first kit.
    pub primary_kit: DosKit,
    /// The second kit.
    pub secondary_kit: DosKit,
    /// The marked player.
    pub marked_player: i16,
    /// The name.
    pub name: FixedName<TEAM_GAME_NAME_LEN>,
    /// The byte after the name (offset 0x27).
    pub field_27: u8,
    /// The own goals.
    pub num_own_goals: i8,
    /// The byte after them (offset 0x29).
    pub field_29: u8,
    /// The players.
    pub players: [DosPlayerGame; TEAM_GAME_PLAYERS],
}

impl TeamGame {
    /// Reads a team's match record, its words in `order`.
    ///
    /// # Errors
    ///
    /// When the data is short.
    pub fn read(reader: &mut ByteReader<'_>, order: WordOrder) -> Result<Self, ReadError> {
        let primary_kit = DosKit::read(reader, order)?;
        let secondary_kit = DosKit::read(reader, order)?;
        let marked_player = read_word(reader, order)?;
        let name = FixedName::from_bytes(reader.read_array()?);
        let field_27 = reader.read_u8()?;
        let num_own_goals = reader.read_i8()?;
        let field_29 = reader.read_u8()?;
        let mut players = Vec::with_capacity(TEAM_GAME_PLAYERS);
        for _ in 0..TEAM_GAME_PLAYERS {
            players.push(DosPlayerGame::read(reader)?);
        }
        Ok(Self {
            primary_kit,
            secondary_kit,
            marked_player,
            name,
            field_27,
            num_own_goals,
            field_29,
            players: players.try_into().expect("16 players were read"),
        })
    }

    /// Writes the record, its words in `order`.
    pub fn write(&self, writer: &mut ByteWriter, order: WordOrder) {
        let Self {
            primary_kit,
            secondary_kit,
            marked_player,
            name,
            field_27,
            num_own_goals,
            field_29,
            players,
        } = self;
        primary_kit.write(writer, order);
        secondary_kit.write(writer, order);
        write_word(writer, order, *marked_player);
        writer.write_bytes(name.as_bytes());
        writer.write_u8(*field_27);
        writer.write_i8(*num_own_goals);
        writer.write_u8(*field_29);
        for player in players {
            player.write(writer);
        }
    }
}

/// A team's match record as the DOS game holds it: little-endian, and a tail after it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DosTeamGame {
    /// The record.
    pub game: TeamGame,
    /// The DOS game's tail (686 bytes; swos-port `unknownTail`), kept as written.
    pub tail: Vec<u8>,
}

impl DosTeamGame {
    /// Reads the DOS game's team record.
    ///
    /// # Errors
    ///
    /// When the data is short.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, ReadError> {
        Ok(Self {
            game: TeamGame::read(reader, WordOrder::Little)?,
            tail: reader.read_bytes(TEAM_GAME_TAIL_LEN)?.to_vec(),
        })
    }

    /// Writes the record. The tail must be its 686 bytes.
    pub(crate) fn write(&self, writer: &mut ByteWriter) {
        self.game.write(writer, WordOrder::Little);
        writer.write_bytes(&self.tail);
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into bytes the tests build
mod tests {
    use super::*;

    #[test]
    fn a_dos_record_is_1704_bytes_and_round_trips() {
        let mut bytes = vec![0; DOS_TEAM_GAME_LEN];
        bytes.iter_mut().enumerate().for_each(|(index, byte)| {
            *byte = u8::try_from(index % 251).expect("a byte");
        });
        let team = DosTeamGame::read(&mut ByteReader::new(&bytes)).expect("a team");
        let mut writer = ByteWriter::new();
        team.write(&mut writer);
        assert_eq!(writer.into_bytes(), bytes);
        assert_eq!(team.tail.len(), TEAM_GAME_TAIL_LEN);
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into a record the test builds
mod order_tests {
    use super::*;

    #[test]
    fn the_word_order_is_the_kits_and_marked_player() {
        let mut bytes = vec![0; TEAM_GAME_LEN];
        bytes[0] = 1;
        bytes[20] = 2;
        let little =
            TeamGame::read(&mut ByteReader::new(&bytes), WordOrder::Little).expect("a team");
        let big = TeamGame::read(&mut ByteReader::new(&bytes), WordOrder::Big).expect("a team");
        assert_eq!(
            (little.primary_kit.shirt_type, little.marked_player),
            (1, 2)
        );
        assert_eq!(
            (big.primary_kit.shirt_type, big.marked_player),
            (0x100, 0x200)
        );
        let mut writer = ByteWriter::new();
        big.write(&mut writer, WordOrder::Big);
        assert_eq!(writer.into_bytes(), bytes);
    }
}
