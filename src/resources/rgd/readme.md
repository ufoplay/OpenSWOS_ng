# openswos_rgd_resource

Reader and writer for swos-port's match recordings (`.RGD`, gzip-compressed as `.RGD.gz`):
the DOS game's state on every frame of a match, recorded by swos-port's rig. They are the
match engine's golden oracle (docs/PLAN.md §7.4): a replay test (M6) drives the engine
with the recorded inputs and compares its state with each frame, field by field.

## Format

A header, then fixed-size frames: no index, no frame lengths. The version in the first
two bytes gives the sizes (upstream docs/TESTS.md §2). Packed, little-endian.

| version | header | frame |
|---|---:|---:|
| 1.0, 1.1 | 3610 | 7404 |
| 1.2 | 3617 | unresolved: not read |
| 1.3 | 3617 | 7475 (+ the bench's state) |
| 1.4 | 3617 | 7481 (+ the key and two timers) |
| 1.5 | 3617 | 7485 (+ two tap timeouts) |

- **The header** (`RgdHeader`): the version, the fixed random value the match was played
  with, the match settings, the two teams (the team crate's `Team`; faithful to the
  original, their global numbers are little-endian in memory, read byte-swapped), the
  coach and player numbers, extra time, penalties, the marked players, the user's six
  tactics (the `tac` crate's `TeamTactics`), the input controls, the initial frame, and
  from 1.2 the career size, the season and the substitutes.
- **Frames** (`RgdFrame`) are kept as their bytes. Their own scalars have accessors:
  the frame number, the players' controls (`Controls`), the camera, the clock, the goals,
  the game's state. Every other field is read by name through the schema.
- **The schema** (`frame_fields(version, level)`, upstream `scripts/rgd_schema.py`): each
  field of the DOS structs (team info, in-game teams and players, statistics, bench,
  sprites) with its type, offset, the replay level that compares it (`Level`: scalars,
  teams, info, sprites) and how (`Compare`: exactly, by truth, only at the DOS view size,
  never, as a sprite index, as a pointer's identity, as input, as bookkeeping). A replay
  compares by name, never by offset, so the two trees' structs may differ.
- **Version 1.2** is not read: its one recording matches no header and frame sizes
  (upstream could not resolve it either).

`RgdReader` streams a recording frame by frame (`RgdReader::gz` for `.RGD.gz`): the
biggest is 290 MB decompressed. `RgdFormat` reads and writes a whole `.RGD` byte for byte.

## The corpus

swos-port's `tests/data`, 15 recordings (82 MB compressed), pinned at commit `4da94f5`
with their checksums (`test_utils::cache::RGD_CORPUS`). That repository declares no
license: they are downloaded into `target/cache/` on first use, never committed (D8).

## Sources

Upstream OpenSWOS: `docs/TESTS.md` §1-2 (the format, the corpus and its known endings),
`scripts/rgd_schema.py` (the fields), `scripts/rgd_convert.py` (the decode invariants).
swos-port: `src/swos/swos.h`, `RecordedDataTest`.

## Tests

- Unit tests: the versions' sizes, the frame's scalars, fields by name, the schema's
  tables and levels (every field inside its frame, names unique).
- `tests/corpus.rs`: every recording's header reads and writes back byte for byte, and
  every frame holds upstream's decode invariants (frame numbers run on, the clock never
  goes back, its first digit is 0, at most 200 minutes and 30 goals). Six recordings end
  as upstream's table says (frames, minutes, score); eight more name their teams; version
  1.2's does not read.
