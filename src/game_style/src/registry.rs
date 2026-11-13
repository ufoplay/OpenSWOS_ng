//! Every style, found by the id the settings store or by upstream's number.

use crate::style::GameStyle;
use crate::styles::{Amiga50Hz, Amiga60Hz, Dos70Hz, Moon50Hz, Xbox60Hz};

/// Every style, in upstream's order: each one at its number (`GAMEPLAY_*`). A new style is a
/// new implementation and one more entry here.
pub static STYLES: [&'static dyn GameStyle; 5] =
    [&Amiga50Hz, &Dos70Hz, &Xbox60Hz, &Amiga60Hz, &Moon50Hz];

/// The default style, and the one upstream plays when given a number it does not know
/// (`Gameplay_SetStyle`): the Amiga's.
pub static DEFAULT: &'static dyn GameStyle = &Amiga50Hz;

/// The style with the id `id` (`[gameplay] style`).
#[must_use]
pub fn by_id(id: &str) -> Option<&'static dyn GameStyle> {
    STYLES.iter().copied().find(|style| style.id() == id)
}

/// The style with upstream's number `code`. Upstream plays [`DEFAULT`] for any other.
#[must_use]
pub fn by_code(code: u8) -> Option<&'static dyn GameStyle> {
    STYLES.get(usize::from(code)).copied()
}

#[cfg(test)]
mod tests {
    use openswos_settings_ini_resource::{GameplayStyle, Spelled};

    use super::{DEFAULT, STYLES, by_code, by_id};

    #[test]
    fn each_style_is_at_its_number() {
        for (index, style) in STYLES.iter().enumerate() {
            assert_eq!(usize::from(style.code()), index, "{}", style.id());
            assert_eq!(
                by_code(style.code()).map(|found| found.id()),
                Some(style.id())
            );
        }
        let past = u8::try_from(STYLES.len()).expect("a few styles");
        assert!(by_code(past).is_none());
        assert!(by_code(u8::MAX).is_none());
    }

    #[test]
    fn the_ids_are_the_settings_spellings_in_order() {
        let spellings: Vec<&str> = GameplayStyle::ALL
            .iter()
            .map(|style| style.spelling())
            .collect();
        let ids: Vec<&str> = STYLES.iter().map(|style| style.id()).collect();
        assert_eq!(ids, spellings);
        for id in ids {
            assert_eq!(by_id(id).map(|style| style.id()), Some(id));
        }
        assert!(by_id("AMIGA").is_none());
    }

    #[test]
    fn the_default_is_the_settings_default() {
        assert_eq!(DEFAULT.id(), GameplayStyle::default().spelling());
    }
}
