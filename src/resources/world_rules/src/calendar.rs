//! `rules/calendar.json`: the world's time.

use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/calendar.json`.
///
/// ```json
/// {
///   "first_year": 1996
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calendar {
    /// The year the world's first season starts (upstream src/world.c `kFirstYear`: 1996,
    /// 2026, 1984). The seasons' years shown and the years a world's rules count from follow
    /// it.
    pub first_year: u16,
}

impl RulesFile for Calendar {
    const FILE_NAME: &'static str = "calendar.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    #[test]
    fn it_reads_and_writes_back() {
        let text = "{\n  \"first_year\": 1996\n}\n";
        let calendar: Calendar = read(text.as_bytes()).expect("a calendar");
        assert_eq!(calendar, Calendar { first_year: 1996 });
        assert_eq!(write(&calendar), text.as_bytes());
    }

    #[test]
    fn an_unknown_key_or_a_missing_one_does_not_read() {
        assert!(read::<Calendar>(br#"{"first_year": 1996, "last_year": 2016}"#).is_err());
        assert!(read::<Calendar>(b"{}").is_err());
    }
}
