//! The migration ledger, `upstream.toml`: the pinned upstream commit, and the state of
//! every upstream file the port covers (docs/PLAN.md §8).

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value};

use crate::error::{Result, UpstreamError};

/// File name of the ledger, at the workspace root.
pub const LEDGER_FILE_NAME: &str = "upstream.toml";

/// Number of hexadecimal digits of a full git commit hash (SHA-1).
pub const COMMIT_HASH_LEN: usize = 40;

/// The comment written at the top of the ledger.
const HEADER: &str = "\
# The migration ledger (docs/PLAN.md §8): the upstream OpenSWOS commit this port tracks,
# and the state of every upstream file it covers, one line per file.
#
# Written by the `upstream` tool (`just upstream_sync`, `just upstream_mark ...`): edit it
# through the tool, which keeps it sorted and formatted.
#
# area:   untriaged | core | resource | world_data | test_data | view | out_of_scope
# status: todo | partial | done | not_ported
";

/// The whole ledger.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    /// Where upstream lives and which commit is pinned.
    pub upstream: Upstream,
    /// The upstream submodules whose files the ledger covers, at the commits upstream pins.
    #[serde(default, rename = "submodule")]
    pub submodules: Vec<Submodule>,
    /// Every covered upstream file, by its path in the upstream repository.
    #[serde(default)]
    pub files: BTreeMap<String, FileEntry>,
}

/// The `[upstream]` section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upstream {
    /// The upstream git repository.
    pub repository: String,
    /// The commit the port is migrated against.
    pub pinned_commit: String,
    /// Glob patterns of the upstream paths the ledger covers (`*` stays within one
    /// directory, `**` crosses directories).
    pub tracked: Vec<String>,
    /// The directories `fetch` checks out (a sparse checkout).
    pub checkout: Vec<String>,
}

/// A `[[submodule]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Submodule {
    /// The submodule's path in the upstream repository.
    pub path: String,
    /// The submodule commit upstream pins at the pinned commit.
    pub commit: String,
}

/// The state of one upstream file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    /// Where the file's content goes in the port.
    pub area: Area,
    /// How far the migration of the file is.
    pub status: Status,
    /// The upstream commit the file was last migrated against.
    #[serde(default)]
    pub migrated_at: Option<String>,
    /// The files of this repository that hold the file's port.
    #[serde(default)]
    pub rust: Vec<String>,
    /// For a big file migrated in steps: the functions already migrated.
    #[serde(default)]
    pub functions_done: Vec<String>,
    /// Free-form notes.
    #[serde(default)]
    pub notes: Option<String>,
}

impl FileEntry {
    /// The entry of a file the ledger has just learned about.
    #[must_use]
    pub fn untriaged() -> Self {
        Self {
            area: Area::Untriaged,
            status: Status::Todo,
            migrated_at: None,
            rust: Vec::new(),
            functions_done: Vec::new(),
            notes: None,
        }
    }

    /// The entry as one inline table, the way the ledger writes it.
    fn to_inline_table(&self) -> InlineTable {
        let Self {
            area,
            status,
            migrated_at,
            rust,
            functions_done,
            notes,
        } = self;
        let mut table = InlineTable::new();
        table.insert("area", Value::from(area.as_str()));
        table.insert("status", Value::from(status.as_str()));
        if let Some(commit) = migrated_at {
            table.insert("migrated_at", Value::from(commit.as_str()));
        }
        if !rust.is_empty() {
            table.insert("rust", Value::Array(rust.iter().collect()));
        }
        if !functions_done.is_empty() {
            table.insert(
                "functions_done",
                Value::Array(functions_done.iter().collect()),
            );
        }
        if let Some(text) = notes {
            table.insert("notes", Value::from(text.as_str()));
        }
        table.fmt();
        table
    }
}

/// Where an upstream file's content goes in the port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum Area {
    /// Not classified yet.
    Untriaged,
    /// Game logic: `src/core`.
    Core,
    /// A file format: `src/resources`, `src/codecs`.
    Resource,
    /// Content that goes into world packages rather than code.
    WorldData,
    /// Upstream test data the port's tests reuse.
    TestData,
    /// Presentation: ported after the core.
    View,
    /// Not ported (Amiga, mobile, netplay, mods, platform layers...).
    OutOfScope,
}

impl Area {
    /// Every area, in display order.
    pub const ALL: &[Self] = &[
        Self::Untriaged,
        Self::Core,
        Self::Resource,
        Self::WorldData,
        Self::TestData,
        Self::View,
        Self::OutOfScope,
    ];

    /// The name the ledger uses.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Untriaged => "untriaged",
            Self::Core => "core",
            Self::Resource => "resource",
            Self::WorldData => "world_data",
            Self::TestData => "test_data",
            Self::View => "view",
            Self::OutOfScope => "out_of_scope",
        }
    }

    /// Whether the area belongs to the current migration (the core milestones).
    #[must_use]
    pub fn in_scope(self) -> bool {
        match self {
            Self::Core | Self::Resource | Self::WorldData | Self::TestData => true,
            Self::Untriaged | Self::View | Self::OutOfScope => false,
        }
    }
}

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How far the migration of an upstream file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum Status {
    /// Not started.
    Todo,
    /// Some of it is migrated (see `functions_done`).
    Partial,
    /// Fully migrated, at `migrated_at`.
    Done,
    /// Deliberately not ported.
    NotPorted,
}

impl Status {
    /// Every status, in display order.
    pub const ALL: &[Self] = &[Self::Todo, Self::Partial, Self::Done, Self::NotPorted];

    /// The name the ledger uses.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Partial => "partial",
            Self::Done => "done",
            Self::NotPorted => "not_ported",
        }
    }

    /// Whether a migration is recorded against a commit (`migrated_at`).
    #[must_use]
    pub fn has_migration(self) -> bool {
        match self {
            Self::Partial | Self::Done => true,
            Self::Todo | Self::NotPorted => false,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether `value` is a full, lower-case commit hash.
#[must_use]
pub fn is_commit_hash(value: &str) -> bool {
    value.len() == COMMIT_HASH_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Ledger {
    /// Parses a ledger from its text. `path` only names the file in errors.
    ///
    /// # Errors
    ///
    /// Fails when the text is not valid TOML or does not match the schema.
    pub fn parse(text: &str, path: &Path) -> Result<Self> {
        toml::from_str(text).map_err(|source| UpstreamError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })
    }

    /// Reads the ledger file.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|source| UpstreamError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text, path)
    }

    /// Writes the ledger file.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.to_toml_string()).map_err(|source| UpstreamError::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    /// The ledger as the tool writes it: sorted, one inline table per file.
    #[must_use]
    pub fn to_toml_string(&self) -> String {
        let Self {
            upstream,
            submodules,
            files,
        } = self;
        let mut doc = DocumentMut::new();

        let mut upstream_table = upstream.to_table();
        upstream_table
            .decor_mut()
            .set_prefix(HEADER.to_owned() + "\n");
        doc.insert("upstream", Item::Table(upstream_table));

        if !submodules.is_empty() {
            let mut tables = ArrayOfTables::new();
            for submodule in submodules {
                tables.push(submodule.to_table());
            }
            doc.insert("submodule", Item::ArrayOfTables(tables));
        }

        let mut files_table = Table::new();
        for (path, entry) in files {
            files_table.insert(
                path,
                Item::Value(Value::InlineTable(entry.to_inline_table())),
            );
        }
        doc.insert("files", Item::Table(files_table));

        doc.to_string()
    }
}

impl Upstream {
    fn to_table(&self) -> Table {
        let Self {
            repository,
            pinned_commit,
            tracked,
            checkout,
        } = self;
        let mut table = Table::new();
        table.insert("repository", Item::Value(Value::from(repository.as_str())));
        table.insert(
            "pinned_commit",
            Item::Value(Value::from(pinned_commit.as_str())),
        );
        table.insert(
            "tracked",
            Item::Value(Value::Array(multiline_array(tracked))),
        );
        table.insert(
            "checkout",
            Item::Value(Value::Array(multiline_array(checkout))),
        );
        table
    }
}

impl Submodule {
    fn to_table(&self) -> Table {
        let Self { path, commit } = self;
        let mut table = Table::new();
        table.insert("path", Item::Value(Value::from(path.as_str())));
        table.insert("commit", Item::Value(Value::from(commit.as_str())));
        table
    }
}

/// A string array written one element per line.
fn multiline_array(items: &[String]) -> Array {
    let mut array: Array = items.iter().collect();
    for value in array.iter_mut() {
        value.decor_mut().set_prefix("\n    ");
    }
    array.set_trailing_comma(true);
    array.set_trailing("\n");
    array
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "ec2ed2be5c26e510f7c7b8d7af2710e5b909ca8b";
    const SUBMODULE_PIN: &str = "87b65278a15007223ec4f387f1027eb4d5cd7625";

    fn sample() -> Ledger {
        let mut files = BTreeMap::new();
        files.insert(
            "src/netplay.c".to_owned(),
            FileEntry {
                area: Area::OutOfScope,
                status: Status::NotPorted,
                ..FileEntry::untriaged()
            },
        );
        files.insert(
            "src/random.c".to_owned(),
            FileEntry {
                area: Area::Core,
                status: Status::Done,
                migrated_at: Some(PIN.to_owned()),
                rust: vec!["src/core/src/rng.rs".to_owned()],
                functions_done: vec!["Rand".to_owned(), "Rand2".to_owned()],
                notes: Some("the \"two\" generators".to_owned()),
            },
        );
        Ledger {
            upstream: Upstream {
                repository: "https://github.com/benbaker76/OpenSWOS".to_owned(),
                pinned_commit: PIN.to_owned(),
                tracked: vec!["src/**".to_owned(), "include/**".to_owned()],
                checkout: vec!["src".to_owned(), "include".to_owned()],
            },
            submodules: vec![Submodule {
                path: "third_party/swos-competition-engine".to_owned(),
                commit: SUBMODULE_PIN.to_owned(),
            }],
            files,
        }
    }

    #[test]
    fn written_ledger_parses_back_to_the_same_ledger() {
        let ledger = sample();
        let parsed = Ledger::parse(&ledger.to_toml_string(), Path::new("upstream.toml"))
            .expect("the written ledger parses");
        assert_eq!(parsed, ledger);
    }

    #[test]
    fn ledger_is_written_one_line_per_file_after_the_header() {
        let text = sample().to_toml_string();
        let expected_tail = format!(
            "[upstream]
repository = \"https://github.com/benbaker76/OpenSWOS\"
pinned_commit = \"{PIN}\"
tracked = [
    \"src/**\",
    \"include/**\",
]
checkout = [
    \"src\",
    \"include\",
]

[[submodule]]
path = \"third_party/swos-competition-engine\"
commit = \"{SUBMODULE_PIN}\"

[files]
\"src/netplay.c\" = {{ area = \"out_of_scope\", status = \"not_ported\" }}
\"src/random.c\" = {{ area = \"core\", status = \"done\", migrated_at = \"{PIN}\", rust = [\"src/core/src/rng.rs\"], functions_done = [\"Rand\", \"Rand2\"], notes = 'the \"two\" generators' }}
"
        );
        assert!(text.starts_with(HEADER), "the header comes first:\n{text}");
        assert!(text.ends_with(&expected_tail), "unexpected layout:\n{text}");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let text = sample()
            .to_toml_string()
            .replace("status = \"done\"", "statuss = \"done\"");
        assert!(Ledger::parse(&text, Path::new("upstream.toml")).is_err());
    }

    #[test]
    fn unknown_area_is_rejected() {
        let text = sample().to_toml_string().replace("\"core\"", "\"engine\"");
        assert!(Ledger::parse(&text, Path::new("upstream.toml")).is_err());
    }

    #[test]
    fn missing_optional_fields_take_their_defaults() {
        let text = format!(
            "[upstream]\nrepository = \"r\"\npinned_commit = \"{PIN}\"\ntracked = []\ncheckout = []\n\n\
             [files]\n\"a.c\" = {{ area = \"view\", status = \"todo\" }}\n"
        );
        let ledger = Ledger::parse(&text, Path::new("upstream.toml")).expect("valid ledger");
        assert!(ledger.submodules.is_empty());
        assert_eq!(
            ledger.files.get("a.c"),
            Some(&FileEntry {
                area: Area::View,
                ..FileEntry::untriaged()
            })
        );
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(LEDGER_FILE_NAME);
        sample().save(&path).expect("saved");
        assert_eq!(Ledger::load(&path).expect("loaded"), sample());
    }

    #[test]
    fn load_reports_a_missing_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let error = Ledger::load(&dir.path().join(LEDGER_FILE_NAME)).expect_err("no file");
        assert!(matches!(error, UpstreamError::Read { .. }));
    }

    #[test]
    fn area_names_round_trip_through_serde() {
        for area in Area::ALL {
            let text = format!("v = \"{area}\"");
            let parsed: BTreeMap<String, Area> = toml::from_str(&text).expect("area parses");
            assert_eq!(parsed.get("v"), Some(area));
        }
    }

    #[test]
    fn status_names_round_trip_through_serde() {
        for status in Status::ALL {
            let text = format!("v = \"{status}\"");
            let parsed: BTreeMap<String, Status> = toml::from_str(&text).expect("status parses");
            assert_eq!(parsed.get("v"), Some(status));
        }
    }

    #[test]
    fn only_the_migration_areas_are_in_scope() {
        let in_scope: Vec<Area> = Area::ALL.iter().copied().filter(|a| a.in_scope()).collect();
        assert_eq!(
            in_scope,
            vec![Area::Core, Area::Resource, Area::WorldData, Area::TestData]
        );
    }

    #[test]
    fn partial_and_done_record_a_migration() {
        let with: Vec<Status> = Status::ALL
            .iter()
            .copied()
            .filter(|s| s.has_migration())
            .collect();
        assert_eq!(with, vec![Status::Partial, Status::Done]);
    }

    #[test]
    fn commit_hashes_are_validated() {
        assert!(is_commit_hash(PIN));
        assert!(!is_commit_hash("ec2ed2b"));
        assert!(!is_commit_hash(&PIN.to_uppercase()));
        assert!(!is_commit_hash(&PIN.replace('e', "g")));
    }
}
