//! The built-in tactics and their files: `tactics/<id>.json`, plus `<id>_original.json` and
//! `<id>_dos.json` for the formations whose data has more than one release.

use openswos_common::Tactics;
use thiserror::Error;

/// The extension of a tactics JSON file.
const JSON_EXTENSION: &str = ".json";

/// What follows a preset's id in the file of the Amiga original's data.
const ORIGINAL_SUFFIX: &str = "_original";

/// What follows a preset's id in the file of the DOS release's data.
const DOS_SUFFIX: &str = "_dos";

/// A built-in tactic: one of the twelve slots before the user's, in the engine's order
/// (upstream `s_presets`, src/tactics_load_json.c; `IDS`, scripts/export_tactics.py).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Preset {
    /// 4-4-2.
    FourFourTwo,
    /// 5-4-1.
    FiveFourOne,
    /// 4-5-1.
    FourFiveOne,
    /// 5-3-2.
    FiveThreeTwo,
    /// 3-5-2.
    ThreeFiveTwo,
    /// 4-3-3.
    FourThreeThree,
    /// 4-2-4.
    FourTwoFour,
    /// 3-4-3.
    ThreeFourThree,
    /// Sweep.
    Sweep,
    /// 5-2-3.
    FiveTwoThree,
    /// Attack.
    Attack,
    /// Defend.
    Defend,
}

/// A user's tactic slot, which has no built-in data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0:?} is a user's slot: it has no built-in tactic")]
pub struct NotAPreset(pub Tactics);

impl Preset {
    /// Every preset, in the engine's order.
    pub const ALL: [Self; 12] = [
        Self::FourFourTwo,
        Self::FiveFourOne,
        Self::FourFiveOne,
        Self::FiveThreeTwo,
        Self::ThreeFiveTwo,
        Self::FourThreeThree,
        Self::FourTwoFour,
        Self::ThreeFourThree,
        Self::Sweep,
        Self::FiveTwoThree,
        Self::Attack,
        Self::Defend,
    ];

    /// The id naming its file (`442`), which its JSON also states.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::FourFourTwo => "442",
            Self::FiveFourOne => "541",
            Self::FourFiveOne => "451",
            Self::FiveThreeTwo => "532",
            Self::ThreeFiveTwo => "352",
            Self::FourThreeThree => "433",
            Self::FourTwoFour => "424",
            Self::ThreeFourThree => "343",
            Self::Sweep => "Sweep",
            Self::FiveTwoThree => "523",
            Self::Attack => "Attack",
            Self::Defend => "Defend",
        }
    }

    /// The label its JSON states (`4-4-2`; upstream `LABEL`, scripts/export_tactics.py).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::FourFourTwo => "4-4-2",
            Self::FiveFourOne => "5-4-1",
            Self::FourFiveOne => "4-5-1",
            Self::FiveThreeTwo => "5-3-2",
            Self::ThreeFiveTwo => "3-5-2",
            Self::FourThreeThree => "4-3-3",
            Self::FourTwoFour => "4-2-4",
            Self::ThreeFourThree => "3-4-3",
            Self::Sweep => "Sweep",
            Self::FiveTwoThree => "5-2-3",
            Self::Attack => "Attack",
            Self::Defend => "Defend",
        }
    }
}

impl From<Preset> for Tactics {
    fn from(preset: Preset) -> Self {
        match preset {
            Preset::FourFourTwo => Self::FourFourTwo,
            Preset::FiveFourOne => Self::FiveFourOne,
            Preset::FourFiveOne => Self::FourFiveOne,
            Preset::FiveThreeTwo => Self::FiveThreeTwo,
            Preset::ThreeFiveTwo => Self::ThreeFiveTwo,
            Preset::FourThreeThree => Self::FourThreeThree,
            Preset::FourTwoFour => Self::FourTwoFour,
            Preset::ThreeFourThree => Self::ThreeFourThree,
            Preset::Sweep => Self::Sweep,
            Preset::FiveTwoThree => Self::FiveTwoThree,
            Preset::Attack => Self::Attack,
            Preset::Defend => Self::Defend,
        }
    }
}

impl TryFrom<Tactics> for Preset {
    type Error = NotAPreset;

    fn try_from(slot: Tactics) -> Result<Self, Self::Error> {
        match slot {
            Tactics::FourFourTwo => Ok(Self::FourFourTwo),
            Tactics::FiveFourOne => Ok(Self::FiveFourOne),
            Tactics::FourFiveOne => Ok(Self::FourFiveOne),
            Tactics::FiveThreeTwo => Ok(Self::FiveThreeTwo),
            Tactics::ThreeFiveTwo => Ok(Self::ThreeFiveTwo),
            Tactics::FourThreeThree => Ok(Self::FourThreeThree),
            Tactics::FourTwoFour => Ok(Self::FourTwoFour),
            Tactics::ThreeFourThree => Ok(Self::ThreeFourThree),
            Tactics::Sweep => Ok(Self::Sweep),
            Tactics::FiveTwoThree => Ok(Self::FiveTwoThree),
            Tactics::Attack => Ok(Self::Attack),
            Tactics::Defend => Ok(Self::Defend),
            Tactics::UserA
            | Tactics::UserB
            | Tactics::UserC
            | Tactics::UserD
            | Tactics::UserE
            | Tactics::UserF => Err(NotAPreset(slot)),
        }
    }
}

/// Which release of a preset's data a file holds (upstream src/tactics_data.c).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Release {
    /// `<id>.json`: what OpenSWOS plays. The original's data, except 4-4-2, 4-3-3, 3-4-3
    /// and Sweep, whose central defenders no longer swap sides (upstream
    /// `SWOS_ORIGINAL_TACTICS`).
    Shipped,
    /// `<id>_original.json`: the Amiga original's data for those four.
    Original,
    /// `<id>_dos.json`: the DOS release's data for those four. It re-tuned 4-4-2, 4-3-3 and
    /// Sweep; its 3-4-3 is the Amiga original's (upstream `SWOS_DOS_TACTICS`).
    Dos,
}

impl Release {
    fn suffix(self) -> &'static str {
        match self {
            Self::Shipped => "",
            Self::Original => ORIGINAL_SUFFIX,
            Self::Dos => DOS_SUFFIX,
        }
    }
}

/// A tactics JSON file: a preset's data, in one release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PresetFile {
    /// The preset.
    pub preset: Preset,
    /// The release.
    pub release: Release,
}

impl PresetFile {
    /// The file's name (`442_dos.json`).
    #[must_use]
    pub fn file_name(self) -> String {
        format!(
            "{}{}{JSON_EXTENSION}",
            self.preset.id(),
            self.release.suffix()
        )
    }

    /// The preset file named `name`, if it names one.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let stem = name.strip_suffix(JSON_EXTENSION)?;
        Preset::ALL.into_iter().find_map(|preset| {
            let release = match stem.strip_prefix(preset.id())? {
                "" => Release::Shipped,
                ORIGINAL_SUFFIX => Release::Original,
                DOS_SUFFIX => Release::Dos,
                _ => return None,
            };
            Some(Self { preset, release })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_the_first_twelve_slots_in_order() {
        let slots: Vec<Tactics> = Preset::ALL.into_iter().map(Tactics::from).collect();
        let first = Tactics::ALL.get(..Preset::ALL.len()).expect("18 slots");
        assert_eq!(slots, first.to_vec());
        for preset in Preset::ALL {
            assert_eq!(Preset::try_from(Tactics::from(preset)), Ok(preset));
        }
        assert_eq!(
            Preset::try_from(Tactics::UserA),
            Err(NotAPreset(Tactics::UserA))
        );
    }

    #[test]
    fn file_names_round_trip() {
        for preset in Preset::ALL {
            for release in [Release::Shipped, Release::Original, Release::Dos] {
                let file = PresetFile { preset, release };
                assert_eq!(PresetFile::parse(&file.file_name()), Some(file));
            }
        }
        assert_eq!(
            PresetFile {
                preset: Preset::Sweep,
                release: Release::Dos
            }
            .file_name(),
            "Sweep_dos.json"
        );
    }

    #[test]
    fn other_names_are_no_preset_file() {
        for name in [
            "442",
            "442.JSON",
            "442_amiga.json",
            "999.json",
            "career_params.json",
        ] {
            assert_eq!(PresetFile::parse(name), None, "{name}");
        }
    }
}
