//! The traits a file format implements: each resource crate has a unit struct (e.g.
//! `TeamFileFormat`) implementing [`Importer`], and [`Exporter`] when the format is
//! written back.
//!
//! An exporter is lossless: `export(import(bytes)) == bytes` for every file the importer
//! accepts (docs/PLAN.md §5.6).

/// Reads a whole file into its model.
pub trait Importer {
    /// The decoded file.
    type Model;
    /// Why a file does not decode.
    type Error: std::error::Error;

    /// Decodes a whole file.
    ///
    /// # Errors
    ///
    /// When `bytes` is not a valid file of the format.
    fn import(bytes: &[u8]) -> Result<Self::Model, Self::Error>;
}

/// Writes a model back into its file format.
pub trait Exporter {
    /// The model to encode.
    type Model;
    /// Why a model cannot be encoded.
    type Error: std::error::Error;

    /// Encodes a whole file. Importing the result gives the model back.
    ///
    /// # Errors
    ///
    /// When the model cannot be represented in the format (e.g. more records than the
    /// format can count).
    fn export(model: &Self::Model) -> Result<Vec<u8>, Self::Error>;
}
