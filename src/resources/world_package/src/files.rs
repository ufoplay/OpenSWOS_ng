//! A package's files, by path: what a folder or a zip holds, before anything is parsed.

use std::collections::BTreeMap;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use thiserror::Error;

/// The most a package may hold, in bytes: 80 MiB, over twice World 2026-27's 36.3 MiB at the
/// pin (27.6 MiB at the baseline). A zip that unpacks to more is refused rather than read.
pub const MAX_PACKAGE_BYTES: u64 = 80 * 1024 * 1024;

/// The separator of package paths.
const SEPARATOR: char = '/';

/// Why a package's files cannot be read or written.
#[derive(Debug, Error)]
pub enum FilesError {
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
    /// The zip does not read.
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    /// A path leaves the package (`..`, absolute, a backslash) or is empty.
    #[error("`{0}` is not a path inside a package")]
    UnsafePath(String),
    /// The package is bigger than [`MAX_PACKAGE_BYTES`].
    #[error("the package holds more than {MAX_PACKAGE_BYTES} bytes")]
    TooLarge,
    /// The folder to write into is not empty.
    #[error("{0} is not empty")]
    NotEmpty(PathBuf),
}

/// A package's files: `/`-separated paths relative to its folder, and their bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageFiles {
    files: BTreeMap<String, Vec<u8>>,
}

impl PackageFiles {
    /// Adds a file.
    ///
    /// # Errors
    ///
    /// When `path` leaves the package.
    pub fn insert(&mut self, path: &str, bytes: Vec<u8>) -> Result<(), FilesError> {
        check_path(path)?;
        self.files.insert(path.to_owned(), bytes);
        Ok(())
    }

    /// The file at `path`.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    /// Every file, by path.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
    }

    /// The files directly in `folder`, by path (`folder/name`), in name order.
    pub fn in_folder<'a>(&'a self, folder: &'a str) -> impl Iterator<Item = (&'a str, &'a [u8])> {
        self.iter().filter(move |(path, _)| {
            path.strip_prefix(folder)
                .and_then(|rest| rest.strip_prefix(SEPARATOR))
                .is_some_and(|name| !name.contains(SEPARATOR))
        })
    }

    /// The files of the folder `dir`, at any depth. Symbolic links are not followed.
    ///
    /// # Errors
    ///
    /// When a file cannot be read, or the folder holds more than [`MAX_PACKAGE_BYTES`].
    pub fn read_dir(dir: &Path) -> Result<Self, FilesError> {
        let mut files = Self::default();
        let mut total = 0;
        read_into(dir, "", &mut files, &mut total)?;
        Ok(files)
    }

    /// The files of a zip. When every one is in a single top folder (upstream's packages:
    /// `OpenSWOS-World-2627/`), the paths are relative to it.
    ///
    /// # Errors
    ///
    /// When the zip does not read, a path leaves the package, or it unpacks to more than
    /// [`MAX_PACKAGE_BYTES`].
    pub fn read_zip(reader: impl Read + Seek) -> Result<Self, FilesError> {
        let mut archive = zip::ZipArchive::new(reader)?;
        let mut entries = Vec::new();
        let mut total = 0u64;
        for index in 0..archive.len() {
            let entry = archive.by_index(index)?;
            if entry.is_dir() {
                continue;
            }
            let path = entry.name()?.into_owned();
            check_path(&path)?;
            total = total.saturating_add(entry.size());
            if total > MAX_PACKAGE_BYTES {
                return Err(FilesError::TooLarge);
            }
            let mut bytes = Vec::new();
            entry
                .take(MAX_PACKAGE_BYTES)
                .read_to_end(&mut bytes)
                .map_err(|source| FilesError::Io {
                    path: PathBuf::from(&path),
                    source,
                })?;
            entries.push((path, bytes));
        }
        let top = single_top_folder(entries.iter().map(|(path, _)| path.as_str()));
        let mut files = Self::default();
        for (path, bytes) in entries {
            let relative = match &top {
                Some(top) => path
                    .strip_prefix(top.as_str())
                    .and_then(|rest| rest.strip_prefix(SEPARATOR))
                    .unwrap_or(&path)
                    .to_owned(),
                None => path,
            };
            files.insert(&relative, bytes)?;
        }
        Ok(files)
    }

    /// Writes the files into `dir`, which must not exist or be empty.
    ///
    /// # Errors
    ///
    /// When `dir` is not empty or a file cannot be written.
    pub fn write_dir(&self, dir: &Path) -> Result<(), FilesError> {
        let io = |path: &Path| {
            let path = path.to_path_buf();
            move |source| FilesError::Io { path, source }
        };
        if dir.exists() && std::fs::read_dir(dir).map_err(io(dir))?.next().is_some() {
            return Err(FilesError::NotEmpty(dir.to_path_buf()));
        }
        for (path, bytes) in self.iter() {
            let target = path
                .split(SEPARATOR)
                .fold(dir.to_path_buf(), |target, part| target.join(part));
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(io(parent))?;
            }
            std::fs::write(&target, bytes).map_err(io(&target))?;
        }
        Ok(())
    }
}

/// Reads the files of `dir` (`prefix` being its path in the package) into `files`.
fn read_into(
    dir: &Path,
    prefix: &str,
    files: &mut PackageFiles,
    total: &mut u64,
) -> Result<(), FilesError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| FilesError::Io { path, source }
    };
    for entry in std::fs::read_dir(dir).map_err(io(dir))? {
        let entry = entry.map_err(io(dir))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        let kind = entry.file_type().map_err(io(&path))?;
        if kind.is_dir() {
            read_into(&path, &format!("{relative}{SEPARATOR}"), files, total)?;
        } else if kind.is_file() {
            let bytes = std::fs::read(&path).map_err(io(&path))?;
            *total = total.saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
            if *total > MAX_PACKAGE_BYTES {
                return Err(FilesError::TooLarge);
            }
            files.insert(&relative, bytes)?;
        }
    }
    Ok(())
}

/// The folder every path is in, if there is a single one.
fn single_top_folder<'a>(paths: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut top: Option<&str> = None;
    for path in paths {
        let (folder, _) = path.split_once(SEPARATOR)?;
        match top {
            Some(top) if top != folder => return None,
            Some(_) => {}
            None => top = Some(folder),
        }
    }
    top.map(str::to_owned)
}

/// Fails when `path` is not a plain path inside a package.
fn check_path(path: &str) -> Result<(), FilesError> {
    let safe = !path.is_empty()
        && !path.contains('\\')
        && !path.contains('\0')
        && path
            .split(SEPARATOR)
            .all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':'));
    if safe {
        Ok(())
    } else {
        Err(FilesError::UnsafePath(path.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, bytes) in entries {
            writer
                .start_file(*path, zip::write::SimpleFileOptions::default())
                .expect("an entry");
            writer.write_all(bytes).expect("written");
        }
        writer.finish().expect("a zip").into_inner()
    }

    #[test]
    fn paths_must_stay_inside_the_package() {
        let mut files = PackageFiles::default();
        for path in ["../x", "/x", "a//b", "a\\b", "", "c:/x", "a/./b"] {
            assert!(
                matches!(
                    files.insert(path, Vec::new()),
                    Err(FilesError::UnsafePath(_))
                ),
                "{path}"
            );
        }
        assert!(files.insert("teams/ALBANIA.json", Vec::new()).is_ok());
        assert_eq!(files.get("teams/ALBANIA.json"), Some(&[][..]));
    }

    #[test]
    fn a_folder_lists_only_its_own_files() {
        let mut files = PackageFiles::default();
        for path in [
            "teams/A.json",
            "teams/x/B.json",
            "teamsX/C.json",
            "WORLD.ini",
        ] {
            files.insert(path, Vec::new()).expect("a safe path");
        }
        let in_teams: Vec<&str> = files.in_folder("teams").map(|(path, _)| path).collect();
        assert_eq!(in_teams, vec!["teams/A.json"]);
    }

    #[test]
    fn a_zip_with_one_top_folder_reads_inside_it() {
        let bytes = zip(&[
            ("OpenSWOS-World-2627/WORLD.ini", b"ini"),
            ("OpenSWOS-World-2627/teams/A.json", b"{}"),
        ]);
        let files = PackageFiles::read_zip(Cursor::new(bytes)).expect("a zip");
        assert_eq!(files.get("WORLD.ini"), Some(&b"ini"[..]));
        assert_eq!(files.get("teams/A.json"), Some(&b"{}"[..]));
        let flat = zip(&[("WORLD.ini", b"ini"), ("teams/A.json", b"{}")]);
        let files = PackageFiles::read_zip(Cursor::new(flat)).expect("a zip");
        assert_eq!(files.get("WORLD.ini"), Some(&b"ini"[..]));
    }

    #[test]
    fn a_zip_leaving_the_package_is_refused() {
        let bytes = zip(&[("../evil", b"x")]);
        assert!(matches!(
            PackageFiles::read_zip(Cursor::new(bytes)),
            Err(FilesError::UnsafePath(_))
        ));
    }

    #[test]
    fn written_files_read_back_and_a_full_folder_is_refused() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        let target = dir.path().join("world");
        let mut files = PackageFiles::default();
        files
            .insert("WORLD.ini", b"tag = x\n".to_vec())
            .expect("a safe path");
        files
            .insert("teams/A.json", b"{}".to_vec())
            .expect("a safe path");
        files.write_dir(&target).expect("written");
        assert_eq!(PackageFiles::read_dir(&target).expect("read"), files);
        assert!(matches!(
            files.write_dir(&target),
            Err(FilesError::NotEmpty(_))
        ));
    }
}
