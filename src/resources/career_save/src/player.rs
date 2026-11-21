//! A player as the save writes it: the 38-byte record (`PlayerFile`), field by field
//! (upstream `jb_player`, `s_player_fields`, src/save_format_json.c).
//!
//! ```text
//! {"nationality":18,"cached_match_value":1,"shirt_number":16,"player_name":"FILIPPO ROSSI",
//!  "position":3,"face":1,"flags":0,"cards":0,"injury":0,"status_bucket":0,"passing":4,
//!  "velocity":3,"heading":2,"tackling":5,"control":4,"speed":3,"finishing":2,"price":12,
//!  "goal_stats":[0,0,0,0],"form":0}
//! ```
//!
//! The bytes the writer prints signed (nationality, cached match value, shirt number,
//! passing, price, form) read with either sign. Passing is the whole byte, its unused high
//! nibble included.
//!
//! The name is printed from its first 22 bytes; the record's byte 25 (the name field's last)
//! is the SWOS 96/97 Mod's look (`openswos_team_resource::looks`): `"looks"` follows the name
//! when the name leaves room for it and it is a look, its look bits only. Reading, a name of
//! 23 bytes (a save made before looks) still fills the field, and `"looks"` is written into
//! byte 25 as it is.

use openswos_common::{ByteReader, ByteWriter, Nibble};
use openswos_save_json_resource::SaveString;
use openswos_team_resource::{LookBits, PLAYER_NAME_LEN, PLAYER_RECORD_LEN, Player};
use serde::{Deserialize, Serialize};

use crate::record::{RecordTextError, bits, name_slot, pack, slot_name};

/// The position's bits in the position-and-face byte (upstream `PF_POSITION_MASK`).
const POSITION_MASK: u8 = 0xE0;

/// The face's bits (upstream `PF_FACE_MASK`).
const FACE_MASK: u8 = 0x18;

/// The runtime flags' bits (upstream: `pf & 0x07`).
const FLAGS_MASK: u8 = 0x07;

/// The cards' bits in the cards-and-injuries byte (upstream `CI_*`, src/career_status.c).
const CARDS_MASK: u8 = 0x03;

/// The injury's bits.
const INJURY_MASK: u8 = 0x1C;

/// The status bucket's bits.
const STATUS_MASK: u8 = 0xE0;

/// Bytes of the goal counts.
const GOAL_STATS_LEN: usize = 4;

/// The name's bytes the writer prints (upstream `jb_key_str(..., 22)`): byte 25 is the look.
const PRINTED_NAME_LEN: usize = PLAYER_NAME_LEN - 1;

/// The name field's byte that holds the look (the record's byte 25).
const LOOK_BYTE: usize = PLAYER_NAME_LEN - 1;

/// The name's last letter: NUL when the name leaves room for a look.
const LAST_LETTER: usize = LOOK_BYTE - 1;

/// A player of a career save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "PlayerText", into = "PlayerText")]
pub struct SavePlayer(pub Player);

/// A player as the save names its fields.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlayerText {
    #[serde(with = "crate::signed")]
    nationality: u8,
    #[serde(with = "crate::signed")]
    cached_match_value: u8,
    #[serde(with = "crate::signed")]
    shirt_number: u8,
    player_name: SaveString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    looks: Option<LookBits>,
    position: u8,
    face: u8,
    flags: u8,
    cards: u8,
    injury: u8,
    status_bucket: u8,
    #[serde(with = "crate::signed")]
    passing: u8,
    velocity: Nibble,
    heading: Nibble,
    tackling: Nibble,
    control: Nibble,
    speed: Nibble,
    finishing: Nibble,
    #[serde(with = "crate::signed")]
    price: u8,
    goal_stats: [u8; GOAL_STATS_LEN],
    #[serde(with = "crate::signed")]
    form: u8,
}

impl PlayerText {
    /// The record the object describes.
    pub(crate) fn to_record(&self) -> Result<[u8; PLAYER_RECORD_LEN], RecordTextError> {
        let Self {
            nationality,
            cached_match_value,
            shirt_number,
            player_name,
            looks,
            position,
            face,
            flags,
            cards,
            injury,
            status_bucket,
            passing,
            velocity,
            heading,
            tackling,
            control,
            speed,
            finishing,
            price,
            goal_stats,
            form,
        } = self;
        let mut writer = ByteWriter::with_capacity(PLAYER_RECORD_LEN);
        writer.write_u8(*nationality);
        writer.write_u8(*cached_match_value);
        writer.write_u8(*shirt_number);
        let mut name = name_slot::<PLAYER_NAME_LEN>("player_name", player_name)?;
        if let (Some(bits), Some(byte)) = (looks, name.get_mut(LOOK_BYTE)) {
            *byte = bits.bits();
        }
        writer.write_bytes(&name);
        writer.write_u8(pack(&[
            (*position, POSITION_MASK, "position"),
            (*face, FACE_MASK, "face"),
            (*flags, FLAGS_MASK, "flags"),
        ])?);
        writer.write_u8(pack(&[
            (*cards, CARDS_MASK, "cards"),
            (*injury, INJURY_MASK, "injury"),
            (*status_bucket, STATUS_MASK, "status_bucket"),
        ])?);
        writer.write_u8(*passing);
        for (high, low) in [(velocity, heading), (tackling, control), (speed, finishing)] {
            writer.write_u8(Nibble::join(*high, *low));
        }
        writer.write_u8(*price);
        writer.write_bytes(goal_stats);
        writer.write_u8(*form);
        let bytes = writer.into_bytes();
        Ok(bytes
            .try_into()
            .expect("a player object makes a whole record"))
    }

    /// The object the writer prints for `record`.
    pub(crate) fn from_record(record: &[u8; PLAYER_RECORD_LEN]) -> Self {
        const WHOLE: &str = "a record holds every field";
        let mut reader = ByteReader::new(record);
        let nationality = reader.read_u8().expect(WHOLE);
        let cached_match_value = reader.read_u8().expect(WHOLE);
        let shirt_number = reader.read_u8().expect(WHOLE);
        let name: [u8; PLAYER_NAME_LEN] = reader.read_array().expect(WHOLE);
        let position_and_face = reader.read_u8().expect(WHOLE);
        let cards_injuries = reader.read_u8().expect(WHOLE);
        let passing = reader.read_u8().expect(WHOLE);
        let (velocity, heading) = Nibble::split(reader.read_u8().expect(WHOLE));
        let (tackling, control) = Nibble::split(reader.read_u8().expect(WHOLE));
        let (speed, finishing) = Nibble::split(reader.read_u8().expect(WHOLE));
        let look = LookBits::from(name.get(LOOK_BYTE).copied().unwrap_or_default()).look_bits();
        let room = name.get(LAST_LETTER) == Some(&0);
        Self {
            nationality,
            cached_match_value,
            shirt_number,
            player_name: slot_name(name.get(..PRINTED_NAME_LEN).unwrap_or_default()),
            looks: (room && !look.is_empty()).then_some(look),
            position: bits(position_and_face, POSITION_MASK),
            face: bits(position_and_face, FACE_MASK),
            flags: bits(position_and_face, FLAGS_MASK),
            cards: bits(cards_injuries, CARDS_MASK),
            injury: bits(cards_injuries, INJURY_MASK),
            status_bucket: bits(cards_injuries, STATUS_MASK),
            passing,
            velocity,
            heading,
            tackling,
            control,
            speed,
            finishing,
            price: reader.read_u8().expect(WHOLE),
            goal_stats: reader.read_array().expect(WHOLE),
            form: reader.read_u8().expect(WHOLE),
        }
    }
}

impl TryFrom<PlayerText> for SavePlayer {
    type Error = RecordTextError;

    fn try_from(text: PlayerText) -> Result<Self, Self::Error> {
        let record = text.to_record()?;
        Ok(Self(
            Player::read(&mut ByteReader::new(&record)).expect("a whole record reads"),
        ))
    }
}

impl From<SavePlayer> for PlayerText {
    fn from(player: SavePlayer) -> Self {
        Self::from_record(&player_record(&player.0))
    }
}

/// `player`'s record.
pub(crate) fn player_record(player: &Player) -> [u8; PLAYER_RECORD_LEN] {
    let mut writer = ByteWriter::with_capacity(PLAYER_RECORD_LEN);
    player.write(&mut writer);
    writer
        .into_bytes()
        .try_into()
        .expect("a player writes a whole record")
}

/// A player of the foreign transfer market (upstream `MarketPlayer`: a 4-byte header,
/// then the player).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketPlayer {
    /// The team the player comes from (upstream `teamId`).
    pub team_id: i16,
    /// The player's place in that team.
    #[serde(with = "crate::signed")]
    pub player_ordinal: u8,
    /// The header's last byte (upstream `padding`), kept as written.
    #[serde(with = "crate::signed")]
    pub padding: u8,
    /// The player.
    pub player: SavePlayer,
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size records
mod tests {
    use super::*;
    use openswos_save_json_resource::{Compact, from_save_bytes, to_save_bytes};
    use proptest::prelude::*;

    const TEXT: &str = r#"{"nationality":-56,"cached_match_value":1,"shirt_number":16,"player_name":"FILIPPO ROSSI","position":3,"face":1,"flags":2,"cards":1,"injury":5,"status_bucket":7,"passing":-108,"velocity":3,"heading":2,"tackling":13,"control":4,"speed":3,"finishing":2,"price":12,"goal_stats":[1,2,3,4],"form":-101}"#;

    fn write(player: &SavePlayer) -> String {
        String::from_utf8(to_save_bytes(player, &Compact).expect("writes")).expect("ASCII")
    }

    #[test]
    fn reads_the_record_the_writer_prints() {
        let player: SavePlayer = from_save_bytes(TEXT.as_bytes()).expect("a player");
        let record = player_record(&player.0);
        assert_eq!(record[0], 200, "nationality, written signed");
        assert_eq!(&record[3..16], b"FILIPPO ROSSI");
        assert_eq!(record[26], 0b0110_1010, "position 3, face 1, flags 2");
        assert_eq!(record[27], 0b1111_0101, "cards 1, injury 5, status 7");
        assert_eq!(record[28], 0x94, "passing: the whole byte");
        assert_eq!(record[30], 0xD4, "tackling 13 (marked 5), control 4");
        assert_eq!(&record[33..37], &[1, 2, 3, 4]);
        assert_eq!(record[37], 0x9B, "form: trialist");
        assert_eq!(write(&player), TEXT);
    }

    #[test]
    fn a_look_follows_the_name_in_byte_25() {
        let text = TEXT.replace(
            "\"player_name\":\"FILIPPO ROSSI\",",
            "\"player_name\":\"FILIPPO ROSSI\",\"looks\":3,",
        );
        let player: SavePlayer = from_save_bytes(text.as_bytes()).expect("a player");
        let record = player_record(&player.0);
        assert_eq!(record[3 + LOOK_BYTE], 3, "the look: shaved");
        assert_eq!(write(&player), text);
        // Bits 3-7 of byte 25 are not written; a full name leaves no room for a look.
        let mut bytes = record;
        bytes[3 + LOOK_BYTE] = 0xF9;
        let read = SavePlayer(Player::read(&mut ByteReader::new(&bytes)).expect("a record"));
        assert_eq!(write(&read), text.replace("\"looks\":3", "\"looks\":1"));
        bytes[3..3 + LOOK_BYTE].copy_from_slice(b"ABCDEFGHIJKLMNOPQRSTUV");
        let full = SavePlayer(Player::read(&mut ByteReader::new(&bytes)).expect("a record"));
        let written = write(&full);
        assert!(written.contains("\"player_name\":\"ABCDEFGHIJKLMNOPQRSTUV\",\"position\""));
        // A save made before looks: a name of 23 bytes fills the field.
        let old = TEXT.replace("FILIPPO ROSSI", "ABCDEFGHIJKLMNOPQRSTUVW");
        let read: SavePlayer = from_save_bytes(old.as_bytes()).expect("an older save's player");
        assert_eq!(&player_record(&read.0)[3..26], b"ABCDEFGHIJKLMNOPQRSTUVW");
    }

    #[test]
    fn what_has_no_place_in_the_record_does_not_read() {
        for (from, to) in [
            ("\"position\":3", "\"position\":8"),
            ("\"face\":1", "\"face\":4"),
            ("\"cards\":1", "\"cards\":4"),
            ("\"velocity\":3", "\"velocity\":16"),
            ("\"FILIPPO ROSSI\"", "\"FILIPPO ROSSI FILIPPO ROSSI\""),
            ("\"form\":-101", "\"form\":256"),
            ("\"form\":-101", "\"form\":-101,\"extra\":1"),
        ] {
            let text = TEXT.replace(from, to);
            assert!(
                from_save_bytes::<SavePlayer>(text.as_bytes()).is_err(),
                "{to}"
            );
        }
    }

    #[test]
    fn a_market_player_carries_its_header() {
        let text = format!(r#"{{"team_id":50,"player_ordinal":6,"padding":63,"player":{TEXT}}}"#);
        let player: MarketPlayer = from_save_bytes(text.as_bytes()).expect("a market player");
        assert_eq!(player.team_id, 50);
        assert_eq!(player.player_ordinal, 6);
        let bytes = to_save_bytes(&player, &Compact).expect("writes");
        assert_eq!(bytes, text.into_bytes());
    }

    proptest! {
        #[test]
        fn every_record_round_trips(record: [u8; PLAYER_RECORD_LEN]) {
            let player = SavePlayer(Player::read(&mut ByteReader::new(&record)).expect("a whole record"));
            let text = PlayerText::from(player);
            let expected = {
                // The writer prints a name from its first 22 bytes, up to its first NUL: the
                // bytes after it are zeroed. Byte 25 keeps its look bits when the name's last
                // letter leaves room.
                let mut expected = record;
                let look = record[3 + LOOK_BYTE] & LookBits::all().bits();
                let room = record[3 + LAST_LETTER] == 0;
                let name = &mut expected[3..3 + PRINTED_NAME_LEN];
                if let Some(end) = name.iter().position(|&byte| byte == 0) {
                    name[end..].fill(0);
                }
                expected[3 + LOOK_BYTE] = if room { look } else { 0 };
                expected
            };
            prop_assert_eq!(text.to_record().expect("a record"), expected);
            let bytes = to_save_bytes(&player, &Compact).expect("writes");
            let read: SavePlayer = from_save_bytes(&bytes).expect("reads back");
            prop_assert_eq!(player_record(&read.0), expected);
        }
    }
}
