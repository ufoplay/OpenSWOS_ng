//! A save file as written: its top-level members in order, each kept as written.

use openswos_common::{Exporter, Importer};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

use crate::formatter::to_save_bytes;
use crate::layout::Layout;
use crate::text::from_save_bytes;
use crate::value::{SaveString, SaveValue};

/// The schema version this port reads and writes (upstream `SAVE_JSON_SCHEMA_VERSION`,
/// src/save_format_json.c). Upstream refuses a save of another.
pub const SCHEMA_VERSION: i64 = 3;

/// The key of the schema version.
pub const SCHEMA_VERSION_KEY: &str = "schema_version";

/// The key of the file kind.
pub const FILE_KIND_KEY: &str = "file_kind";

/// The schema version: [`SCHEMA_VERSION`], the only one read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SchemaVersion;

impl Serialize for SchemaVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(SCHEMA_VERSION)
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let version = i64::deserialize(deserializer)?;
        if version == SCHEMA_VERSION {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "schema version {version}: saves of version {SCHEMA_VERSION} are read"
            )))
        }
    }
}

/// The members of a career save written `"key": value`: the schema version, the file
/// kind, the world and the selected teams' count (`SaveJson_WriteCareer`).
const CAREER_SPACED_MEMBERS: usize = 4;

/// What a save holds (upstream `KIND_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileKind {
    /// A career (`.CAR`).
    Career,
    /// A preset competition (`.SEA`, `.PRE`).
    Preset,
    /// A DIY competition (`.DIY`).
    Diy,
    /// A match's highlights (`.HIL`).
    Highlights,
    /// A tactic (`.TAC`).
    Tactics,
}

impl FileKind {
    /// Every kind.
    pub const ALL: [Self; 5] = [
        Self::Career,
        Self::Preset,
        Self::Diy,
        Self::Highlights,
        Self::Tactics,
    ];

    /// Its name in the file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Career => "career",
            Self::Preset => "preset",
            Self::Diy => "diy",
            Self::Highlights => "highlights",
            Self::Tactics => "tactics",
        }
    }

    /// The kind named `name`, if any.
    #[must_use]
    pub fn from_name(name: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name().as_bytes() == name)
    }

    /// How many of its first top-level members upstream writes `"key": value`.
    #[must_use]
    pub const fn spaced_members(self) -> usize {
        match self {
            Self::Career => CAREER_SPACED_MEMBERS,
            Self::Preset | Self::Diy | Self::Highlights | Self::Tactics => 0,
        }
    }

    /// Upstream's layout of a save of this kind with `members` top-level members, each on
    /// a line of its own.
    #[must_use]
    pub fn layout(self, members: usize) -> Layout {
        Layout::upstream(members, self.spaced_members())
    }
}

/// Why bytes are not a save.
#[derive(Debug, Error)]
pub enum SaveJsonError {
    /// Not the saves' JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Not an object.
    #[error("a save is an object")]
    NotAnObject,
    /// No schema version, or not this port's.
    #[error("schema version {0:?}: saves of version {SCHEMA_VERSION} are read")]
    SchemaVersion(Option<SaveValue>),
    /// No file kind, or not a save's.
    #[error("file kind {0:?} is not a save's")]
    FileKind(Option<SaveValue>),
    /// Parsed, but not laid out as a save (an empty object).
    #[error("not laid out as a save")]
    Layout,
}

/// A save: its top-level members in the order written, and the whitespace around them.
/// It has the schema version this port reads and a known file kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveDocument {
    kind: FileKind,
    members: Vec<(String, SaveValue)>,
    layout: Layout,
}

impl SaveDocument {
    /// The save made of `members`.
    ///
    /// # Errors
    ///
    /// When the schema version is not [`SCHEMA_VERSION`] or the file kind is not a save's.
    pub fn new(members: Vec<(String, SaveValue)>, layout: Layout) -> Result<Self, SaveJsonError> {
        let member = |key: &str| {
            members
                .iter()
                .find_map(|(name, value)| (name == key).then_some(value))
        };
        match member(SCHEMA_VERSION_KEY) {
            Some(SaveValue::Integer(SCHEMA_VERSION)) => {}
            other => return Err(SaveJsonError::SchemaVersion(other.cloned())),
        }
        let kind = match member(FILE_KIND_KEY) {
            Some(SaveValue::String(SaveString(name))) => FileKind::from_name(name),
            Some(_) | None => None,
        }
        .ok_or_else(|| SaveJsonError::FileKind(member(FILE_KIND_KEY).cloned()))?;
        Ok(Self {
            kind,
            members,
            layout,
        })
    }

    /// What the save holds.
    #[must_use]
    pub fn kind(&self) -> FileKind {
        self.kind
    }

    /// Its top-level members, in order.
    #[must_use]
    pub fn members(&self) -> &[(String, SaveValue)] {
        &self.members
    }

    /// The value of the first top-level member named `key`.
    #[must_use]
    pub fn member(&self, key: &str) -> Option<&SaveValue> {
        self.members
            .iter()
            .find_map(|(name, value)| (name == key).then_some(value))
    }

    /// The whitespace around the top-level members.
    #[must_use]
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// Its members.
    #[must_use]
    pub fn into_members(self) -> Vec<(String, SaveValue)> {
        self.members
    }
}

/// The saves' JSON, read and written member by member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveJsonFormat;

impl Importer for SaveJsonFormat {
    type Model = SaveDocument;
    type Error = SaveJsonError;

    fn import(bytes: &[u8]) -> Result<SaveDocument, SaveJsonError> {
        match from_save_bytes::<SaveValue>(bytes)? {
            SaveValue::Object(members) => {
                SaveDocument::new(members, Layout::of(bytes).ok_or(SaveJsonError::Layout)?)
            }
            SaveValue::Null
            | SaveValue::Bool(_)
            | SaveValue::Integer(_)
            | SaveValue::String(_)
            | SaveValue::Array(_) => Err(SaveJsonError::NotAnObject),
        }
    }
}

impl Exporter for SaveJsonFormat {
    type Model = SaveDocument;
    type Error = serde_json::Error;

    fn export(model: &SaveDocument) -> Result<Vec<u8>, serde_json::Error> {
        to_save_bytes(&SaveValue::Object(model.members.clone()), &model.layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAREER: &[u8] = b"{\n  \"schema_version\": 3,\n  \"file_kind\": \"career\",\n  \"world\": \"9697\",\n  \"num_selected_teams\": 2,\n  \"season_informations\":[{\"club_no\":5,\"club_name\":\"M\xdcNCHEN\"}],\n  \"primary_country_code\":20\n}\n";

    const DIY: &[u8] = b"{\n  \"schema_version\":3,\n  \"file_kind\":\"diy\",\n  \"world\":\"9697\",\n  \"num_diy_teams\":16,\n  \"comp_slot\":{\"type\":1}\n}\n";

    #[test]
    fn a_career_round_trips() {
        let document = SaveJsonFormat::import(CAREER).expect("a save");
        assert_eq!(document.kind(), FileKind::Career);
        assert_eq!(
            document.member("world"),
            Some(&SaveValue::String(SaveString::from("9697")))
        );
        assert_eq!(document.members().len(), 6);
        assert_eq!(
            SaveJsonFormat::export(&document).ok(),
            Some(CAREER.to_vec())
        );
    }

    #[test]
    fn a_diy_round_trips_without_spaces() {
        let document = SaveJsonFormat::import(DIY).expect("a save");
        assert_eq!(document.kind(), FileKind::Diy);
        assert_eq!(SaveJsonFormat::export(&document).ok(), Some(DIY.to_vec()));
    }

    #[test]
    fn other_versions_and_kinds_do_not_read() {
        for (text, expected) in [
            (
                &b"{\"schema_version\":2,\"file_kind\":\"diy\"}"[..],
                "schema version",
            ),
            (b"{\"file_kind\":\"diy\"}", "schema version"),
            (
                b"{\"schema_version\":3,\"file_kind\":\"save\"}",
                "file kind",
            ),
            (b"{\"schema_version\":3}", "file kind"),
            (b"{\"schema_version\":3,\"file_kind\":1}", "file kind"),
            (b"[]", "a save is an object"),
        ] {
            let error = SaveJsonFormat::import(text).expect_err("not a save");
            assert!(error.to_string().starts_with(expected), "{error}");
        }
    }

    #[test]
    fn a_document_is_written_in_its_kinds_layout() {
        let read = SaveJsonFormat::import(CAREER).expect("a save");
        let members = read.clone().into_members();
        let layout = FileKind::Career.layout(members.len());
        assert_eq!(&layout, read.layout());
        let document = SaveDocument::new(members, layout).expect("a save");
        assert_eq!(
            SaveJsonFormat::export(&document).ok(),
            Some(CAREER.to_vec())
        );
        let diy = SaveJsonFormat::import(DIY).expect("a save");
        assert_eq!(&FileKind::Diy.layout(diy.members().len()), diy.layout());
    }

    #[test]
    fn kinds_have_their_names() {
        for kind in FileKind::ALL {
            assert_eq!(FileKind::from_name(kind.name().as_bytes()), Some(kind));
        }
        assert_eq!(FileKind::from_name(b"Career"), None);
    }
}
