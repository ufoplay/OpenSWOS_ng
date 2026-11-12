//! The other drawn chunks (upstream src/srp_writer.c): the sounds (`AUDI`), the seek table
//! (`SEEK`), the chapters (`CHAP`) and who played (`PLYR`). Big-endian.

use openswos_common::{ByteReader, ByteWriter};

use crate::error::ChunkError;

/// The seek table's interval upstream writes (`SEEK_INTERVAL`): a keyframe every 50 frames.
pub const SEEK_INTERVAL: u16 = 50;

/// The kind of a preset sound event.
const AUDIO_PRESET: u8 = 1;
/// The kind of a wave.
const AUDIO_WAVE: u8 = 2;
/// The kind of a channel's volume.
const AUDIO_VOLUME: u8 = 3;
/// The kind of a PC game's sound event.
const AUDIO_DOS_EVENT: u8 = 4;

/// The kind of a goal chapter.
const CHAPTER_GOAL: u8 = 1;
/// The kind of a half's start.
const CHAPTER_HALF: u8 = 2;

/// What a sound event is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioEventKind {
    /// A preset applied to the channels (upstream `AudioApplyPresetToChannels`).
    Preset,
    /// A wave queued on a channel.
    Wave,
    /// A channel's volume (channel 255: all of them).
    Volume,
    /// Something the PC game has a sound for (`Audio_DosEvent`): the channel is the event,
    /// the id its argument.
    DosEvent,
    /// A kind of a later build, which upstream's reader skips. Never 1 to 4 when read.
    Unknown(u8),
}

impl From<u8> for AudioEventKind {
    fn from(kind: u8) -> Self {
        match kind {
            AUDIO_PRESET => Self::Preset,
            AUDIO_WAVE => Self::Wave,
            AUDIO_VOLUME => Self::Volume,
            AUDIO_DOS_EVENT => Self::DosEvent,
            other => Self::Unknown(other),
        }
    }
}

impl From<AudioEventKind> for u8 {
    fn from(kind: AudioEventKind) -> Self {
        match kind {
            AudioEventKind::Preset => AUDIO_PRESET,
            AudioEventKind::Wave => AUDIO_WAVE,
            AudioEventKind::Volume => AUDIO_VOLUME,
            AudioEventKind::DosEvent => AUDIO_DOS_EVENT,
            AudioEventKind::Unknown(other) => other,
        }
    }
}

/// A sound the match made (8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioEvent {
    /// The frame.
    pub frame: u32,
    /// What it is.
    pub kind: AudioEventKind,
    /// The channel (or the PC game's event).
    pub channel: u8,
    /// The sound (or the event's argument).
    pub id: u16,
}

/// A keyframe of the seek table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SeekEntry {
    /// The frame.
    pub frame: u32,
    /// Where it starts in `FRMS`'s content.
    pub frms_offset: u32,
    /// The first sound event at or after it.
    pub audio_index: u32,
}

/// The seek table: a keyframe every `interval` frames.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SeekTable {
    /// The frames between keyframes.
    pub interval: u16,
    /// The word after it, kept as written (0).
    pub reserved: u16,
    /// The keyframes.
    pub entries: Vec<SeekEntry>,
}

/// What a chapter marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChapterKind {
    /// A goal.
    Goal,
    /// A half's start.
    Half,
}

/// A chapter: a goal or a half's start, to jump to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chapter {
    /// The frame.
    pub frame: u32,
    /// What it marks.
    pub kind: ChapterKind,
    /// The team (a goal's).
    pub team: u8,
    /// The value (the half).
    pub value: u16,
}

/// Who played an online match, as the lobby named them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SrpPlayers {
    /// The chunk's version (1).
    pub version: u16,
    /// Player 1's (the host's) name, as its bytes.
    pub player1: Vec<u8>,
    /// Player 2's name.
    pub player2: Vec<u8>,
}

/// The sounds of an `AUDI` chunk's content.
///
/// # Errors
///
/// When the content is not whole events.
pub fn decode_audio(data: &[u8]) -> Result<Vec<AudioEvent>, ChunkError> {
    let mut reader = ByteReader::new(data);
    let mut events = Vec::new();
    while !reader.is_at_end() {
        events.push(AudioEvent {
            frame: reader.read_u32_be()?,
            kind: AudioEventKind::from(reader.read_u8()?),
            channel: reader.read_u8()?,
            id: reader.read_u16_be()?,
        });
    }
    Ok(events)
}

/// The `AUDI` chunk's content.
#[must_use]
pub fn encode_audio(events: &[AudioEvent]) -> Vec<u8> {
    let mut writer = ByteWriter::new();
    for AudioEvent {
        frame,
        kind,
        channel,
        id,
    } in events
    {
        writer.write_u32_be(*frame);
        writer.write_u8(u8::from(*kind));
        writer.write_u8(*channel);
        writer.write_u16_be(*id);
    }
    writer.into_bytes()
}

/// The seek table of a `SEEK` chunk's content.
///
/// # Errors
///
/// When the content is short of its keyframes, or longer.
pub fn decode_seek(data: &[u8]) -> Result<SeekTable, ChunkError> {
    let mut reader = ByteReader::new(data);
    let interval = reader.read_u16_be()?;
    let reserved = reader.read_u16_be()?;
    let count = reader.read_u32_be()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        entries.push(SeekEntry {
            frame: reader.read_u32_be()?,
            frms_offset: reader.read_u32_be()?,
            audio_index: reader.read_u32_be()?,
        });
    }
    reader.finish()?;
    Ok(SeekTable {
        interval,
        reserved,
        entries,
    })
}

/// The `SEEK` chunk's content.
///
/// # Errors
///
/// When there are more keyframes than a long word counts.
pub fn encode_seek(table: &SeekTable) -> Result<Vec<u8>, ChunkError> {
    let SeekTable {
        interval,
        reserved,
        entries,
    } = table;
    let mut writer = ByteWriter::new();
    writer.write_u16_be(*interval);
    writer.write_u16_be(*reserved);
    writer.write_u32_be(u32::try_from(entries.len()).map_err(|_| ChunkError::TooMany)?);
    for SeekEntry {
        frame,
        frms_offset,
        audio_index,
    } in entries
    {
        writer.write_u32_be(*frame);
        writer.write_u32_be(*frms_offset);
        writer.write_u32_be(*audio_index);
    }
    Ok(writer.into_bytes())
}

/// The chapters of a `CHAP` chunk's content.
///
/// # Errors
///
/// When the content is short of its chapters or longer, or a chapter's kind is not one.
pub fn decode_chapters(data: &[u8]) -> Result<Vec<Chapter>, ChunkError> {
    let mut reader = ByteReader::new(data);
    let count = reader.read_u32_be()?;
    let mut chapters = Vec::new();
    for _ in 0..count {
        let frame = reader.read_u32_be()?;
        let kind = match reader.read_u8()? {
            CHAPTER_GOAL => ChapterKind::Goal,
            CHAPTER_HALF => ChapterKind::Half,
            other => return Err(ChunkError::ChapterKind(other)),
        };
        chapters.push(Chapter {
            frame,
            kind,
            team: reader.read_u8()?,
            value: reader.read_u16_be()?,
        });
    }
    reader.finish()?;
    Ok(chapters)
}

/// The `CHAP` chunk's content.
///
/// # Errors
///
/// When there are more chapters than a long word counts.
pub fn encode_chapters(chapters: &[Chapter]) -> Result<Vec<u8>, ChunkError> {
    let mut writer = ByteWriter::new();
    writer.write_u32_be(u32::try_from(chapters.len()).map_err(|_| ChunkError::TooMany)?);
    for Chapter {
        frame,
        kind,
        team,
        value,
    } in chapters
    {
        writer.write_u32_be(*frame);
        writer.write_u8(match kind {
            ChapterKind::Goal => CHAPTER_GOAL,
            ChapterKind::Half => CHAPTER_HALF,
        });
        writer.write_u8(*team);
        writer.write_u16_be(*value);
    }
    Ok(writer.into_bytes())
}

/// The players of a `PLYR` chunk's content.
///
/// # Errors
///
/// When the content is short of its names, or longer.
pub fn decode_players(data: &[u8]) -> Result<SrpPlayers, ChunkError> {
    let mut reader = ByteReader::new(data);
    let version = reader.read_u16_be()?;
    let mut name = || -> Result<Vec<u8>, ChunkError> {
        let len = usize::from(reader.read_u8()?);
        Ok(reader.read_bytes(len)?.to_vec())
    };
    let player1 = name()?;
    let player2 = name()?;
    reader.finish()?;
    Ok(SrpPlayers {
        version,
        player1,
        player2,
    })
}

/// The `PLYR` chunk's content.
///
/// # Errors
///
/// When a name is longer than 255 bytes.
pub fn encode_players(players: &SrpPlayers) -> Result<Vec<u8>, ChunkError> {
    let SrpPlayers {
        version,
        player1,
        player2,
    } = players;
    let mut writer = ByteWriter::new();
    writer.write_u16_be(*version);
    for name in [player1, player2] {
        writer.write_u8(u8::try_from(name.len()).map_err(|_| ChunkError::TooMany)?);
        writer.write_bytes(name);
    }
    Ok(writer.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_events_round_trip() {
        let audio = vec![
            AudioEvent {
                frame: 3,
                kind: AudioEventKind::Wave,
                channel: 2,
                id: 77,
            },
            AudioEvent {
                frame: 9,
                kind: AudioEventKind::Unknown(9),
                channel: 0,
                id: 1,
            },
        ];
        assert_eq!(decode_audio(&encode_audio(&audio)).expect("decodes"), audio);
        let seek = SeekTable {
            interval: SEEK_INTERVAL,
            reserved: 0,
            entries: vec![SeekEntry {
                frame: 0,
                frms_offset: 0,
                audio_index: 0,
            }],
        };
        assert_eq!(
            decode_seek(&encode_seek(&seek).expect("encodes")).expect("decodes"),
            seek
        );
        let chapters = vec![Chapter {
            frame: 100,
            kind: ChapterKind::Goal,
            team: 1,
            value: 0,
        }];
        let data = encode_chapters(&chapters).expect("encodes");
        assert_eq!(decode_chapters(&data).expect("decodes"), chapters);
        let players = SrpPlayers {
            version: 1,
            player1: b"ANA".to_vec(),
            player2: Vec::new(),
        };
        assert_eq!(
            decode_players(&encode_players(&players).expect("encodes")).expect("decodes"),
            players
        );
    }

    #[test]
    fn malformed_events_do_not_read() {
        assert!(decode_audio(&[0; 7]).is_err());
        assert!(decode_seek(&[0, 50, 0, 0, 0, 0, 0, 1]).is_err());
        assert!(matches!(
            decode_chapters(&[0, 0, 0, 1, 0, 0, 0, 0, 3, 0, 0, 0]),
            Err(ChunkError::ChapterKind(3))
        ));
    }
}
