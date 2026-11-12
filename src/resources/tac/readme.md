# openswos_tac_resource

Reader and writer for the SWOS 96/97 tactics files (`.TAC`): one tactic, as the game's
tactics editor saves and loads it, binary or as OpenSWOS's JSON.

## Format

A 370-byte tactics record (upstream `TeamTactics`), then nothing the game reads.

| offset | bytes | field |
|---|---|---|
| 0 | 9 | name |
| 9 | 10 × 35 | where each outfield player goes, per ball area |
| 359 | 10 | the outfield players' pairs: same id, linked; `0xFF`, none |
| 369 | 1 | the tactic slot used while the ball is out of play |

- **Ball areas.** The pitch is split into 7 bands along it and 5 across: 35 areas, the
  ball's being `row × 5 + column` (upstream `ball_physics.c`). For each one, a player's
  byte is the grid cell to go to: x (across) in the high nibble, y (along) in the low one
  (upstream `player_positioning.c`). The goalkeeper is not in the tactic: the keeper's
  place follows the ball.
- **The ball-out-of-play slot** is one of the engine's 18 tactic slots (`Tactics`). A byte
  that is not one does not decode.
- **The trailer.** The game copies the first 370 bytes of the file and ignores the rest
  (upstream `ReadTacticsFile`). Those bytes are kept as read: SWOS 2020's CPU tactics end
  with six zero bytes.
- Names keep every byte (`FixedName`), so every file round-trips byte for byte.

## OpenSWOS's JSON tactics

OpenSWOS saves a tactic as JSON (upstream `write_tactics_buf`): the record member by
member, in the saves' JSON (the `save_json` crate), a member a line. `JsonTacFormat`
reads it as the record (`TeamTactics`) and writes it back byte for byte.

| member | value |
|---|---|
| `schema_version`, `file_kind` | 3, `tactics` |
| `name` | the name, up to its first NUL |
| `player_positions` | 10 arrays of the 35 position bytes |
| `pair_ids` | the 10 pair bytes |
| `ball_out_of_play_tactics` | the slot's byte |

Faithful to the original: the name's bytes after its NUL are not written, so a record
holding them writes without them. Upstream's loader keeps at most 8 bytes of a name (the
writer writes 9); the model keeps what is written. What does not read: another schema
version or file kind, a member missing or unknown, a name longer than 9 bytes or holding a
NUL, arrays of other lengths, and what the record does not decode (the slot).

## Sources

Upstream OpenSWOS: `include/game_types.h` (`TeamTactics`, `PlayerPositions`,
`TACTIC_*`), `src/file_io.c` (`ReadTacticsFile`, `WriteTacticsFile`),
`src/save_format_json.c` (`write_tactics_buf`, `load_tactics_buf`),
`src/player_positioning.c` (the position bytes), `src/ball_physics.c` (the ball areas),
`src/edit_tactics.c` (the pairs).

## Tests

- Unit tests and proptests: every field, any valid record and any trailer round-trip.
- `assets/TAC/` (from SWOS 2020): a DOS 4-4-2 checked against its JSON sidecar, a 4-3-3
  whose name keeps bytes after its end, and a CPU 5-3-2 with SWOS 2020's six-byte trailer.
  All of them round-trip. Printed as `write_tactics_buf` prints them, each reads back as
  its record (its name up to the NUL) and writes back byte for byte.
- JSON unit tests: the writer's text (escapes, pairs) round-trips; a name is written up
  to its NUL; what does not read fails for its reason.
- `tests/real_saves.rs`: every `.TAC` in `OPENSWOS_SAVES_DIR` (`just test_saves
  <folder>`), JSON or binary, round-trips byte for byte.
