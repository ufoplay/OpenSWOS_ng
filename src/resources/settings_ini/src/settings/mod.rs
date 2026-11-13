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
mod tests {
    //! The typed settings: read as the game reads them, written as upstream's table writes them.

    use super::*;
    use crate::writer::FRESH_FILE_HEADER;

    /// The bytes inih reads of a line: its 200-byte buffer, less the NUL.
    const LINE_BYTES: usize = 199;

    /// What `language`'s 16-byte buffer keeps.
    const LANGUAGE_BYTES: usize = 15;

    fn file(text: &str) -> IniFile {
        IniFile::new(text.as_bytes().to_vec())
    }

    /// The value upstream's table writes for `key` of `section`.
    fn table_value(settings: &Settings, section: &str, key: &str) -> String {
        let setting = settings
            .table()
            .into_iter()
            .find(|setting| setting.section == section && setting.key == key)
            .expect("a setting of the table");
        String::from_utf8(setting.value).expect("text")
    }

    #[test]
    fn the_table_is_upstreams_order_and_defaults() {
        let table = Settings::default().table();
        assert_eq!(table.len(), table::DEFINITIONS.len());
        let first_keys: Vec<(&str, &str)> = table
            .iter()
            .take(2)
            .map(|setting| (setting.section.as_str(), setting.key.as_str()))
            .collect();
        assert_eq!(
            first_keys,
            [("gameplay", "fire_hold_lockout"), ("gameplay", "style")]
        );
        let defaults = Settings::default();
        for (section, key, value) in [
            ("gameplay", "style", "amiga"),
            ("replays", "frame_overlay", "true"),
            ("replays", "speed", "normal"),
            ("general", "language", "en"),
            ("general", "currency", "pound"),
            ("general", "world", "9697"),
            ("general", "file_sort", "name"),
            ("tweaks", "cpu_cards", "false"),
            ("tweaks", "five_substitutes", "true"),
            ("options", "game_length", "3"),
            ("options", "pitch_type", "seasonal"),
            ("options", "auto_replays", "true"),
            ("options", "all_teams_equal", "false"),
        ] {
            assert_eq!(
                table_value(&defaults, section, key),
                value,
                "{section}.{key}"
            );
        }
    }

    #[test]
    fn a_fresh_file_reads_back_as_its_settings() {
        let fresh = Settings::default().fresh_file();
        let text = String::from_utf8(fresh.as_bytes().to_vec()).expect("text");
        assert!(text.starts_with(FRESH_FILE_HEADER));
        let sections: Vec<&str> = text.lines().filter(|line| line.starts_with('[')).collect();
        assert_eq!(
            sections,
            [
                "[gameplay]         ; deliberate departures from the original Amiga game",
                "[replays]",
                "[testing]",
                "[general]",
                "[mods]",
                "[tweaks]",
                "[options]"
            ]
        );
        assert_eq!(fresh.read().first_error, None);
        let read = Settings::read(&fresh);
        assert_eq!(read.table(), Settings::default().table());
        // Read back, the options are set: the table wrote them.
        assert_eq!(read.options.game_length, Some(GameLength::Three));
        assert_eq!(Settings::default().save(Some(&fresh), None), None);
    }

    /// Upstream's self-tests (`Settings_GameOptionsSelfTest`, `Settings_LauncherSelfTest`)
    /// for the game's sections, and every other value a setting takes: written alone in a
    /// file, read and saved, it comes back as written.
    #[test]
    fn every_value_comes_back() {
        let mut cases: Vec<(&str, &str, String)> = Vec::new();
        for style in GameplayStyle::ALL {
            cases.push(("gameplay", "style", style.spelling().to_owned()));
        }
        for tactics in CpuTactics::ALL {
            cases.push(("gameplay", "cpu_tactics", tactics.spelling().to_owned()));
        }
        for speed in ReplaySpeed::ALL {
            cases.push(("replays", "speed", speed.spelling().to_owned()));
        }
        for length in GameLength::ALL {
            cases.push(("options", "game_length", length.spelling().to_owned()));
        }
        for pitch in PitchType::ALL {
            cases.push(("options", "pitch_type", pitch.spelling().to_owned()));
        }
        for world in ["9697", "2627"] {
            cases.push(("general", "world", world.to_owned()));
        }
        for language in ["en", "de", "fr", "es", "it", "nl"] {
            cases.push(("general", "language", language.to_owned()));
        }
        for definition in &table::DEFINITIONS {
            if Settings::default().table().iter().any(|setting| {
                setting.key == definition.key
                    && (setting.value == b"true" || setting.value == b"false")
            }) {
                for value in ["true", "false"] {
                    cases.push((definition.section, definition.key, value.to_owned()));
                }
            }
        }
        for (section, key, value) in cases {
            let written = file(&format!("[{section}]\n{key}={value}\n"));
            let settings = Settings::read(&written);
            assert_eq!(
                table_value(&settings, section, key),
                value,
                "{section}.{key}"
            );
            let saved = settings.save(None, None).expect("a fresh file");
            assert_eq!(
                table_value(&Settings::read(&saved), section, key),
                value,
                "{section}.{key}"
            );
        }
    }

    #[test]
    fn values_the_game_does_not_take_are_ignored() {
        let settings = Settings::read(&file(
            "[gameplay]\nstyle=pc\nfire_hold_lockout=maybe\n[general]\nlanguage=\nworld=\nfile_sort=size\n[options]\ngame_length=4\n",
        ));
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn the_game_reads_sections_and_keys_in_any_case_but_the_tweaks() {
        let settings = Settings::read(&file(
            "[GamePlay]\nSTYLE=Dos\n[TWEAKS]\ncpu_cards=yes\nMore_Fouls=on\n",
        ));
        assert_eq!(settings.gameplay.style, GameplayStyle::Dos);
        assert!(settings.tweaks.contains(Tweaks::CPU_CARDS));
        // Faithful to the original: `Tweaks_Bit` compares the key exactly.
        assert!(!settings.tweaks.contains(Tweaks::MORE_FOULS));
    }

    #[test]
    fn the_last_value_taken_counts() {
        let settings = Settings::read(&file(
            "[gameplay]\nstyle=dos\nstyle=xbox\nstyle=bogus\n[tweaks]\nthree_points_for_a_win=off\n",
        ));
        assert_eq!(settings.gameplay.style, GameplayStyle::Xbox);
        assert!(!settings.tweaks.contains(Tweaks::THREE_POINTS));
        assert!(settings.tweaks.contains(Tweaks::FIVE_SUBSTITUTES));
    }

    #[test]
    fn texts_are_cut_to_the_games_buffers() {
        let long = "x".repeat(2 * LINE_BYTES);
        let settings = Settings::read(&file(&format!(
            "[general]\nlanguage={long}\nworld=20262027\nfile_sort=DATE\n[mods]\nenabled={long}\n"
        )));
        assert_eq!(settings.general.language.len(), LANGUAGE_BYTES);
        assert_eq!(settings.general.world, b"2026202");
        // The spelling is kept.
        assert_eq!(settings.general.file_sort, b"DATE");
        // The line's 199 bytes come first: `enabled=` leaves 191 of the 255 the buffer keeps.
        assert_eq!(settings.mods.enabled.len(), LINE_BYTES - "enabled=".len());
    }

    #[test]
    fn any_word_sets_the_tactics_and_the_speed() {
        let settings = Settings::read(&file(
            "[gameplay]\ncpu_tactics=expert\ncpu_tactics=whatever\n[replays]\nspeed=half\n",
        ));
        assert_eq!(settings.gameplay.cpu_tactics, CpuTactics::Normal);
        assert_eq!(settings.replays.speed, ReplaySpeed::Half);
    }

    #[test]
    fn a_setting_the_player_left_keeps_the_files_value() {
        let fresh =
            String::from_utf8(Settings::default().fresh_file().as_bytes().to_vec()).expect("text");
        let file = IniFile::new(fresh.replace("style=amiga", "style=bogus").into_bytes());
        let read = Settings::read(&file);
        // `bogus` is ignored: the setting is `amiga`, and the player leaves it.
        assert_eq!(read.gameplay.style, GameplayStyle::Amiga);
        assert_eq!(read.save(Some(&file), Some(&read)), None);
        // Without the baseline the table's value replaces it.
        let saved = read.save(Some(&file), None).expect("rewritten");
        assert_eq!(Settings::read(&saved), read);
        assert!(!String::from_utf8_lossy(saved.as_bytes()).contains("bogus"));
        // Changed by the player, the setting is written.
        let mut changed = read.clone();
        changed.gameplay.style = GameplayStyle::Dos;
        let saved = changed.save(Some(&file), Some(&read)).expect("rewritten");
        assert_eq!(Settings::read(&saved).gameplay.style, GameplayStyle::Dos);
        // A key twice is rewritten whatever the baseline.
        let twice = IniFile::new([fresh.as_bytes(), b"[gameplay]\nstyle=dos\n"].concat());
        let read = Settings::read(&twice);
        assert_eq!(read.gameplay.style, GameplayStyle::Dos);
        assert!(read.save(Some(&twice), Some(&read)).is_some());
    }
}
