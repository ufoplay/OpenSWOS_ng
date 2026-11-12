//! The whitespace of a save's top-level object.
//!
//! Upstream writes `{`, one top-level member a line indented by two spaces, then `\n}\n`;
//! values are compact. A career's first four members are written `"key": value`, every
//! other member `"key":value`. The writers break the pattern here and there: a career's
//! `cache_scores` follows `cache_strides` on the same line. So the whitespace is a
//! [`TopLevelLayout`]: data read with a file ([`Layout`]), or a rule that knows the
//! writer ([`UpstreamLayout`]).

/// What upstream writes before a top-level member's key: a line break and two spaces.
const MEMBER_INDENT: &[u8] = b"\n  ";

/// What upstream writes after the colon of a member written `"key": value`.
const SPACE: &[u8] = b" ";

/// What upstream writes after the last top-level member, and after the closing brace.
const NEWLINE: &[u8] = b"\n";

/// The whitespace around a save's top-level members. A member is known by its place
/// (from 0) and its key.
pub trait TopLevelLayout {
    /// Before the `{`.
    fn before_open(&self) -> &[u8];
    /// Before a member's key.
    fn before_key(&self, index: usize, key: &[u8]) -> &[u8];
    /// Between a member's key and the colon.
    fn after_key(&self, index: usize, key: &[u8]) -> &[u8];
    /// Between the colon and a member's value.
    fn after_colon(&self, index: usize, key: &[u8]) -> &[u8];
    /// After a member's value, when another member follows.
    fn after_value(&self, index: usize, key: &[u8]) -> &[u8];
    /// After the last member's value, given the number of members.
    fn before_close(&self, members: usize) -> &[u8];
    /// After the `}`.
    fn after_close(&self) -> &[u8];
}

/// No whitespace at all: a value written compact, as the values inside a save are.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Compact;

impl TopLevelLayout for Compact {
    fn before_open(&self) -> &[u8] {
        &[]
    }

    fn before_key(&self, _index: usize, _key: &[u8]) -> &[u8] {
        &[]
    }

    fn after_key(&self, _index: usize, _key: &[u8]) -> &[u8] {
        &[]
    }

    fn after_colon(&self, _index: usize, _key: &[u8]) -> &[u8] {
        &[]
    }

    fn after_value(&self, _index: usize, _key: &[u8]) -> &[u8] {
        &[]
    }

    fn before_close(&self, _members: usize) -> &[u8] {
        &[]
    }

    fn after_close(&self) -> &[u8] {
        &[]
    }
}

/// Upstream's layout: a member a line, the first `spaced_members` written
/// `"key": value`, the members named in `inline` on the line of the one before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpstreamLayout {
    /// How many members are written `"key": value`.
    pub spaced_members: usize,
    /// The keys written on the line of the member before.
    pub inline: &'static [&'static str],
}

impl TopLevelLayout for UpstreamLayout {
    fn before_open(&self) -> &[u8] {
        &[]
    }

    fn before_key(&self, _index: usize, key: &[u8]) -> &[u8] {
        if self.inline.iter().any(|inline| inline.as_bytes() == key) {
            &[]
        } else {
            MEMBER_INDENT
        }
    }

    fn after_key(&self, _index: usize, _key: &[u8]) -> &[u8] {
        &[]
    }

    fn after_colon(&self, index: usize, _key: &[u8]) -> &[u8] {
        if index < self.spaced_members {
            SPACE
        } else {
            &[]
        }
    }

    fn after_value(&self, _index: usize, _key: &[u8]) -> &[u8] {
        &[]
    }

    fn before_close(&self, _members: usize) -> &[u8] {
        NEWLINE
    }

    fn after_close(&self) -> &[u8] {
        NEWLINE
    }
}

/// The whitespace around a top-level member, as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct MemberGaps {
    /// Before its key, after the `{` or the `,`.
    pub before_key: Vec<u8>,
    /// Between its key and the colon.
    pub after_key: Vec<u8>,
    /// Between the colon and its value.
    pub after_colon: Vec<u8>,
    /// After its value, before the `,` or the `}`.
    pub after_value: Vec<u8>,
}

/// The whitespace of a save's top-level object, as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Layout {
    /// Before the `{`.
    pub before_open: Vec<u8>,
    /// Around each top-level member, in order. A member past the list gets a line of its
    /// own.
    pub members: Vec<MemberGaps>,
    /// After the `}`.
    pub after_close: Vec<u8>,
}

impl Layout {
    /// Upstream's layout of `members` top-level members, the first `spaced_members` of
    /// them written `"key": value`, each on a line of its own.
    #[must_use]
    pub fn upstream(members: usize, spaced_members: usize) -> Self {
        let gaps = (0..members)
            .map(|index| MemberGaps {
                before_key: MEMBER_INDENT.to_vec(),
                after_key: Vec::new(),
                after_colon: if index < spaced_members {
                    SPACE.to_vec()
                } else {
                    Vec::new()
                },
                after_value: if index + 1 == members {
                    NEWLINE.to_vec()
                } else {
                    Vec::new()
                },
            })
            .collect();
        Self {
            before_open: Vec::new(),
            members: gaps,
            after_close: NEWLINE.to_vec(),
        }
    }

    /// The layout of the save text `bytes`, if it is an object with at least one member.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Option<Self> {
        let mut scanner = Scanner { bytes, at: 0 };
        let before_open = scanner.whitespace();
        scanner.expect(b'{')?;
        let mut members = Vec::new();
        let mut before_key = scanner.whitespace();
        loop {
            scanner.string()?;
            let after_key = scanner.whitespace();
            scanner.expect(b':')?;
            let after_colon = scanner.whitespace();
            scanner.value()?;
            let after_value = scanner.whitespace();
            members.push(MemberGaps {
                before_key,
                after_key,
                after_colon,
                after_value,
            });
            match scanner.next()? {
                b',' => before_key = scanner.whitespace(),
                b'}' => break,
                _ => return None,
            }
        }
        let after_close = scanner.whitespace();
        scanner.at_end().then_some(Self {
            before_open,
            members,
            after_close,
        })
    }

    fn gaps(&self, index: usize) -> Option<&MemberGaps> {
        self.members.get(index)
    }
}

impl TopLevelLayout for Layout {
    fn before_open(&self) -> &[u8] {
        &self.before_open
    }

    fn before_key(&self, index: usize, _key: &[u8]) -> &[u8] {
        self.gaps(index)
            .map_or(MEMBER_INDENT, |gaps| gaps.before_key.as_slice())
    }

    fn after_key(&self, index: usize, _key: &[u8]) -> &[u8] {
        self.gaps(index)
            .map_or(&[], |gaps| gaps.after_key.as_slice())
    }

    fn after_colon(&self, index: usize, _key: &[u8]) -> &[u8] {
        self.gaps(index)
            .map_or(&[], |gaps| gaps.after_colon.as_slice())
    }

    fn after_value(&self, index: usize, _key: &[u8]) -> &[u8] {
        self.gaps(index)
            .map_or(&[], |gaps| gaps.after_value.as_slice())
    }

    fn before_close(&self, members: usize) -> &[u8] {
        members
            .checked_sub(1)
            .and_then(|last| self.gaps(last))
            .map_or(NEWLINE, |gaps| gaps.after_value.as_slice())
    }

    fn after_close(&self) -> &[u8] {
        &self.after_close
    }
}

/// Walks save text, byte by byte.
struct Scanner<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.at += 1;
        Some(byte)
    }

    fn expect(&mut self, wanted: u8) -> Option<()> {
        (self.next()? == wanted).then_some(())
    }

    fn at_end(&self) -> bool {
        self.at == self.bytes.len()
    }

    /// The whitespace from here on.
    fn whitespace(&mut self) -> Vec<u8> {
        let start = self.at;
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.at += 1;
        }
        self.bytes.get(start..self.at).unwrap_or_default().to_vec()
    }

    /// Past a string, from its opening quote.
    fn string(&mut self) -> Option<()> {
        self.expect(b'"')?;
        loop {
            match self.next()? {
                b'\\' => {
                    self.next()?;
                }
                b'"' => return Some(()),
                _ => {}
            }
        }
    }

    /// Past a value.
    fn value(&mut self) -> Option<()> {
        match self.peek()? {
            b'"' => self.string(),
            b'{' | b'[' => {
                let mut depth = 0usize;
                loop {
                    match self.peek()? {
                        b'"' => self.string()?,
                        b'{' | b'[' => {
                            depth += 1;
                            self.at += 1;
                        }
                        b'}' | b']' => {
                            depth -= 1;
                            self.at += 1;
                            if depth == 0 {
                                return Some(());
                            }
                        }
                        _ => self.at += 1,
                    }
                }
            }
            _ => {
                let start = self.at;
                while self.peek().is_some_and(|byte| {
                    !matches!(byte, b',' | b'}' | b']') && !byte.is_ascii_whitespace()
                }) {
                    self.at += 1;
                }
                (self.at > start).then_some(())
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // members the tests just read
mod tests {
    use super::*;

    #[test]
    fn reads_the_gaps_around_each_member() {
        let text = b" {\"a\" : 3,\"b\":\t\"x\" ,\n \"c\":[{\"d\":[1]},[]]}\r\n";
        let layout = Layout::of(text).expect("a layout");
        assert_eq!(layout.before_open, b" ".to_vec());
        assert_eq!(layout.members.len(), 3);
        assert_eq!(layout.members[0].after_key, b" ".to_vec());
        assert_eq!(layout.members[1].after_colon, b"\t".to_vec());
        assert_eq!(layout.members[1].after_value, b" ".to_vec());
        assert_eq!(layout.members[2].before_key, b"\n ".to_vec());
        assert_eq!(layout.after_close, b"\r\n".to_vec());
    }

    #[test]
    fn a_layout_needs_an_object_with_members() {
        for text in [
            &b"{}"[..],
            b"[1]",
            b"{\"a\":1",
            b"{\"a\":1} x",
            b"{\"a\" 1}",
            b"{\"a\":}",
        ] {
            assert_eq!(Layout::of(text), None, "{}", String::from_utf8_lossy(text));
        }
        assert!(Layout::of(b"{\"a\":\"}\\\"\",\"b\":[\"]\"]}").is_some());
    }

    #[test]
    fn compact_adds_nothing() {
        assert_eq!(Compact.before_key(0, b"a"), b"");
        assert_eq!(Compact.before_close(1), b"");
        assert_eq!(Compact.after_close(), b"");
    }

    #[test]
    fn upstreams_rule_matches_upstreams_data() {
        let rule = UpstreamLayout {
            spaced_members: 2,
            inline: &["c"],
        };
        let data = Layout::upstream(3, 2);
        for (index, key) in [(0, &b"a"[..]), (1, b"b")] {
            assert_eq!(rule.before_key(index, key), data.before_key(index, key));
            assert_eq!(rule.after_colon(index, key), data.after_colon(index, key));
        }
        assert_eq!(rule.before_key(2, b"c"), b"");
        assert_eq!(data.before_key(2, b"c"), MEMBER_INDENT);
        assert_eq!(rule.before_close(3), data.before_close(3));
        assert_eq!(rule.after_close(), data.after_close());
    }
}
