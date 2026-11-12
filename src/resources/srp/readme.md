# openswos_srp_resource

Reader and writer for OpenSWOS's match recordings (`.SRP`, replay format v2; upstream
docs/REPLAY_FORMAT_V2.md): a whole match, its frames as drawn, the sounds it made, and
for a netplay match what was played (setup, kick-off state, inputs, hashes). Every
recording upstream writes round-trips byte for byte.

## The container

A 32-byte header (magic `SWRP`, version 1, flags, tick rate, quirks, the pitch's size,
the frames, the chunks, a CRC-32 of everything after it), a 16-byte directory entry per
chunk (fourcc, offset, stored size, raw size), then the chunks back to back. Big-endian.
A chunk is stored as it is, or raw deflate (its stored size then differs from its raw
size). A file with another magic or version, a CRC that is not its own, chunks not laid
out back to back, or a chunk inflating to another size does not read.

**Byte for byte.** Upstream deflates with miniz at its best compression, which no other
deflater reproduces exactly. So a deflated chunk keeps the bytes it was read with
(`Storage::Deflated`) and writes them back while its content is unchanged;
`SrpChunk::set_data` drops them, and the chunk is deflated here when written (by
upstream's rule: when it saves bytes, over 256 of them). A recording made by this port
is therefore not upstream's byte for byte where it deflates: that needs miniz's output
(the match milestone, M6, if it is wanted).

## The chunks

Typed views decode a chunk's content, and encode back to it:

| chunk | type | content |
|---|---|---|
| `META` | `SrpMeta` | the match: screen width, gameplay flags, view height, time, build, final score, pitch, substitutes, competition and round, the two teams (the `hil` crate's `TeamGame`, big-endian) |
| `FRMS` | `SrpFrame` | a frame as drawn: the camera (16.16), the score, the crowd, the minute, the ad's line, its sprites (image, world X, Y; at most 78) |
| `AUDI` | `AudioEvent` | a sound: frame, kind (preset, wave, volume, the PC game's event, or a later kind), channel, id |
| `SEEK` | `SeekTable` | a keyframe every 50 frames: frame, offset in `FRMS`, first sound |
| `CHAP` | `Chapter` | a goal or a half's start |
| `PLYR` | `SrpPlayers` | who played an online match |

The netplay chunks (`SIMV`, `SETP`, `STAT`, `INPT`, `HASH`, `PRED`, `SEEN`, `READ`) and
any chunk of a later build are kept as their content: what they mean is the match's and
netplay's (M6).

## Sources

Upstream OpenSWOS: `docs/REPLAY_FORMAT_V2.md`, `include/srp_writer.h` (the layout's
constants), `src/srp_writer.c` (`Srp_Build`, `chunk_make`, the chunks' builders),
`src/replay_v2.c` (`build_meta`, `write_head`, `ReplayRec_CaptureSprites`).

## Tests

- Unit tests: the container (a file reads back, keeps its deflated bytes, deflates new
  content; a damaged CRC, magic or version does not read), each chunk's view.
- `tests/real_saves.rs`: every `.SRP` in `OPENSWOS_SAVES_DIR` (`just test_saves
  <folder>`) reads and writes back byte for byte, and each typed chunk decodes and
  encodes back to its content, the frames as many as the header says. Recordings are
  never committed: they hold the game's data.
