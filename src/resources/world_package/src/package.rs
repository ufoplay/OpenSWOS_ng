//! A world package: `WORLD.ini`, `mod.json`, `teams/*.json`, `competitions/*.json` (with the
//! world's national teams, B teams and stadiums, each a format of its own).

use std::collections::BTreeMap;
use std::path::Path;

use openswos_common::{Exporter, Importer};
use openswos_competitions_json_resource::{
    CompetitionsDocument, CompetitionsJsonError, CompetitionsJsonFormat,
};
use openswos_national_teams_json_resource::{NationalTeamsDocument, NationalTeamsJsonFormat};
use openswos_team_json_resource::{TeamJsonDocument, TeamJsonError, TeamJsonFormat};
use thiserror::Error;

use crate::bteams::BTeamsDocument;
use crate::files::{FilesError, PackageFiles};
use crate::fingerprint::{Fingerprint, FingerprintCheck};
use crate::ini::{IniError, WorldIdentity, WorldIni, WorldIniFormat};
use crate::manifest::{ModManifestDocument, ModManifestFormat};
use crate::stadiums::StadiumsDocument;

/// The path of `WORLD.ini`.
pub const WORLD_INI: &str = "WORLD.ini";

/// The path of `mod.json`.
pub const MOD_JSON: &str = "mod.json";

/// The folder of the rosters.
pub const TEAMS_FOLDER: &str = "teams";

/// The folder of the competitions.
pub const COMPETITIONS_FOLDER: &str = "competitions";

/// The world's national teams, in `competitions/` (upstream `DataJson_LoadNationalTeams`
/// reads a world's there first).
pub const NATIONAL_TEAMS_JSON: &str = "national_teams.json";

/// The world's B teams, in `competitions/` (upstream src/bteam.c).
pub const BTEAMS_JSON: &str = "bteams.json";

/// The world's stadium names, in `competitions/` (upstream src/world_stadium.c).
pub const STADIUMS_JSON: &str = "stadiums.json";

/// The extension of the files read in `teams/` and `competitions/`.
const JSON_EXTENSION: &str = ".json";

/// A world package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPackage {
    /// Who the world is (`WORLD.ini` but its fingerprint, which the files give).
    pub identity: WorldIdentity,
    /// Its mod manifest, if it has one.
    pub manifest: Option<ModManifestDocument>,
    /// Its rosters, by file name (`ALBANIA.json`).
    pub teams: BTreeMap<String, TeamJsonDocument>,
    /// Its competitions, by file name (`leagues.json`).
    pub competitions: BTreeMap<String, CompetitionsDocument>,
    /// Its national teams (`competitions/national_teams.json`), if it has its own.
    pub national_teams: Option<NationalTeamsDocument>,
    /// Its B teams (`competitions/bteams.json`), if it has any.
    pub bteams: Option<BTeamsDocument>,
    /// Its stadium names (`competitions/stadiums.json`), if it has any.
    pub stadiums: Option<StadiumsDocument>,
}

/// A package as opened: the package, its fingerprint check, and what was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedPackage {
    /// The package.
    pub package: WorldPackage,
    /// `WORLD.ini`'s fingerprint against the files'. A mismatch is not an error: the game
    /// itself only warns (upstream src/main.c).
    pub fingerprint: FingerprintCheck,
    /// The paths not read: `data/` (the Amiga's team files, D9: in the fingerprint all the
    /// same), `teams/POOLPLYR.DAT` (the world's foreign players' pool, RNC-packed: D9, in the
    /// fingerprint too), and anything else the package holds.
    pub ignored: Vec<String>,
}

/// Why a package does not open or write.
#[derive(Debug, Error)]
pub enum WorldPackageError {
    /// Its files do not read or write.
    #[error(transparent)]
    Files(#[from] FilesError),
    /// It has no `WORLD.ini`.
    #[error("the package has no {WORLD_INI}")]
    MissingIni,
    /// Its `WORLD.ini` does not read.
    #[error(transparent)]
    Ini(#[from] IniError),
    /// Its `mod.json` does not read or write.
    #[error("{MOD_JSON}: {0}")]
    Manifest(serde_json::Error),
    /// A roster does not read or write.
    #[error("{TEAMS_FOLDER}/{file}: {source}")]
    Team {
        /// The file.
        file: String,
        /// Why.
        source: TeamJsonError,
    },
    /// A competition file does not read or write.
    #[error("{COMPETITIONS_FOLDER}/{file}: {source}")]
    Competitions {
        /// The file.
        file: String,
        /// Why.
        source: CompetitionsJsonError,
    },
    /// The national teams, the B teams or the stadiums do not read or write.
    #[error("{COMPETITIONS_FOLDER}/{file}: {source}")]
    WorldFile {
        /// The file.
        file: &'static str,
        /// Why.
        source: serde_json::Error,
    },
}

/// A `competitions/` file of its own format's error.
fn world_file(file: &'static str) -> impl Fn(serde_json::Error) -> WorldPackageError {
    move |source| WorldPackageError::WorldFile { file, source }
}

impl WorldPackage {
    /// The package in `files`.
    ///
    /// # Errors
    ///
    /// When it has no `WORLD.ini`, or a file it reads does not decode.
    pub fn from_files(files: &PackageFiles) -> Result<OpenedPackage, WorldPackageError> {
        let ini =
            WorldIniFormat::import(files.get(WORLD_INI).ok_or(WorldPackageError::MissingIni)?)?;
        let mut package = Self {
            identity: ini.identity,
            manifest: None,
            teams: BTreeMap::new(),
            competitions: BTreeMap::new(),
            national_teams: None,
            bteams: None,
            stadiums: None,
        };
        let mut ignored = Vec::new();
        for (path, bytes) in files.iter() {
            match classify(path) {
                Entry::Ini => {}
                Entry::Manifest => {
                    package.manifest = Some(
                        ModManifestFormat::import(bytes).map_err(WorldPackageError::Manifest)?,
                    );
                }
                Entry::Team(file) => {
                    let document = TeamJsonFormat::import(bytes).map_err(|source| {
                        WorldPackageError::Team {
                            file: file.to_owned(),
                            source,
                        }
                    })?;
                    package.teams.insert(file.to_owned(), document);
                }
                Entry::Competitions(file) => {
                    let document = CompetitionsJsonFormat::import(bytes).map_err(|source| {
                        WorldPackageError::Competitions {
                            file: file.to_owned(),
                            source,
                        }
                    })?;
                    package.competitions.insert(file.to_owned(), document);
                }
                Entry::NationalTeams => {
                    package.national_teams = Some(
                        NationalTeamsJsonFormat::import(bytes)
                            .map_err(world_file(NATIONAL_TEAMS_JSON))?,
                    );
                }
                Entry::BTeams => {
                    package.bteams =
                        Some(BTeamsDocument::read(bytes).map_err(world_file(BTEAMS_JSON))?);
                }
                Entry::Stadiums => {
                    package.stadiums =
                        Some(StadiumsDocument::read(bytes).map_err(world_file(STADIUMS_JSON))?);
                }
                Entry::Ignored => ignored.push(path.to_owned()),
            }
        }
        Ok(OpenedPackage {
            package,
            fingerprint: FingerprintCheck {
                stated: ini.fingerprint,
                computed: Fingerprint::of(files),
            },
            ignored,
        })
    }

    /// The package's files, as written: its `WORLD.ini` states their fingerprint.
    ///
    /// # Errors
    ///
    /// When a file does not encode.
    pub fn to_files(&self) -> Result<PackageFiles, WorldPackageError> {
        let mut files = PackageFiles::default();
        for (file, document) in &self.teams {
            let bytes =
                TeamJsonFormat::export(document).map_err(|source| WorldPackageError::Team {
                    file: file.clone(),
                    source,
                })?;
            files.insert(&format!("{TEAMS_FOLDER}/{file}"), bytes)?;
        }
        for (file, document) in &self.competitions {
            let bytes = CompetitionsJsonFormat::export(document).map_err(|source| {
                WorldPackageError::Competitions {
                    file: file.clone(),
                    source,
                }
            })?;
            files.insert(&format!("{COMPETITIONS_FOLDER}/{file}"), bytes)?;
        }
        if let Some(document) = &self.national_teams {
            let bytes = NationalTeamsJsonFormat::export(document)
                .map_err(world_file(NATIONAL_TEAMS_JSON))?;
            files.insert(
                &format!("{COMPETITIONS_FOLDER}/{NATIONAL_TEAMS_JSON}"),
                bytes,
            )?;
        }
        if let Some(document) = &self.bteams {
            let bytes = document.to_bytes().map_err(world_file(BTEAMS_JSON))?;
            files.insert(&format!("{COMPETITIONS_FOLDER}/{BTEAMS_JSON}"), bytes)?;
        }
        if let Some(document) = &self.stadiums {
            let bytes = document.to_bytes().map_err(world_file(STADIUMS_JSON))?;
            files.insert(&format!("{COMPETITIONS_FOLDER}/{STADIUMS_JSON}"), bytes)?;
        }
        if let Some(manifest) = &self.manifest {
            let bytes = ModManifestFormat::export(manifest).map_err(WorldPackageError::Manifest)?;
            files.insert(MOD_JSON, bytes)?;
        }
        let ini = WorldIni {
            identity: self.identity.clone(),
            fingerprint: Fingerprint::of(&files),
        };
        let bytes = WorldIniFormat::export(&ini).unwrap_or_else(|never| match never {});
        files.insert(WORLD_INI, bytes)?;
        Ok(files)
    }

    /// The package in the folder `dir`.
    ///
    /// # Errors
    ///
    /// As [`PackageFiles::read_dir`] and [`WorldPackage::from_files`].
    pub fn open_dir(dir: &Path) -> Result<OpenedPackage, WorldPackageError> {
        Self::from_files(&PackageFiles::read_dir(dir)?)
    }

    /// The package in the zip at `path`.
    ///
    /// # Errors
    ///
    /// As [`PackageFiles::read_zip`] and [`WorldPackage::from_files`].
    pub fn open_zip(path: &Path) -> Result<OpenedPackage, WorldPackageError> {
        let file = std::fs::File::open(path).map_err(|source| FilesError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_files(&PackageFiles::read_zip(std::io::BufReader::new(file))?)
    }

    /// Writes the package into the folder `dir`, which must not exist or be empty.
    ///
    /// # Errors
    ///
    /// As [`WorldPackage::to_files`] and [`PackageFiles::write_dir`].
    pub fn write_dir(&self, dir: &Path) -> Result<(), WorldPackageError> {
        Ok(self.to_files()?.write_dir(dir)?)
    }
}

/// What a package path holds.
enum Entry<'a> {
    /// `WORLD.ini`.
    Ini,
    /// `mod.json`.
    Manifest,
    /// A roster: its file name.
    Team(&'a str),
    /// A competition file: its file name.
    Competitions(&'a str),
    /// The world's national teams.
    NationalTeams,
    /// The world's B teams.
    BTeams,
    /// The world's stadium names.
    Stadiums,
    /// Something this port does not read.
    Ignored,
}

fn classify(path: &str) -> Entry<'_> {
    let json_in = |folder: &str| {
        path.strip_prefix(folder)
            .and_then(|rest| rest.strip_prefix('/'))
            .filter(|name| !name.contains('/') && name.ends_with(JSON_EXTENSION))
    };
    if path == WORLD_INI {
        Entry::Ini
    } else if path == MOD_JSON {
        Entry::Manifest
    } else if let Some(file) = json_in(TEAMS_FOLDER) {
        Entry::Team(file)
    } else if let Some(file) = json_in(COMPETITIONS_FOLDER) {
        match file {
            NATIONAL_TEAMS_JSON => Entry::NationalTeams,
            BTEAMS_JSON => Entry::BTeams,
            STADIUMS_JSON => Entry::Stadiums,
            _ => Entry::Competitions(file),
        }
    } else {
        Entry::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ini::WorldTag;

    /// A small package: an empty roster, a career index, national teams, B teams, stadiums,
    /// an Amiga file, a pool, a stray file.
    fn files() -> PackageFiles {
        let mut files = PackageFiles::default();
        for (path, bytes) in [
            (
                WORLD_INI,
                &b"tag = 9697\nname = World 1996-97\nfingerprint = 0000000000000000\n"[..],
            ),
            (
                "teams/EMPTY.json",
                br#"{"type": "team", "country": null, "count": 0, "teams": []}"#,
            ),
            (
                "competitions/career.json",
                br#"{"type": "career_index", "count": 0, "contests": []}"#,
            ),
            (
                "competitions/national_teams.json",
                br#"{"records": [{"country": 80, "ordinal": 0, "nationality": 0, "team_file": 0, "skill": 19}]}"#,
            ),
            ("competitions/bteams.json", br#"{"rules": [], "teams": []}"#),
            ("competitions/stadiums.json", br#"{"clubs": [], "venues": []}"#),
            ("data/TEAM.000", &[0, 0]),
            ("teams/POOLPLYR.DAT", b"RNC"),
            ("README.txt", b"hello"),
        ] {
            files.insert(path, bytes.to_vec()).expect("a safe path");
        }
        files
    }

    #[test]
    fn opens_what_it_knows_and_lists_the_rest() {
        let opened = WorldPackage::from_files(&files()).expect("a package");
        assert_eq!(opened.package.identity.tag, WorldTag("9697".to_owned()));
        assert_eq!(
            opened.package.teams.keys().collect::<Vec<_>>(),
            vec!["EMPTY.json"]
        );
        assert_eq!(
            opened.package.competitions.keys().collect::<Vec<_>>(),
            vec!["career.json"]
        );
        assert!(opened.package.national_teams.is_some());
        assert!(opened.package.bteams.is_some());
        assert!(opened.package.stadiums.is_some());
        assert_eq!(
            opened.ignored,
            vec!["README.txt", "data/TEAM.000", "teams/POOLPLYR.DAT"]
        );
        assert!(!opened.fingerprint.is_match(), "the stated one is made up");
        assert_eq!(opened.fingerprint.computed, Fingerprint::of(&files()));
    }

    #[test]
    fn writing_states_the_fingerprint_of_what_is_written() {
        let opened = WorldPackage::from_files(&files()).expect("a package");
        let written = opened.package.to_files().expect("written");
        assert!(
            written.get("data/TEAM.000").is_none(),
            "data/ is not written"
        );
        for file in ["national_teams.json", "bteams.json", "stadiums.json"] {
            assert!(
                written.get(&format!("competitions/{file}")).is_some(),
                "{file}"
            );
        }
        let reopened = WorldPackage::from_files(&written).expect("a package");
        assert!(reopened.fingerprint.is_match());
        assert!(reopened.ignored.is_empty());
        assert_eq!(reopened.package, opened.package);
    }

    #[test]
    fn a_broken_world_file_is_named() {
        let mut files = files();
        files
            .insert("competitions/bteams.json", b"[]".to_vec())
            .expect("a safe path");
        let error = WorldPackage::from_files(&files).expect_err("broken B teams");
        assert!(
            error.to_string().starts_with("competitions/bteams.json: "),
            "{error}"
        );
    }

    #[test]
    fn a_package_needs_its_ini() {
        let mut files = PackageFiles::default();
        files
            .insert("teams/A.json", b"{}".to_vec())
            .expect("a safe path");
        assert!(matches!(
            WorldPackage::from_files(&files),
            Err(WorldPackageError::MissingIni)
        ));
    }

    #[test]
    fn a_broken_file_is_named() {
        let mut files = files();
        files
            .insert("teams/BROKEN.json", b"[]".to_vec())
            .expect("a safe path");
        let error = WorldPackage::from_files(&files).expect_err("a broken roster");
        assert!(
            error.to_string().starts_with("teams/BROKEN.json: "),
            "{error}"
        );
    }
}
