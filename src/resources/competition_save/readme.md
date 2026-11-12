# openswos_competition_save_resource

Reader and writer for OpenSWOS's competition saves, byte for byte (D4): a preset
competition (`.SEA`, `.PRE`: file kind `preset`) or a DIY one (`.DIY`: `diy`); and for
the DOS game's binary `.DIY` (D9). The JSON saves are built on
`openswos_save_json_resource` (the saves' JSON dialect), with
`openswos_career_save_resource`'s sections.

## The save

`CompetitionSave` has one field per top-level member, in the order upstream's writer
(`write_competition_buf`) writes them:

| member | type |
|---|---|
| `schema_version`, `file_kind` | `SchemaVersion`, `CompetitionKind`: only 3, and `preset` or `diy`, read |
| `world` | the world's tag; absent in saves made before worlds |
| `num_diy_teams` | `diy_teams`' length: must agree, on reading and on writing |
| `comp_scalars` | `CompetitionScalars`: the 35 scalars of the career's table tagged for the competition saves, in its order (the primary slot's header and raw state, the team id lists, the save's version, the match settings, the user's tactics) |
| `comp_slot` | the competition: a `CompetitionSlot` |
| `competition_overrides` | its league table by team identity |
| `comp_slot_stage` … `stage_score_pen` | a tournament's current stage, only for a tournament |
| `diy_teams` | the teams, as the team crate's `Team` |

The layout is the writer's: one member a line, none written `"key": value`, the stage's
cup list on the line of the stage. Unknown members do not read.

## The DOS game's `.DIY` (M2 #11i)

The PC game and SWOS 2020 save a DIY competition as a memory dump (SWOS2.s
`SaveDIYFile`): the 5189-byte competition region, the teams' count, the 684-byte team
records. Words are little-endian. `DosDiyFormat` reads and writes it byte for byte
(`DosDiyFile`):

| offset | bytes | field |
|---|---|---|
| 0 | 95 | the header (`DosCompetitionHeader`): type, name (35 bytes), the round state… |
| 95 | 2815 | the body, by the competition's type, kept as bytes |
| 0xB5E | 6 × 370 | the user's tactics, the `tac` crate's `TeamTactics` |
| 0x140A | 12 | the match options (`DosMatchOptions`) |
| 0x1416 | 47 | the tail: option words and random seeds, kept as bytes |
| 5189 | 2 | the teams' count (1..=64; the header's must agree) |
| 5191 | 684 × count | the teams, the team crate's `Team` |

- **The body** depends on the type. A league or cup: the rest of its `DIYCompetitionEx`,
  then its 64 standings rows selecting their teams by byte offset into the teams. A
  tournament: its team order, team ids, group map, six stage slots and the current stage
  (a `DIYCompetitionEx` at 0x41F). What it holds is the competitions' (M5).
- **Faithful to the original:** as in an `.EDT`, the teams' global numbers are
  little-endian in memory; the team crate reads them byte-swapped (the game recomputes
  them).
- **Making our JSON `.DIY` from one** (upstream `--convert-diy`) goes through the
  competition's state: the JSON save writes its slot's raw state in the x86_64 layout.
  That is the competitions' (M5, M7), as upstream's goes through the game's memory.

## Sources

Upstream OpenSWOS: `src/save_format_json.c` (`write_competition_buf`,
`SaveJson_WritePreset`, `SaveJson_WriteDIY`, `emit_scalar_table_comp`, the `FXC`/`FAC`/
`FXS`/`FAS` entries of `s_career_scalars`), `src/diy_import_dos.c` (the DOS `.DIY`'s
layout: `DiyImportDos_IsDosFile`, `kRegion`, `kHeader`, `kTactics`, `kOptions`…),
`docs/SAVE_DATA.md`.

## Tests

- Unit tests: the layout of a DIY cup and of a tournament with its stage, the count
  check, what does not read.
- `tests/upstream.rs`: with upstream's checkout in the cache, `CompetitionScalars` is
  exactly the table's tagged fields, in order.
- `tests/dos_fixtures.rs`: SWOS 2020's two DIY cups (`assets/DIY/`) round-trip byte for
  byte and decode as they are (names, teams, tactics, options, the little-endian global
  numbers). Unit tests: what the DOS format refuses.
- `tests/real_saves.rs`: every JSON competition save in `OPENSWOS_SAVES_DIR` (`just
  test_saves <folder>`) reads as a typed competition, every DOS `.DIY` as the DOS file,
  and each writes back byte for byte. No JSON competition save has been available so
  far: that format is checked against the writer's code. Saves are never committed.
