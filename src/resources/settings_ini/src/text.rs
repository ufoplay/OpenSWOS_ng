//! What upstream's C string handling makes of a file's bytes: lines as `fgets` reads them,
//! strings ending at their first NUL, `isspace` and `tolower` in the C locale.

/// The byte ending a C string.
const NUL: u8 = 0;

/// The byte ending a line.
pub(crate) const NEWLINE: u8 = b'\n';

/// A carriage return.
pub(crate) const RETURN: u8 = b'\r';

/// The vertical tab: a space to `isspace`.
const VERTICAL_TAB: u8 = 0x0B;

/// The form feed: a space to `isspace`.
const FORM_FEED: u8 = 0x0C;

/// Reads a file's bytes as `fgets` does.
pub(crate) struct Fgets<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Fgets<'a> {
    /// Reads from the start of `bytes`.
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    /// The next read into a buffer holding `max_read` bytes and the NUL: at most that many,
    /// through the first newline. `None` at the end.
    pub(crate) fn read(&mut self, max_read: usize) -> Option<&'a [u8]> {
        let rest = self.bytes.get(self.at..).filter(|rest| !rest.is_empty())?;
        let window = rest.get(..max_read).unwrap_or(rest);
        let len = window
            .iter()
            .position(|&byte| byte == NEWLINE)
            .map_or(window.len(), |at| at + 1);
        self.at += len;
        window.get(..len)
    }
}

/// The C string a buffer holds: its bytes up to the first NUL.
pub(crate) fn c_str(buffer: &[u8]) -> &[u8] {
    buffer.split(|&byte| byte == NUL).next().unwrap_or_default()
}

/// Whether `isspace` holds for `byte` in the C locale.
pub(crate) fn is_c_space(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'\t' | NEWLINE | VERTICAL_TAB | FORM_FEED | RETURN
    )
}

/// Whether a byte is a blank: a space or a tab.
pub(crate) fn is_blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// `bytes` without the leading ones `skip` holds for.
pub(crate) fn skip_start(bytes: &[u8], skip: fn(u8) -> bool) -> &[u8] {
    let skipped = bytes.iter().take_while(|&&byte| skip(byte)).count();
    bytes.get(skipped..).unwrap_or_default()
}

/// `bytes` without the trailing ones `skip` holds for.
pub(crate) fn skip_end(bytes: &[u8], skip: fn(u8) -> bool) -> &[u8] {
    let skipped = bytes.iter().rev().take_while(|&&byte| skip(byte)).count();
    bytes.get(..bytes.len() - skipped).unwrap_or_default()
}

/// The first `max` bytes of `bytes`: what a C buffer of `max + 1` bytes keeps of them.
pub(crate) fn truncated(bytes: &[u8], max: usize) -> &[u8] {
    bytes.get(..max).unwrap_or(bytes)
}

/// Upstream's `eqi`: equal but for ASCII case (`tolower` in the C locale).
pub(crate) fn eqi(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A read's size in the tests.
    const READ: usize = 4;

    #[test]
    fn fgets_stops_at_a_newline_or_the_buffer() {
        let mut lines = Fgets::new(b"ab\ncdefgh");
        assert_eq!(lines.read(READ), Some(&b"ab\n"[..]));
        assert_eq!(lines.read(READ), Some(&b"cdef"[..]));
        assert_eq!(lines.read(READ), Some(&b"gh"[..]));
        assert_eq!(lines.read(READ), None);
    }

    #[test]
    fn strings_end_at_a_nul_and_trim_as_c_does() {
        assert_eq!(c_str(b"ab\0cd"), b"ab");
        assert_eq!(c_str(b"ab"), b"ab");
        assert!(is_c_space(VERTICAL_TAB) && !is_c_space(0xA0));
        assert_eq!(skip_start(b" \tx ", is_blank), b"x ");
        assert_eq!(skip_end(b" x \x0B", is_c_space), b" x");
        assert_eq!(truncated(b"abc", 2), b"ab");
        assert!(eqi(b"Video", b"vIDEO") && !eqi(b"\xC9", b"\xE9"));
    }
}
