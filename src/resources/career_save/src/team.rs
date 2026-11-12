//! A team as the save writes it: the 684-byte record (`TeamFileEx`), its header field by
//! field and its 16 players (upstream `emit_team_file_ex_body`, `s_team_file_fields`,
//! src/save_format_json.c).
//!
//! ```text
//! {"team":{"country_number":20,"team_ordinal":191,"global_team_number":575,
//!   "team_controls":2,"team_name":"L'AQUILA",…,"player_numbers":[0,1,…,15]},
//!  "players":[{…},… 16 players]}
//! ```

use openswos_common::{ByteReader, ByteWriter};
use openswos_save_json_resource::SaveString;
use openswos_team_resource::{
    COACH_NAME_LEN, PLAYER_RECORD_LEN, PLAYERS_PER_TEAM, TEAM_NAME_LEN, TEAM_RECORD_LEN, Team,
};
use serde::{Deserialize, Serialize};

use crate::player::PlayerText;
use crate::record::{RecordTextError, name_slot, slot_name};

/// A team of a career save.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "TeamText", into = "TeamText")]
pub struct SaveTeam(pub Team);

/// A team as the save names its fields.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TeamText {
    team: HeaderText,
    players: [PlayerText; PLAYERS_PER_TEAM],
}

/// A team's header as the save names its fields.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeaderText {
    country_number: u8,
    team_ordinal: u8,
    global_team_number: i16,
    team_controls: u8,
    team_name: SaveString,
    writes_zero_here: u8,
    and_with_0xfe: u8,
    tactics: u8,
    league: u8,
    primary_shirt_type: u8,
    primary_stripes_color: u8,
    primary_basic_color: u8,
    primary_shorts_color: u8,
    primary_socks_color: u8,
    secondary_shirt_type: u8,
    secondary_stripes_color: u8,
    secondary_basic_color: u8,
    secondary_shorts_color: u8,
    secondary_socks_color: u8,
    coach_name: SaveString,
    player_values_dirty: u8,
    player_numbers: [u8; PLAYERS_PER_TEAM],
}

impl TryFrom<TeamText> for SaveTeam {
    type Error = RecordTextError;

    fn try_from(text: TeamText) -> Result<Self, Self::Error> {
        let TeamText { team, players } = text;
        let HeaderText {
            country_number,
            team_ordinal,
            global_team_number,
            team_controls,
            team_name,
            writes_zero_here,
            and_with_0xfe,
            tactics,
            league,
            primary_shirt_type,
            primary_stripes_color,
            primary_basic_color,
            primary_shorts_color,
            primary_socks_color,
            secondary_shirt_type,
            secondary_stripes_color,
            secondary_basic_color,
            secondary_shorts_color,
            secondary_socks_color,
            coach_name,
            player_values_dirty,
            player_numbers,
        } = team;
        let mut writer = ByteWriter::with_capacity(TEAM_RECORD_LEN);
        writer.write_bytes(&[country_number, team_ordinal]);
        // The team record holds the number big-endian, as the team files do.
        writer.write_i16_be(global_team_number);
        writer.write_u8(team_controls);
        writer.write_bytes(&name_slot::<TEAM_NAME_LEN>("team_name", &team_name)?);
        writer.write_bytes(&[
            writes_zero_here,
            and_with_0xfe,
            tactics,
            league,
            primary_shirt_type,
            primary_stripes_color,
            primary_basic_color,
            primary_shorts_color,
            primary_socks_color,
            secondary_shirt_type,
            secondary_stripes_color,
            secondary_basic_color,
            secondary_shorts_color,
            secondary_socks_color,
        ]);
        writer.write_bytes(&name_slot::<COACH_NAME_LEN>("coach_name", &coach_name)?);
        writer.write_u8(player_values_dirty);
        writer.write_bytes(&player_numbers);
        for player in &players {
            writer.write_bytes(&player.to_record()?);
        }
        let bytes = writer.into_bytes();
        Ok(Self(Team::read(&mut ByteReader::new(&bytes))?))
    }
}

impl From<SaveTeam> for TeamText {
    fn from(team: SaveTeam) -> Self {
        const WHOLE: &str = "a team record holds every field";
        let mut writer = ByteWriter::with_capacity(TEAM_RECORD_LEN);
        team.0.write(&mut writer);
        let bytes = writer.into_bytes();
        let mut reader = ByteReader::new(&bytes);
        let [country_number, team_ordinal] = reader.read_array().expect(WHOLE);
        let global_team_number = reader.read_i16_be().expect(WHOLE);
        let team_controls = reader.read_u8().expect(WHOLE);
        let team_name: [u8; TEAM_NAME_LEN] = reader.read_array().expect(WHOLE);
        let [
            writes_zero_here,
            and_with_0xfe,
            tactics,
            league,
            primary_shirt_type,
            primary_stripes_color,
            primary_basic_color,
            primary_shorts_color,
            primary_socks_color,
            secondary_shirt_type,
            secondary_stripes_color,
            secondary_basic_color,
            secondary_shorts_color,
            secondary_socks_color,
        ] = reader.read_array().expect(WHOLE);
        let coach_name: [u8; COACH_NAME_LEN] = reader.read_array().expect(WHOLE);
        let player_values_dirty = reader.read_u8().expect(WHOLE);
        let player_numbers = reader.read_array().expect(WHOLE);
        let players = std::array::from_fn(|_| {
            let record: [u8; PLAYER_RECORD_LEN] = reader.read_array().expect(WHOLE);
            PlayerText::from_record(&record)
        });
        Self {
            team: HeaderText {
                country_number,
                team_ordinal,
                global_team_number,
                team_controls,
                team_name: slot_name(&team_name),
                writes_zero_here,
                and_with_0xfe,
                tactics,
                league,
                primary_shirt_type,
                primary_stripes_color,
                primary_basic_color,
                primary_shorts_color,
                primary_socks_color,
                secondary_shirt_type,
                secondary_stripes_color,
                secondary_basic_color,
                secondary_shorts_color,
                secondary_socks_color,
                coach_name: slot_name(&coach_name),
                player_values_dirty,
                player_numbers,
            },
            players,
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into fixed-size records
mod tests {
    use super::*;
    use openswos_save_json_resource::{Compact, from_save_bytes, to_save_bytes};
    use openswos_team_resource::{KitColour, TeamControls};

    /// A team record: a name, a coach, kit colours, a control, the rest zero.
    fn record() -> Vec<u8> {
        let mut record = vec![0; TEAM_RECORD_LEN];
        record[0] = 20;
        record[1] = 191;
        record[2..4].copy_from_slice(&575i16.to_be_bytes());
        record[4] = 2;
        record[5..13].copy_from_slice(b"L'AQUILA");
        record[28] = 10;
        record[36..41].copy_from_slice(b"COACH");
        for (index, order) in record[60..76].iter_mut().enumerate() {
            *order = u8::try_from(index).expect("16 players");
        }
        record[76 + 3..76 + 8].copy_from_slice(b"ROSSI");
        record
    }

    fn team() -> SaveTeam {
        SaveTeam(Team::read(&mut ByteReader::new(&record())).expect("a team"))
    }

    #[test]
    fn writes_the_header_field_by_field() {
        let bytes = to_save_bytes(&team(), &Compact).expect("writes");
        let text = String::from_utf8(bytes.clone()).expect("ASCII");
        assert!(
            text.starts_with(r#"{"team":{"country_number":20,"team_ordinal":191,"global_team_number":575,"team_controls":2,"team_name":"L'AQUILA","writes_zero_here":0,"and_with_0xfe":0,"tactics":0,"league":0,"primary_shirt_type":0,"primary_stripes_color":0,"primary_basic_color":10,"#),
            "{text}"
        );
        assert!(text.contains(r#""coach_name":"COACH","player_values_dirty":0,"player_numbers":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]},"players":[{"nationality":0"#));
        let read: SaveTeam = from_save_bytes(&bytes).expect("reads back");
        assert_eq!(read, team());
        assert_eq!(read.0.runtime.controls, TeamControls::PlayerCoach);
        assert_eq!(read.0.primary_kit.basic, KitColour::Purple);
        assert_eq!(read.0.players[0].name.text(), "ROSSI");
    }

    #[test]
    fn codes_outside_their_sets_do_not_read() {
        let bytes = to_save_bytes(&team(), &Compact).expect("writes");
        let text = String::from_utf8(bytes).expect("ASCII");
        for (from, to) in [
            ("\"tactics\":0", "\"tactics\":18"),
            ("\"team_controls\":2", "\"team_controls\":4"),
            ("\"L'AQUILA\"", "\"L'AQUILA L'AQUILA L'AQUILA\""),
        ] {
            assert!(
                from_save_bytes::<SaveTeam>(text.replace(from, to).as_bytes()).is_err(),
                "{to}"
            );
        }
    }
}
