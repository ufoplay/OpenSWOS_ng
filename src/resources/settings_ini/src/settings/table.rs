//! The game's settings, each once: its section, key and comment, how the game reads it
//! (upstream `handler`) and how upstream's table writes it (`settings_build`), in that
//! table's order. Upstream builds every tweak (`PROJECT_TWEAKS`) and the testing rule
//! (`SWOS_TEST_RULES`, its Makefile's default).

use super::comments;
use super::tweaks::Tweaks;
use super::values::{
    CpuTactics, GameLength, PitchType, ReplaySpeed, Spelled, bool_text, is_on, kept, parse_bool,
};
use super::{
    DEFAULT_CURRENCY, DEFAULT_FILE_SORT, DEFAULT_LANGUAGE, DEFAULT_WORLD, GameplayStyle, Settings,
};
use crate::text::eqi;

/// `[general] language`'s buffer (`language[16]`), its NUL included.
const LANGUAGE_BUFFER: usize = 16;

/// `[general] currency`'s buffer (`currency[8]`).
const CURRENCY_BUFFER: usize = 8;

/// `[general] assets_reminder_off`'s buffer (`assets_reminder_off[64]`).
const ASSETS_REMINDER_OFF_BUFFER: usize = 64;

/// `[general] world`'s buffer (`world[8]`).
const WORLD_BUFFER: usize = 8;

/// `[general] file_sort`'s buffer (`file_sort[8]`).
const FILE_SORT_BUFFER: usize = 8;

/// The file sorts the game takes (`name`, `date`).
const FILE_SORTS: [&str; 2] = [DEFAULT_FILE_SORT, "date"];

/// `[mods]`' buffers (`mods[256]`, `mods_disabled[256]`).
const MODS_BUFFER: usize = 256;

/// How a setting's key is compared with the file's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyMatch {
    /// Ignoring ASCII case (`eqi`).
    IgnoreCase,
    /// Exactly: the tweaks' (`Tweaks_Bit` compares byte for byte).
    Exact,
}

/// A setting of the game.
pub(crate) struct Definition {
    /// Its section.
    pub(crate) section: &'static str,
    /// Its key.
    pub(crate) key: &'static str,
    /// How its key is compared.
    pub(crate) key_match: KeyMatch,
    /// The comment upstream's table writes before it.
    pub(crate) comment: Option<&'static str>,
    /// Takes a value of the file, if the game takes it.
    pub(crate) read: fn(&mut Settings, &[u8]),
    /// The value the table writes.
    pub(crate) write: fn(&Settings) -> Vec<u8>,
}

impl Definition {
    /// Whether the file's `name` in `section` is this setting.
    pub(crate) fn matches(&self, section: &[u8], name: &[u8]) -> bool {
        eqi(section, self.section.as_bytes())
            && match self.key_match {
                KeyMatch::IgnoreCase => eqi(name, self.key.as_bytes()),
                KeyMatch::Exact => name == self.key.as_bytes(),
            }
    }
}

/// A boolean setting: the game takes `parse_bool`'s words.
macro_rules! flag {
    ($section:literal, $key:literal, $comment:expr, $($field:ident).+) => {
        Definition {
            section: $section,
            key: $key,
            key_match: KeyMatch::IgnoreCase,
            comment: $comment,
            read: |settings, value| {
                if let Some(on) = parse_bool(value) {
                    settings.$($field).+ = on;
                }
            },
            write: |settings| bool_text(settings.$($field).+),
        }
    };
}

/// A tweak: its key compared exactly.
macro_rules! tweak {
    ($key:literal, $comment:expr, $tweak:ident) => {
        Definition {
            section: "tweaks",
            key: $key,
            key_match: KeyMatch::Exact,
            comment: $comment,
            read: |settings, value| {
                if let Some(on) = parse_bool(value) {
                    settings.tweaks.set(Tweaks::$tweak, on);
                }
            },
            write: |settings| bool_text(settings.tweaks.contains(Tweaks::$tweak)),
        }
    };
}

/// A boolean option: unset, the table writes the game's default.
macro_rules! option_flag {
    ($key:literal, $comment:expr, $field:ident, $default:literal) => {
        Definition {
            section: "options",
            key: $key,
            key_match: KeyMatch::IgnoreCase,
            comment: $comment,
            read: |settings, value| {
                if let Some(on) = parse_bool(value) {
                    settings.options.$field = Some(on);
                }
            },
            write: |settings| bool_text(settings.options.$field.unwrap_or($default)),
        }
    };
}

/// `text`, or `default` when it is empty (the table's `x[0] ? x : default`).
fn or_default(text: &[u8], default: &str) -> Vec<u8> {
    if text.is_empty() {
        default.as_bytes().to_vec()
    } else {
        text.to_vec()
    }
}

/// The game's settings, in upstream's table order.
pub(crate) const DEFINITIONS: [Definition; 38] = [
    flag!(
        "gameplay",
        "fire_hold_lockout",
        Some(comments::GAMEPLAY_FIRE_HOLD_LOCKOUT),
        gameplay.fire_hold_lockout
    ),
    Definition {
        section: "gameplay",
        key: "style",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::GAMEPLAY_STYLE),
        read: |settings, value| {
            if let Some(style) = GameplayStyle::parse(value) {
                settings.gameplay.style = style;
            }
        },
        write: |settings| settings.gameplay.style.spelling().as_bytes().to_vec(),
    },
    flag!(
        "replays",
        "auto_save",
        Some(comments::REPLAYS_AUTO_SAVE),
        replays.auto_save
    ),
    flag!(
        "replays",
        "frame_overlay",
        Some(comments::REPLAYS_FRAME_OVERLAY),
        replays.frame_overlay
    ),
    Definition {
        section: "replays",
        key: "speed",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::REPLAYS_SPEED),
        read: |settings, value| settings.replays.speed = ReplaySpeed::read(value),
        write: |settings| settings.replays.speed.spelling().as_bytes().to_vec(),
    },
    flag!(
        "career",
        "exact_continuation",
        Some(comments::CAREER_EXACT_CONTINUATION),
        career.exact_continuation
    ),
    flag!(
        "testing",
        "no_injuries_cards",
        Some(comments::TESTING_NO_INJURIES_CARDS),
        testing.no_injuries_cards
    ),
    Definition {
        section: "general",
        key: "language",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::GENERAL_LANGUAGE),
        read: |settings, value| {
            if !value.is_empty() {
                settings.general.language = kept(value, LANGUAGE_BUFFER);
            }
        },
        write: |settings| or_default(&settings.general.language, DEFAULT_LANGUAGE),
    },
    Definition {
        section: "general",
        key: "currency",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::GENERAL_CURRENCY),
        read: |settings, value| {
            if !value.is_empty() {
                settings.general.currency = kept(value, CURRENCY_BUFFER);
            }
        },
        write: |settings| or_default(&settings.general.currency, DEFAULT_CURRENCY),
    },
    Definition {
        section: "general",
        key: "assets_reminder_off",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::GENERAL_ASSETS_REMINDER_OFF),
        read: |settings, value| {
            settings.general.assets_reminder_off = kept(value, ASSETS_REMINDER_OFF_BUFFER);
        },
        write: |settings| settings.general.assets_reminder_off.clone(),
    },
    Definition {
        section: "general",
        key: "world",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::GENERAL_WORLD),
        read: |settings, value| {
            if !value.is_empty() {
                settings.general.world = kept(value, WORLD_BUFFER);
            }
        },
        write: |settings| or_default(&settings.general.world, DEFAULT_WORLD),
    },
    Definition {
        section: "general",
        key: "original_main_menu",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::GENERAL_ORIGINAL_MAIN_MENU),
        read: |settings, value| settings.general.original_main_menu = is_on(value),
        write: |settings| bool_text(settings.general.original_main_menu),
    },
    Definition {
        section: "general",
        key: "file_sort",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::GENERAL_FILE_SORT),
        read: |settings, value| {
            if FILE_SORTS.iter().any(|sort| eqi(value, sort.as_bytes())) {
                settings.general.file_sort = kept(value, FILE_SORT_BUFFER);
            }
        },
        write: |settings| or_default(&settings.general.file_sort, DEFAULT_FILE_SORT),
    },
    flag!(
        "general",
        "quick_access",
        Some(comments::GENERAL_QUICK_ACCESS),
        general.quick_access
    ),
    Definition {
        section: "mods",
        key: "enabled",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::MODS_ENABLED),
        read: |settings, value| settings.mods.enabled = kept(value, MODS_BUFFER),
        write: |settings| settings.mods.enabled.clone(),
    },
    Definition {
        section: "mods",
        key: "disabled",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::MODS_DISABLED),
        read: |settings, value| settings.mods.disabled = kept(value, MODS_BUFFER),
        write: |settings| settings.mods.disabled.clone(),
    },
    tweak!("cpu_cards", Some(comments::TWEAKS_CPU_CARDS), CPU_CARDS),
    tweak!(
        "no_teammate_tackles",
        Some(comments::TWEAKS_NO_TEAMMATE_TACKLES),
        NO_TEAMMATE_TACKLES
    ),
    tweak!(
        "tackle_controlled_player",
        Some(comments::TWEAKS_TACKLE_CONTROLLED_PLAYER),
        TACKLE_CONTROLLED_PLAYER
    ),
    tweak!("more_fouls", Some(comments::TWEAKS_MORE_FOULS), MORE_FOULS),
    tweak!(
        "ai_injury_subs",
        Some(comments::TWEAKS_AI_INJURY_SUBS),
        AI_INJURY_SUBS
    ),
    tweak!(
        "stronger_goalkeeper_ai",
        Some(comments::TWEAKS_STRONGER_GOALKEEPER_AI),
        STRONGER_GOALKEEPER_AI
    ),
    tweak!(
        "unlimited_transfers",
        Some(comments::TWEAKS_UNLIMITED_TRANSFERS),
        UNLIMITED_TRANSFERS
    ),
    tweak!(
        "contest_own_ratings",
        Some(comments::TWEAKS_CONTEST_OWN_RATINGS),
        CONTEST_OWN_RATINGS
    ),
    tweak!(
        "league_crash_fixes",
        Some(comments::TWEAKS_LEAGUE_CRASH_FIXES),
        LEAGUE_CRASH_FIXES
    ),
    tweak!(
        "league_rules_2020",
        Some(comments::TWEAKS_LEAGUE_RULES_2020),
        LEAGUE_RULES_2020
    ),
    tweak!(
        "three_points_for_a_win",
        Some(comments::TWEAKS_THREE_POINTS_FOR_A_WIN),
        THREE_POINTS
    ),
    tweak!(
        "five_substitutes",
        Some(comments::TWEAKS_FIVE_SUBSTITUTES),
        FIVE_SUBSTITUTES
    ),
    flag!(
        "general",
        "skip_intro",
        Some(comments::GENERAL_SKIP_INTRO),
        general.skip_intro
    ),
    flag!(
        "general",
        "skip_image_reels",
        Some(comments::GENERAL_SKIP_IMAGE_REELS),
        general.skip_image_reels
    ),
    flag!(
        "general",
        "no_load_pause",
        Some(comments::GENERAL_NO_LOAD_PAUSE),
        general.no_load_pause
    ),
    Definition {
        section: "gameplay",
        key: "cpu_tactics",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::GAMEPLAY_CPU_TACTICS),
        read: |settings, value| settings.gameplay.cpu_tactics = CpuTactics::read(value),
        write: |settings| settings.gameplay.cpu_tactics.spelling().as_bytes().to_vec(),
    },
    Definition {
        section: "options",
        key: "game_length",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::OPTIONS_GAME_LENGTH),
        read: |settings, value| {
            if let Some(length) = GameLength::parse(value) {
                settings.options.game_length = Some(length);
            }
        },
        // Unset, the first length (upstream `kLength[0]`).
        write: |settings| {
            settings
                .options
                .game_length
                .unwrap_or(GameLength::Three)
                .spelling()
                .as_bytes()
                .to_vec()
        },
    },
    Definition {
        section: "options",
        key: "pitch_type",
        key_match: KeyMatch::IgnoreCase,
        comment: Some(comments::OPTIONS_PITCH_TYPE),
        read: |settings, value| {
            if let Some(pitch) = PitchType::parse(value) {
                settings.options.pitch_type = Some(pitch);
            }
        },
        // Unset, the first pitch (upstream `kPitch[0]`).
        write: |settings| {
            settings
                .options
                .pitch_type
                .unwrap_or(PitchType::Seasonal)
                .spelling()
                .as_bytes()
                .to_vec()
        },
    },
    option_flag!(
        "auto_replays",
        Some(comments::OPTIONS_AUTO_REPLAYS),
        auto_replays,
        true
    ),
    option_flag!("all_teams_equal", None, all_teams_equal, false),
    option_flag!("auto_save_highlights", None, auto_save_highlights, true),
    option_flag!("menu_music", None, menu_music, true),
];
