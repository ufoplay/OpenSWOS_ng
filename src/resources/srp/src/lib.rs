#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod container;
pub mod error;
pub mod events;
pub mod frames;
pub mod meta;
mod views;

pub use container::{
    FourCc, SWRP_DIR_LEN, SWRP_HEADER_LEN, SWRP_MAGIC, SWRP_VERSION, SrpChunk, SrpError, SrpFile,
    SrpFlags, SrpFormat, SrpHeader, SrpQuirks, Storage,
};
pub use error::ChunkError;
pub use events::{
    AudioEvent, AudioEventKind, Chapter, ChapterKind, SEEK_INTERVAL, SeekEntry, SeekTable,
    SrpPlayers, decode_audio, decode_chapters, decode_players, decode_seek, encode_audio,
    encode_chapters, encode_players, encode_seek,
};
pub use frames::{
    FRAME_HEAD_LEN, MAX_FRAME_SPRITES, SPRITE_LEN, SrpFrame, SrpSprite, decode_frames,
    encode_frames,
};
pub use meta::{BUILD_LEN, GAME_NAME_LEN, GameplayFlags, SrpMeta};
