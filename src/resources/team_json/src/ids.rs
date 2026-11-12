//! Team and player ids, and team checksums (upstream docs/TEAM_IDS.md).
//!
//! An id names fixed data. Every team JSON states its ids, and what a file states is what
//! is used, so ids are read and written as data. The binary team files have no room for
//! ids: these functions generate them when World 1996-97's team JSON is made from those
//! files (the same ids OpenSWOS gives the same teams), and give the checksum, which is
//! always derived from the team's data.

use openswos_common::TeamId;
use openswos_team_resource::{PLAYERS_PER_TEAM, Team};
use sha1::{Digest, Sha1};
use std::fmt::Write as _;
use uuid::Uuid;

use crate::model::{CHECKSUM_HEX_LEN, TeamChecksum};

/// The namespace of derived ids: `uuid5(NAMESPACE_URL, "https://us.swos.eu/team-data")`.
/// Never change it (docs/TEAM_IDS.md §3).
pub const TEAM_ID_NAMESPACE: Uuid = Uuid::from_u128(0xaf19_cc5a_6529_537b_935b_407b_d73b_75fb);

/// The world tag in the derived ids of World 1996-97's teams.
pub const WORLD_1996_97_TAG: &str = "96-97";

/// Bytes hashed per player by the checksum: position and the seven skill nibbles, then the
/// price.
const CHECKSUM_BYTES_PER_PLAYER: usize = 9;

/// The derived id of a team: `uuid5(namespace, "<tag>/team/<file>/<ordinal>")`.
#[must_use]
pub fn derived_team_uuid(tag: &str, id: TeamId) -> Uuid {
    Uuid::new_v5(&TEAM_ID_NAMESPACE, team_path(tag, id).as_bytes())
}

/// The derived id of a team's `index`-th player:
/// `uuid5(namespace, "<tag>/team/<file>/<ordinal>/player/<index>")`.
#[must_use]
pub fn derived_player_uuid(tag: &str, id: TeamId, index: usize) -> Uuid {
    let name = format!("{}/player/{index}", team_path(tag, id));
    Uuid::new_v5(&TEAM_ID_NAMESPACE, name.as_bytes())
}

fn team_path(tag: &str, id: TeamId) -> String {
    format!("{tag}/team/{}/{}", id.file.0, id.ordinal.0)
}

/// The team's checksum: the first 16 hex digits of the SHA-1 of, per player in order,
/// position, passing, velocity, heading, tackling, control, speed, finishing and price.
/// Names, faces, nationalities, shirt numbers and kits are not in it.
#[must_use]
pub fn team_checksum(team: &Team) -> TeamChecksum {
    let mut data = Vec::with_capacity(PLAYERS_PER_TEAM * CHECKSUM_BYTES_PER_PLAYER);
    for player in &team.players {
        let skills = player.skills;
        data.push(u8::from(player.position));
        data.extend(
            [
                skills.passing,
                skills.velocity,
                skills.heading,
                skills.tackling,
                skills.control,
                skills.speed,
                skills.finishing,
            ]
            .map(u8::from),
        );
        data.push(player.price.0);
    }
    let digest = Sha1::digest(&data);
    let mut hex = String::with_capacity(CHECKSUM_HEX_LEN);
    for byte in digest.iter().take(CHECKSUM_HEX_LEN / 2) {
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    TeamChecksum::try_from(hex).expect("16 lower-case hex digits")
}

#[cfg(test)]
mod tests {
    use super::*;
    use openswos_common::{TeamFileNumber, TeamOrdinal};

    fn team_id(file: u8, ordinal: u8) -> TeamId {
        TeamId {
            file: TeamFileNumber(file),
            ordinal: TeamOrdinal(ordinal),
        }
    }

    #[test]
    fn the_namespace_is_derived_from_its_url() {
        assert_eq!(
            TEAM_ID_NAMESPACE,
            Uuid::new_v5(&Uuid::NAMESPACE_URL, b"https://us.swos.eu/team-data")
        );
    }

    /// The example of docs/TEAM_IDS.md §2: BRAZIL (file 82, ordinal 2) and its first player.
    #[test]
    fn derived_ids_match_upstreams_example() {
        assert_eq!(
            derived_team_uuid(WORLD_1996_97_TAG, team_id(82, 2)).to_string(),
            "30e26617-4001-56c3-a068-8d88849c8398"
        );
        assert_eq!(
            derived_player_uuid(WORLD_1996_97_TAG, team_id(82, 2), 0).to_string(),
            "3d5c7af0-764f-5b50-95fe-7c859e4c452d"
        );
    }
}
