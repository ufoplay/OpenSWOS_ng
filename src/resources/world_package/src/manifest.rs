//! `mod.json`: the world as an OpenSWOS mod, for OpenSWOS Setup to install (upstream
//! scripts/make_world_package.sh, docs/MODS.md §14, §17). Written by Python's
//! `json.dumps(indent=1)` and a newline. Older packages (the pinned World 2026-27 among
//! them) have none.

use openswos_common::json::{Indent, NonAscii, TrailingNewline, to_text};
use openswos_common::{Exporter, Importer};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A world's mod manifest, as upstream's package writes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModManifest {
    /// The mod's identity, the world's for ever.
    pub uuid: Uuid,
    /// Its slug (`world2627`).
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its version: the world's fingerprint when made.
    pub version: String,
    /// Its authors.
    pub author: String,
    /// What it is.
    pub about: String,
    /// The builds it is for (`sdl`, `amiga`).
    pub targets: Vec<String>,
    /// What it changes (`data`).
    pub changes: String,
    /// Whether it is on by default.
    pub default: bool,
}

/// A `mod.json` and whether its text ends with a newline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModManifestDocument {
    /// The manifest.
    pub manifest: ModManifest,
    /// Whether its text ends with a newline (upstream's does).
    pub trailing_newline: TrailingNewline,
}

/// The `mod.json` format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModManifestFormat;

impl Importer for ModManifestFormat {
    type Model = ModManifestDocument;
    type Error = serde_json::Error;

    fn import(bytes: &[u8]) -> Result<ModManifestDocument, serde_json::Error> {
        Ok(ModManifestDocument {
            manifest: serde_json::from_slice(bytes)?,
            trailing_newline: TrailingNewline::of(bytes),
        })
    }
}

impl Exporter for ModManifestFormat {
    type Model = ModManifestDocument;
    type Error = serde_json::Error;

    fn export(model: &ModManifestDocument) -> Result<Vec<u8>, serde_json::Error> {
        Ok(to_text(
            &model.manifest,
            Indent::One,
            NonAscii::Escaped,
            model.trailing_newline,
        )?
        .into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// World 2026-27's `mod.json`, as upstream scripts/make_world_package.sh writes it
    /// (computed with it).
    const WORLD_2627_MANIFEST: &str = "{\n \"uuid\": \"15ea9117-6aa0-4968-a293-6cf33092ff33\",\n \"id\": \"world2627\",\n \"name\": \"World 2026-27\",\n \"version\": \"efe1464b6d1d62b8\",\n \"author\": \"the SWOS 96/97 Mod (Davide Lorigliola and its contributors)\",\n \"about\": \"The football world of 2026-27: its countries, leagues, cups and squads.\",\n \"targets\": [\n  \"sdl\",\n  \"amiga\"\n ],\n \"changes\": \"data\",\n \"default\": false\n}\n";

    #[test]
    fn upstreams_manifest_round_trips() {
        let document =
            ModManifestFormat::import(WORLD_2627_MANIFEST.as_bytes()).expect("a manifest");
        assert_eq!(document.manifest.id, "world2627");
        assert_eq!(document.manifest.targets, vec!["sdl", "amiga"]);
        assert!(!document.manifest.default);
        assert_eq!(document.trailing_newline, TrailingNewline::Present);
        assert_eq!(
            ModManifestFormat::export(&document).ok(),
            Some(WORLD_2627_MANIFEST.as_bytes().to_vec())
        );
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let text =
            WORLD_2627_MANIFEST.replace("\"default\": false", "\"default\": false, \"x\": 1");
        assert!(ModManifestFormat::import(text.as_bytes()).is_err());
    }
}
