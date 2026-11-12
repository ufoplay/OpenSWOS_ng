//! The typed views of a recording's chunks.

use crate::container::{FourCc, SrpFile};
use crate::error::ChunkError;
use crate::events::{
    AudioEvent, Chapter, SeekTable, SrpPlayers, decode_audio, decode_chapters, decode_players,
    decode_seek,
};
use crate::frames::{SrpFrame, decode_frames};
use crate::meta::SrpMeta;

impl SrpFile {
    fn view<T>(
        &self,
        fourcc: FourCc,
        decode: impl FnOnce(&[u8]) -> Result<T, ChunkError>,
    ) -> Option<Result<T, ChunkError>> {
        self.chunk(fourcc).map(|chunk| decode(chunk.data()))
    }

    /// The match (`META`), if the file has it.
    #[must_use]
    pub fn meta(&self) -> Option<Result<SrpMeta, ChunkError>> {
        self.view(FourCc::META, SrpMeta::decode)
    }

    /// The frames (`FRMS`), if the file has them.
    #[must_use]
    pub fn frames(&self) -> Option<Result<Vec<SrpFrame>, ChunkError>> {
        self.view(FourCc::FRMS, decode_frames)
    }

    /// The sounds (`AUDI`), if the file has them.
    #[must_use]
    pub fn audio(&self) -> Option<Result<Vec<AudioEvent>, ChunkError>> {
        self.view(FourCc::AUDI, decode_audio)
    }

    /// The seek table (`SEEK`), if the file has it.
    #[must_use]
    pub fn seek(&self) -> Option<Result<SeekTable, ChunkError>> {
        self.view(FourCc::SEEK, decode_seek)
    }

    /// The chapters (`CHAP`), if the file has them.
    #[must_use]
    pub fn chapters(&self) -> Option<Result<Vec<Chapter>, ChunkError>> {
        self.view(FourCc::CHAP, decode_chapters)
    }

    /// Who played (`PLYR`), if the file says.
    #[must_use]
    pub fn players(&self) -> Option<Result<SrpPlayers, ChunkError>> {
        self.view(FourCc::PLYR, decode_players)
    }
}
