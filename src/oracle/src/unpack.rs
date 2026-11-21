//! Upstream's RNC unpacker (src/rnc.c), for `world_extract`: World 2026-27's foreign players'
//! pool is RNC-packed, and the completion unpacks it with upstream's own decoder, so no
//! importer of the port reads RNC and the port has no codec of its own (D9). Always built:
//! without the checkout it is [`UnpackError::Unavailable`].

use thiserror::Error;

use crate::tables::Unavailable;

/// What an RNC file starts with (upstream src/rnc.c `RNC_SIG_V1`, without its method byte).
const SIGNATURE: &[u8] = b"RNC";

/// The bytes of an RNC file's header: signature, method, unpacked and packed sizes, their
/// checksums, the leeway and the chunks (upstream `RncDecompressV2` starts its stream at
/// `src + 0x12`).
const HEADER_LEN: usize = 18;

/// Where the method byte is.
const METHOD_AT: usize = 3;

/// Where the unpacked size is (32 bits, big-endian).
const UNPACKED_AT: usize = 4;

/// Where the packed size is (32 bits, big-endian).
const PACKED_AT: usize = 8;

/// The bytes of a size field.
const SIZE_LEN: usize = 4;

/// Why an RNC file does not unpack.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnpackError {
    /// The oracle is not built.
    #[error(transparent)]
    Unavailable(#[from] Unavailable),
    /// The bytes are no RNC file of a method upstream unpacks.
    #[error("not an RNC file packed with method 1 or 2")]
    NotPacked,
    /// The header states more packed bytes than the file holds.
    #[error("the RNC header states {stated} packed bytes, the file holds {held}")]
    Truncated {
        /// The packed bytes the header states.
        stated: usize,
        /// The bytes after the header.
        held: usize,
    },
    /// Upstream's decoder halted (0xE003: the first method's result is not the header's).
    #[error("upstream's unpacker halted: reason {0:#06X}")]
    Halted(u16),
}

/// `packed` (an RNC file) unpacked by upstream's decoder. Its header is checked here; the
/// stream itself is trusted, so `packed` must be upstream's packer's output: World 2026-27's
/// pool, from the checksum-pinned download.
///
/// # Errors
///
/// [`UnpackError`] without the checkout, for bytes that are no RNC file, a header stating more
/// than the file holds, or upstream's halt.
pub fn rnc(packed: &[u8]) -> Result<Vec<u8>, UnpackError> {
    if !packed.starts_with(SIGNATURE) {
        return Err(UnpackError::NotPacked);
    }
    let method = packed
        .get(METHOD_AT)
        .copied()
        .ok_or(UnpackError::NotPacked)?;
    let size = |at: usize| -> Result<usize, UnpackError> {
        let bytes: [u8; SIZE_LEN] = packed
            .get(at..at + SIZE_LEN)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(UnpackError::NotPacked)?;
        usize::try_from(u32::from_be_bytes(bytes)).map_err(|_| UnpackError::NotPacked)
    };
    let unpacked = size(UNPACKED_AT)?;
    let stated = size(PACKED_AT)?;
    let held = packed.len().saturating_sub(HEADER_LEN);
    if packed.len() < HEADER_LEN || stated > held {
        return Err(UnpackError::Truncated { stated, held });
    }
    run(method, packed, unpacked)
}

/// Unpacks with the decoder of `method`.
#[cfg(not(oracle_unavailable))]
fn run(method: u8, packed: &[u8], unpacked: usize) -> Result<Vec<u8>, UnpackError> {
    use crate::ffi::unpack::{METHOD_1, METHOD_2};
    if method != METHOD_1 && method != METHOD_2 {
        return Err(UnpackError::NotPacked);
    }
    let _held = crate::Oracle::lock();
    crate::ffi::unpack::rnc(method, packed, unpacked).map_err(UnpackError::Halted)
}

/// No unpacker without the checkout.
#[cfg(oracle_unavailable)]
fn run(_method: u8, _packed: &[u8], _unpacked: usize) -> Result<Vec<u8>, UnpackError> {
    Err(UnpackError::Unavailable(Unavailable))
}
