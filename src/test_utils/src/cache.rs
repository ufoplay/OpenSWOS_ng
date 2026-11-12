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

/// The URL of a file of swos-port's `tests/data`, at the commit pinned: its last change to
/// that folder (4da94f5, 2026-09-06).
macro_rules! swos_port_data {
    ($file:literal) => {
        concat!(
            "https://raw.githubusercontent.com/zlatkok/swos-port/",
            "4da94f538846e4c9b638711166315c9927236383/tests/data/",
            $file
        )
    };
}

/// swos-port's match recordings (`.RGD.gz`): the DOS game's state on every frame of a
/// match, the match engine's golden oracle (docs/PLAN.md §7.4). That repository declares
/// no license, so they are downloaded, never committed (D8). The checksums are the files'
/// at the pinned commit.
pub const RGD_CORPUS: [Download<'static>; 15] = [
    Download {
        url: swos_port_data!("2xcpu-08030320.RGD.gz"),
        file_name: "2xcpu-08030320.RGD.gz",
        sha256: Some("0e9ba792d0658312e3ca482034f68b3b4b215c75d6730eed7c0585d209be437e"),
    },
    Download {
        url: swos_port_data!("playa-10090613.RGD.gz"),
        file_name: "playa-10090613.RGD.gz",
        sha256: Some("af8defa667eb40ed15663816372a35695a4b7ec717f215f5f890503beab79ca3"),
    },
    Download {
        url: swos_port_data!("playa-10095214.RGD.gz"),
        file_name: "playa-10095214.RGD.gz",
        sha256: Some("f8a780072569e0647ad7d136f247584b90240dd7b7b3ac35cf629b2a8815da6e"),
    },
    Download {
        url: swos_port_data!("playa-10144556.RGD.gz"),
        file_name: "playa-10144556.RGD.gz",
        sha256: Some("c70157a1005b7999572ccd632146e3bca81d0b3e435fa9cbe2fc38fb2907b999"),
    },
    Download {
        url: swos_port_data!("shootout-08200245.RGD.gz"),
        file_name: "shootout-08200245.RGD.gz",
        sha256: Some("0a135d070fdcb97b86efb0a44d2370feefa23f631ab132517e820b01aa7a0181"),
    },
    Download {
        url: swos_port_data!("zkz-10140228.RGD.gz"),
        file_name: "zkz-10140228.RGD.gz",
        sha256: Some("cf36e82dc67a0d1bdda07cc17da6cf6294c48f63f7a66aed0325c29a77d3958e"),
    },
    Download {
        url: swos_port_data!("zkz-10173518.RGD.gz"),
        file_name: "zkz-10173518.RGD.gz",
        sha256: Some("162d984eea20e1a92cd8639b6635080b5ee77fd2c9cb858cbfe37c38ede4e5f0"),
    },
    Download {
        url: swos_port_data!("zkz-10174427.RGD.gz"),
        file_name: "zkz-10174427.RGD.gz",
        sha256: Some("185bd7d1edb8a1ba60578342cdaf1df6125445c347b5de3dbcb1a83bce56b40f"),
    },
    Download {
        url: swos_port_data!("zkz-10190651.RGD.gz"),
        file_name: "zkz-10190651.RGD.gz",
        sha256: Some("6a1d8d83aaf75e5e47604ad8fa8e3ae007f85da57f627fea0c212afd398a6a6a"),
    },
    Download {
        url: swos_port_data!("zkz-11021807.RGD.gz"),
        file_name: "zkz-11021807.RGD.gz",
        sha256: Some("edd4038cd3536dead1f618a13471addc1e5c2aaa1ff29e321707ce7d7b46ff35"),
    },
    Download {
        url: swos_port_data!("zkz-11054632.RGD.gz"),
        file_name: "zkz-11054632.RGD.gz",
        sha256: Some("f13b763c79b91db3ae62c2459df1ca4d0204813aa433fb2d60fe28c27e7db2f6"),
    },
    Download {
        url: swos_port_data!("zkz-11122841.RGD.gz"),
        file_name: "zkz-11122841.RGD.gz",
        sha256: Some("287c517944b0570c24ab800d23d036ddb4997ffda724bf6ad121b4905022005b"),
    },
    Download {
        url: swos_port_data!("zkz-11215003.RGD.gz"),
        file_name: "zkz-11215003.RGD.gz",
        sha256: Some("83825668e34d39dc44604e1774a1c1c97358c3d2874f31d84c9b25e603f31cb9"),
    },
    Download {
        url: swos_port_data!("zkz-12065730.RGD.gz"),
        file_name: "zkz-12065730.RGD.gz",
        sha256: Some("19a132370d485d82b7efb31f14ca56486a6833ecb3b9f0ab5c9e0f4df31fc68a"),
    },
    Download {
        url: swos_port_data!("zkz-12080203.RGD.gz"),
        file_name: "zkz-12080203.RGD.gz",
        sha256: Some("cd29c7512e0505f6938a643de001b82bf229cb6b59021e603e9b08fccb33b575"),
    },
];

/// The recording of `RGD_CORPUS` named `file_name`, if any.
#[must_use]
pub fn rgd_recording(file_name: &str) -> Option<&'static Download<'static>> {
    RGD_CORPUS
        .iter()
        .find(|download| download.file_name == file_name)
}

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
