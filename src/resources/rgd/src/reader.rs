//! Reading a recording frame by frame: the biggest are 290 MB once decompressed.

use std::io::Read;

use flate2::read::GzDecoder;
use openswos_common::ByteReader;

use crate::error::RgdError;
use crate::frame::RgdFrame;
use crate::header::RgdHeader;
use crate::version::RgdVersion;

/// Bytes of the version, at the header's start.
const VERSION_LEN: usize = 2;

/// A recording read from a stream: its header, then its frames one by one.
#[derive(Debug)]
pub struct RgdReader<R> {
    stream: R,
    header: RgdHeader,
    frame_len: usize,
}

impl<R: Read> RgdReader<R> {
    /// Reads the header of the recording in `stream`.
    ///
    /// # Errors
    ///
    /// When the header does not read, or the version's frames are not known (1.2).
    pub fn new(mut stream: R) -> Result<Self, RgdError> {
        let mut version = [0; VERSION_LEN];
        stream.read_exact(&mut version)?;
        let [major, minor] = version;
        let header_len = RgdVersion::new(major, minor)?.header_len();
        let mut bytes = version.to_vec();
        bytes.resize(header_len, 0);
        stream.read_exact(bytes.get_mut(VERSION_LEN..).unwrap_or_default())?;
        let mut reader = ByteReader::new(&bytes);
        let header = RgdHeader::read(&mut reader)?;
        reader.finish()?;
        let frame_len = header.version.frame_len()?;
        Ok(Self {
            stream,
            header,
            frame_len,
        })
    }

    /// The header.
    #[must_use]
    pub fn header(&self) -> &RgdHeader {
        &self.header
    }

    /// The next frame; `None` at the stream's end.
    ///
    /// # Errors
    ///
    /// When the stream cannot be read, or ends inside a frame.
    pub fn next_frame(&mut self) -> Result<Option<RgdFrame>, RgdError> {
        let mut bytes = vec![0; self.frame_len];
        let mut filled = 0;
        while filled < self.frame_len {
            let read = self
                .stream
                .read(bytes.get_mut(filled..).unwrap_or_default())?;
            if read == 0 {
                break;
            }
            filled += read;
        }
        match filled {
            0 => Ok(None),
            whole if whole == self.frame_len => Ok(Some(RgdFrame::new(bytes))),
            partial => Err(RgdError::PartialFrame(partial)),
        }
    }
}

impl<R: Read> RgdReader<GzDecoder<R>> {
    /// Reads the header of the gzip-compressed recording in `stream` (`.RGD.gz`).
    ///
    /// # Errors
    ///
    /// As [`RgdReader::new`], and when the stream is not gzip.
    pub fn gz(stream: R) -> Result<Self, RgdError> {
        Self::new(GzDecoder::new(stream))
    }
}

impl<R: Read> Iterator for RgdReader<R> {
    type Item = Result<RgdFrame, RgdError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_frame().transpose()
    }
}
