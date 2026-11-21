//! A player's look: the SWOS 96/97 Mod's receding hairline, shaved head and white hair
//! (upstream include/player_looks.h, src/player_looks.c `PlayerLooks_*`).
//!
//! A look is in the player record's byte 25: the name field's last byte
//! (`PlayerFile.playerName[22]`), the name's terminator when it has its full 22 letters. So
//! only a record whose name is shorter has room for one. The original game's files hold none;
//! a world's data and its custom teams may.

use bitflags::bitflags;
use openswos_common::FixedName;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::player::{PLAYER_NAME_LEN, Player};

/// The name field's byte that holds the look (`playerName[22]`).
const LOOK_BYTE: usize = PLAYER_NAME_LEN - 1;

/// The name's last letter (`playerName[21]`): NUL when the name is shorter than 22 letters,
/// which leaves room for a look.
const LAST_LETTER: usize = LOOK_BYTE - 1;

bitflags! {
    /// The look byte (upstream `LOOKS_BIT_*`): bit 0 bald (alone: receding), bit 1 shaved
    /// (with bit 0), bit 2 white hair (bit 0 wins). Bits 3-7 are kept as they are.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct LookBits: u8 {
        /// Bald: a receding hairline alone, a shaved head with [`LookBits::SHAVED`].
        const BALD = 0b001;
        /// Shaved, with [`LookBits::BALD`].
        const SHAVED = 0b010;
        /// White hair, unless [`LookBits::BALD`].
        const WHITE = 0b100;
    }
}

impl LookBits {
    /// The look these bits draw (`PlayerLooks_FromBits`).
    #[must_use]
    pub fn look(self) -> Look {
        if self.contains(Self::BALD) {
            if self.contains(Self::SHAVED) {
                Look::Shaved
            } else {
                Look::Receding
            }
        } else if self.contains(Self::WHITE) {
            Look::White
        } else {
            Look::None
        }
    }

    /// The bits the Mod writes for `look` (`PlayerLooks_Bits`).
    #[must_use]
    pub fn of(look: Look) -> Self {
        match look {
            Look::None => Self::empty(),
            Look::Receding => Self::BALD,
            Look::Shaved => Self::BALD | Self::SHAVED,
            Look::White => Self::WHITE,
        }
    }

    /// Only the look's bits (`& LOOKS_BITS`): what the writers of the team JSON and the saves
    /// write.
    #[must_use]
    pub fn look_bits(self) -> Self {
        self.intersection(Self::all())
    }
}

impl From<u8> for LookBits {
    fn from(byte: u8) -> Self {
        Self::from_bits_retain(byte)
    }
}

impl From<LookBits> for u8 {
    fn from(bits: LookBits) -> Self {
        bits.bits()
    }
}

/// Written as its byte, as upstream writes it (`"looks": 5`).
impl Serialize for LookBits {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.bits())
    }
}

impl<'de> Deserialize<'de> for LookBits {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        u8::deserialize(deserializer).map(Self::from_bits_retain)
    }
}

/// A look (upstream `LOOK_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Look {
    /// The hair the face has.
    None,
    /// A receding hairline.
    Receding,
    /// A shaved head.
    Shaved,
    /// White hair.
    White,
}

impl Player {
    /// The look byte as the record holds it (byte 25, the name field's last byte), whatever
    /// the name.
    #[must_use]
    pub fn look_byte(&self) -> LookBits {
        LookBits::from_bits_retain(self.name_byte(LOOK_BYTE))
    }

    /// Whether the record has room for a look: a name shorter than 22 letters
    /// (`PlayerLooks_Room`).
    #[must_use]
    pub fn has_room_for_a_look(&self) -> bool {
        self.name_byte(LAST_LETTER) == 0
    }

    /// The player's look (`PlayerLooks_Of`): its bits' when the record has room for one,
    /// [`Look::None`] otherwise.
    #[must_use]
    pub fn look(&self) -> Look {
        if self.has_room_for_a_look() {
            self.look_byte().look()
        } else {
            Look::None
        }
    }

    /// Writes `look` into the record, bits 3-7 kept (`PlayerLooks_Set`); `false`, and nothing
    /// written, when the name leaves no room.
    pub fn set_look(&mut self, look: Look) -> bool {
        if !self.has_room_for_a_look() {
            return false;
        }
        let byte = self.look_byte().difference(LookBits::all()) | LookBits::of(look);
        self.set_look_byte(byte);
        true
    }

    /// Writes the look byte as it is, whatever the name (what a reader of the JSON does).
    pub fn set_look_byte(&mut self, bits: LookBits) {
        let mut bytes = *self.name.as_bytes();
        if let Some(byte) = bytes.get_mut(LOOK_BYTE) {
            *byte = bits.bits();
        }
        self.name = FixedName::from_bytes(bytes);
    }

    /// The name field's byte at `index` (an index of the field: always there).
    fn name_byte(&self, index: usize) -> u8 {
        self.name.as_bytes().get(index).copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PLAYER_RECORD_LEN;
    use openswos_common::ByteReader;

    fn player(name: &str) -> Player {
        let mut player =
            Player::read(&mut ByteReader::new(&[0; PLAYER_RECORD_LEN])).expect("a player");
        player.name = FixedName::from_text(name).expect("a name that fits");
        player
    }

    #[test]
    fn the_bits_draw_upstreams_looks() {
        for (byte, look) in [
            (0, Look::None),
            (1, Look::Receding),
            (3, Look::Shaved),
            (4, Look::White),
            // Bald wins over white; shaved alone is nothing.
            (5, Look::Receding),
            (2, Look::None),
            (0xF8, Look::None),
        ] {
            assert_eq!(LookBits::from(byte).look(), look, "{byte:#04x}");
        }
        for look in [Look::None, Look::Receding, Look::Shaved, Look::White] {
            assert_eq!(LookBits::of(look).look(), look);
        }
        assert_eq!(LookBits::from(0xFD).look_bits(), LookBits::from(5));
    }

    #[test]
    fn a_look_needs_a_name_shorter_than_22_letters() {
        let mut short = player("ROBERTO BAGGIO");
        assert!(short.has_room_for_a_look());
        assert_eq!(short.look(), Look::None);
        assert!(short.set_look(Look::Shaved));
        assert_eq!(short.look(), Look::Shaved);
        assert_eq!(short.name.text(), "ROBERTO BAGGIO");
        assert_eq!(short.look_byte(), LookBits::BALD | LookBits::SHAVED);

        let mut full = player("ABCDEFGHIJKLMNOPQRSTUV");
        assert!(!full.has_room_for_a_look());
        assert!(!full.set_look(Look::White));
        full.set_look_byte(LookBits::WHITE);
        // The byte is written, but a full name leaves no room: no look.
        assert_eq!(full.look_byte(), LookBits::WHITE);
        assert_eq!(full.look(), Look::None);
    }

    #[test]
    fn setting_a_look_keeps_the_other_bits() {
        let mut player = player("TOTTI");
        player.set_look_byte(LookBits::from(0b1000_0100));
        assert!(player.set_look(Look::Receding));
        assert_eq!(player.look_byte().bits(), 0b1000_0001);
        assert!(player.set_look(Look::None));
        assert_eq!(player.look_byte().bits(), 0b1000_0000);
    }

    #[test]
    fn the_bits_are_written_as_their_byte() {
        assert_eq!(
            serde_json::to_string(&LookBits::from(5)).expect("serializes"),
            "5"
        );
        assert_eq!(
            serde_json::from_str::<LookBits>("200").expect("a byte"),
            LookBits::from(200)
        );
        assert!(serde_json::from_str::<LookBits>("256").is_err());
    }
}
