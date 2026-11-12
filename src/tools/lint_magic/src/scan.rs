//! Which files are linted, and the findings across them.

use std::fmt;
use std::path::{Path, PathBuf};

use thiserror::Error;
use walkdir::WalkDir;

use crate::visitor::{Finding, lint_source};

/// Directories never linted: build output, integration tests, benchmarks and examples
/// (test code, like `#[cfg(test)]` modules), and hidden directories.
const SKIPPED_DIRS: &[&str] = &["target", "tests", "benches", "examples"];

/// Why a file could not be linted.
#[derive(Debug, Error)]
pub enum ScanError {
    /// A directory could not be walked.
    #[error("cannot walk {path}: {source}")]
    Walk {
        /// Where the walk failed.
        path: PathBuf,
        /// The cause.
        source: walkdir::Error,
    },
    /// A file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// A file is not valid Rust.
    #[error("cannot parse {path}: {source}")]
    Parse {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: syn::Error,
    },
}

/// A finding in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFinding {
    /// The file.
    pub path: PathBuf,
    /// The finding.
    pub finding: Finding,
}

impl fmt::Display for FileFinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.path.display(), self.finding)
    }
}

/// The Rust files under `root`, sorted, skipping [`SKIPPED_DIRS`] and hidden directories.
///
/// # Errors
///
/// Fails when a directory cannot be walked.
pub fn rust_files(root: &Path) -> Result<Vec<PathBuf>, ScanError> {
    let mut files = Vec::new();
    let walker = WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            let skipped = entry.depth() > 0
                && entry.file_type().is_dir()
                && (name.starts_with('.') || SKIPPED_DIRS.contains(&name.as_ref()));
            !skipped
        });
    for entry in walker {
        let entry = entry.map_err(|source| ScanError::Walk {
            path: root.to_path_buf(),
            source,
        })?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "rs") {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}

/// Every finding in the Rust files under `roots`.
///
/// # Errors
///
/// Fails when a file cannot be found, read or parsed.
pub fn lint_paths(roots: &[PathBuf]) -> Result<Vec<FileFinding>, ScanError> {
    let mut findings = Vec::new();
    for root in roots {
        for path in rust_files(root)? {
            let source = std::fs::read_to_string(&path).map_err(|source| ScanError::Read {
                path: path.clone(),
                source,
            })?;
            let found = lint_source(&source).map_err(|source| ScanError::Parse {
                path: path.clone(),
                source,
            })?;
            findings.extend(found.into_iter().map(|finding| FileFinding {
                path: path.clone(),
                finding,
            }));
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, path: &str, content: &str) {
        let full = root.join(path);
        std::fs::create_dir_all(full.parent().expect("has a parent")).expect("dirs");
        std::fs::write(full, content).expect("written");
    }

    #[test]
    fn walks_rust_sources_and_skips_test_and_build_dirs() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "crate/src/lib.rs", "");
        write(dir.path(), "crate/src/sub/mod.rs", "");
        write(dir.path(), "crate/tests/it.rs", "");
        write(dir.path(), "crate/benches/b.rs", "");
        write(dir.path(), "target/debug/build.rs", "");
        write(dir.path(), ".hidden/x.rs", "");
        write(dir.path(), "crate/readme.md", "");
        let files: Vec<PathBuf> = rust_files(dir.path())
            .expect("walked")
            .into_iter()
            .map(|path| {
                path.strip_prefix(dir.path())
                    .expect("under root")
                    .to_path_buf()
            })
            .collect();
        assert_eq!(
            files,
            vec![
                PathBuf::from("crate/src/lib.rs"),
                PathBuf::from("crate/src/sub/mod.rs")
            ]
        );
    }

    #[test]
    fn reports_findings_with_their_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "src/lib.rs", "fn f() -> u8 { 42 }\n");
        let findings = lint_paths(&[dir.path().to_path_buf()]).expect("linted");
        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.path, dir.path().join("src").join("lib.rs"));
        assert_eq!(
            finding.finding.to_string(),
            "1:16: magic number `42`: name it with a documented const"
        );
        assert!(
            finding
                .to_string()
                .ends_with(":1:16: magic number `42`: name it with a documented const")
        );
    }

    #[test]
    fn unparsable_files_are_errors() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "src/lib.rs", "fn (");
        let error = lint_paths(&[dir.path().to_path_buf()]).expect_err("parse error");
        assert!(matches!(error, ScanError::Parse { .. }), "{error}");
    }

    #[test]
    fn a_missing_root_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let error = lint_paths(&[dir.path().join("missing")]).expect_err("no such dir");
        assert!(matches!(error, ScanError::Walk { .. }), "{error}");
    }
}
