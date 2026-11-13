//! The built-in tactics: the twelve presets, and the other releases of those that have one
//! (upstream `bin/data_json/tactics/`).

use std::collections::BTreeMap;

use openswos_tac_resource::TeamTactics;
use openswos_tactics_json_resource::{Preset, PresetFile, Release};

use crate::validation::{Location, Problem, Problems};

/// The folder of the tactics files (upstream `bin/data_json/tactics/`).
pub const TACTICS_FOLDER: &str = "tactics";

/// The built-in tactics: every preset as OpenSWOS plays it, and the other releases given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetTactics(BTreeMap<PresetFile, TeamTactics>);

impl PresetTactics {
    /// `preset` as OpenSWOS plays it (`<id>.json`).
    #[must_use]
    pub fn shipped(&self, preset: Preset) -> &TeamTactics {
        self.0
            .get(&PresetFile {
                preset,
                release: Release::Shipped,
            })
            .expect("validation requires every preset's shipped release")
    }

    /// The tactics of `file`, if given.
    #[must_use]
    pub fn get(&self, file: PresetFile) -> Option<&TeamTactics> {
        self.0.get(&file)
    }

    /// The tactics given, which must hold every preset's shipped release (each missing one
    /// reported).
    pub(crate) fn build(
        files: BTreeMap<PresetFile, TeamTactics>,
        problems: &mut Problems,
    ) -> Option<Self> {
        let mut complete = true;
        for preset in Preset::ALL {
            let file = PresetFile {
                preset,
                release: Release::Shipped,
            };
            if !files.contains_key(&file) {
                problems.push(
                    Location::file(&format!("{TACTICS_FOLDER}/{}", file.file_name())),
                    Problem::MissingFile,
                );
                complete = false;
            }
        }
        complete.then_some(Self(files))
    }
}
