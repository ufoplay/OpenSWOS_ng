//! Fixture helpers: listing a format's fixtures, comparing a decoded model with its JSON
//! sidecar, and checking that a file round-trips byte for byte.

use std::path::{Path, PathBuf};

use openswos_common::{Exporter, Importer};
use serde::Serialize;

use crate::assets_path;

/// Environment variable that rewrites the JSON sidecars from the current output instead
/// of comparing with them (any value but `0`). Review the diff before committing.
pub const BLESS_ENV: &str = "OPENSWOS_BLESS";

/// The value of [`BLESS_ENV`] that keeps comparing.
const COMPARE_VALUE: &str = "0";

/// What is appended to a fixture's file name to name its sidecar: `TEAM.008` has the
/// sidecar `TEAM.008.json`.
const SIDECAR_SUFFIX: &str = ".json";

/// The fixtures of `assets/<format_dir>` whose file names satisfy `matches`, sorted.
///
/// # Panics
///
/// When the folder cannot be read: the test names a format folder that does not exist.
pub fn fixtures(format_dir: &str, matches: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    list_fixtures(&assets_path().join(format_dir), matches)
}

/// The files of `dir` whose names satisfy `matches`, sorted.
///
/// # Panics
///
/// When the folder cannot be read.
pub fn list_fixtures(dir: &Path, matches: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot list the fixtures in {}: {error}", dir.display()));
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| matches(&entry.file_name().to_string_lossy()))
        .map(|entry| entry.path())
        .collect();
    files.sort();
    files
}

/// The JSON sidecar of a fixture: its file name with `.json` appended.
#[must_use]
pub fn sidecar_path(fixture: &Path) -> PathBuf {
    let mut name = fixture.file_name().unwrap_or_default().to_os_string();
    name.push(SIDECAR_SUFFIX);
    fixture.with_file_name(name)
}

/// What [`check_sidecar`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarMode {
    /// Compare the model with the sidecar.
    Compare,
    /// Write the sidecar from the model.
    Bless,
}

impl SidecarMode {
    /// The mode [`BLESS_ENV`] asks for.
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var_os(BLESS_ENV) {
            Some(value) if !value.is_empty() && value != COMPARE_VALUE => Self::Bless,
            Some(_) | None => Self::Compare,
        }
    }
}

/// Checks a fixture's decoded `model` against its JSON sidecar (or writes the sidecar,
/// with [`BLESS_ENV`] set).
///
/// # Panics
///
/// When they differ, or the sidecar is missing.
pub fn assert_sidecar<T: Serialize>(fixture: &Path, model: &T) {
    if let Err(message) = check_sidecar(fixture, model, SidecarMode::from_env()) {
        panic!("{message}");
    }
}

/// [`assert_sidecar`] with an explicit mode, returning the failure.
///
/// # Errors
///
/// A message when the model differs from the sidecar, or the sidecar cannot be read or
/// written.
pub fn check_sidecar<T: Serialize>(
    fixture: &Path,
    model: &T,
    mode: SidecarMode,
) -> Result<(), String> {
    let sidecar = sidecar_path(fixture);
    let actual = serde_json::to_value(model)
        .map_err(|error| format!("cannot serialize the model: {error}"))?;
    let actual_text = pretty(&actual);
    match mode {
        SidecarMode::Bless => std::fs::write(&sidecar, actual_text)
            .map_err(|error| format!("cannot write {}: {error}", sidecar.display())),
        SidecarMode::Compare => {
            let expected_text = std::fs::read_to_string(&sidecar).map_err(|error| {
                format!(
                    "cannot read {}: {error} (run with {BLESS_ENV}=1 to write it)",
                    sidecar.display()
                )
            })?;
            let expected: serde_json::Value = serde_json::from_str(&expected_text)
                .map_err(|error| format!("{} is not valid JSON: {error}", sidecar.display()))?;
            if expected == actual {
                Ok(())
            } else {
                Err(format!(
                    "{} decodes differently from {}\n--- expected\n{}--- actual\n{actual_text}",
                    fixture.display(),
                    sidecar.display(),
                    pretty(&expected)
                ))
            }
        }
    }
}

fn pretty(value: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

/// Imports `bytes` with the format `F`, exports the model back, and checks the result is
/// `bytes` again. Returns the model.
///
/// # Panics
///
/// When the import or the export fails, or the bytes differ.
pub fn assert_round_trip<F>(bytes: &[u8]) -> <F as Importer>::Model
where
    F: Importer + Exporter<Model = <F as Importer>::Model>,
{
    round_trip::<F>(bytes).unwrap_or_else(|message| panic!("{message}"))
}

/// [`assert_round_trip`], returning the failure.
///
/// # Errors
///
/// A message when the import or the export fails, or the bytes differ.
pub fn round_trip<F>(bytes: &[u8]) -> Result<<F as Importer>::Model, String>
where
    F: Importer + Exporter<Model = <F as Importer>::Model>,
{
    let model = F::import(bytes).map_err(|error| format!("import failed: {error}"))?;
    let exported = F::export(&model).map_err(|error| format!("export failed: {error}"))?;
    match first_difference(bytes, &exported) {
        None => Ok(model),
        Some(offset) => Err(format!(
            "the export differs from the original at offset {offset} (original: {} bytes, export: {} bytes)",
            bytes.len(),
            exported.len()
        )),
    }
}

/// The first offset where `a` and `b` differ, if they do.
fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .or_else(|| (a.len() != b.len()).then(|| a.len().min(b.len())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    /// A toy format: a count byte, then that many bytes.
    struct CountedBytes;

    #[derive(Debug)]
    struct ToyError;

    impl fmt::Display for ToyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bad toy file")
        }
    }

    impl std::error::Error for ToyError {}

    impl Importer for CountedBytes {
        type Model = Vec<u8>;
        type Error = ToyError;

        fn import(bytes: &[u8]) -> Result<Vec<u8>, ToyError> {
            match bytes.split_first() {
                Some((&count, rest)) if usize::from(count) == rest.len() => Ok(rest.to_vec()),
                Some(_) | None => Err(ToyError),
            }
        }
    }

    impl Exporter for CountedBytes {
        type Model = Vec<u8>;
        type Error = ToyError;

        fn export(model: &Vec<u8>) -> Result<Vec<u8>, ToyError> {
            let count = u8::try_from(model.len()).map_err(|_| ToyError)?;
            Ok([&[count], model.as_slice()].concat())
        }
    }

    /// The same format, but its exporter forgets the last byte.
    struct LossyBytes;

    impl Importer for LossyBytes {
        type Model = Vec<u8>;
        type Error = ToyError;

        fn import(bytes: &[u8]) -> Result<Vec<u8>, ToyError> {
            CountedBytes::import(bytes)
        }
    }

    impl Exporter for LossyBytes {
        type Model = Vec<u8>;
        type Error = ToyError;

        fn export(model: &Vec<u8>) -> Result<Vec<u8>, ToyError> {
            let mut bytes = CountedBytes::export(model)?;
            bytes.pop();
            Ok(bytes)
        }
    }

    #[test]
    fn a_faithful_format_round_trips() {
        assert_eq!(assert_round_trip::<CountedBytes>(&[2, 7, 9]), vec![7, 9]);
    }

    #[test]
    fn a_lossy_export_is_reported_with_its_offset() {
        let error = round_trip::<LossyBytes>(&[2, 7, 9]).expect_err("lossy");
        assert_eq!(
            error,
            "the export differs from the original at offset 2 (original: 3 bytes, export: 2 bytes)"
        );
    }

    #[test]
    fn a_failed_import_is_reported() {
        let error = round_trip::<CountedBytes>(&[5, 1]).expect_err("bad count");
        assert_eq!(error, "import failed: bad toy file");
    }

    #[test]
    fn differences_are_located() {
        assert_eq!(first_difference(&[1, 2, 3], &[1, 2, 3]), None);
        assert_eq!(first_difference(&[1, 2, 3], &[1, 5, 3]), Some(1));
        assert_eq!(first_difference(&[1, 2], &[1, 2, 3]), Some(2));
    }

    #[test]
    fn sidecars_append_json_to_the_file_name() {
        assert_eq!(
            sidecar_path(Path::new("assets/TEAM/TEAM.008")),
            Path::new("assets/TEAM/TEAM.008.json")
        );
    }

    #[test]
    fn sidecars_are_blessed_then_compared() {
        let dir = tempfile::tempdir().expect("temp dir");
        let fixture = dir.path().join("TEAM.008");
        let model = serde_json::json!({ "name": "RANGERS", "players": 16 });

        let missing =
            check_sidecar(&fixture, &model, SidecarMode::Compare).expect_err("no sidecar yet");
        assert!(missing.contains("OPENSWOS_BLESS=1"), "{missing}");

        check_sidecar(&fixture, &model, SidecarMode::Bless).expect("blessed");
        assert!(dir.path().join("TEAM.008.json").is_file());
        check_sidecar(&fixture, &model, SidecarMode::Compare).expect("equal");

        let changed = serde_json::json!({ "name": "CELTIC", "players": 16 });
        let error = check_sidecar(&fixture, &changed, SidecarMode::Compare).expect_err("differs");
        assert!(
            error.contains("decodes differently") && error.contains("CELTIC"),
            "{error}"
        );
    }

    #[test]
    fn sidecars_compare_values_not_formatting() {
        let dir = tempfile::tempdir().expect("temp dir");
        let fixture = dir.path().join("x.bin");
        std::fs::write(sidecar_path(&fixture), "{\"b\":2,   \"a\":1}").expect("sidecar");
        check_sidecar(
            &fixture,
            &serde_json::json!({ "a": 1, "b": 2 }),
            SidecarMode::Compare,
        )
        .expect("equal values");
    }

    #[test]
    fn fixtures_are_listed_sorted_and_filtered() {
        let dir = tempfile::tempdir().expect("temp dir");
        for name in ["TEAM.033", "TEAM.008", "TEAM.008.json", "README.md"] {
            std::fs::write(dir.path().join(name), b"").expect("file");
        }
        std::fs::create_dir(dir.path().join("TEAM.999")).expect("a folder, not a fixture");
        let listed = list_fixtures(dir.path(), |name| {
            name.starts_with("TEAM.") && !name.ends_with(".json")
        });
        assert_eq!(
            listed,
            vec![dir.path().join("TEAM.008"), dir.path().join("TEAM.033")]
        );
    }
}
