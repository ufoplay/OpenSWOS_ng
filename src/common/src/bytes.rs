//! Bounds-checked reading and writing of the game's binary records.
//!
//! SWOS mixes byte orders: the records it inherited from the Amiga (team files, the
//! transfer pool) are big-endian, the PC game's own formats little-endian. Every integer
//! read and write names its byte order.

use thiserror::Error;

/// Why a read failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReadError {
    /// The data ended before the value.
    #[error("unexpected end of data at offset {offset}: {needed} bytes needed, {available} left")]
    UnexpectedEnd {
        /// Where the value starts.
        offset: usize,
        /// The bytes the value needs.
        needed: usize,
        /// The bytes left.
        available: usize,
    },
    /// Bytes are left after the last value of a format that must be read whole.
    #[error("{remaining} unexpected bytes after the end of the data, at offset {offset}")]
    TrailingBytes {
        /// Where the unread bytes start.
        offset: usize,
        /// How many there are.
        remaining: usize,
    },
}

/// Reads values from a byte slice, front to back. Every read is bounds-checked and moves
/// past the value; a failed read moves nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteReader<'a> {
    rest: &'a [u8],
    offset: usize,
}

/// Generates the integer reads of [`ByteReader`].
macro_rules! read_integer {
    ($(#[$doc:meta] $name:ident -> $ty:ty, $from:ident;)*) => {
        $(
            #[$doc]
            ///
            /// # Errors
            ///
            /// [`ReadError::UnexpectedEnd`] when the data is too short.
            pub fn $name(&mut self) -> Result<$ty, ReadError> {
                self.read_array().map(<$ty>::$from)
            }
        )*
    };
}

impl<'a> ByteReader<'a> {
    /// A reader at the start of `data`.
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            rest: data,
            offset: 0,
        }
    }

    /// How many bytes were read so far: the offset of the next value.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// How many bytes are left.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.rest.len()
    }

    /// Whether every byte was read.
    #[must_use]
    pub fn is_at_end(&self) -> bool {
        self.rest.is_empty()
    }

    /// Reads the next `N` bytes.
    ///
    /// # Errors
    ///
    /// [`ReadError::UnexpectedEnd`] when fewer are left.
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ReadError> {
        let (head, tail) = self
            .rest
            .split_first_chunk::<N>()
            .ok_or_else(|| self.unexpected_end(N))?;
        self.rest = tail;
        self.offset += N;
        Ok(*head)
    }

    /// Reads the next `len` bytes.
    ///
    /// # Errors
    ///
    /// [`ReadError::UnexpectedEnd`] when fewer are left.
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ReadError> {
        let (head, tail) = self
            .rest
            .split_at_checked(len)
            .ok_or_else(|| self.unexpected_end(len))?;
        self.rest = tail;
        self.offset += len;
        Ok(head)
    }

    /// Skips the next `len` bytes.
    ///
    /// # Errors
    ///
    /// [`ReadError::UnexpectedEnd`] when fewer are left.
    pub fn skip(&mut self, len: usize) -> Result<(), ReadError> {
        self.read_bytes(len).map(drop)
    }

    read_integer! {
        /// Reads a byte.
        read_u8 -> u8, from_be_bytes;
        /// Reads a signed byte.
        read_i8 -> i8, from_be_bytes;
        /// Reads a big-endian `u16`.
        read_u16_be -> u16, from_be_bytes;
        /// Reads a little-endian `u16`.
        read_u16_le -> u16, from_le_bytes;
        /// Reads a big-endian `i16`.
        read_i16_be -> i16, from_be_bytes;
        /// Reads a little-endian `i16`.
        read_i16_le -> i16, from_le_bytes;
        /// Reads a big-endian `u32`.
        read_u32_be -> u32, from_be_bytes;
        /// Reads a little-endian `u32`.
        read_u32_le -> u32, from_le_bytes;
        /// Reads a big-endian `i32`.
        read_i32_be -> i32, from_be_bytes;
        /// Reads a little-endian `i32`.
        read_i32_le -> i32, from_le_bytes;
    }

    /// Ends the reading of a format that must be read whole.
    ///
    /// # Errors
    ///
    /// [`ReadError::TrailingBytes`] when bytes are left.
    pub fn finish(self) -> Result<(), ReadError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(ReadError::TrailingBytes {
                offset: self.offset,
                remaining: self.rest.len(),
            })
        }
    }

    fn unexpected_end(&self, needed: usize) -> ReadError {
        ReadError::UnexpectedEnd {
            offset: self.offset,
            needed,
            available: self.rest.len(),
        }
    }
}

/// Writes values into a growing byte buffer, front to back.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ByteWriter {
    bytes: Vec<u8>,
}

/// Generates the integer writes of [`ByteWriter`].
macro_rules! write_integer {
    ($(#[$doc:meta] $name:ident($ty:ty), $to:ident;)*) => {
        $(
            #[$doc]
            pub fn $name(&mut self, value: $ty) {
                self.bytes.extend_from_slice(&value.$to());
            }
        )*
    };
}

impl ByteWriter {
    /// An empty writer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty writer with room for `capacity` bytes.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    /// How many bytes were written.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether nothing was written.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Writes `bytes` as they are.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    write_integer! {
        /// Writes a byte.
        write_u8(u8), to_be_bytes;
        /// Writes a signed byte.
        write_i8(i8), to_be_bytes;
        /// Writes a big-endian `u16`.
        write_u16_be(u16), to_be_bytes;
        /// Writes a little-endian `u16`.
        write_u16_le(u16), to_le_bytes;
        /// Writes a big-endian `i16`.
        write_i16_be(i16), to_be_bytes;
        /// Writes a little-endian `i16`.
        write_i16_le(i16), to_le_bytes;
        /// Writes a big-endian `u32`.
        write_u32_be(u32), to_be_bytes;
        /// Writes a little-endian `u32`.
        write_u32_le(u32), to_le_bytes;
        /// Writes a big-endian `i32`.
        write_i32_be(i32), to_be_bytes;
        /// Writes a little-endian `i32`.
        write_i32_le(i32), to_le_bytes;
    }

    /// The bytes written.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn reads_integers_in_both_byte_orders() {
        let data = [0x12, 0x34, 0x12, 0x34, 0xFF, 0xFE, 0x01, 0x02, 0x03, 0x04];
        let mut reader = ByteReader::new(&data);
        assert_eq!(reader.read_u16_be(), Ok(0x1234));
        assert_eq!(reader.read_u16_le(), Ok(0x3412));
        assert_eq!(reader.read_i16_be(), Ok(-2));
        assert_eq!(reader.read_u32_be(), Ok(0x0102_0304));
        assert!(reader.is_at_end());
        assert_eq!(reader.offset(), data.len());
    }

    #[test]
    fn reads_signed_values() {
        let data = [0x80, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF, 0x7F];
        let mut reader = ByteReader::new(&data);
        assert_eq!(reader.read_i8(), Ok(i8::MIN));
        assert_eq!(reader.read_i8(), Ok(-1));
        assert_eq!(reader.read_i32_be(), Ok(-2));
        assert_eq!(reader.read_i32_le(), Ok(0x7FFF_FFFF));
    }

    #[test]
    fn reads_bytes_and_arrays() {
        let data = *b"SWOS96/97";
        let mut reader = ByteReader::new(&data);
        assert_eq!(reader.read_array::<4>(), Ok(*b"SWOS"));
        assert_eq!(reader.read_bytes(2), Ok(&b"96"[..]));
        reader.skip(1).expect("one byte left to skip");
        assert_eq!(reader.remaining(), 2);
        assert_eq!(reader.read_u8(), Ok(b'9'));
        assert_eq!(
            reader.read_i16_le().map(drop),
            Err(ReadError::UnexpectedEnd {
                offset: 8,
                needed: 2,
                available: 1,
            })
        );
    }

    #[test]
    fn a_failed_read_moves_nothing() {
        let data = [1, 2, 3];
        let mut reader = ByteReader::new(&data);
        assert!(reader.read_u32_le().is_err());
        assert!(reader.read_bytes(4).is_err());
        assert!(reader.skip(4).is_err());
        assert_eq!(reader.offset(), 0);
        assert_eq!(reader.remaining(), 3);
    }

    #[test]
    fn finish_requires_every_byte_read() {
        let data = [1, 2, 3];
        let mut reader = ByteReader::new(&data);
        reader.read_u8().expect("a byte");
        assert_eq!(
            reader.clone().finish(),
            Err(ReadError::TrailingBytes {
                offset: 1,
                remaining: 2,
            })
        );
        reader.skip(2).expect("two bytes");
        assert_eq!(reader.finish(), Ok(()));
    }

    #[test]
    fn errors_describe_themselves() {
        let error = ReadError::UnexpectedEnd {
            offset: 684,
            needed: 38,
            available: 2,
        };
        assert_eq!(
            error.to_string(),
            "unexpected end of data at offset 684: 38 bytes needed, 2 left"
        );
    }

    #[test]
    fn writes_integers_in_both_byte_orders() {
        let mut writer = ByteWriter::with_capacity(16);
        assert!(writer.is_empty());
        writer.write_u16_be(0x1234);
        writer.write_u16_le(0x1234);
        writer.write_i8(-1);
        writer.write_u8(7);
        writer.write_bytes(b"AB");
        writer.write_i16_be(-2);
        writer.write_i16_le(-2);
        assert_eq!(writer.len(), 12);
        assert_eq!(
            writer.into_bytes(),
            vec![
                0x12, 0x34, 0x34, 0x12, 0xFF, 7, b'A', b'B', 0xFF, 0xFE, 0xFE, 0xFF
            ]
        );
    }

    #[test]
    fn writes_32_bit_integers() {
        let mut writer = ByteWriter::new();
        writer.write_u32_be(0x0102_0304);
        writer.write_u32_le(0x0102_0304);
        writer.write_i32_be(-2);
        writer.write_i32_le(-2);
        assert_eq!(
            writer.into_bytes(),
            vec![
                1, 2, 3, 4, 4, 3, 2, 1, 0xFF, 0xFF, 0xFF, 0xFE, 0xFE, 0xFF, 0xFF, 0xFF
            ]
        );
    }

    proptest! {
        #[test]
        fn written_integers_read_back(a: u16, b: i16, c: u32, d: i32, e: u8, f: i8) {
            let mut writer = ByteWriter::new();
            writer.write_u16_be(a);
            writer.write_u16_le(a);
            writer.write_i16_be(b);
            writer.write_i16_le(b);
            writer.write_u32_be(c);
            writer.write_u32_le(c);
            writer.write_i32_be(d);
            writer.write_i32_le(d);
            writer.write_u8(e);
            writer.write_i8(f);
            let bytes = writer.into_bytes();
            let mut reader = ByteReader::new(&bytes);
            prop_assert_eq!(reader.read_u16_be(), Ok(a));
            prop_assert_eq!(reader.read_u16_le(), Ok(a));
            prop_assert_eq!(reader.read_i16_be(), Ok(b));
            prop_assert_eq!(reader.read_i16_le(), Ok(b));
            prop_assert_eq!(reader.read_u32_be(), Ok(c));
            prop_assert_eq!(reader.read_u32_le(), Ok(c));
            prop_assert_eq!(reader.read_i32_be(), Ok(d));
            prop_assert_eq!(reader.read_i32_le(), Ok(d));
            prop_assert_eq!(reader.read_u8(), Ok(e));
            prop_assert_eq!(reader.read_i8(), Ok(f));
            prop_assert_eq!(reader.finish(), Ok(()));
        }

        #[test]
        fn reading_never_panics_and_tracks_the_offset(data: Vec<u8>, lens: Vec<usize>) {
            let mut reader = ByteReader::new(&data);
            for len in lens {
                let before = reader.offset();
                match reader.read_bytes(len) {
                    Ok(bytes) => prop_assert_eq!(bytes.len(), len),
                    Err(_) => prop_assert_eq!(reader.offset(), before),
                }
                prop_assert_eq!(reader.offset() + reader.remaining(), data.len());
            }
        }
    }
}
