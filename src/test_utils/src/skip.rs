//! Tests whose data is missing are skipped, and say why.
//!
//! Rust tests cannot mark themselves skipped at run time, so a test without its data
//! reports the reason on stderr and returns (it passes). Where the data must be present
//! (e.g. a CI job that provides it), [`STRICT_ENV`] turns every skip into a failure.

use std::ffi::OsStr;
use std::fmt;
use std::io::Write;

/// Environment variable that turns skipped tests into failures (any value but `0`).
pub const STRICT_ENV: &str = "OPENSWOS_STRICT_TEST_DATA";

/// The value of [`STRICT_ENV`] that keeps skips lenient.
const LENIENT_VALUE: &str = "0";

/// Why a test cannot run: its data is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    reason: String,
}

impl Skip {
    /// A skip for `reason`.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    /// Why the test cannot run.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)
    }
}

/// What a missing piece of test data does to a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    /// The test is skipped.
    Lenient,
    /// The test fails.
    Strict,
}

impl Strictness {
    /// The strictness [`STRICT_ENV`] asks for.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_value(std::env::var_os(STRICT_ENV).as_deref())
    }

    /// The strictness a value of [`STRICT_ENV`] asks for: strict when it is set to
    /// anything but `0` or nothing.
    #[must_use]
    pub fn from_value(value: Option<&OsStr>) -> Self {
        match value {
            Some(value) if !value.is_empty() && value != LENIENT_VALUE => Self::Strict,
            Some(_) | None => Self::Lenient,
        }
    }
}

/// Reports a skipped test, named after the running test: on stderr, or as a failure when
/// [`STRICT_ENV`] is set.
///
/// # Panics
///
/// In strict mode, always.
pub fn report(skip: &Skip) {
    report_with(skip, Strictness::from_env());
}

/// [`report`] with an explicit strictness.
///
/// # Panics
///
/// When `strictness` is [`Strictness::Strict`].
pub fn report_with(skip: &Skip, strictness: Strictness) {
    let current = std::thread::current();
    let test = current.name().unwrap_or("a test");
    match strictness {
        // Straight to the stderr handle: the test harness captures `eprintln!` of passing
        // tests, and a skip must be seen.
        Strictness::Lenient => {
            let _ignored = writeln!(std::io::stderr().lock(), "SKIPPED {test}: {skip}");
        }
        Strictness::Strict => panic!("{test} lacks its test data ({STRICT_ENV} is set): {skip}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_carry_their_reason() {
        let skip = Skip::new("set OPENSWOS_GAME_DIR");
        assert_eq!(skip.reason(), "set OPENSWOS_GAME_DIR");
        assert_eq!(skip.to_string(), "set OPENSWOS_GAME_DIR");
    }

    #[test]
    fn strictness_follows_the_variable() {
        assert_eq!(Strictness::from_value(None), Strictness::Lenient);
        assert_eq!(
            Strictness::from_value(Some(OsStr::new(""))),
            Strictness::Lenient
        );
        assert_eq!(
            Strictness::from_value(Some(OsStr::new("0"))),
            Strictness::Lenient
        );
        assert_eq!(
            Strictness::from_value(Some(OsStr::new("1"))),
            Strictness::Strict
        );
        assert_eq!(
            Strictness::from_value(Some(OsStr::new("yes"))),
            Strictness::Strict
        );
    }

    #[test]
    fn a_lenient_report_returns() {
        report_with(
            &Skip::new("expected: this test checks the skip report itself"),
            Strictness::Lenient,
        );
    }

    #[test]
    #[should_panic(expected = "lacks its test data")]
    fn a_strict_report_fails_the_test() {
        report_with(&Skip::new("no data"), Strictness::Strict);
    }

    #[test]
    fn require_returns_early_on_a_skip() {
        fn body(data: Result<u8, Skip>, reached: &mut bool) {
            let value = crate::require!(data);
            assert_eq!(value, 7);
            *reached = true;
        }
        let mut reached = false;
        body(Ok(7), &mut reached);
        assert!(reached);
        let mut reached = false;
        if Strictness::from_env() == Strictness::Lenient {
            body(
                Err(Skip::new(
                    "expected: this test checks the skip report itself",
                )),
                &mut reached,
            );
            assert!(!reached);
        }
    }
}
