# openswos_hil_resource

Reader and writer for replays, byte for byte: the DOS game's (the PC game and SWOS 2020,
D9) highlights (`.HIL`, a scene for each goal) and whole matches (`.RPL`, one long
scene), and OpenSWOS's JSON highlights (`.HIL`, [below](#openswoss-json-highlights)).
OpenSWOS's other `.HIL`, big-endian, is the Amiga's layout (not read, D9).

## The DOS format

A 3626-byte header, then the scenes. Little-endian (upstream src/dos_replay_load.c).

| offset | bytes | field |
|---|---|---|
| 0 | 4 | the signature: 8 |
| 4 | 4 | a byte, the game style (the playback rate), a word |
| 8 | 1704 | the first team (`DosTeamGame`) |
| 1712 | 1704 | the second team |
| 3416 | 40 | the competition's name |
| 3456 | 160 | zero in SWOS 2020's files, kept as written |
| 3616 | 2 × 5 | the scenes, the two teams' goals, the pitch, the most substitutes |

- **The teams** are the DOS game's match records (swos-port `TeamGame`): the two kits,
  the marked player, the name, 16 players' match data and short and full names
  (`DosPlayerGame`), and a 686-byte tail the DOS game keeps, as bytes.
- **A scene** (`Scene`) is frames of little-endian long words: a camera word (bit 31
  set: Y << 16 | X), the score (the second team's goals << 16 | the first's), an
  animation state, then the frame's sprites (image << 20 | X << 10 | Y, the coordinates
  10-bit signed, the images the DOS bank's: the Amiga's + 114). `0xFFFFFFFF` ends it.
- **A `.HIL`** holds its scenes in slots of one size: 19000 bytes in the PC game's
  files, 61000 in SWOS 2020's (a widescreen frame holds more). The scenes must divide
  the bytes after the header. A scene's slot holds, after its end, what it held before:
  those bytes are kept as written.
- **A `.RPL`** holds one scene, the whole match: SWOS 2020's reach 2.4 MB.

What does not read: another signature, a scene with no end, a sprite before a scene's
first camera word, a scene ending inside a frame's header. A frame whose score or
animation state would read as the end word does not write.

Converting a replay for the engine (upstream: into the Amiga layout, its sprites less
114, at the rate its game style gives) is the view's and the match's (M3, M6).

## OpenSWOS's JSON highlights

OpenSWOS saves highlights as JSON (`SaveJson_WriteHighlights`): its highlights buffer
(the Amiga's layout: a big-endian header, then a 19000-byte slot a goal) member by
member, in the saves' JSON (the `save_json` crate). `JsonHilFile` is the file, its
members in the writer's order:

| member | type |
|---|---|
| `schema_version`, `file_kind` | `SchemaVersion`, `HighlightsKind`: only 3 and `highlights` read |
| `signature`, `max_size` | the header's two long words, as the buffer held them |
| `top_team`, `bottom_team` | `TeamGame`, the record `.SRP`s hold: the kits' words and the marked player written unsigned, the three bytes after the name unsigned, the players' bytes signed |
| `game_name`, `game_round` | 100-byte names |
| `num_goals` … `max_substitutes` | the scenes' count, the final score, the pitch, the most substitutes |
| `replay_fps` | the rate the match was played at (70 DOS, 50 Amiga); absent in older files |
| `scenes` | `JsonScene`: the slot's frames as the writer decodes them, where its end word leaves off (`eos_offset`) and the slot's bytes after it (`trailing`) |

A frame (`JsonFrame`) is the camera (X 15 bits, Y 16), the score, the animation state and
the sprites (`JsonSprite`: index 12 bits, X and Y 10 bits unsigned). Sprites before a
slot's first marker get a made-up frame, `"_synthetic_no_frame_marker":true`, its other
values 0. The writer lays the file out a member a line, `num_goals` and the five numbers
after it on one line.

Names are written up to their first NUL (upstream `emit_str_at`): a team record holding
bytes after it writes without them, and reads back NUL-padded. Faithful to the original:
an animation state's word holds it twice, and only the high half is written.

What does not read: another schema version or file kind, an unknown member, a name
longer than its field or holding a NUL, `num_goals` other than the scenes' count, a
scene's end past its 19000 bytes or `trailing` not the rest of them, a value wider than
its bits, a frame whose marker is the end word (`0x7FFF`, `0xFFFF`). Filling the buffer
back from the file is upstream's loader's, in the Amiga's layout (D9: not done here).

## Sources

Upstream OpenSWOS: `src/dos_replay_load.c` (the header's offsets, the frame words,
`replay_fps_from_gamestyle`), `docs/AMIGA_DOS_DIFF.md` (the sprite numbering),
`docs/TESTS.md` and `scripts/rgd_schema.py` (`TeamGame`, `PlayerGame`),
`src/save_format_json.c` (`SaveJson_WriteHighlights`, `emit_team_game`, `emit_scene`,
`SaveJson_LoadHighlights`, `load_team_game_slice`, `load_scene_slice`, `HilFileHeader`).

## Tests

- Unit tests: the frame words (camera, score, sprites, every coordinate), the leftover,
  what does not read or write, the team record's 1704 bytes.
- `tests/fixtures.rs`: a SWOS 2020 highlight of one goal and the first 100 frames of a
  SWOS 2020 match (`assets/HIL/`) round-trip byte for byte and decode as they are (the
  score before and after the goal, 27 sprites a frame, the teams' and competition's
  names).
- JSON unit tests: texts printed as `SaveJson_WriteHighlights` prints them (a made-up
  frame, a Latin-1 name, escapes, negative bytes, unsigned words, with and without
  `replay_fps`) round-trip byte for byte; what does not read fails for its reason.
- `tests/real_saves.rs`: every `.HIL` in `OPENSWOS_SAVES_DIR` (`just test_saves
  <folder>`), JSON or DOS, round-trips byte for byte. Highlights are never committed.
