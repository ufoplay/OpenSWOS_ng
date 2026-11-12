//! The JSON model to and from the typed team model (`openswos_team_resource`). The codes
//! and skills are checked as the JSON is read; every other range is checked here on the way
//! in. On the way out, nothing can fail: what the game does not read and JSON cannot keep
//! (bytes after a name's end, the passing byte's high nibble) is not written.

use openswos_common::{FixedName, NameError, Nibble, TeamFileNumber, TeamId, TeamOrdinal};
use openswos_team_resource::{
    Division, Form, GlobalTeamNumber, GoalCounts, Kit, Nationality, PLAYERS_PER_TEAM, Player,
    PlayerRuntime, PriceId, Skills, Team, TeamRuntime,
};
use thiserror::Error;
use uuid::Uuid;

use crate::ids::{derived_player_uuid, derived_team_uuid, team_checksum};
use crate::model::{PlayerEntry, TeamChecksum, TeamEntry, TeamHeader};
use openswos_common::json::{JsonByte, JsonWord};

/// The JSON's `cards`: bits 0-1 of the cards-and-injuries byte.
const CARDS_MASK: u8 = 0b11;

/// Where the JSON's `injury` starts: bits 2-4.
const INJURY_SHIFT: u32 = 2;

/// The JSON's `injury` once shifted down.
const INJURY_MASK: u8 = 0b111;

/// Where the JSON's `status_bucket` starts: bits 5-7.
const STATUS_SHIFT: u32 = 5;

/// The JSON's `status_bucket` once shifted down.
const STATUS_MASK: u8 = 0b111;

/// Why a team entry does not convert.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EntryError {
    /// A field's value is outside its range.
    #[error("{field}: {value} is out of range")]
    OutOfRange {
        /// The JSON field.
        field: &'static str,
        /// Its value.
        value: u8,
    },
    /// A name does not fit its field.
    #[error("{field}: {source}")]
    Name {
        /// The JSON field.
        field: &'static str,
        /// Why.
        source: NameError,
    },
}

/// The ids a team entry carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryIds {
    /// The team's id.
    pub team: Uuid,
    /// Its players' ids, in squad order.
    pub players: [Uuid; PLAYERS_PER_TEAM],
    /// Its checksum.
    pub checksum: TeamChecksum,
}

impl EntryIds {
    /// The ids an entry states.
    #[must_use]
    pub fn of(entry: &TeamEntry) -> Self {
        Self {
            team: entry.uuid,
            players: entry.players.each_ref().map(|player| player.uuid),
            checksum: entry.checksum.clone(),
        }
    }

    /// The derived ids of a team in the world tagged `tag`, and its checksum.
    #[must_use]
    pub fn derived(tag: &str, team: &Team) -> Self {
        Self {
            team: derived_team_uuid(tag, team.id),
            players: std::array::from_fn(|index| derived_player_uuid(tag, team.id, index)),
            checksum: team_checksum(team),
        }
    }
}

impl TeamEntry {
    /// The typed team.
    ///
    /// # Errors
    ///
    /// When a value is out of range or a name does not fit its field.
    pub fn to_team(&self) -> Result<Team, EntryError> {
        let TeamHeader {
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
        } = &self.team;
        let players: Vec<Player> = self.players.iter().map(player).collect::<Result<_, _>>()?;
        Ok(Team {
            id: TeamId {
                file: TeamFileNumber(*country_number),
                ordinal: TeamOrdinal(*team_ordinal),
            },
            global_number: GlobalTeamNumber(global_team_number.word().cast_signed()),
            name: name(team_name, "team_name")?,
            tactics: *tactics,
            division: Division(*league),
            primary_kit: Kit {
                shirt_type: *primary_shirt_type,
                stripes: *primary_stripes_color,
                basic: *primary_basic_color,
                shorts: *primary_shorts_color,
                socks: *primary_socks_color,
            },
            secondary_kit: Kit {
                shirt_type: *secondary_shirt_type,
                stripes: *secondary_stripes_color,
                basic: *secondary_basic_color,
                shorts: *secondary_shorts_color,
                socks: *secondary_socks_color,
            },
            coach: name(coach_name, "coach_name")?,
            player_order: *player_numbers,
            players: players
                .try_into()
                .expect("an entry has PLAYERS_PER_TEAM players"),
            runtime: TeamRuntime {
                controls: *team_controls,
                own_goals: *writes_zero_here,
                match_flags: *and_with_0xfe,
                player_values: *player_values_dirty,
            },
        })
    }

    /// The entry of `team`, with `ids`, its numbers written unsigned. What the game does not
    /// read is not written: names are their text (bytes after a name's end are dropped), and
    /// passing is its skill (the byte's high nibble is dropped).
    #[must_use]
    pub fn from_team(team: &Team, ids: EntryIds) -> Self {
        let Team {
            id,
            global_number: GlobalTeamNumber(global_number),
            name,
            tactics,
            division: Division(division),
            primary_kit,
            secondary_kit,
            coach,
            player_order,
            players,
            runtime:
                TeamRuntime {
                    controls,
                    own_goals,
                    match_flags,
                    player_values,
                },
        } = team;
        let EntryIds {
            team: team_uuid,
            players: player_uuids,
            checksum,
        } = ids;
        let entries: Vec<PlayerEntry> = players
            .iter()
            .zip(player_uuids)
            .map(|(player, uuid)| player_entry(player, uuid))
            .collect();
        Self {
            uuid: team_uuid,
            checksum,
            team: TeamHeader {
                country_number: id.file.0,
                team_ordinal: id.ordinal.0,
                global_team_number: JsonWord::from(global_number.cast_unsigned()),
                team_controls: *controls,
                team_name: name.text(),
                writes_zero_here: *own_goals,
                and_with_0xfe: *match_flags,
                tactics: *tactics,
                league: *division,
                primary_shirt_type: primary_kit.shirt_type,
                primary_stripes_color: primary_kit.stripes,
                primary_basic_color: primary_kit.basic,
                primary_shorts_color: primary_kit.shorts,
                primary_socks_color: primary_kit.socks,
                secondary_shirt_type: secondary_kit.shirt_type,
                secondary_stripes_color: secondary_kit.stripes,
                secondary_basic_color: secondary_kit.basic,
                secondary_shorts_color: secondary_kit.shorts,
                secondary_socks_color: secondary_kit.socks,
                coach_name: coach.text(),
                player_values_dirty: *player_values,
                player_numbers: *player_order,
            },
            players: entries
                .try_into()
                .expect("a team has PLAYERS_PER_TEAM players"),
        }
    }
}

/// The typed player of an entry.
fn player(entry: &PlayerEntry) -> Result<Player, EntryError> {
    let PlayerEntry {
        uuid: _,
        nationality,
        cached_match_value,
        shirt_number,
        player_name,
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
        goal_stats: [league, domestic_cup, league_cup, continental_cup],
        form,
    } = entry;
    Ok(Player {
        nationality: Nationality(nationality.byte()),
        shirt_number: shirt_number.byte(),
        name: name(player_name, "player_name")?,
        position: *position,
        face: *face,
        skills: Skills {
            passing: *passing,
            velocity: *velocity,
            heading: *heading,
            tackling: *tackling,
            control: *control,
            speed: *speed,
            finishing: *finishing,
            passing_high: Nibble::ZERO,
        },
        price: PriceId(price.byte()),
        runtime: PlayerRuntime {
            cached_match_value: cached_match_value.byte().cast_signed(),
            flags: *flags,
            cards_injuries: cards_injuries(*cards, *injury, *status_bucket)?,
            goals: GoalCounts {
                league: *league,
                domestic_cup: *domestic_cup,
                league_cup: *league_cup,
                continental_cup: *continental_cup,
            },
            form: Form::from(form.byte().cast_signed()),
        },
    })
}

/// The entry of a player.
fn player_entry(player: &Player, uuid: Uuid) -> PlayerEntry {
    let Player {
        nationality: Nationality(nationality),
        shirt_number,
        name,
        position,
        face,
        skills:
            Skills {
                passing,
                velocity,
                heading,
                tackling,
                control,
                speed,
                finishing,
                // Unused by the game, 0 in every file; the JSON keeps only the skill.
                passing_high: _,
            },
        price: PriceId(price),
        runtime:
            PlayerRuntime {
                cached_match_value,
                flags,
                cards_injuries,
                goals:
                    GoalCounts {
                        league,
                        domestic_cup,
                        league_cup,
                        continental_cup,
                    },
                form,
            },
    } = player;
    PlayerEntry {
        uuid,
        nationality: JsonByte::from(*nationality),
        cached_match_value: JsonByte::from(cached_match_value.cast_unsigned()),
        shirt_number: JsonByte::from(*shirt_number),
        player_name: name.text(),
        position: *position,
        face: *face,
        flags: *flags,
        cards: cards_injuries & CARDS_MASK,
        injury: (cards_injuries >> INJURY_SHIFT) & INJURY_MASK,
        status_bucket: (cards_injuries >> STATUS_SHIFT) & STATUS_MASK,
        passing: *passing,
        velocity: *velocity,
        heading: *heading,
        tackling: *tackling,
        control: *control,
        speed: *speed,
        finishing: *finishing,
        price: JsonByte::from(*price),
        goal_stats: [*league, *domestic_cup, *league_cup, *continental_cup],
        form: JsonByte::from(i8::from(*form).cast_unsigned()),
    }
}

/// The cards-and-injuries byte of the JSON's three fields.
fn cards_injuries(cards: u8, injury: u8, status_bucket: u8) -> Result<u8, EntryError> {
    let checked = |value: u8, mask: u8, field: &'static str| {
        if value <= mask {
            Ok(value)
        } else {
            Err(out_of_range(field, value))
        }
    };
    Ok(checked(cards, CARDS_MASK, "cards")?
        | (checked(injury, INJURY_MASK, "injury")? << INJURY_SHIFT)
        | (checked(status_bucket, STATUS_MASK, "status_bucket")? << STATUS_SHIFT))
}

fn name<const N: usize>(text: &str, field: &'static str) -> Result<FixedName<N>, EntryError> {
    FixedName::from_text(text).map_err(|source| EntryError::Name { field, source })
}

fn out_of_range(field: &'static str, value: u8) -> EntryError {
    EntryError::OutOfRange { field, value }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed indexes into fixed-size test squads
mod tests {
    use super::*;
    use crate::ids::WORLD_1996_97_TAG;
    use openswos_common::Importer;
    use openswos_team_resource::TeamFileFormat;
    use openswos_test_utils::assets_path;

    /// The four teams of the PC game's TEAM.085 (Oceania).
    fn oceania() -> Vec<Team> {
        let path = assets_path().join("TEAM").join("TEAM.085");
        let bytes = std::fs::read(&path).expect("the committed fixture");
        TeamFileFormat::import(&bytes)
            .expect("a valid team file")
            .teams
    }

    #[test]
    fn teams_survive_the_json() {
        for team in oceania() {
            let ids = EntryIds::derived(WORLD_1996_97_TAG, &team);
            let entry = TeamEntry::from_team(&team, ids.clone());
            assert_eq!(EntryIds::of(&entry), ids);
            assert_eq!(entry.to_team(), Ok(team.clone()), "{}", team.name);
        }
    }

    #[test]
    fn the_cards_byte_splits_as_the_json_splits_it() {
        let mut team = oceania().remove(0);
        // status 0b101 (bits 5-7), injury 0b011 (bits 2-4), cards 0b10 (bits 0-1)
        team.players[0].runtime.cards_injuries = 0b1010_1110;
        let entry = TeamEntry::from_team(&team, EntryIds::derived(WORLD_1996_97_TAG, &team));
        let player = &entry.players[0];
        assert_eq!(
            (player.cards, player.injury, player.status_bucket),
            (0b10, 0b011, 0b101)
        );
        assert_eq!(
            entry
                .to_team()
                .map(|team| team.players[0].runtime.cards_injuries),
            Ok(0b1010_1110)
        );
    }

    #[test]
    fn numbers_are_written_unsigned_and_read_either_way() {
        let mut team = oceania().remove(0);
        team.players[0].nationality = Nationality(200);
        let ids = EntryIds::derived(WORLD_1996_97_TAG, &team);
        let mut entry = TeamEntry::from_team(&team, ids);
        assert_eq!(entry.players[0].nationality.written(), 200);
        // OpenSWOS Setup writes the same byte signed.
        entry.players[0].nationality = JsonByte::try_from(-56_i16).expect("in range");
        assert_eq!(entry.to_team(), Ok(team));
    }

    #[test]
    fn out_of_range_values_are_named() {
        let team = oceania().remove(0);
        let ids = EntryIds::derived(WORLD_1996_97_TAG, &team);
        let entry = TeamEntry::from_team(&team, ids);
        let mut bad = entry.clone();
        bad.players[0].injury = 8;
        assert_eq!(
            bad.to_team(),
            Err(EntryError::OutOfRange {
                field: "injury",
                value: 8
            })
        );
        let mut bad = entry;
        bad.team.team_name = "A NAME FAR TOO LONG".to_owned();
        assert!(matches!(
            bad.to_team(),
            Err(EntryError::Name {
                field: "team_name",
                ..
            })
        ));
    }

    #[test]
    fn codes_outside_their_sets_do_not_read() {
        let team = oceania().remove(0);
        let entry = TeamEntry::from_team(&team, EntryIds::derived(WORLD_1996_97_TAG, &team));
        let json = serde_json::to_value(&entry).expect("serializes");
        for (pointer, value, message) in [
            ("/players/3/position", 8, "8 is not a player position"),
            ("/players/3/face", 4, "4 is not a face"),
            (
                "/players/3/flags",
                8,
                "0x08 holds bits that are not player flags",
            ),
            (
                "/players/0/passing",
                16,
                "16 is not a skill (0..=15: a level 0..=7, plus 8 when marked)",
            ),
            (
                "/players/0/heading",
                16,
                "16 is not a skill (0..=15: a level 0..=7, plus 8 when marked)",
            ),
            ("/team/secondary_shirt_type", 4, "4 is not a shirt type"),
            ("/team/primary_socks_color", 11, "11 is not a kit colour"),
            ("/team/tactics", 18, "18 is not a tactic slot"),
            ("/team/team_controls", 4, "4 is not a team control"),
            (
                "/team/player_values_dirty",
                2,
                "2 is neither clean (0) nor dirty (1) player values",
            ),
        ] {
            let mut bad = json.clone();
            *bad.pointer_mut(pointer).expect("a field") = value.into();
            let error = serde_json::from_value::<TeamEntry>(bad).expect_err(pointer);
            assert_eq!(error.to_string(), message, "{pointer}");
        }
    }

    #[test]
    fn the_form_byte_reads_its_statuses() {
        let mut team = oceania().remove(0);
        team.players[0].runtime.form = Form::Trialist;
        team.players[1].runtime.form = Form::Reserve;
        team.players[2].runtime.form = Form::from(-100);
        let entry = TeamEntry::from_team(&team, EntryIds::derived(WORLD_1996_97_TAG, &team));
        let written: Vec<i16> = entry.players[..3]
            .iter()
            .map(|player| player.form.written())
            .collect();
        assert_eq!(written, vec![0x9B, 0x9A, 156]);
        assert_eq!(entry.to_team(), Ok(team));
    }

    #[test]
    fn the_unused_passing_nibble_is_not_written() {
        let clean = oceania().remove(0);
        let mut team = clean.clone();
        team.players[2].skills.passing_high = Nibble::try_from(3).expect("a nibble");
        let entry = TeamEntry::from_team(&team, EntryIds::derived(WORLD_1996_97_TAG, &team));
        assert_eq!(entry.players[2].passing, clean.players[2].skills.passing);
        assert_eq!(entry.to_team(), Ok(clean));
    }

    #[test]
    fn a_negative_passing_does_not_read() {
        let team = oceania().remove(0);
        let entry = TeamEntry::from_team(&team, EntryIds::derived(WORLD_1996_97_TAG, &team));
        let json = serde_json::to_value(&entry).expect("serializes");
        let mut negative = json.clone();
        *negative
            .pointer_mut("/players/0/passing")
            .expect("a passing field") = (-1).into();
        assert!(serde_json::from_value::<TeamEntry>(negative).is_err());
    }

    #[test]
    fn bytes_after_a_name_are_not_written() {
        let clean = oceania().remove(0);
        let mut team = clean.clone();
        let mut bytes = *team.name.as_bytes();
        bytes[16] = b'X';
        team.name = FixedName::from_bytes(bytes);
        let mut bytes = *team.players[4].name.as_bytes();
        bytes[22] = b'Y';
        team.players[4].name = FixedName::from_bytes(bytes);
        assert!(!team.name.is_canonical());
        let entry = TeamEntry::from_team(&team, EntryIds::derived(WORLD_1996_97_TAG, &team));
        assert_eq!(entry.team.team_name, "AUSTRALIA");
        assert_eq!(entry.to_team(), Ok(clean));
    }
}
