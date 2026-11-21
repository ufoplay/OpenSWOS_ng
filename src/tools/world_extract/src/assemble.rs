//! World 1996-97, assembled (docs/PLAN.md §5.2): the committed package (`data/worlds/1996-97/`:
//! upstream's competitions, its career params) with the rosters and the foreign players' pool
//! of your copy of the PC game. The game's data stays in `target/cache/`: never committed.

use std::collections::BTreeMap;
use std::path::Path;

use openswos_common::{Exporter, TeamFileNumber};
use openswos_core::World;
use openswos_team_json_resource::{
    EntryIds, TeamJsonDocument, TeamJsonFile, WORLD_1996_97_TAG, WORLD_PACKAGE_INDENT,
    WORLD_PACKAGE_NON_ASCII, WORLD_PACKAGE_TRAILING_NEWLINE,
};
use openswos_world_package_resource::{
    Fingerprint, PackageFiles, WORLD_INI, WorldIdentity, WorldIni, WorldIniFormat, WorldPackage,
    WorldTag,
};

use crate::error::ExtractError;
use crate::game::GameCopy;

/// World 1996-97's tag: 96/97 is upstream's world 0, `"9697"` (src/world.c `kTags`).
pub const TAG: &str = "9697";

/// World 1996-97's name, as World 2026-27's package names itself (`name = World 2026-27`).
pub const NAME: &str = "World 1996-97";

/// The comment its `WORLD.ini` starts with, after the `;`: what the package holds.
const INI_COMMENT: &str = " The original game's world, assembled by OpenSWOS_ng's world_extract \
                           from your copy of the PC game: its rosters are the game's, not to be \
                           shared.";

/// A roster to make: the country's number (its team file's), its name, its file in `teams/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterSource {
    /// The country's number: `DATA/TEAM.<number>` holds its clubs.
    pub number: TeamFileNumber,
    /// Its name (`ENGLAND`), the roster's `country_name`.
    pub name: String,
    /// The roster's file name in `teams/` (`ENGLAND.json`).
    pub file: String,
}

/// World 1996-97's identity.
#[must_use]
pub fn identity() -> WorldIdentity {
    WorldIdentity {
        comments: vec![INI_COMMENT.to_owned()],
        tag: WorldTag(TAG.to_owned()),
        name: NAME.to_owned(),
        other: Vec::new(),
    }
}

/// World 1996-97 assembled: the committed package in `committed_dir`, the rosters of its
/// countries from `game`'s team files, and `game`'s pool.
///
/// # Errors
///
/// When the committed package does not open or is not a valid world, or the game lacks a
/// file or holds one that does not decode.
pub fn world_1996_97(committed_dir: &Path, game: &GameCopy) -> Result<WorldPackage, ExtractError> {
    let mut package = committed(committed_dir)?;
    let world = World::from_package(&package)?;
    package.teams = rosters(&roster_sources(&world), game)?;
    package.pool = Some(game.pool()?);
    Ok(package)
}

/// The committed package in `dir`, which holds no `WORLD.ini` (its fingerprint covers the
/// rosters, which only the assembled package has): its identity is [`identity`].
///
/// # Errors
///
/// When a file does not read, or is not one a package holds.
pub fn committed(dir: &Path) -> Result<WorldPackage, ExtractError> {
    let mut files = PackageFiles::read_dir(dir)?;
    let ini = WorldIni {
        identity: identity(),
        fingerprint: Fingerprint::of(&files),
    };
    let bytes = WorldIniFormat::export(&ini).unwrap_or_else(|never| match never {});
    files.insert(WORLD_INI, bytes)?;
    let opened = WorldPackage::from_files(&files)?;
    if !opened.ignored.is_empty() {
        return Err(ExtractError::UnknownFiles {
            dir: dir.to_path_buf(),
            paths: opened.ignored,
        });
    }
    Ok(opened.package)
}

/// The rosters `world`'s countries name, in its list's order (each once).
#[must_use]
pub fn roster_sources(world: &World) -> Vec<RosterSource> {
    world
        .countries()
        .iter()
        .filter_map(|country| {
            country.roster.as_ref().map(|file| RosterSource {
                number: country.number,
                name: country.name.clone(),
                file: file.clone(),
            })
        })
        .collect()
}

/// Each roster made from `game`'s team file, as OpenSWOS Setup makes it (upstream
/// tools/swosdata/lib/swosdata.c `step_teams`, team_json.c `Sd_TeamFileToJson`), laid out as
/// World 2026-27's package lays out its clubs, every team with its derived ids and checksum
/// (upstream docs/TEAM_IDS.md), by file name.
///
/// # Errors
///
/// When the game lacks a team file or holds one that does not decode.
pub fn rosters(
    sources: &[RosterSource],
    game: &GameCopy,
) -> Result<BTreeMap<String, TeamJsonDocument>, ExtractError> {
    sources
        .iter()
        .map(|source| {
            let file = game.team_file(source.number)?;
            let json = TeamJsonFile::from_team_file(&file, Some(&source.name), |team| {
                EntryIds::derived(WORLD_1996_97_TAG, team)
            });
            let document = TeamJsonDocument {
                file: json,
                indent: WORLD_PACKAGE_INDENT,
                non_ascii: WORLD_PACKAGE_NON_ASCII,
                trailing_newline: WORLD_PACKAGE_TRAILING_NEWLINE,
            };
            Ok((source.file.clone(), document))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use openswos_team_json_resource::TeamJsonFormat;
    use openswos_test_utils::{assets_path, world_1996_97_path};

    use super::*;

    /// A game folder holding England's and Oceania's team files.
    fn game() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temp dir");
        let data = dir.path().join("DATA");
        std::fs::create_dir(&data).expect("DATA/");
        for name in ["TEAM.008", "TEAM.085"] {
            std::fs::copy(assets_path().join("TEAM").join(name), data.join(name))
                .expect("a fixture");
        }
        dir
    }

    fn source(number: u8, name: &str) -> RosterSource {
        RosterSource {
            number: TeamFileNumber(number),
            name: name.to_owned(),
            file: format!("{name}.json"),
        }
    }

    #[test]
    fn the_identity_says_what_the_package_holds() {
        let identity = identity();
        assert_eq!(identity.tag, WorldTag("9697".to_owned()));
        assert_eq!(identity.name, "World 1996-97");
        assert!(
            identity
                .comments
                .iter()
                .any(|line| line.contains("not to be shared"))
        );
    }

    #[test]
    fn a_roster_is_the_team_json_upstream_makes() {
        let dir = game();
        let game = GameCopy::open(dir.path()).expect("a game");
        let rosters =
            rosters(&[source(85, "OCEANIA"), source(8, "ENGLAND")], &game).expect("the rosters");
        assert_eq!(
            rosters.keys().collect::<Vec<_>>(),
            ["ENGLAND.json", "OCEANIA.json"]
        );
        assert_eq!(rosters["ENGLAND.json"].file.count, 93);
        // `assets/TEAM_JSON/OCEANIA.json`: World 1996-97's JSON of `TEAM.085`, checked
        // against upstream's ids by the team_json crate.
        let expected =
            std::fs::read(assets_path().join("TEAM_JSON/OCEANIA.json")).expect("the fixture");
        let written = TeamJsonFormat::export(&rosters["OCEANIA.json"]).expect("written");
        assert!(written == expected, "OCEANIA.json differs from the fixture");
    }

    #[test]
    fn a_missing_team_file_is_named() {
        let dir = game();
        let game = GameCopy::open(dir.path()).expect("a game");
        let error = rosters(&[source(0, "ALBANIA")], &game).expect_err("no TEAM.000");
        assert!(error.to_string().contains("TEAM.000"), "{error}");
    }

    #[test]
    fn the_committed_package_is_upstreams_competitions_and_career_params() {
        let package = committed(&world_1996_97_path()).expect("the committed package");
        assert_eq!(package.identity, identity());
        assert_eq!(package.competitions.len(), 10);
        assert!(package.career_params.is_some());
        assert!(package.teams.is_empty() && package.pool.is_none());
        let world = World::from_package(&package).expect("a valid world");
        let sources = roster_sources(&world);
        assert_eq!(sources.len(), 71, "one roster a country");
        assert!(sources.contains(&source(8, "ENGLAND")));
        assert!(sources.contains(&source(85, "OCEANIA")));
    }

    #[test]
    fn a_stray_committed_file_is_refused() {
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::create_dir(dir.path().join("rules")).expect("rules/");
        std::fs::write(dir.path().join("rules/europe.json"), "{}").expect("a stray file");
        let error = committed(dir.path()).expect_err("an unknown file");
        assert!(error.to_string().contains("rules/europe.json"), "{error}");
    }
}
