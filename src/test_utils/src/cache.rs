//! The download cache (docs/PLAN.md §6, D8): test data that cannot be committed is
//! downloaded once, verified, and kept in `target/cache/downloads/`.

use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::cache_path;
use crate::skip::Skip;

/// The cache folder for downloads, under [`cache_path`].
const DOWNLOADS_DIR: &str = "downloads";

/// The most one download may weigh, in bytes: 64 MiB. The biggest expected file is a
/// match recording of about 7 MB; the cap stops a wrong URL from filling memory.
const MAX_DOWNLOAD_BYTES: u64 = 64 * 1024 * 1024;

/// A file the tests download once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Download<'a> {
    /// Where it is downloaded from.
    pub url: &'a str,
    /// Its name in the cache.
    pub file_name: &'a str,
    /// Its expected SHA-256, lower-case hex, if known.
    pub sha256: Option<&'a str>,
}

/// World 2026-27's package for OpenSWOS (the URL of upstream's Setup, `tools/swosdata/app/
/// main.cpp`). Its data is the SWOS 96/97 Mod's and is not ours to commit (D8). Its
/// `WORLD.ini` fingerprint is `efe1464b6d1d62b8`, upstream's at the pinned commit. A new
/// package on the server fails this checksum on purpose: check it, then update both.
pub const WORLD_2627_PACKAGE: Download<'static> = Download {
    url: "https://worldofswos.com/static/openswos/OpenSWOS-World-2627.zip",
    file_name: "OpenSWOS-World-2627.zip",
    sha256: Some("80647e3423f1d9bd1949c7954ef85d8c2aa546eb32248d106313d44d88507acd"),
};

/// Where downloaded test data is kept: `target/cache/downloads`.
#[must_use]
pub fn downloads_path() -> PathBuf {
    cache_path().join(DOWNLOADS_DIR)
}

/// The cached file, downloaded on first use.
///
/// # Errors
///
/// A [`Skip`] when the file is not cached and cannot be downloaded (no network, the URL
/// gone...).
///
/// # Panics
///
/// When the file does not match its expected checksum: the data changed upstream, which a
/// test must not silently accept. Also when the cache cannot be written.
pub fn fetch(download: &Download<'_>) -> Result<PathBuf, Skip> {
    fetch_into(&downloads_path(), download)
}

/// [`fetch`] into the cache folder `dir`.
///
/// # Errors
///
/// As [`fetch`].
///
/// # Panics
///
/// As [`fetch`].
pub fn fetch_into(dir: &Path, download: &Download<'_>) -> Result<PathBuf, Skip> {
    let path = dir.join(download.file_name);
    if path.is_file() {
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read the cached {}: {error}", path.display()));
        verify(download, &bytes);
        return Ok(path);
    }
    let bytes = http_get(download.url).map_err(|error| {
        Skip::new(format!(
            "{} is not cached and cannot be downloaded from {}: {error}",
            download.file_name, download.url
        ))
    })?;
    verify(download, &bytes);
    store(dir, &path, &bytes);
    Ok(path)
}

/// The SHA-256 of `bytes`, lower-case hex.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// # Panics
///
/// When `bytes` do not match the download's expected checksum.
fn verify(download: &Download<'_>, bytes: &[u8]) {
    if let Some(expected) = download.sha256 {
        let actual = sha256_hex(bytes);
        assert!(
            actual == expected,
            "{} from {} has SHA-256 {actual}, not {expected}: the data changed upstream; check it, \
             then update the expected checksum (or delete the cached file)",
            download.file_name,
            download.url
        );
    }
}

/// Writes `bytes` to `path` atomically, so a test running in parallel never sees half a
/// file.
///
/// # Panics
///
/// When the cache cannot be written.
fn store(dir: &Path, path: &Path, bytes: &[u8]) {
    if let Err(error) = write_atomically(dir, path, bytes) {
        panic!("cannot write {}: {error}", path.display());
    }
}

/// Writes a temporary file in `dir`, then renames it to `path`.
fn write_atomically(dir: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(bytes)?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn http_get(url: &str) -> Result<Vec<u8>, ureq::Error> {
    ureq::get(url)
        .call()?
        .into_body()
        .into_with_config()
        .limit(MAX_DOWNLOAD_BYTES)
        .read_to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A URL nothing answers: the discard port of the local machine, which refuses the
    /// connection at once.
    const UNREACHABLE: &str = "http://127.0.0.1:9/missing";

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn hashes_are_lower_case_hex() {
        assert_eq!(sha256_hex(b"abc"), ABC_SHA256);
    }

    #[test]
    fn a_cached_file_is_used_without_the_network() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("abc.bin"), b"abc").expect("cached");
        let download = Download {
            url: UNREACHABLE,
            file_name: "abc.bin",
            sha256: Some(ABC_SHA256),
        };
        assert_eq!(
            fetch_into(dir.path(), &download),
            Ok(dir.path().join("abc.bin"))
        );
    }

    #[test]
    #[should_panic(expected = "the data changed upstream")]
    fn a_cached_file_with_another_checksum_fails() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("abc.bin"), b"abd").expect("cached");
        let download = Download {
            url: UNREACHABLE,
            file_name: "abc.bin",
            sha256: Some(ABC_SHA256),
        };
        let _ = fetch_into(dir.path(), &download);
    }

    #[test]
    fn an_unreachable_download_skips() {
        let dir = tempfile::tempdir().expect("temp dir");
        let download = Download {
            url: UNREACHABLE,
            file_name: "missing.bin",
            sha256: None,
        };
        let skip = fetch_into(dir.path(), &download).expect_err("nothing answers");
        assert!(
            skip.reason()
                .starts_with("missing.bin is not cached and cannot be downloaded"),
            "{skip}"
        );
        assert!(!dir.path().join("missing.bin").exists());
    }

    #[test]
    fn stored_files_appear_whole() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = dir.path().join("downloads");
        let path = cache.join("x.bin");
        store(&cache, &path, b"payload");
        assert_eq!(std::fs::read(&path).expect("stored"), b"payload");
        store(&cache, &path, b"replaced");
        assert_eq!(std::fs::read(&path).expect("stored"), b"replaced");
    }

    #[test]
    fn the_cache_lives_in_target() {
        assert_eq!(
            downloads_path(),
            crate::target_path().join("cache").join("downloads")
        );
    }
}
