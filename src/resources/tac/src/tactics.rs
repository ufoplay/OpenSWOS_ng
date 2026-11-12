//! The 370-byte tactics record (upstream `TeamTactics`, include/game_types.h).
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 9 | name |
//! | 9 | 10 × 35 | where each outfield player goes, per ball area ([`PlayerSpots`]) |
//! | 359 | 10 | the outfield players' pairs (upstream `pairIds`) |
//! | 369 | 1 | the tactics while the ball is out of play |

use openswos_common::{
    ByteReader, ByteWriter, FixedName, InvalidTactics, Nibble, ReadError, Tactics,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Bytes of a tactics record (upstream `sizeof(TeamTactics)`).
pub const TACTICS_RECORD_LEN: usize = 370;

/// Bytes of the name field.
pub const TACTICS_NAME_LEN: usize = 9;

/// The players a tactic places: the outfield ones. The goalkeeper follows the ball
/// (upstream `SetPlayerWithNoBallDestination`, src/player_positioning.c).
pub const OUTFIELD_PLAYERS: usize = 10;

/// Ball areas across the pitch: 4 limits make 5 (upstream src/ball_physics.c,
/// `_ballXQuadrantLimits`).
pub const BALL_AREA_COLUMNS: usize = 5;

/// Ball areas along the pitch: 6 limits make 7 (upstream src/ball_physics.c,
/// `_ballYQuadrantLimits`).
pub const BALL_AREA_ROWS: usize = 7;

/// The pair byte of a player in no pair (upstream `pairIds`: `0xFF` is unlinked).
const NO_PAIR: u8 = 0xFF;

/// A cell of the pitch grid, where a player goes (upstream src/player_positioning.c: x in
/// the high nibble of the byte, y in the low one).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Spot {
    /// Across the pitch.
    pub x: Nibble,
    /// Along the pitch.
    pub y: Nibble,
}

impl From<u8> for Spot {
    fn from(byte: u8) -> Self {
        let (x, y) = Nibble::split(byte);
        Self { x, y }
    }
}

impl From<Spot> for u8 {
    fn from(spot: Spot) -> Self {
        Nibble::join(spot.x, spot.y)
    }
}

/// Where an outfield player goes for each area the ball can be in, as `[row][column]`. The
/// ball's area is `row × 5 + column`: its band along the pitch, then across it (upstream
/// src/ball_physics.c, `_ballQuadrantIndex`). The C struct declares the same 35 bytes as
/// `[5][7]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerSpots(pub [[Spot; BALL_AREA_COLUMNS]; BALL_AREA_ROWS]);

impl PlayerSpots {
    fn read(reader: &mut ByteReader<'_>) -> Result<Self, ReadError> {
        let mut rows = [[Spot::default(); BALL_AREA_COLUMNS]; BALL_AREA_ROWS];
        for row in &mut rows {
            *row = reader.read_array::<BALL_AREA_COLUMNS>()?.map(Spot::from);
        }
        Ok(Self(rows))
    }

    fn write(&self, writer: &mut ByteWriter) {
        for row in &self.0 {
            writer.write_bytes(&row.map(u8::from));
        }
    }
}

/// The id of a pair of linked outfield players: those with the same id (upstream
/// `pairIds`, src/edit_tactics.c). Any byte but `0xFF`, which marks no pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct PairId(u8);

/// The byte that marks no pair, given as a pair id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0:#04x} marks no pair: it is no pair id")]
pub struct NotAPairId(pub u8);

impl PairId {
    /// The id.
    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }

    /// The pair a pair byte names, if any.
    fn of(byte: u8) -> Option<Self> {
        (byte != NO_PAIR).then_some(Self(byte))
    }

    /// The pair byte of `pair`.
    fn byte(pair: Option<Self>) -> u8 {
        pair.map_or(NO_PAIR, |pair| pair.0)
    }
}

impl TryFrom<u8> for PairId {
    type Error = NotAPairId;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Self::of(byte).ok_or(NotAPairId(byte))
    }
}

impl From<PairId> for u8 {
    fn from(pair: PairId) -> Self {
        pair.0
    }
}

/// Why a tactics record does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TacticsError {
    /// The data ended inside the record.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The ball-out-of-play tactics are not a slot.
    #[error("ball out of play: {0}")]
    BallOutOfPlay(#[from] InvalidTactics),
}

/// A tactic (upstream `TeamTactics`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamTactics {
    /// Its name.
    pub name: FixedName<TACTICS_NAME_LEN>,
    /// Where each outfield player goes, by the ball's area.
    pub players: [PlayerSpots; OUTFIELD_PLAYERS],
    /// Each outfield player's pair, if linked.
    pub pairs: [Option<PairId>; OUTFIELD_PLAYERS],
    /// The slot whose positions the team takes while the ball is out of play: the keeper
    /// holds it, a goal kick (upstream src/player_positioning.c). The game also orders the
    /// line-up by it (upstream src/match_sim.c).
    pub ball_out_of_play: Tactics,
}

impl TeamTactics {
    /// Reads a tactics record.
    ///
    /// # Errors
    ///
    /// When the data is too short or the ball-out-of-play tactics are not a slot.
    pub fn read(reader: &mut ByteReader<'_>) -> Result<Self, TacticsError> {
        let name = FixedName::from_bytes(reader.read_array()?);
        let players = {
            let mut players = [PlayerSpots::default(); OUTFIELD_PLAYERS];
            for spots in &mut players {
                *spots = PlayerSpots::read(reader)?;
            }
            players
        };
        let pairs = reader.read_array::<OUTFIELD_PLAYERS>()?.map(PairId::of);
        let ball_out_of_play = Tactics::try_from(reader.read_u8()?)?;
        Ok(Self {
            name,
            players,
            pairs,
            ball_out_of_play,
        })
    }

    /// Writes the tactics record.
    pub fn write(&self, writer: &mut ByteWriter) {
        let Self {
            name,
            players,
            pairs,
            ball_out_of_play,
        } = self;
        writer.write_bytes(name.as_bytes());
        for spots in players {
            spots.write(writer);
        }
        writer.write_bytes(&pairs.map(PairId::byte));
        writer.write_u8(u8::from(*ball_out_of_play));
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size test records
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Offset of the players' spots in a record.
    const PLAYERS_AT: usize = 9;

    /// Offset of the pairs in a record.
    const PAIRS_AT: usize = 359;

    /// Offset of the ball-out-of-play tactics in a record.
    const BALL_OUT_OF_PLAY_AT: usize = 369;

    /// Bytes of one player's spots.
    const SPOTS_LEN: usize = BALL_AREA_ROWS * BALL_AREA_COLUMNS;

    /// A record whose every spot is told apart, with a valid ball-out-of-play slot.
    fn record() -> Vec<u8> {
        let mut bytes = vec![0u8; TACTICS_RECORD_LEN];
        bytes[..5].copy_from_slice(b"4-4-2");
        for (index, byte) in bytes[PLAYERS_AT..PAIRS_AT].iter_mut().enumerate() {
            *byte = u8::try_from(index % 256).expect("a byte");
        }
        bytes[PAIRS_AT..BALL_OUT_OF_PLAY_AT].copy_from_slice(&[0, NO_PAIR, 1, 1, 0, 2, 2, 3, 3, 4]);
        bytes[BALL_OUT_OF_PLAY_AT] = 5;
        bytes
    }

    fn read(bytes: &[u8]) -> Result<TeamTactics, TacticsError> {
        TeamTactics::read(&mut ByteReader::new(bytes))
    }

    #[test]
    fn decodes_every_field() {
        let bytes = record();
        let tactics = read(&bytes).expect("a whole record");
        assert_eq!(tactics.name.text(), "4-4-2");
        // Player 2, ball in row 3, column 4: byte 2 × 35 + 3 × 5 + 4 of the spots.
        let offset = PLAYERS_AT + 2 * SPOTS_LEN + 3 * BALL_AREA_COLUMNS + 4;
        assert_eq!(tactics.players[2].0[3][4], Spot::from(bytes[offset]));
        assert_eq!(
            tactics.pairs.map(|pair| pair.map(PairId::value)),
            [
                Some(0),
                None,
                Some(1),
                Some(1),
                Some(0),
                Some(2),
                Some(2),
                Some(3),
                Some(3),
                Some(4)
            ]
        );
        assert_eq!(tactics.ball_out_of_play, Tactics::FourThreeThree);
    }

    #[test]
    fn a_spot_is_x_then_y() {
        let spot = Spot::from(0x4A);
        assert_eq!((spot.x.value(), spot.y.value()), (4, 10));
        assert_eq!(u8::from(spot), 0x4A);
    }

    #[test]
    fn encodes_back_the_same_bytes() {
        let bytes = record();
        let tactics = read(&bytes).expect("a whole record");
        let mut writer = ByteWriter::with_capacity(TACTICS_RECORD_LEN);
        tactics.write(&mut writer);
        assert_eq!(writer.into_bytes(), bytes);
    }

    #[test]
    fn an_invalid_ball_out_of_play_slot_is_reported() {
        let mut bytes = record();
        bytes[BALL_OUT_OF_PLAY_AT] = 18;
        let error = read(&bytes).expect_err("slot 18");
        assert_eq!(error, TacticsError::BallOutOfPlay(InvalidTactics(18)));
        assert_eq!(
            error.to_string(),
            "ball out of play: 18 is not a tactic slot"
        );
    }

    #[test]
    fn a_short_record_fails() {
        let bytes = record();
        let error = read(&bytes[..TACTICS_RECORD_LEN - 1]).expect_err("one byte short");
        assert!(
            matches!(error, TacticsError::Read(ReadError::UnexpectedEnd { .. })),
            "{error}"
        );
    }

    #[test]
    fn pair_ids_are_any_byte_but_no_pair() {
        assert_eq!(PairId::try_from(3).map(u8::from), Ok(3));
        assert_eq!(PairId::try_from(NO_PAIR), Err(NotAPairId(NO_PAIR)));
        assert_eq!(
            NotAPairId(NO_PAIR).to_string(),
            "0xff marks no pair: it is no pair id"
        );
        assert!(serde_json::from_str::<PairId>("255").is_err());
    }

    #[test]
    fn the_record_is_370_bytes() {
        assert_eq!(
            TACTICS_NAME_LEN + OUTFIELD_PLAYERS * SPOTS_LEN + OUTFIELD_PLAYERS + 1,
            TACTICS_RECORD_LEN
        );
    }

    /// The number of tactic slots.
    fn slot_count() -> u8 {
        u8::try_from(Tactics::ALL.len()).expect("fewer than 256 slots")
    }

    /// Any record whose ball-out-of-play slot is valid.
    fn any_record() -> impl Strategy<Value = Vec<u8>> {
        (
            proptest::collection::vec(any::<u8>(), TACTICS_RECORD_LEN),
            0..slot_count(),
        )
            .prop_map(|(mut bytes, slot)| {
                bytes[BALL_OUT_OF_PLAY_AT] = slot;
                bytes
            })
    }

    proptest! {
        #[test]
        fn any_valid_record_round_trips(bytes in any_record()) {
            let tactics = read(&bytes).expect("a valid record");
            let mut writer = ByteWriter::new();
            tactics.write(&mut writer);
            prop_assert_eq!(writer.into_bytes(), bytes);
        }
    }
}
