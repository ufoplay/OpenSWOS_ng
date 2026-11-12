//! The `.SRP` container (upstream docs/REPLAY_FORMAT_V2.md §4, src/srp_writer.c): a 32-byte
//! header, a 16-byte directory entry per chunk, the chunks back to back. Big-endian.
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 4 | `SWRP` |
//! | 4 | 2 | version: 1 |
//! | 6 | 2 | flags: 1 audio, 2 seek, 4 chapters |
//! | 8 | 2 | the tick rate (50 Amiga gameplay, 70 DOS) |
//! | 10 | 2 | quirks: 1 the original fire-hold lockout |
//! | 12 | 2 + 2 | the pitch's width and height |
//! | 16 | 4 | the frames |
//! | 20 | 2 | the chunks |
//! | 22 | 2 | 0 |
//! | 24 | 4 | the CRC-32 of everything after the header |
//! | 28 | 4 | 0 |
//!
//! A directory entry is the chunk's fourcc, offset, stored size and raw size. A chunk whose
//! stored size is its raw size is stored as it is; any other is raw deflate.
//!
//! **Byte for byte.** Upstream deflates with miniz at its best compression, which no other
//! deflater reproduces byte for byte. So a deflated chunk keeps the bytes it was read with,
//! and writes them back while its content is unchanged; content set anew is deflated
//! here, not as upstream would.

use std::io::{Read, Write};

use bitflags::bitflags;
use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use openswos_common::{ByteReader, ByteWriter, Exporter, Importer, ReadError};
use thiserror::Error;

/// The file's magic.
pub const SWRP_MAGIC: [u8; 4] = *b"SWRP";

/// The version this crate reads (upstream `SWRP_VERSION`).
pub const SWRP_VERSION: u16 = 1;

/// Bytes of the header (upstream `SWRP_HEADER_BYTES`).
pub const SWRP_HEADER_LEN: usize = 32;

/// Bytes of a directory entry (upstream `SWRP_DIR_BYTES`).
pub const SWRP_DIR_LEN: usize = 16;

/// The bytes a chunk must exceed for upstream to try deflating it (`chunk_make`).
const DEFLATE_MIN: usize = 256;

bitflags! {
    /// What a recording holds (upstream `SWRP_FLAG_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SrpFlags: u16 {
        /// Its sounds.
        const AUDIO = 0x0001;
        /// A seek table.
        const SEEK = 0x0002;
        /// Chapters.
        const CHAPTERS = 0x0004;
        // Flags of later builds are kept as read.
        const _ = !0;
    }
}

bitflags! {
    /// The original's behaviours a recording was made against (upstream `SWRP_QUIRK_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SrpQuirks: u16 {
        /// The original's fire-hold lockout (an opponent's tackle wipes a held shot).
        const ORIGINAL_FIRE_HOLD_LOCKOUT = 0x0001;
        // Quirks of later builds are kept as read.
        const _ = !0;
    }
}

/// A chunk's four-character code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FourCc(pub [u8; 4]);

impl FourCc {
    /// The match: its teams, settings and score (stored).
    pub const META: Self = Self(*b"META");
    /// The frames (deflated).
    pub const FRMS: Self = Self(*b"FRMS");
    /// The sounds (deflated).
    pub const AUDI: Self = Self(*b"AUDI");
    /// The seek table (stored).
    pub const SEEK: Self = Self(*b"SEEK");
    /// The chapters (stored).
    pub const CHAP: Self = Self(*b"CHAP");
    /// Who played (stored).
    pub const PLYR: Self = Self(*b"PLYR");
    /// Which builds can re-simulate the match (stored).
    pub const SIMV: Self = Self(*b"SIMV");
    /// The match setup (deflated).
    pub const SETP: Self = Self(*b"SETP");
    /// The state at frame 0 (stored; deflated already).
    pub const STAT: Self = Self(*b"STAT");
    /// Both players' input, a frame each (deflated).
    pub const INPT: Self = Self(*b"INPT");
    /// Each frame's state hash (stored).
    pub const HASH: Self = Self(*b"HASH");
    /// What each player's game guessed (deflated).
    pub const PRED: Self = Self(*b"PRED");
    /// Where each player saw things (deflated).
    pub const SEEN: Self = Self(*b"SEEN");
    /// What each player's game read (deflated).
    pub const READ: Self = Self(*b"READ");
}

/// How a chunk is held in the file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Storage {
    /// As it is.
    Stored,
    /// Raw deflate: the bytes as read, while the content is the one they inflate to.
    Deflated(Option<Vec<u8>>),
}

/// A chunk: its code, its content, how it is held.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SrpChunk {
    /// Its code.
    pub fourcc: FourCc,
    data: Vec<u8>,
    storage: Storage,
}

impl SrpChunk {
    /// A chunk of `data`, stored or deflated as `deflate` says.
    #[must_use]
    pub fn new(fourcc: FourCc, data: Vec<u8>, deflate: bool) -> Self {
        let storage = if deflate {
            Storage::Deflated(None)
        } else {
            Storage::Stored
        };
        Self {
            fourcc,
            data,
            storage,
        }
    }

    /// Its content.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// How it is held.
    #[must_use]
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Sets its content: a deflated chunk is deflated again when written.
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
        if let Storage::Deflated(read) = &mut self.storage {
            *read = None;
        }
    }

    /// Its bytes in the file.
    fn stored_bytes(&self) -> Result<Vec<u8>, SrpError> {
        match &self.storage {
            Storage::Stored => Ok(self.data.clone()),
            Storage::Deflated(Some(read)) => Ok(read.clone()),
            Storage::Deflated(None) => {
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
                encoder.write_all(&self.data)?;
                let packed = encoder.finish()?;
                // Upstream's rule: deflated when it saves anything; otherwise as it is
                // (which the directory then says, its stored size being its raw size).
                if self.data.len() > DEFLATE_MIN && packed.len() < self.data.len() {
                    Ok(packed)
                } else {
                    Ok(self.data.clone())
                }
            }
        }
    }
}

/// A recording's header, but its chunk count and CRC (the file's own).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SrpHeader {
    /// What it holds.
    pub flags: SrpFlags,
    /// Frames a second it was played at.
    pub tick_rate: u16,
    /// The original's behaviours it was made against.
    pub quirks: SrpQuirks,
    /// The pitch's width (0x2A0).
    pub pitch_width: u16,
    /// The pitch's height (0x370).
    pub pitch_height: u16,
    /// The frames.
    pub frame_count: u32,
    /// The word after the chunk count, kept as written (0).
    pub reserved_22: u16,
    /// The long word after the CRC, kept as written (0).
    pub reserved_28: u32,
}

/// A recording: its header and its chunks, in the file's order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SrpFile {
    /// The header.
    pub header: SrpHeader,
    /// The chunks.
    pub chunks: Vec<SrpChunk>,
}

impl SrpFile {
    /// The first chunk with `fourcc`.
    #[must_use]
    pub fn chunk(&self, fourcc: FourCc) -> Option<&SrpChunk> {
        self.chunks.iter().find(|chunk| chunk.fourcc == fourcc)
    }

    /// The first chunk with `fourcc`, to change.
    pub fn chunk_mut(&mut self, fourcc: FourCc) -> Option<&mut SrpChunk> {
        self.chunks.iter_mut().find(|chunk| chunk.fourcc == fourcc)
    }
}

/// Why a recording does not read or write.
#[derive(Debug, Error)]
pub enum SrpError {
    /// The data ended early.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// A chunk could not be inflated or deflated.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Not `SWRP`.
    #[error("not a .SRP: {0:?}")]
    Magic([u8; 4]),
    /// Another version.
    #[error("version {0}: version {SWRP_VERSION} is read")]
    Version(u16),
    /// The CRC is not the file's.
    #[error("the CRC says {stated:08X}, the file's is {computed:08X}")]
    Crc {
        /// The header's.
        stated: u32,
        /// The file's.
        computed: u32,
    },
    /// A chunk is not where the one before it ends, or past the file.
    #[error("chunk {index} at {offset}: not right after the one before it, as written")]
    Layout {
        /// The chunk.
        index: usize,
        /// Its offset.
        offset: u32,
    },
    /// A chunk inflates to another size than its entry says.
    #[error("{fourcc:?}: inflates to {inflated} bytes, its entry says {raw}")]
    RawSize {
        /// The chunk.
        fourcc: FourCc,
        /// Its entry's raw size.
        raw: u32,
        /// What it inflated to.
        inflated: usize,
    },
    /// More chunks, or a bigger file, than the header's fields hold.
    #[error("too big for the header's fields")]
    TooBig,
}

/// The `.SRP` format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrpFormat;

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(bytes);
    crc.sum()
}

impl Importer for SrpFormat {
    type Model = SrpFile;
    type Error = SrpError;

    fn import(bytes: &[u8]) -> Result<SrpFile, SrpError> {
        let mut reader = ByteReader::new(bytes);
        let magic: [u8; 4] = reader.read_array()?;
        if magic != SWRP_MAGIC {
            return Err(SrpError::Magic(magic));
        }
        let version = reader.read_u16_be()?;
        if version != SWRP_VERSION {
            return Err(SrpError::Version(version));
        }
        let flags = SrpFlags::from_bits_retain(reader.read_u16_be()?);
        let tick_rate = reader.read_u16_be()?;
        let quirks = SrpQuirks::from_bits_retain(reader.read_u16_be()?);
        let pitch_width = reader.read_u16_be()?;
        let pitch_height = reader.read_u16_be()?;
        let frame_count = reader.read_u32_be()?;
        let count = usize::from(reader.read_u16_be()?);
        let reserved_22 = reader.read_u16_be()?;
        let stated = reader.read_u32_be()?;
        let reserved_28 = reader.read_u32_be()?;
        let computed = crc32(bytes.get(SWRP_HEADER_LEN..).unwrap_or_default());
        if stated != computed {
            return Err(SrpError::Crc { stated, computed });
        }
        let mut next = SWRP_HEADER_LEN + count * SWRP_DIR_LEN;
        let mut chunks = Vec::with_capacity(count);
        for index in 0..count {
            let fourcc = FourCc(reader.read_array()?);
            let offset = reader.read_u32_be()?;
            let stored = reader.read_u32_be()?;
            let raw = reader.read_u32_be()?;
            let layout = SrpError::Layout { index, offset };
            let start = usize::try_from(offset).map_err(|_| SrpError::TooBig)?;
            let len = usize::try_from(stored).map_err(|_| SrpError::TooBig)?;
            if start != next {
                return Err(layout);
            }
            let held = bytes.get(start..start + len).ok_or(layout)?;
            next = start + len;
            let chunk = if stored == raw {
                SrpChunk::new(fourcc, held.to_vec(), false)
            } else {
                let mut data = Vec::new();
                DeflateDecoder::new(held).read_to_end(&mut data)?;
                if usize::try_from(raw).ok() != Some(data.len()) {
                    return Err(SrpError::RawSize {
                        fourcc,
                        raw,
                        inflated: data.len(),
                    });
                }
                SrpChunk {
                    fourcc,
                    data,
                    storage: Storage::Deflated(Some(held.to_vec())),
                }
            };
            chunks.push(chunk);
        }
        if next != bytes.len() {
            return Err(SrpError::Layout {
                index: count,
                offset: u32::try_from(next).unwrap_or(u32::MAX),
            });
        }
        Ok(SrpFile {
            header: SrpHeader {
                flags,
                tick_rate,
                quirks,
                pitch_width,
                pitch_height,
                frame_count,
                reserved_22,
                reserved_28,
            },
            chunks,
        })
    }
}

impl Exporter for SrpFormat {
    type Model = SrpFile;
    type Error = SrpError;

    fn export(model: &SrpFile) -> Result<Vec<u8>, SrpError> {
        let SrpFile { header, chunks } = model;
        let SrpHeader {
            flags,
            tick_rate,
            quirks,
            pitch_width,
            pitch_height,
            frame_count,
            reserved_22,
            reserved_28,
        } = header;
        let held = chunks
            .iter()
            .map(SrpChunk::stored_bytes)
            .collect::<Result<Vec<_>, _>>()?;
        let mut body = ByteWriter::new();
        let mut offset = SWRP_HEADER_LEN + chunks.len() * SWRP_DIR_LEN;
        let size = |value: usize| u32::try_from(value).map_err(|_| SrpError::TooBig);
        for (chunk, bytes) in chunks.iter().zip(&held) {
            body.write_bytes(&chunk.fourcc.0);
            body.write_u32_be(size(offset)?);
            body.write_u32_be(size(bytes.len())?);
            body.write_u32_be(size(chunk.data.len())?);
            offset += bytes.len();
        }
        for bytes in &held {
            body.write_bytes(bytes);
        }
        let body = body.into_bytes();
        let mut writer = ByteWriter::with_capacity(SWRP_HEADER_LEN + body.len());
        writer.write_bytes(&SWRP_MAGIC);
        writer.write_u16_be(SWRP_VERSION);
        writer.write_u16_be(flags.bits());
        writer.write_u16_be(*tick_rate);
        writer.write_u16_be(quirks.bits());
        writer.write_u16_be(*pitch_width);
        writer.write_u16_be(*pitch_height);
        writer.write_u32_be(*frame_count);
        writer.write_u16_be(u16::try_from(chunks.len()).map_err(|_| SrpError::TooBig)?);
        writer.write_u16_be(*reserved_22);
        writer.write_u32_be(crc32(&body));
        writer.write_u32_be(*reserved_28);
        writer.write_bytes(&body);
        Ok(writer.into_bytes())
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into bytes the tests build
mod tests {
    use super::*;

    fn file(chunks: Vec<SrpChunk>) -> SrpFile {
        SrpFile {
            header: SrpHeader {
                flags: SrpFlags::AUDIO | SrpFlags::SEEK | SrpFlags::CHAPTERS,
                tick_rate: 50,
                quirks: SrpQuirks::empty(),
                pitch_width: 0x2A0,
                pitch_height: 0x370,
                frame_count: 0,
                reserved_22: 0,
                reserved_28: 0,
            },
            chunks,
        }
    }

    #[test]
    fn a_file_reads_back_and_keeps_its_deflated_bytes() {
        let big = vec![7; 1000];
        let model = file(vec![
            SrpChunk::new(FourCc::META, vec![1, 2, 3], false),
            SrpChunk::new(FourCc::FRMS, big.clone(), true),
            SrpChunk::new(FourCc::AUDI, vec![9; 8], true),
        ]);
        let bytes = SrpFormat::export(&model).expect("writes");
        let read = SrpFormat::import(&bytes).expect("reads");
        assert_eq!(read.chunk(FourCc::FRMS).map(SrpChunk::data), Some(&big[..]));
        assert!(matches!(
            read.chunk(FourCc::FRMS).map(SrpChunk::storage),
            Some(Storage::Deflated(Some(_)))
        ));
        assert_eq!(
            read.chunk(FourCc::AUDI).map(SrpChunk::storage),
            Some(&Storage::Stored),
            "too small to deflate: stored"
        );
        assert_eq!(SrpFormat::export(&read).ok(), Some(bytes));
    }

    #[test]
    fn a_damaged_file_does_not_read() {
        let model = file(vec![SrpChunk::new(FourCc::META, vec![1, 2, 3], false)]);
        let bytes = SrpFormat::export(&model).expect("writes");
        let mut damaged = bytes.clone();
        if let Some(last) = damaged.last_mut() {
            *last ^= 1;
        }
        assert!(matches!(
            SrpFormat::import(&damaged),
            Err(SrpError::Crc { .. })
        ));
        let mut magic = bytes.clone();
        magic[0] = b'X';
        assert!(matches!(SrpFormat::import(&magic), Err(SrpError::Magic(_))));
        let mut version = bytes;
        version[5] = 2;
        assert!(matches!(
            SrpFormat::import(&version),
            Err(SrpError::Version(2))
        ));
    }

    #[test]
    fn new_content_is_deflated_again() {
        let mut model = file(vec![SrpChunk::new(FourCc::FRMS, vec![1; 1000], true)]);
        let bytes = SrpFormat::export(&model).expect("writes");
        let mut read = SrpFormat::import(&bytes).expect("reads");
        read.chunk_mut(FourCc::FRMS)
            .expect("frames")
            .set_data(vec![2; 2000]);
        let again = SrpFormat::import(&SrpFormat::export(&read).expect("writes")).expect("reads");
        assert_eq!(
            again.chunk(FourCc::FRMS).map(|chunk| chunk.data().len()),
            Some(2000)
        );
        model.chunks.clear();
        assert!(SrpFormat::export(&model).is_ok());
    }
}
