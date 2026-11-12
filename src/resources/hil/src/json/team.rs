//! A team in the JSON highlights (upstream `emit_team_game`, `load_team_game_slice`): the
//! [`TeamGame`] record, member by member. The kits' words and the marked player are
//! written unsigned, the three bytes after the name unsigned, the players' bytes signed,
//! the names up to their first NUL.

use openswos_common::FixedName;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::name;
use crate::team_game::{
    DosKit, DosPlayerGame, FULL_NAME_LEN, SHORT_NAME_LEN, TEAM_GAME_NAME_LEN, TEAM_GAME_PLAYERS,
    TeamGame,
};

/// The two kits, as the writer names their words.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KitsJson {
    pr_shirt_type: u16,
    pr_shirt_col: u16,
    pr_stripes_col: u16,
    pr_shorts_col: u16,
    pr_socks_col: u16,
    sec_shirt_type: u16,
    sec_shirt_col: u16,
    sec_stripes: u16,
    sec_shorts: u16,
    sec_socks: u16,
}

/// A player's match record, as the writer names its bytes.
#[derive(Serialize, Deserialize)]
#[serde(remote = "DosPlayerGame", deny_unknown_fields)]
struct PlayerGameDef {
    skip: i8,
    player_index: i8,
    goals_scored: i8,
    shirt_number: i8,
    position: i8,
    face: i8,
    is_injured: i8,
    field_7: i8,
    field_8: i8,
    previous_cards: i8,
    num_yellow_cards: i8,
    field_b: i8,
    #[serde(with = "name")]
    short_name: FixedName<SHORT_NAME_LEN>,
    passing: i8,
    shooting: i8,
    heading: i8,
    tackling: i8,
    ball_control: i8,
    speed: i8,
    finishing: i8,
    goalie_skill: i8,
    #[serde(rename = "injuries_bits")]
    injuries_bitfield: i8,
    half_played: i8,
    #[serde(rename = "face_2")]
    face2: i8,
    #[serde(with = "name")]
    full_name: FixedName<FULL_NAME_LEN>,
}

/// A player of the team's object.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct PlayerJson(#[serde(with = "PlayerGameDef")] DosPlayerGame);

/// The team's object, its members in the writer's order.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TeamJson {
    kit: KitsJson,
    marked_player: u16,
    #[serde(with = "name")]
    team_name: FixedName<TEAM_GAME_NAME_LEN>,
    reserved1: u8,
    num_own_goals: u8,
    reserved3: u8,
    players: [PlayerJson; TEAM_GAME_PLAYERS],
}

impl From<&TeamGame> for TeamJson {
    fn from(team: &TeamGame) -> Self {
        let TeamGame {
            primary_kit,
            secondary_kit,
            marked_player,
            name,
            field_27,
            num_own_goals,
            field_29,
            players,
        } = team;
        Self {
            kit: KitsJson {
                pr_shirt_type: primary_kit.shirt_type.cast_unsigned(),
                pr_shirt_col: primary_kit.shirt_colour.cast_unsigned(),
                pr_stripes_col: primary_kit.stripes_colour.cast_unsigned(),
                pr_shorts_col: primary_kit.shorts_colour.cast_unsigned(),
                pr_socks_col: primary_kit.socks_colour.cast_unsigned(),
                sec_shirt_type: secondary_kit.shirt_type.cast_unsigned(),
                sec_shirt_col: secondary_kit.shirt_colour.cast_unsigned(),
                sec_stripes: secondary_kit.stripes_colour.cast_unsigned(),
                sec_shorts: secondary_kit.shorts_colour.cast_unsigned(),
                sec_socks: secondary_kit.socks_colour.cast_unsigned(),
            },
            marked_player: marked_player.cast_unsigned(),
            team_name: *name,
            reserved1: *field_27,
            num_own_goals: num_own_goals.cast_unsigned(),
            reserved3: *field_29,
            players: players.clone().map(PlayerJson),
        }
    }
}

impl From<TeamJson> for TeamGame {
    fn from(team: TeamJson) -> Self {
        let TeamJson {
            kit,
            marked_player,
            team_name,
            reserved1,
            num_own_goals,
            reserved3,
            players,
        } = team;
        let KitsJson {
            pr_shirt_type,
            pr_shirt_col,
            pr_stripes_col,
            pr_shorts_col,
            pr_socks_col,
            sec_shirt_type,
            sec_shirt_col,
            sec_stripes,
            sec_shorts,
            sec_socks,
        } = kit;
        Self {
            primary_kit: DosKit {
                shirt_type: pr_shirt_type.cast_signed(),
                shirt_colour: pr_shirt_col.cast_signed(),
                stripes_colour: pr_stripes_col.cast_signed(),
                shorts_colour: pr_shorts_col.cast_signed(),
                socks_colour: pr_socks_col.cast_signed(),
            },
            secondary_kit: DosKit {
                shirt_type: sec_shirt_type.cast_signed(),
                shirt_colour: sec_shirt_col.cast_signed(),
                stripes_colour: sec_stripes.cast_signed(),
                shorts_colour: sec_shorts.cast_signed(),
                socks_colour: sec_socks.cast_signed(),
            },
            marked_player: marked_player.cast_signed(),
            name: team_name,
            field_27: reserved1,
            num_own_goals: num_own_goals.cast_signed(),
            field_29: reserved3,
            players: players.map(|PlayerJson(player)| player),
        }
    }
}

/// Writes `team` as the writer does: names up to their first NUL.
pub(super) fn serialize<S: Serializer>(team: &TeamGame, serializer: S) -> Result<S::Ok, S::Error> {
    TeamJson::from(team).serialize(serializer)
}

/// Reads a team; its names NUL-padded.
pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<TeamGame, D::Error> {
    TeamJson::deserialize(deserializer).map(TeamGame::from)
}
