//! The DOS game's replays (the PC game and SWOS 2020): highlights (`.HIL`), a header and
//! one slot a goal, and the whole match (`.RPL`), a header and one long scene.

use openswos_common::{ByteReader, ByteWriter, Exporter, Importer};

use crate::error::DosReplayError;
use crate::header::{DOS_HEADER_LEN, DosReplayHeader};
use crate::scene::Scene;

/// A DOS highlights file: its scenes, each in a slot of the same size (19000 bytes in
/// the PC game's files, 61000 in SWOS 2020's, whose widescreen frames hold more).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DosHilFile {
    /// The header. Its scene count is the scenes'.
    pub header: DosReplayHeader,
    /// The goals' scenes.
    pub scenes: Vec<Scene>,
}

/// A DOS whole-match replay: one scene.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DosRplFile {
    /// The header.
    pub header: DosReplayHeader,
    /// The match's scene.
    pub scene: Scene,
}

fn read_header(bytes: &[u8]) -> Result<(DosReplayHeader, &[u8]), DosReplayError> {
    let mut reader = ByteReader::new(bytes);
    let header = DosReplayHeader::read(&mut reader)?;
    Ok((header, bytes.get(DOS_HEADER_LEN..).unwrap_or_default()))
}

fn header_bytes(header: &DosReplayHeader, capacity: usize) -> Result<ByteWriter, DosReplayError> {
    let mut writer = ByteWriter::with_capacity(DOS_HEADER_LEN + capacity);
    header.write(&mut writer)?;
    Ok(writer)
}

fn scene_error(index: usize) -> impl Fn(crate::scene::SceneError) -> DosReplayError {
    move |error| DosReplayError::Scene { index, error }
}

/// The DOS `.HIL` format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DosHilFormat;

impl Importer for DosHilFormat {
    type Model = DosHilFile;
    type Error = DosReplayError;

    fn import(bytes: &[u8]) -> Result<DosHilFile, DosReplayError> {
        let (header, stream) = read_header(bytes)?;
        let scenes = usize::from(header.num_scenes);
        let slots_error = DosReplayError::Slots {
            scenes: header.num_scenes,
            bytes: stream.len(),
        };
        if scenes == 0 || stream.len() % scenes != 0 {
            return Err(slots_error);
        }
        let slot_len = stream.len() / scenes;
        let scenes = stream
            .chunks(slot_len.max(1))
            .enumerate()
            .map(|(index, slot)| Scene::parse(slot).map_err(scene_error(index)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(DosHilFile { header, scenes })
    }
}

impl Exporter for DosHilFormat {
    type Model = DosHilFile;
    type Error = DosReplayError;

    fn export(model: &DosHilFile) -> Result<Vec<u8>, DosReplayError> {
        let DosHilFile { header, scenes } = model;
        let encoded = scenes
            .iter()
            .enumerate()
            .map(|(index, scene)| scene.encode().map_err(scene_error(index)))
            .collect::<Result<Vec<_>, _>>()?;
        let sizes: Vec<usize> = encoded.iter().map(Vec::len).collect();
        let one_size = sizes.windows(2).all(|pair| pair.first() == pair.last());
        if usize::from(header.num_scenes) != scenes.len() || scenes.is_empty() || !one_size {
            return Err(DosReplayError::Scenes {
                stated: header.num_scenes,
                scenes: scenes.len(),
                sizes,
            });
        }
        let mut writer = header_bytes(header, sizes.iter().sum())?;
        for scene in encoded {
            writer.write_bytes(&scene);
        }
        Ok(writer.into_bytes())
    }
}

/// The DOS `.RPL` format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DosRplFormat;

impl Importer for DosRplFormat {
    type Model = DosRplFile;
    type Error = DosReplayError;

    fn import(bytes: &[u8]) -> Result<DosRplFile, DosReplayError> {
        let (header, stream) = read_header(bytes)?;
        let scene = Scene::parse(stream).map_err(scene_error(0))?;
        Ok(DosRplFile { header, scene })
    }
}

impl Exporter for DosRplFormat {
    type Model = DosRplFile;
    type Error = DosReplayError;

    fn export(model: &DosRplFile) -> Result<Vec<u8>, DosReplayError> {
        let scene = model.scene.encode().map_err(scene_error(0))?;
        let mut writer = header_bytes(&model.header, scene.len())?;
        writer.write_bytes(&scene);
        Ok(writer.into_bytes())
    }
}
