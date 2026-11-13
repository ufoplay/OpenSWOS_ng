//! The game's settings, typed (upstream src/swos_settings.c `handler`, `settings_build`;
//! src/main.c, their defaults): the sections the game itself reads. The view's (`[video]`,
//! `[audio]`, `[input]`, `[graphics]`), netplay's (`[netplay]`, `[chat]`) and the arcade's
//! (`[arcade]`) are not typed: they stay in the file as read, and a save leaves them alone.
//!
//! Reading is the game's: from the defaults, each value of the file in order, a value the
//! game does not take ignored (it stays in the file). Saving is the game's too: upstream's
//! table, and a setting the player did not change since the file was read keeps the file's
//! value (`Settings_Save`'s baseline).

mod comments;
mod table;
mod tweaks;
mod values;

use crate::file::IniFile;
use crate::writer::{SectionIntro, Setting, SettingsTable};

pub use tweaks::Tweaks;
pub use values::{CpuTactics, GameLength, GameplayStyle, PitchType, ReplaySpeed, Spelled};

/// The language the game starts in, and the table writes for none (`[general] language`).
const DEFAULT_LANGUAGE: &str = "en";

/// The currency the game starts with, and the table writes for none.
const DEFAULT_CURRENCY: &str = "pound";

/// The files' order the game starts with, and the table writes for none.
const DEFAULT_FILE_SORT: &str = "name";

/// The world the table writes for none: World 1996-97's tag (upstream include/world.h).
const DEFAULT_WORLD: &str = "9697";

/// What a fresh file writes after a section's header (upstream `section_intro`).
pub const SECTION_INTROS: [SectionIntro; 3] = [
    SectionIntro {
        section: "gameplay",
        intro: "         ; deliberate departures from the original Amiga game",
    },
    SectionIntro {
        section: "dos",
        intro: "              ; PC-only extras (Amiga is the definitive version)",
    },
    SectionIntro {
        section: "arcade",
        intro: "           ; coin-op mode (PC-only). See docs/ARCADE.md",
    },
];

/// `[gameplay]`: how a match plays.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GameplaySettings {
    /// The style.
    pub style: GameplayStyle,
    /// The original Amiga game's bug kept: an opponent's tackle wipes a held shot's charge.
    pub fire_hold_lockout: bool,
    /// The computer's formations.
    pub cpu_tactics: CpuTactics,
}

/// `[replays]`: the match recordings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReplaySettings {
    /// A dated copy of every match kept, not only `LATEST.SRP`.
    pub auto_save: bool,
    /// The viewer's frame box shown at first.
    pub frame_overlay: bool,
    /// The speed a recording starts at.
    pub speed: ReplaySpeed,
}

impl Default for ReplaySettings {
    fn default() -> Self {
        Self {
            auto_save: false,
            frame_overlay: true,
            speed: ReplaySpeed::Normal,
        }
    }
}

/// `[testing]`: rules for testing, off line only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TestingSettings {
    /// Nobody booked, sent off, injured or suspended.
    pub no_injuries_cards: bool,
}

/// `[general]`. Texts are bytes, as the game keeps them: cut to its buffers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GeneralSettings {
    /// The menus' language code (at most 15 bytes).
    pub language: Vec<u8>,
    /// The money symbol: `pound`, `dollar`, `euro` (at most 7 bytes).
    pub currency: Vec<u8>,
    /// The data versions whose reminder was dismissed (at most 63 bytes).
    pub assets_reminder_off: Vec<u8>,
    /// The world's tag (`9697`, `2627`; at most 7 bytes); empty for World 1996-97.
    pub world: Vec<u8>,
    /// The file lists' order: `name` or `date`, in any case, as written.
    pub file_sort: Vec<u8>,
    /// SWOS 2020's QUICK ACCESS after a friendly.
    pub quick_access: bool,
    /// No intro at start.
    pub skip_intro: bool,
    /// The intro's title only.
    pub skip_image_reels: bool,
    /// No pause on the stadium screen.
    pub no_load_pause: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: DEFAULT_LANGUAGE.as_bytes().to_vec(),
            currency: DEFAULT_CURRENCY.as_bytes().to_vec(),
            assets_reminder_off: Vec::new(),
            world: Vec::new(),
            file_sort: DEFAULT_FILE_SORT.as_bytes().to_vec(),
            quick_access: false,
            skip_intro: false,
            skip_image_reels: false,
            no_load_pause: false,
        }
    }
}

/// `[mods]`: the mods' lists, as written (at most 255 bytes each, and what a line of the file
/// holds).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ModSettings {
    /// The mods to play with, by their folders' names.
    pub enabled: Vec<u8>,
    /// The game's own mods turned off.
    pub disabled: Vec<u8>,
}

/// `[options]`: the OPTIONS screen as it was left; `None`: not in the file, the game's own
/// default (upstream `OPTION_UNSET`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GameOptions {
    /// The match length.
    pub game_length: Option<GameLength>,
    /// The pitch.
    pub pitch_type: Option<PitchType>,
    /// Goals replayed.
    pub auto_replays: Option<bool>,
    /// All the players' teams equal.
    pub all_teams_equal: Option<bool>,
    /// Highlights saved after each match.
    pub auto_save_highlights: Option<bool>,
    /// Music in the menus.
    pub menu_music: Option<bool>,
}

/// The game's settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Settings {
    /// `[gameplay]`.
    pub gameplay: GameplaySettings,
    /// `[replays]`.
    pub replays: ReplaySettings,
    /// `[testing]`.
    pub testing: TestingSettings,
    /// `[general]`.
    pub general: GeneralSettings,
    /// `[mods]`.
    pub mods: ModSettings,
    /// `[tweaks]`.
    pub tweaks: Tweaks,
    /// `[options]`.
    pub options: GameOptions,
}

impl Settings {
    /// The settings the game reads from `file` (`Settings_Load`): the defaults, then each
    /// value the game takes, in the file's order.
    #[must_use]
    pub fn read(file: &IniFile) -> Self {
        let mut settings = Self::default();
        for value in file.read().values {
            if let Some(definition) = table::DEFINITIONS
                .iter()
                .find(|definition| definition.matches(&value.section, &value.name))
            {
                (definition.read)(&mut settings, &value.value);
            }
        }
        settings
    }

    /// Upstream's table of these settings (`settings_build`), in its order, none kept.
    #[must_use]
    pub fn table(&self) -> Vec<Setting> {
        table::DEFINITIONS
            .iter()
            .map(|definition| Setting {
                section: definition.section.to_owned(),
                key: definition.key.to_owned(),
                comment: definition.comment.map(str::to_owned),
                value: (definition.write)(self),
                keep: false,
            })
            .collect()
    }

    /// A fresh file of these settings (`settings_write_full`).
    #[must_use]
    pub fn fresh_file(&self) -> IniFile {
        let settings = self.table();
        SettingsTable {
            settings: &settings,
            intros: &SECTION_INTROS,
        }
        .fresh_file()
    }

    /// Saves these settings over `file` (`None`: there is none yet) as the game does
    /// (`Settings_Save`). `baseline` is what the game read from that file, if it did: a
    /// setting the same there keeps the file's value. `None` when the file is to be left as
    /// it is.
    #[must_use]
    pub fn save(&self, file: Option<&IniFile>, baseline: Option<&Self>) -> Option<IniFile> {
        let mut settings = self.table();
        if let Some(baseline) = baseline {
            for (now, was) in settings.iter_mut().zip(baseline.table()) {
                now.keep = now.value == was.value;
            }
        }
        SettingsTable {
            settings: &settings,
            intros: &SECTION_INTROS,
        }
        .save(file)
    }
}

#[cfg(test)]
mod tests;
