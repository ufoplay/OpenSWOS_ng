# openswos_tac_resource

Reader and writer for the SWOS 96/97 tactics files (`.TAC`): one tactic, as the game's
tactics editor saves and loads it.

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

OpenSWOS can also save a tactic as JSON (upstream `save_format_json.c`, with a
`schema_version`). That variant comes with the JSON saves (M2 #11).

## Sources

Upstream OpenSWOS: `include/game_types.h` (`TeamTactics`, `PlayerPositions`,
`TACTIC_*`), `src/file_io.c` (`ReadTacticsFile`, `WriteTacticsFile`),
`src/player_positioning.c` (the position bytes), `src/ball_physics.c` (the ball areas),
`src/edit_tactics.c` (the pairs).

## Tests

- Unit tests and proptests: every field, any valid record and any trailer round-trip.
- `assets/TAC/` (from SWOS 2020): a DOS 4-4-2 checked against its JSON sidecar, a 4-3-3
  whose name keeps bytes after its end, and a CPU 5-3-2 with SWOS 2020's six-byte trailer.
  All of them round-trip.
