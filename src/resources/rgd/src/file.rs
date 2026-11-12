//! A whole recording (`.RGD`, uncompressed), read and written byte for byte.

use openswos_common::{ByteWriter, Exporter, Importer};

use crate::error::RgdError;
use crate::frame::RgdFrame;
use crate::header::RgdHeader;
use crate::reader::RgdReader;

/// A recording: its header and its frames.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RgdFile {
    /// The header.
    pub header: RgdHeader,
    /// The frames, in order.
    pub frames: Vec<RgdFrame>,
}

/// The `.RGD` format (uncompressed; the corpus is gzip-compressed, `RgdReader::gz`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgdFormat;

impl Importer for RgdFormat {
    type Model = RgdFile;
    type Error = RgdError;

    fn import(bytes: &[u8]) -> Result<RgdFile, RgdError> {
        let mut reader = RgdReader::new(bytes)?;
        let header = reader.header().clone();
        let frames = reader.by_ref().collect::<Result<Vec<_>, _>>()?;
        Ok(RgdFile { header, frames })
    }
}

impl Exporter for RgdFormat {
    type Model = RgdFile;
    type Error = RgdError;

    fn export(model: &RgdFile) -> Result<Vec<u8>, RgdError> {
        let RgdFile { header, frames } = model;
        let version = header.version;
        let frame_len = version.frame_len()?;
        let mut writer = ByteWriter::with_capacity(version.header_len() + frames.len() * frame_len);
        header.write(&mut writer)?;
        for frame in frames {
            let len = frame.as_bytes().len();
            if len != frame_len {
                return Err(RgdError::FrameLength {
                    len,
                    expected: frame_len,
                    version,
                });
            }
            writer.write_bytes(frame.as_bytes());
        }
        Ok(writer.into_bytes())
    }
}
