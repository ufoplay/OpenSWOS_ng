# openswos_save_json_resource

The JSON dialect of OpenSWOS's saves (`.CAR`, `.SEA`, `.PRE`, `.DIY`, and the JSON
`.HIL` and `.TAC`), read and written byte for byte (D4). The typed saves
(`career_save`, `competition_save`, the `hil` crate's JSON highlights) build on it.

## The dialect

Upstream writes saves with mjson's printer (`src/mjson.c`) and the writers of
`src/save_format_json.c`:

- **Strings are bytes.** Only `\b \f \n \r \t \\ \"` are escaped. Every other byte is
  written as it is: a byte past `0x7F` (a name in the game's character set) or a control
  character, raw. So a save need not be UTF-8, nor strict JSON.
- **Numbers are integers.**
- **Layout.** `{`, one top-level member a line indented by two spaces, `\n}\n`. Values
  are compact. A career's first four members (schema version, file kind, world, selected
  teams) are written `"key": value`; every other member, in every kind, `"key":value`.
  The writers break the pattern here and there: a career's `cache_scores` follows
  `cache_strides` on the same line. So the whitespace around each top-level member is
  data (`Layout`), read with the file; `FileKind::layout` gives upstream's usual one.

`text` reads such text with `serde_json` by mapping it to standard JSON first (each
byte past ASCII as the character of its code, each raw control character in a string
as an escape). `formatter` writes it with `SaveFormatter`, a `serde_json` formatter that
maps back and lays the top level out by a `TopLevelLayout` (`layout`): `Layout`, the
whitespace read with a file, or `UpstreamLayout`, the writer's rule (spaced members,
members written inline), which the typed saves use. The formatter holds each top-level
key back until it is complete, so a rule can place whitespace by the key's name. Any
`Serialize` / `Deserialize` type reads and writes saves this way (`from_save_bytes`,
`to_save_bytes`). `SaveString` holds a string's bytes; `SchemaVersion` is the typed
saves' `schema_version` member, reading only 3.

## The document

`SaveDocument` is a save as written: its top-level members in order, each a
`SaveValue` (objects keep their members' order), and its layout. It requires upstream's schema version
(3: upstream refuses others) and a known file kind (`career`, `preset`, `diy`,
`highlights`, `tactics`). Every save upstream writes round-trips byte for byte.

## Sources

Upstream OpenSWOS: `src/mjson.c` (`mjson_print_str`, the escapes), `src/save_format_json.c`
(`SAVE_JSON_SCHEMA_VERSION`, `KIND_*`, the writers' layout: `SaveJson_WriteCareer`,
`write_competition_buf`, `SaveJson_WriteHighlights`, `write_tactics_buf`),
`docs/SAVE_FORMAT_JSON.md`.

## Tests

- Unit tests: the escapes and raw bytes, the layout of each kind, the errors, and property
  tests (any value round-trips and re-writes identically).
- `tests/real_saves.rs`: every JSON save in `OPENSWOS_SAVES_DIR` (`just test_saves
  <folder>`) round-trips byte for byte. Saves are never committed: they hold the game's
  data.
