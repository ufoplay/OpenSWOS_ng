# openswos_team_json_resource

Reader and writer for OpenSWOS's team JSON, `teams/<COUNTRY>.json`: the rosters of world
packages (World 2026-27's download, and the World 1996-97 package made from your PC game).
It is the binary team file (`openswos_team_resource`) as JSON, every packed byte split into
named fields, with a `uuid` per team and player and a `checksum` per team.

## Layers

- **`model`**: the JSON, field for field, in upstream's order. Codes (position, face,
  flags, shirt types, kit colours, tactics, team control, the player-values state, the
  match flags) are `openswos_team_resource`'s enums and flags, and skills its `Skill`,
  all written as their numbers; a number outside a code's or a skill's range does not
  import. Other numbers are kept exactly as written.
  OpenSWOS Setup writes some fields signed (`-56`) and the World 2026-27 import writes them
  unsigned (`200`); both mean the same byte (`JsonByte`, `JsonWord`).
- **`text`**: upstream's layout, Python's `json.dump(indent=2)` with `ensure_ascii`. Upstream
  files round-trip byte for byte, trailing newline included (Setup writes none, the World
  2026-27 import writes one).
- **What this port writes:** the World 2026-27 package's style. Numbers are always written
  unsigned: nationalities, shirt numbers and prices are unsigned codes, and Setup's signs
  come from the C structs' `int8_t`. The world packages it makes (World 1996-97) end with a
  newline (`WORLD_PACKAGE_TRAILING_NEWLINE`). Setup's signed files are still read. Writing
  never fails: what the game does not read and JSON cannot keep is dropped, namely bytes
  after a name's end (the PC game's `SAFE.EDT` has some) and the passing byte's high
  nibble.
- **`convert`**: the model to and from the typed `Team`, checking the remaining ranges
  (cards and injuries, names). The JSON
  splits the cards-and-injuries byte as `cards` (bits 0-1), `injury` (bits 2-4) and
  `status_bucket` (bits 5-7). Those names are the JSON's own: the game code reads injuries
  from bits 5-7. The typed model keeps the byte whole.
- **`ids`**: **every team JSON states its ids**: a `uuid` and a `checksum` per team, a
  `uuid` per player. A file missing one does not import. Ids are data ("what a file states
  is what is used"), so a file's ids are read and written as they are.
  - **World 1996-97:** the PC game's binary `TEAM.nnn` files have no room for ids, so they
    are generated when the world's `teams/*.json` are made from those files.
    `EntryIds::derived(WORLD_1996_97_TAG, …)` gives `uuid5` of
    `"96-97/team/<file>/<ordinal>"` (and `…/player/<index>`): the very ids OpenSWOS gives
    the same teams, so saves and team references agree across both games.
  - **World 2026-27:** its package states ids made from the SWOS 96/97 Mod's own club
    ids, which no formula can rebuild. They are only read.
  - **The checksum** (SHA-1 of positions, skills and prices) is always derived from the
    team's data, and every file's stated one is checked against it.

## Player stats

The JSON writes each skill exactly as the record stores it: one 4-bit nibble, `0..=15`.
Nothing is scaled or masked.

| JSON | record byte | bits | the game's letter |
|---|---|---|---|
| `passing` | 28 | 0-3 (bits 4-7 are unused, see below) | P |
| `velocity` (shot power) | 29 | 4-7 | V |
| `heading` | 29 | 0-3 | H |
| `tackling` | 30 | 4-7 | T |
| `control` (ball control) | 30 | 0-3 | C |
| `speed` | 31 | 4-7 | S |
| `finishing` | 31 | 0-3 | F |

A skill value has two parts, which the typed model keeps as a `Skill { level, marked }`:

- **Bits 0-2: the level, `0..=7`.** The match engine reads only these bits (upstream
  `match_sim.c` masks the four skill bytes with `0x07777777`), and so do the letters of
  a player's three best skills that the menus show (upstream `play_match_menu.c`).
- **Bit 3 (+8): a marker.** The match ignores it. The transfer market's skill search lists
  only players whose searched skill has it (upstream `playerMarketPlayerMatchesFilters`),
  and the trialists the game generates get it on their position's skills
  (`_trialistSkillBitMasks`). In the PC game's files, most outfield players have it on two
  skills.

Passing has byte 28 to itself; its high nibble is unused, 0 in every original file and
never set by the game. Upstream's writer writes the whole byte (signed, as the C struct
declares it), but with that nibble 0 the number is the skill's, so this port reads and
writes `passing` as a nibble like the others. A typed team whose high nibble is set (the
binary reader keeps it) is written without it.

So `"speed": 0` is level 0, `"tackling": 7` is level 7 unmarked, `"tackling": 15` is level 7
marked (8 + 7), and `"control": 14` is level 6 marked. To change a level, edit bits 0-2;
to mark the skill, add 8.

The other playing fields:

- `position` (byte 26, bits 5-7): 0 GK, 1 RB, 2 LB, 3 CD, 4 RW, 5 LW, 6 MF, 7 ST.
- `face` (byte 26, bits 3-4): 0 white, 1 ginger, 2 black, 3 black with blond hair (World
  2026-27 only).
- `price` (byte 32): an index into the game's price table, not an amount: `0..=49` in the PC
  game's files.
- Goalkeepers: all seven skills are 0 in every PC team file (3,588 goalkeepers).

The rest of the record holds runtime state, 0 in every PC file: `flags` (byte 26, bits 0-2),
`cards`/`injury`/`status_bucket` (byte 27, see `convert` above), and `cached_match_value`,
`goal_stats` and `form`.

**Example:** PAUL OKON, Australia's centre-back in `assets/TEAM/TEAM.085`.

| record byte | hex | JSON |
|---|---|---|
| 26 | `60` | `"position": 3` (CD), `"face": 0`, `"flags": 0` |
| 28 | `0e` | `"passing": 14` |
| 29 | `d2` | `"velocity": 13`, `"heading": 2` |
| 30 | `3e` | `"tackling": 3`, `"control": 14` |
| 31 | `e1` | `"speed": 14`, `"finishing": 1` |
| 32 | `1d` | `"price": 29` |

The levels are P 6, V 5, H 2, T 3, C 6, S 6, F 1, with passing, velocity, control and
speed marked.

The team's `checksum` hashes every player's position, raw skill nibbles (bit 3 included)
and price. Changing any of them changes the checksum, which `team_checksum` recomputes.

## Sources

Upstream OpenSWOS: `tools/swosdata/lib/team_json.c` (the writer), `docs/TEAM_IDS.md`,
`docs/DATA_PIPELINE.md`.

## Tests

- Unit tests: signed and unsigned numbers, layout and escaping, upstream's documented id
  example (BRAZIL), conversions and their errors, round trips of the PC game's `TEAM.085`.
- `assets/TEAM_JSON/OCEANIA.json`: World 1996-97's team JSON for the PC game's `TEAM.085`,
  as this port writes it (derived 96/97 ids, unsigned numbers, a trailing newline). It
  round-trips byte for byte, and decodes to exactly the teams of `assets/TEAM/TEAM.085`.
- World 2026-27 (downloaded into `target/cache/`, D8): every `teams/*.json` of the package
  round-trips byte for byte, converts to typed teams and back, and states the checksums
  its data gives.
- `just test_real_assets <game_dir>`: every PC team file round-trips through this JSON,
  every `*.EDT` file exports to it with every name's text, and BRAZIL's checksum is
  upstream's documented one.
