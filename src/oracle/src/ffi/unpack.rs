//! src/rnc.c: upstream's RNC unpacker.

use super::bindings::rnc_c;

/// The method byte of an RNC file packed with upstream's first method (Huffman).
pub(crate) const METHOD_1: u8 = 1;

/// The method byte of the second (LZ).
pub(crate) const METHOD_2: u8 = 2;

/// `packed` (an RNC file, its header checked) unpacked into `unpacked_len` bytes by upstream's
/// decoder of `method`, or upstream's halt (the first method checks its result: 0xE003).
pub(crate) fn rnc(method: u8, packed: &[u8], unpacked_len: usize) -> Result<Vec<u8>, u16> {
    let mut source = packed.to_vec();
    let mut target = vec![0; unpacked_len];
    super::halting(|| {
        // SAFETY: the header the caller checked states `unpacked_len` bytes, the size of
        // `target`, and holds the stream `source` ends with; the stream is the
        // checksum-pinned download's, as upstream's packer wrote it.
        unsafe {
            if method == METHOD_1 {
                rnc_c::RncDecompressV1(source.as_mut_ptr(), target.as_mut_ptr());
            } else {
                rnc_c::RncDecompressV2(source.as_mut_ptr(), target.as_mut_ptr());
            }
        }
    })?;
    Ok(target)
}
