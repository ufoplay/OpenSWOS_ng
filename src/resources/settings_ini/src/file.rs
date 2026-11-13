//! The settings file (`SWOS.ini`), its bytes as read. What the game reads from it
//! ([`IniFile::read`], inih's rules) and how a save rewrites it ([`SettingsTable::save`], the
//! writer's rules) each work on the bytes by rules of their own, so the file is kept whole:
//! any bytes are a settings file, and it writes back as read.
//!
//! [`SettingsTable::save`]: crate::SettingsTable::save

use std::convert::Infallible;

use openswos_common::{Exporter, Importer};

use crate::reader::{IniRead, read};

/// A settings file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct IniFile {
    bytes: Vec<u8>,
}

impl IniFile {
    /// The file holding `bytes`.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Its bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// What the game reads from it.
    #[must_use]
    pub fn read(&self) -> IniRead {
        read(&self.bytes)
    }
}

/// The settings file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IniFileFormat;

impl Importer for IniFileFormat {
    type Model = IniFile;
    type Error = Infallible;

    fn import(bytes: &[u8]) -> Result<IniFile, Infallible> {
        Ok(IniFile::new(bytes.to_vec()))
    }
}

impl Exporter for IniFileFormat {
    type Model = IniFile;
    type Error = Infallible;

    fn export(model: &IniFile) -> Result<Vec<u8>, Infallible> {
        Ok(model.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn any_bytes_round_trip_and_read(bytes: Vec<u8>) {
            let file = IniFileFormat::import(&bytes).expect("any bytes");
            prop_assert_eq!(IniFileFormat::export(&file).expect("writes"), bytes);
            let _ = file.read();
        }
    }
}
