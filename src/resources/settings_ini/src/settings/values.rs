//! The values the game's settings take, as `SWOS.ini` spells them (upstream
//! src/swos_settings.c `handler`, `settings_build`).

use crate::text::{eqi, truncated};

/// The words `parse_bool` reads as true.
const TRUE_WORDS: [&str; 4] = ["on", "true", "yes", "1"];

/// The words `parse_bool` reads as false.
const FALSE_WORDS: [&str; 4] = ["off", "false", "no", "0"];

/// How the table writes true.
const TRUE_TEXT: &str = "true";

/// How the table writes false.
const FALSE_TEXT: &str = "false";

/// A boolean as the game reads it (`parse_bool`): one of its words, ignoring ASCII case;
/// `None` for anything else, which the game ignores.
pub(crate) fn parse_bool(value: &[u8]) -> Option<bool> {
    let is_one_of = |words: &[&str]| words.iter().any(|word| eqi(value, word.as_bytes()));
    if is_one_of(&TRUE_WORDS) {
        Some(true)
    } else if is_one_of(&FALSE_WORDS) {
        Some(false)
    } else {
        None
    }
}

/// The words `[general] original_main_menu` reads as true: `parse_bool`'s but `yes`.
const ON_WORDS: [&str; 3] = ["true", "on", "1"];

/// A boolean as `[general] original_main_menu` reads it (upstream `handler`): one of
/// [`ON_WORDS`], ignoring ASCII case, is true; any other value is false. Faithful to the
/// original: unlike `parse_bool`, no value is ignored, and `yes` is false.
pub(crate) fn is_on(value: &[u8]) -> bool {
    ON_WORDS.iter().any(|word| eqi(value, word.as_bytes()))
}

/// A boolean as the table writes it.
pub(crate) fn bool_text(value: bool) -> Vec<u8> {
    if value { TRUE_TEXT } else { FALSE_TEXT }
        .as_bytes()
        .to_vec()
}

/// What `snprintf` keeps of `value` in a buffer of `buffer` bytes: the bytes before its NUL.
pub(crate) fn kept(value: &[u8], buffer: usize) -> Vec<u8> {
    truncated(value, buffer - 1).to_vec()
}

/// A setting's values, each spelled one way in the file.
pub trait Spelled: Sized + Copy + 'static {
    /// Every value, in the game's order.
    const ALL: &'static [Self];

    /// How the file spells it.
    fn spelling(self) -> &'static str;

    /// The value `text` spells, ignoring ASCII case (the game's `eqi`).
    fn parse(text: &[u8]) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| eqi(text, value.spelling().as_bytes()))
    }
}

/// How a match plays (`[gameplay] style`; upstream `GAMEPLAY_*`, include/dos_mode.h).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GameplayStyle {
    /// The Amiga's: 50 frames a second, the definitive release.
    #[default]
    Amiga,
    /// The PC game's: 70 frames a second, its physics, timers, formations and view.
    Dos,
    /// SWOS 2020's XBOX style: 60 frames a second, the Amiga's ball and view, the DOS
    /// goalkeepers.
    Xbox,
    /// SWOS 2020's AMIGA (60FPS): the Amiga's game at 60 frames a second.
    Amiga60,
    /// SWOS 2020's MOON SOCCER: the Amiga's, with a moon's gravity.
    Moon,
}

impl Spelled for GameplayStyle {
    const ALL: &'static [Self] = &[
        Self::Amiga,
        Self::Dos,
        Self::Xbox,
        Self::Amiga60,
        Self::Moon,
    ];

    fn spelling(self) -> &'static str {
        match self {
            Self::Amiga => "amiga",
            Self::Dos => "dos",
            Self::Xbox => "xbox",
            Self::Amiga60 => "amiga60",
            Self::Moon => "moon",
        }
    }
}

/// The computer's formations (`[gameplay] cpu_tactics`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CpuTactics {
    /// The game's own.
    #[default]
    Normal,
    /// SWOS 2020's improved formations (`--improved-cpu-tactics`).
    Expert,
}

impl Spelled for CpuTactics {
    const ALL: &'static [Self] = &[Self::Normal, Self::Expert];

    fn spelling(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Expert => "expert",
        }
    }
}

impl CpuTactics {
    /// As the game reads it: `expert`, or anything else normal.
    pub(crate) fn read(text: &[u8]) -> Self {
        if eqi(text, Self::Expert.spelling().as_bytes()) {
            Self::Expert
        } else {
            Self::Normal
        }
    }
}

/// The speed a recording starts at in the viewer (`[replays] speed`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ReplaySpeed {
    /// The match's.
    #[default]
    Normal,
    /// Half of it.
    Half,
}

impl Spelled for ReplaySpeed {
    const ALL: &'static [Self] = &[Self::Normal, Self::Half];

    fn spelling(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Half => "half",
        }
    }
}

impl ReplaySpeed {
    /// As the game reads it: `half`, or anything else normal.
    pub(crate) fn read(text: &[u8]) -> Self {
        if eqi(text, Self::Half.spelling().as_bytes()) {
            Self::Half
        } else {
            Self::Normal
        }
    }
}

/// A match's length (`[options] game_length`; upstream `_gameLength`, 0 to 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameLength {
    /// 3 minutes.
    Three,
    /// 5 minutes.
    Five,
    /// 7 minutes.
    Seven,
    /// 10 minutes.
    Ten,
}

impl Spelled for GameLength {
    const ALL: &'static [Self] = &[Self::Three, Self::Five, Self::Seven, Self::Ten];

    fn spelling(self) -> &'static str {
        match self {
            Self::Three => "3",
            Self::Five => "5",
            Self::Seven => "7",
            Self::Ten => "10",
        }
    }
}

/// The pitch (`[options] pitch_type`; upstream `_pitchType`, -2 to 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PitchType {
    /// The season's.
    Seasonal,
    /// Any.
    Random,
    /// Frozen.
    Frozen,
    /// Muddy.
    Muddy,
    /// Wet.
    Wet,
    /// Soft.
    Soft,
    /// Normal.
    Normal,
    /// Dry.
    Dry,
    /// Hard.
    Hard,
}

impl Spelled for PitchType {
    const ALL: &'static [Self] = &[
        Self::Seasonal,
        Self::Random,
        Self::Frozen,
        Self::Muddy,
        Self::Wet,
        Self::Soft,
        Self::Normal,
        Self::Dry,
        Self::Hard,
    ];

    fn spelling(self) -> &'static str {
        match self {
            Self::Seasonal => "seasonal",
            Self::Random => "random",
            Self::Frozen => "frozen",
            Self::Muddy => "muddy",
            Self::Wet => "wet",
            Self::Soft => "soft",
            Self::Normal => "normal",
            Self::Dry => "dry",
            Self::Hard => "hard",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A buffer of four bytes keeps three.
    const BUFFER: usize = 4;

    #[test]
    fn booleans_are_the_games_words_in_any_case() {
        for word in ["on", "TRUE", "Yes", "1"] {
            assert_eq!(parse_bool(word.as_bytes()), Some(true), "{word}");
        }
        for word in ["off", "False", "NO", "0"] {
            assert_eq!(parse_bool(word.as_bytes()), Some(false), "{word}");
        }
        assert_eq!(parse_bool(b"2"), None);
        assert_eq!(parse_bool(b""), None);
        assert_eq!(bool_text(true), b"true");
    }

    #[test]
    fn on_is_three_words_in_any_case_and_anything_else_is_off() {
        for word in ["true", "ON", "1"] {
            assert!(is_on(word.as_bytes()), "{word}");
        }
        for word in ["yes", "false", "off", "0", "2", ""] {
            assert!(!is_on(word.as_bytes()), "{word}");
        }
    }

    #[test]
    fn values_are_read_in_any_case_and_written_one_way() {
        assert_eq!(
            GameplayStyle::parse(b"AMIGA60"),
            Some(GameplayStyle::Amiga60)
        );
        assert_eq!(GameplayStyle::parse(b"pc"), None);
        assert_eq!(PitchType::parse(b"Muddy"), Some(PitchType::Muddy));
        assert_eq!(GameLength::parse(b"10"), Some(GameLength::Ten));
        for style in GameplayStyle::ALL {
            assert_eq!(
                GameplayStyle::parse(style.spelling().as_bytes()),
                Some(*style)
            );
        }
        assert_eq!(CpuTactics::read(b"anything"), CpuTactics::Normal);
        assert_eq!(CpuTactics::read(b"Expert"), CpuTactics::Expert);
        assert_eq!(ReplaySpeed::read(b"HALF"), ReplaySpeed::Half);
        assert_eq!(kept(b"abcdef", BUFFER), b"abc");
    }
}
