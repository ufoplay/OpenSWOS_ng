# openswos_tactics_json_resource

Reader and writer for OpenSWOS's built-in tactics JSON (`bin/data_json/tactics/*.json`):
the twelve preset tactics (4-4-2 … Defend), one file each. They are an input of
`world_extract` (docs/PLAN.md §5.2).

## Format

A straight serialisation of the 370-byte tactics record (`TeamTactics`, read by
`openswos_tac_resource`), plus the preset's id and label:

| key | value |
|---|---|
| `id` | the preset's id (`442`), which names its file. The game finds a file by its name. |
| `label` | the preset's label (`4-4-2`), which the game does not read |
| `name_bytes` | the record's 9 name bytes, each `"0xHH"` |
| `players` | 10 outfield players × 5 groups × 7 position bytes, each `"0xHH"` |
| `pair_ids` | the 10 pair bytes, signed (`-1` is `0xFF`, no pair) |
| `ball_out_of_play` | the tactic slot used while the ball is out of play |

- **Position groups.** A player's 35 bytes come in the C declaration's grouping,
  `positions[5][7]`. They mean 7 bands of 5 ball areas (`PlayerSpots`); only the
  grouping differs.
- **Spellings.** Upstream's exporter writes every byte as `"0x34"`. Its loaders (`strtol`
  in C, `int(…, 0)` in Python) also read `"0X34"`, `"0x4"`, `"52"`, `"-1"` and numbers,
  and so does this crate. Everything is written back in the exporter's spelling. A
  leading zero (`"034"`) does not read: octal to one loader, an error to the other.
- **Layout.** Written as the exporter writes it: `json.dumps(indent=1,
  ensure_ascii=False)` with every innermost array on one line, and a newline at the end.
  Upstream's files round-trip byte for byte.

## Files

| file | holds |
|---|---|
| `<id>.json` | what OpenSWOS plays: the original's data, except 4-4-2, 4-3-3, 3-4-3 and Sweep, corrected so that the central defenders stop swapping sides |
| `<id>_original.json` | the Amiga original's data for those four |
| `<id>_dos.json` | the DOS release's data for those four: it re-tuned 4-4-2 (68 position bytes), Sweep (64) and 4-3-3 (10); its 3-4-3 is the Amiga's |

`PresetFile` names and parses these files; `Preset` is the twelve presets in the engine's
order (`Preset::ALL`), with their ids and labels.

## Sources

Upstream OpenSWOS: `scripts/export_tactics.py` (the writer, `IDS`, `LABEL`),
`scripts/emit_tactics.py` and `src/tactics_load_json.c` (the readers),
`src/tactics_data.c` (the releases), `include/game_types.h` (`TeamTactics`,
`PlayerPositions`).

## Tests

- Unit tests: spellings, layout, errors, presets and file names, and a property test (any
  record and strings round-trip).
- `tests/fixtures.rs`: upstream's 20 files (`assets/TACTICS_JSON/`, GPL) round-trip byte
  for byte; each names its preset in its id and label; the releases differ where
  upstream's notes say; SWOS 2020's CPU 5-3-2 (`assets/TAC/04_5-3-2.tac`) is the built-in
  one.
