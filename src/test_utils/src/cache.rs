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

/// World 2026-27's package for OpenSWOS (the URL upstream's Setup downloads it from,
/// `PROJECT_WORLD_PACKAGE_URL` in `include/project_identity.h`: OpenSWOS hosts it itself since
/// 2026-11-12). Its data is the SWOS 96/97 Mod's (with Playaveli's national teams and balance)
/// and is not ours to commit (D8). Its `WORLD.ini` fingerprint is `80e20cdd3e92ec9c`,
/// upstream's at the pinned commit. A new package on the server fails this checksum on
/// purpose: check it, then update both.
pub const WORLD_2627_PACKAGE: Download<'static> = Download {
    url: "https://openswos.net/dlc/OpenSWOS-World-2627.zip",
    file_name: "OpenSWOS-World-2627.zip",
    sha256: Some("50f95dd9b5c3a90e460f51d3a05bc27f61a71221481aa0e3e749473d6a12cb84"),
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

/// The URL of a file of the rules library's `tests/rules/specs` (swos-competition-engine, MIT:
/// Davide Lorigliola's), at its tag v1 (`87b6527`, 2026-11-02), the commit docs/PLAN.md pins.
macro_rules! rules_specs {
    ($file:literal) => {
        concat!(
            "https://raw.githubusercontent.com/aminta/swos-competition-engine/",
            "87b65278a15007223ec4f387f1027eb4d5cd7625/tests/rules/specs/",
            $file
        )
    };
}

/// The folder of the rules library's packs, under [`downloads_path`].
const RULES_SPECS_DIR: &str = "rules_specs";

/// The rules library's packs (its `tests/rules/specs`): each World 2026-27 league's divisions'
/// clubs (`shapes.txt`: a code and its counts a line) and its knockouts' spec (`<code>.spec`:
/// the Mod's `compile_spec` bytes), the oracle's knockouts checked over them. World 2026-27's
/// data: downloaded, never committed (D8). The checksums are the files' at the pinned commit.
pub const RULES_SPECS: [Download<'static>; 36] = [
    Download {
        url: rules_specs!("shapes.txt"),
        file_name: "shapes.txt",
        sha256: Some("de38fff4533d4ef514c786080bb3e84cc59e81485a0c0ec41656bf96701af28d"),
    },
    Download {
        url: rules_specs!("arg.spec"),
        file_name: "arg.spec",
        sha256: Some("6b4e53216614dd410d6b2c970cdcafbcdddc44438561fc632b24d98f42494986"),
    },
    Download {
        url: rules_specs!("aus.spec"),
        file_name: "aus.spec",
        sha256: Some("8b8abe9a11f1f0ee158c2c8bca841d4435372fc0e8c00d62daf37d626de152c1"),
    },
    Download {
        url: rules_specs!("aut.spec"),
        file_name: "aut.spec",
        sha256: Some("55502369123ec883b1e786cf01ebff1b40767b58e0f7e7aff1e265fa5651eafa"),
    },
    Download {
        url: rules_specs!("bra.spec"),
        file_name: "bra.spec",
        sha256: Some("6f49c9e076ec68df568cba83c1843b1883a21f6515220d55878d50f9d00a0b49"),
    },
    Download {
        url: rules_specs!("bul.spec"),
        file_name: "bul.spec",
        sha256: Some("0ed6d0088c5eff7f0bcbd1ef50363c3b1d645471b23bf278b0325a06361d9adb"),
    },
    Download {
        url: rules_specs!("can.spec"),
        file_name: "can.spec",
        sha256: Some("99bf8c885822ff2088f9bb2790db7c1b077f50ccd176989ec61784747c87d836"),
    },
    Download {
        url: rules_specs!("col.spec"),
        file_name: "col.spec",
        sha256: Some("02d69206fa8f903269f8181d5b1d9095b6b91155258e56772565ae084d64588c"),
    },
    Download {
        url: rules_specs!("crc.spec"),
        file_name: "crc.spec",
        sha256: Some("9439c5ba4f3f7ae397ea127c5110c73a1020cb47dad7bed7d6277c3c44127a83"),
    },
    Download {
        url: rules_specs!("cze.spec"),
        file_name: "cze.spec",
        sha256: Some("c49913ca11d3d50eb3c93fe1849c3240d3daf6ade2673f0129f56feed06cc79b"),
    },
    Download {
        url: rules_specs!("den.spec"),
        file_name: "den.spec",
        sha256: Some("96d258680448badc462115e698cdce37fe18d2538cda62e81c5282fc210eaa37"),
    },
    Download {
        url: rules_specs!("eng.spec"),
        file_name: "eng.spec",
        sha256: Some("e39f1c8069b781edc78839ce386ee2d2c32a6009604c8e7de2cfb316e843495d"),
    },
    Download {
        url: rules_specs!("esp.spec"),
        file_name: "esp.spec",
        sha256: Some("bdb4d412bdede56dc1f886528c2b82d2fa6e59ab34dc1a09b7de6e48ea5cf50d"),
    },
    Download {
        url: rules_specs!("fra.spec"),
        file_name: "fra.spec",
        sha256: Some("65d1645b33aaedae8e6e1512c0c4ba268b01d3f750447a3d3c0cb73a3e3716d3"),
    },
    Download {
        url: rules_specs!("ger.spec"),
        file_name: "ger.spec",
        sha256: Some("7959e808bbac105af18fa411ff3e53f94d474c9da6480a9f55135045db924727"),
    },
    Download {
        url: rules_specs!("gua.spec"),
        file_name: "gua.spec",
        sha256: Some("408c9682d20080b7edd822b1c6230f6e41ef81d9aed0c61c8adf7e3f7d88c255"),
    },
    Download {
        url: rules_specs!("hon.spec"),
        file_name: "hon.spec",
        sha256: Some("408c9682d20080b7edd822b1c6230f6e41ef81d9aed0c61c8adf7e3f7d88c255"),
    },
    Download {
        url: rules_specs!("ita.spec"),
        file_name: "ita.spec",
        sha256: Some("8d82deac8d519e11310ea72043027c70d6f018112a95b53744b97b8acf99626b"),
    },
    Download {
        url: rules_specs!("jpn.spec"),
        file_name: "jpn.spec",
        sha256: Some("98c4d152a1be9db5f9cabedc98939d7d7438999e06f37d3bc29d941b1e8f5c8b"),
    },
    Download {
        url: rules_specs!("mex.spec"),
        file_name: "mex.spec",
        sha256: Some("80395d18f3224eea470ba96ba114d76d1d94d6dcc7d98110e1e688299fb4dfb4"),
    },
    Download {
        url: rules_specs!("mlt.spec"),
        file_name: "mlt.spec",
        sha256: Some("0ebb361be2d9963497893d3290d5ecee729f5c2c20a5080b145057d14501cc0f"),
    },
    Download {
        url: rules_specs!("ned.spec"),
        file_name: "ned.spec",
        sha256: Some("331f6bd9a660cecabd10e51f4ce688a57770de540761d08b5f6b6fdf54fffecb"),
    },
    Download {
        url: rules_specs!("nir.spec"),
        file_name: "nir.spec",
        sha256: Some("c0cc19ab22817308f668b06178ea26fe6814ec59edc66ab1f46485080c976307"),
    },
    Download {
        url: rules_specs!("nzl.spec"),
        file_name: "nzl.spec",
        sha256: Some("30c6860738786831490428c438da52389f6adb9e80c883e3443210b68d3668e2"),
    },
    Download {
        url: rules_specs!("pan.spec"),
        file_name: "pan.spec",
        sha256: Some("ed00058393d499c919c7532f5402e49a940294be41c1cca9656e039112ff061c"),
    },
    Download {
        url: rules_specs!("par.spec"),
        file_name: "par.spec",
        sha256: Some("f68ed20f4c002e03330dc62015f0cfc7eb1f9a6b6bfd128d53be4df424173379"),
    },
    Download {
        url: rules_specs!("per.spec"),
        file_name: "per.spec",
        sha256: Some("ad870df16686ee0fb4d7f40eea02c6b58d6ba7a874a8a734e28aea6fe32e970f"),
    },
    Download {
        url: rules_specs!("por.spec"),
        file_name: "por.spec",
        sha256: Some("c220d551a3294dd96b7c061cbcc3ced98e4075b672624d0fa1c313710342fce9"),
    },
    Download {
        url: rules_specs!("rou.spec"),
        file_name: "rou.spec",
        sha256: Some("3a229379fa631ff3fecf92bb9e699aec36169d6b1e2dba83de871a0a219c18d8"),
    },
    Download {
        url: rules_specs!("sco.spec"),
        file_name: "sco.spec",
        sha256: Some("39e0661bc7267dcc847e2bd64af8e60c2725e701939fee3b5c19b027b55d1049"),
    },
    Download {
        url: rules_specs!("slv.spec"),
        file_name: "slv.spec",
        sha256: Some("b258dabfe3c3f827c4be0cd0a30ee9498f432337ebc2ef3bcaa5d978fce6f8ab"),
    },
    Download {
        url: rules_specs!("smr.spec"),
        file_name: "smr.spec",
        sha256: Some("276d2cc999dede76ffd6832fb902a8bac6135c9bc26b09781fb1e7c2f277c138"),
    },
    Download {
        url: rules_specs!("tur.spec"),
        file_name: "tur.spec",
        sha256: Some("4e7376126165642b3170ea59d96b1ca4dc1c4745045c2607eb4e3cd5821fa971"),
    },
    Download {
        url: rules_specs!("uru.spec"),
        file_name: "uru.spec",
        sha256: Some("49ad13063bee8c185b17d3931c96be2f76e8d0052f47751479df3fc06d001d05"),
    },
    Download {
        url: rules_specs!("usa.spec"),
        file_name: "usa.spec",
        sha256: Some("35e18e2fde96e3df53abaf2c7720a07719bf2cbc4c46ddd6a881d3810c6f7df7"),
    },
    Download {
        url: rules_specs!("wal.spec"),
        file_name: "wal.spec",
        sha256: Some("8e87c9eba9efdbb35450b540afbafa0a2ccd7ac664b5a1a8053e45d6a7f8f21a"),
    },
];

/// Where the rules library's packs are kept: `target/cache/downloads/rules_specs`.
#[must_use]
pub fn rules_specs_path() -> PathBuf {
    downloads_path().join(RULES_SPECS_DIR)
}

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
    fn the_rules_specs_are_pinned_and_kept_apart() {
        let [shapes, packs @ ..] = &RULES_SPECS;
        assert_eq!(shapes.file_name, "shapes.txt");
        for download in RULES_SPECS {
            assert!(download.url.ends_with(download.file_name), "{download:?}");
            assert!(
                download
                    .url
                    .contains("87b65278a15007223ec4f387f1027eb4d5cd7625")
            );
            assert!(
                download
                    .sha256
                    .is_some_and(|sha| sha.len() == ABC_SHA256.len())
            );
        }
        assert!(packs.iter().all(|pack| pack.file_name.ends_with(".spec")));
        assert_eq!(rules_specs_path(), downloads_path().join("rules_specs"));
    }

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
