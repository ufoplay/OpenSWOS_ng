//! Upstream's RNC unpacker (`openswos_oracle::unpack`): World 2026-27's foreign pool unpacks to
//! its 900 records; what is no RNC file, or states more than it holds, is refused before the C
//! is called. The pool comes from the download cache (D8): without the network, that part is
//! skipped.

#![cfg(not(oracle_unavailable))]

use openswos_oracle::unpack::{UnpackError, rnc};
use openswos_test_utils::cache::{WORLD_2627_PACKAGE, fetch};
use openswos_test_utils::require;
use openswos_world_package_resource::PackageFiles;

/// The pool's path in the download.
const POOL: &str = "teams/POOLPLYR.DAT";

/// The bytes of a pool record (upstream `PoolPlayer`).
const POOL_RECORD: usize = 58;

/// The players of the pool.
const POOL_PLAYERS: usize = 900;

#[test]
fn world_2627s_pool_unpacks_to_its_900_records() {
    let zip = require!(fetch(&WORLD_2627_PACKAGE));
    let file = std::fs::File::open(&zip).expect("the zip");
    let files = PackageFiles::read_zip(std::io::BufReader::new(file)).expect("the package");
    let packed = files.get(POOL).expect("its pool");
    let unpacked = rnc(packed).expect("unpacked");
    assert_eq!(unpacked.len(), POOL_PLAYERS * POOL_RECORD);
}

#[test]
fn what_is_no_rnc_file_is_refused() {
    assert_eq!(rnc(b"hello"), Err(UnpackError::NotPacked));
    let mut header = b"RNC\x07".to_vec();
    header.extend([0; 14]);
    assert_eq!(rnc(&header), Err(UnpackError::NotPacked), "method 7");
    let mut truncated = b"RNC\x01\x00\x00\x00\x10\x00\x00\x00\x20".to_vec();
    truncated.extend([0; 6]);
    assert_eq!(
        rnc(&truncated),
        Err(UnpackError::Truncated {
            stated: 32,
            held: 0
        })
    );
}
