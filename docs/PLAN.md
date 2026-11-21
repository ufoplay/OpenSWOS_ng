# OpenSWOS_ng — migration plan

A Rust port of [OpenSWOS](https://github.com/benbaker76/OpenSWOS) (C, GPL-3.0-or-later).
The first goal is a **view-independent game core**: the match engine, competitions and
career, plus readers and writers for every asset file. Rendering, audio and input come
later.

---

## 1. Decisions taken

| # | Decision |
|---|---|
| D1 | The game styles are ported: the Amiga's 50 Hz and the DOS game's 70 Hz, and the three SWOS 2020 styles upstream plays (XBOX and Amiga at 60 Hz, Moon). They are modelled as a **`GameStyle` trait with an implementation each**, not as constant tables, so new styles can be added later (§5.3). |
| D2 | A **small, curated set of original game files** may be committed under `assets/` for tests (§6). |
| D3 | The test-only `oracle` crate is the **only** crate allowed to contain `unsafe` (FFI to upstream C). Every other crate is `forbid(unsafe_code)`. |
| D4 | Saves are **compatible with OpenSWOS's JSON saves**: we read them, and we write them byte-identically, so upstream's golden careers become an oracle (§7.4). |
| D5 | Crate prefix `openswos_` (e.g. `openswos_team_resource`). |
| D6 | **Rules are data from day one.** The original game is just another world. There is one logic and no `if original … else …`. Anything the original hardcodes (e.g. Italy has 2 divisions) becomes data in the World 1996-97 package (§5.2). |
| D7 | **"World 1996-97" is a world package with the same file structure as World 2026-27** (OpenSWOS's world package). It is in effect a mod that reproduces the original 96/97 game exactly, with no hardcoded logic (§5.2). |
| D8 | Test data we can't commit (World 2026-27, the RGD recordings) is **downloaded on demand and cached under `target/`** (§6). |
| D9 | **Importers read only the DOS (PC) game's file formats**, plus OpenSWOS's own platform-neutral formats that D4/D7 need (JSON saves, team/competition JSON, world packages, `.SRP`). Amiga formats are not read: RNC-packed files, the `data/` folder of world packages (Amiga `TEAM.nnn`), OCS/AGA graphics, RJP1 music. The installed PC game's files are not RNC-packed (RNC only wraps the floppy release's installer archives), so there is no RNC codec. |

---

## 2. Scope

**In scope now**

- Readers and writers for every game file format (`src/resources/*`), including the
  ones only the view will use.
- **World packages**: OpenSWOS's world package structure, extended (in the same
  structure) with every rule upstream still hardcodes or bakes into C. World 1996-97 and
  World 2026-27 both load through one loader into one engine.
- `core`: the match simulation, competitions, career and the save mapping, all driven by
  the loaded world. It has no rendering, audio, input or file-system access.
- Rust tools for tracking the migration, extracting world data, fetching test data,
  testing against upstream, and linting.

**Out of scope for now**

- Mobile (Android/iOS/touch), the Nintendo Switch, WebAssembly.
- Amiga support: the m68k target, the emulated Amiga hardware (blitter, copper, Paula,
  CIA, interrupts, AGA), and Amiga-only data formats (OCS/AGA `.RAW`/`.MAP`, RJP1
  `.SNG`/`.INS`, ADF/HDF/LHA/WHDLoad). The Amiga *game style* is in scope (D1).
  Under D9, only DOS (PC) binary formats are read.
- Netplay/online/rollback, arcade mode, the Jaguar sprites.
- The mod *system* (hooks, cosmetic mods). Mods that change rules (e.g. Career+ knobs,
  the 36-club European formats) are covered because their rules are world data (D6).
- All view code: menus UI, sprite drawing, pitch scrolling, HUD, fonts, CRT/shaders,
  sound and music playback.

---

## 3. What upstream looks like (findings that shape this plan)

| Fact | Consequence |
|---|---|
| ~243k lines of C in `src/` (276k with headers). Pinned baseline: `ec2ed2be5c26e510f7c7b8d7af2710e5b909ca8b` (2026-11-10). 2300 commits, **~590 in the last 30 days**. | Upstream moves fast. Migration has to be tracked per file, against a pinned commit, with tooling (§8). |
| Rough sizes: match engine ≈31k lines, competitions ≈20k, career ≈29k, resource code ≈17k, worlds ≈15k (of which `world_data.c` ≈10k is baked data). Later: menus/UI ≈43k. Out of scope: mods ≈14k, netplay ≈18k. | About 100k lines of C go into `core` + `resources`. Several files mix logic and UI (`transfer_market.c`, `diy_competition.c`, `career_status.c`, `bench_subs.c`, `statistics.c`), so their logic has to be pulled out. |
| **Upstream has two paths:** 96/97 runs the original season-end code untouched, and World 2026-27 runs the ported Mod modules (`world_*.c` + the `swos-competition-engine` rules library) on top of it. There are 76 world-specific references (`World_Current()`/`WORLD_*`) in 15 files. | We merge them (D6). One generic engine, and two world packages that must each reproduce their upstream path exactly. The upstream branch points are the inventory of what must become data. |
| **The World 2026-27 package** (`OpenSWOS-World-2627.zip`, 2.9 MB, from `worldofswos.com`, the URL in upstream's Setup) holds `WORLD.ini` (tag, name, fingerprint), `teams/` (101 team JSON files), `data/` (101 `TEAM.nnn`), and `competitions/leagues.json` (91 leagues, the same schema as upstream's 96/97 `bin/data_json/competitions/leagues.json`). Newer builds of the script add `mod.json`. | This is the structure (D7). Its JSON schemas are upstream's existing ones, so the resource crates read upstream packages unchanged. |
| **The package is not the whole world.** Upstream applies it *over* 96/97's records, and the rest of World 2026-27 (96 cups, splits, play-off win lines, league levels, parallel groups, cup quotas/orders, the UEFA/continental specs) is baked into `world_data.c` by `world_import.py`. Upstream has no JSON for any of that. | We extend the package, in the same structure, with files for everything baked or hardcoded. Our packages are **self-contained**: no layering over 96/97 (§5.2). |
| `WORLD.ini`'s fingerprint = the first 16 hex digits of SHA-1 over `teams/`, `competitions/`, `data/` (each file's path + NUL + bytes, in path order). The pinned upstream expects `efe1464b6d1d62b8`. | The download cache is validated against the pin's fingerprint (D8). |
| World 2026-27 *data* (the SWOS 96/97 Mod's) is **not MIT** ("ask the maintainer"). `swos-competition-engine` (rules library ≈2.2k lines, contract tests) **is** MIT. | 2026-27 is never committed: we download and cache it (D8). The rules library is an oracle source; its contract tests are known answers. |
| The C keeps the original's memory layout: globals, BSS overlays, adjacent-global structs. | Not copied. The Rust state is explicit structs passed by reference. Upstream's own maps of that state guide the Rust structs: `src/sim_state.c` for the match, the save JSON schema for the career. |
| `src/sim_state.c` lists every variable the match reads (tier SIM). Crowd chants, advert boards and crowd animation **call `Rand()`**, and the camera/on-screen flags are **read by the match**. | This is the blueprint for `MatchState`. Those features' logic belongs in `core`, or bit-exactness is lost. |
| The match is deterministic. It runs one frame at a time from 2 player inputs (`match_step.h`). | Core API: `Match::step([PlayerInput; 2]) -> FrameEvents`. |
| Game styles differ in constants and behaviour: `ApplyGameStyle`, `docs/DOS_GAMEPLAY.md` §3 (ball physics, goalkeeper, clock thresholds, formations) and §8 (the 41/64 movement speed factor). | The inventory of `GameStyle` trait methods (§5.3). |
| `TEAM.nnn`, `*.TMD` and `POOLPLYR.DAT` are RNC-packed on the Amiga, in the OpenSWOS assets folder and in world packages, and raw in the PC (GOG) `DATA/`. The layouts are identical, and big-endian. | We read the PC files (D9): no RNC. World packages' rosters are read from `teams/*.json`, not from their Amiga `data/`. |
| `.RGD` recordings capture the original DOS game's state on every frame. They're published in [swos-port](https://github.com/zlatkok/swos-port) `tests/data` (≈6.5 MB each; that repo declares no license). Upstream replays 297,918 frames at 100% for scalars/teams/info. | The match engine's golden oracle (§7.4), downloaded on demand like World 2026-27 (D8). |
| Golden careers: upstream plays 5 seasons per career with the autopilot and hashes the JSON save each season. Its list covers **both** worlds (e.g. `BRA-FLAMENGO`, `ARG2627-BOCA`). | With D4 these become the end-to-end oracle for the career, for both world packages (§7.4). |
| Upstream already compiles single `.c` files with stubbed externs (`tests/league_playoffs/*`). | A C oracle per translation unit is known to work (§7.3). |
| `docs/GAMEPLAY.md` specifies the engine: units, pitch geometry, timers, structs, state enums. `SWOS-RE` has the disassembly notes. | These are the sources for constant names and doc comments. |

---

## 4. Workspace layout

Follows [infinitier](https://github.com/ufoplay/infinitier): edition 2024, resolver 3,
`[workspace.package]` and `[workspace.dependencies]` in the root, `just` for every step.

```
OpenSWOS_ng/
├── Cargo.toml              # workspace, [workspace.lints], shared deps
├── justfile                # imports just/*.just
├── just/
│   ├── build.just          # build, clean
│   ├── code_check.just     # fmt, clippy, check_code
│   ├── test.just           # test, test_oracle, test_oracle_long, test_real_assets, test_golden, coverage
│   ├── upstream.just       # upstream_fetch, upstream_status, upstream_diff, upstream_bump
│   └── worlds.just         # world_extract_9697, world_fetch_2627, world_validate, fetch_test_data
├── upstream.toml           # pinned upstream commit + per-file migration ledger (§8)
├── data/                   # runtime data: the files the game loads (committed, GPL) — §6
│   ├── worlds/1996-97/     # the World 1996-97 package, without rosters — §5.2
│   ├── tactics/            # the built-in tactics (upstream bin/data_json/tactics/)
│   ├── career_params.json  # the career params (upstream bin/data_json/career_params.json)
│   └── README.md           # provenance + license of every file
├── assets/                 # test-only fixtures, one folder per format (§6)
│   └── README.md           # provenance + license of every file
├── docs/
└── src/
    ├── common/             # openswos_common
    ├── rng/                # openswos_rng — the original's two generators (random.c)
    ├── math/               # openswos_math — 68k arithmetic, directions and steps (math68k.c)
    ├── game_style/         # openswos_game_style — trait GameStyle and the styles (dos_gameplay.c)
    ├── resources/<fmt>/    # openswos_<fmt>_resource — one crate per file format
    ├── core/               # openswos_core
    ├── test_utils/         # publish = false; includes the download cache (§6)
    ├── oracle/             # publish = false; test-only; compiles upstream C; the ONLY crate with unsafe (D3)
    └── tools/
        ├── upstream/       # migration ledger tool (§8)
        └── world_extract/  # builds/completes world packages from upstream's tables (§5.2)

target/cache/               # all temporary data lives under target/ (`cargo clean` removes it)
├── upstream/OpenSWOS/      # the upstream checkout at the pin (`just upstream_fetch`, §8)
├── downloads/              # D8: OpenSWOS-World-2627.zip, *.RGD.gz (as fetched)
└── worlds/
    ├── 1996-97/            # data/worlds/1996-97 + rosters from your game copy
    └── 2026-27/            # the download, verified, then completed (§5.2)
```

**Dependency rules**

- `common`, `rng` and `math` have no workspace dependencies; `game_style` depends on `math`.
- `resources/*` depend on `common` (a resource may also depend on another resource for a
  shared record such as `TeamFile`).
- `core` depends on `common`, `rng`, `math`, `game_style` and `resources/*`. `rng`, `math`
  and `game_style` follow `core`'s rules (no globals, no floats, no hash-ordered
  collections).
- `test_utils` appears only in `[dev-dependencies]` (and in `world_extract`). `oracle` is
  never shipped: the differential tests live in its own `tests/` and depend on the port
  (§7.3), and `world_extract` links it to read upstream's compiled tables (§5.2).

### `common`

- `bytes`: a bounds-checked big-/little-endian reader and writer. SWOS data mixes
  Amiga-origin BE records with DOS LE formats.
- `FixedName<const N: usize>`: a fixed-size name field that keeps the exact bytes (§5.6).
- Shared types. `TeamId` `{file, ordinal}`. Country, nationality and team-file numbers are
  numeric newtypes: their meaning is world data (D6), and World 2026-27 renumbers them.
  Only values the file format fixes are enums (e.g. `Position`, 3 bits).
- The `Importer`/`Exporter` traits (infinitier naming), and the base error types
  (`thiserror`).

### `core` (module map)

The RNG (`Rand`/`Rand2`, the save stack: random.c), the math (68k arithmetic, the sine
and angle tables, `CalculateDeltaXAndY`: math68k.c) and the game styles (trait `GameStyle`
and its implementations: `ApplyGameStyle`, dos_gameplay.c) are the crates `openswos_rng`,
`openswos_math` and `openswos_game_style`. The 16.16 fixed-point type joins `math` with its
first user, the sprites.

```
core/src/
  world/                the typed, validated World built from a world package: countries,
                        clubs, competitions, stages, outcomes, calendar, season-end steps,
                        access lists, coefficients, career rules, national teams, limits
  data/                 GameData = World + rosters + pool + tactics
  competition/          the generic engine: leagues/divisions, cups, tournaments, groups,
                        play-offs/outs, splits, league phase; fixtures, tie-breakers,
                        deciders, draws, pyramid moves, access lists
                        (competition_data.c, diy_competition.c, fixture_*.c, league_*.c,
                         uefa_*.c, world_*.c, the rules library)
  sim/                  results of matches nobody plays, predictions (match_sim.c, match_prediction.c)
  match_engine/         state (from the sim_state.c SIM tier), step, game state machine &
                        timers, ball physics, players, control/passing/shooting, AI,
                        goalkeeper, tackles/fouls/cards/injuries, referee, restarts,
                        bench/subs/tactics, camera & on-screen, crowd/advert RNG
                        consumers, statistics, highlights ring
  career/               setup, calendar & season loop, finances, transfers & pool, job
                        offers, board & chairman, records/trophies, autopilot (test driver)
                        — every parameter comes from the world
  save/                 core state <-> OpenSWOS save models (D4)
```

**API shape.** The core is a deterministic state machine. It takes commands and returns
events and never calls the view.

```rust
let data = GameData::new(world, rosters, pool, tactics)?;           // World 1996-97, 2026-27, …
let mut m = Match::new(&setup, &data, &Dos70Hz)?;                   // &dyn GameStyle
let events: FrameEvents = m.step([p1_input, p2_input]);

let mut career = Career::new(&career_setup, &data)?;
let outcome = career.apply(CareerCommand::PlaceBid { player, amount })?;
```

---

## 5. Design principles

### 5.1 Rust rules (from the `rust-code` skill, enforced by lints where possible)

- **No `unsafe` code.** `unsafe_code = "forbid"` in every crate except `oracle` (D3).
  There, `unsafe` is confined to its FFI module behind safe wrappers.
- `thiserror` errors. No `.unwrap()`; `.expect("why")` only when the case is truly
  impossible.
- `TryFrom` for every raw → typed conversion. Never a lossy `From`.
- **No `_` arm on enums.** Every variant is spelled out (`clippy::wildcard_enum_match_arm`).
- Match on slices instead of indexing. Destructure in impls. No `..Default::default()` in
  struct literals.
- Newtypes for ids and quantities (`TeamId`, `Money`, `Frames`, `GameUnits`…). Enums
  instead of bool parameters.
- Small files and focused modules. Unit tests sit next to the code; integration tests
  live in `tests/`.

Root `[workspace.lints]` (each crate opts in with `[lints] workspace = true`; `oracle`
overrides `unsafe_code` only):

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
[workspace.lints.clippy]
unwrap_used = "deny"
wildcard_enum_match_arm = "deny"
cast_possible_truncation = "deny"
cast_sign_loss = "deny"
cast_possible_wrap = "deny"
```

On top of those: `core` adds `#![deny(clippy::float_arithmetic)]` and
`clippy::disallowed_types` for `HashMap`/`HashSet`. `resources` and `common` add
`#![deny(clippy::indexing_slicing)]`, because they parse untrusted bytes.

### 5.2 Rules are data: world packages (D6, D7)

**One structure for every world.** It is OpenSWOS's world package, extended within its own
structure:

```
<world>/                          same layout for 1996-97, 2026-27 and any future world
  WORLD.ini                       tag, name, fingerprint                           (upstream)
  mod.json                        the world as a mod (optional: older packages lack it)  (upstream)
  teams/<COUNTRY>.json            rosters, upstream team JSON                      (upstream)
  data/TEAM.nnn                   the same rosters for the Amiga build: not read (D9)  (upstream)
  competitions/leagues.json       upstream competition JSON                        (upstream)
  competitions/cups.json, tournaments.json, playoffs.json, qualifying.json,
               league_blocks.json, seed_buckets.json, countries.json,
               country_competitions.json, career.json
                                  upstream's schemas: today only the 96/97 set uses them
  rules/…                         NEW: what upstream still hardcodes or bakes into C — season-end
                                  steps, splits, play-off win lines, league levels, parallel
                                  groups, continental access & coefficients, cup quotas/orders,
                                  national teams, career params, tweaks, limits
```

The principles:

- **Upstream's schemas are reused exactly.** The existing files keep upstream's JSON
  schemas, so an upstream package loads unchanged. We add new files only for what upstream
  has no file for. Their schemas follow upstream's JSON style and are documented in the
  crate readmes.
- **Packages are self-contained.** Upstream applies World 2026-27 over 96/97's records.
  Our packages carry the whole world, and the completion step (below) materialises that
  overlay. The engine loads exactly one world, never a stack.
- **One engine, no world checks.** `core` contains no world identifiers, no world checks,
  and no 96/97-specific branches. Every fact the original hardcodes becomes package data.
  Examples:
  - divisions per league and clubs per division;
  - promotion/relegation counts and play-offs;
  - which countries take part in the season end;
  - European cup slots (16/32/32 + Copa), qualification and access;
  - calendars (⅛-month positions);
  - national teams and nationalities;
  - pool and list capacities;
  - career economy;
  - OpenSWOS's tweaks;
  - the original's limits (93 clubs per file, 4 divisions…), as `rules/limits`.
- **Behaviour components, chosen by data.** Sometimes the original and the Mod do the same
  job with different algorithms. Examples: tie-breaker order, fixtures for odd-sized
  leagues, draws that use the game's `Rand2` versus the Mod's table-seeded LCG. In those
  cases the engine has one implementation **per behaviour**, behind a trait or an enum,
  and the package names the one it uses. Components are named after what they do, never
  after a world. A new behaviour adds a component, never a branch.

**World 1996-97: a mod that reproduces the original.**

- Committed in `data/worlds/1996-97/`, without `teams/` and `data/` (the rosters are the
  original game's).
- `competitions/*.json`: upstream's committed 96/97 files (GPL), copied as they are.
- `rules/`, extracted part: `src/tools/world_extract` links the oracle and reads the 96/97
  tables compiled into upstream (national teams, career params, continent lists, seed
  lists…). Reading them from the compiled C avoids transcription errors.
- `rules/`, authored part: rules the original encodes in code (e.g. how the season end
  moves clubs, the European qualification order) are written by hand, each citing the
  upstream function.
- `world_extract` also assembles the full package in `target/cache/worlds/1996-97/`.
  - It adds `teams/*.json` made from your PC game copy's `DATA/TEAM.nnn`, as OpenSWOS
    Setup does, written in the World 2026-27 package's style: unsigned numbers, a trailing
    newline. Setup's signed files are still read.
  - Team ids are mandatory in every package: the binary files have none, so they are
    generated here. Every team and player gets `uuid5("96-97/team/<file>/<ordinal>[/player/
    <index>]")` (upstream `docs/TEAM_IDS.md`, the same ids OpenSWOS gives them), and every
    team its checksum.
  - It computes `WORLD.ini`'s fingerprint.
- Proof: the Rust engine + this package reproduces upstream's 96/97 path (an oracle test
  per function, golden careers end to end).

**World 2026-27: downloaded, verified, completed.**

- `just world_fetch_2627` downloads `OpenSWOS-World-2627.zip` into
  `target/cache/downloads/`.
- It checks `WORLD.ini`'s fingerprint against the pinned upstream's
  `World_DataFingerprint`. A mismatch fails with "package and upstream pin out of step".
- `world_extract` completes it into `target/cache/worlds/2026-27/`:
  - the 96/97 records the package overrides get merged in;
  - the tables upstream bakes into `world_data.c` go into the same `competitions/` and
    `rules/` files World 1996-97 uses.
- Never committed (the data is not ours to redistribute). A test re-runs the completion at
  every upstream pin and diffs.
- Proof: the Rust engine + this package reproduces upstream's 2627 path (the rules library,
  `world_*.c`, and the 2627 golden careers).

### 5.3 Variation points are traits (D1)

- `trait GameStyle` holds every constant and behaviour that differs between the styles.
  Examples: ball physics, goalkeeper timings, match-clock thresholds, the per-frame
  movement speed factor, formation selection. The inventory comes from `ApplyGameStyle`
  and `docs/DOS_GAMEPLAY.md`; the match engine's units add the behaviour methods they need
  as they are ported (§10).
- The trait is object-safe. The match holds a `&'static dyn GameStyle`. The styles
  (`Amiga50Hz`, `Dos70Hz`, `Xbox60Hz`, `Amiga60Hz`, `Moon50Hz`) are stateless unit structs
  in `openswos_game_style`. A registry maps a stable string id (stored in settings) and
  upstream's number (carried by recordings) to an implementation, so a new style means a
  new impl plus one registry line.
- Each method has a doc comment on the trait (meaning, unit). Each impl returns
  module-level documented constants with their provenance, so §5.4 still holds.
- Both styles are oracle-tested: the C side runs with the corresponding upstream game
  style set.
- The same pattern applies to other variation points (§5.2's behaviour components).

### 5.4 No magic numbers

- Every literal from the C becomes a `const` with a doc comment. The comment gives the
  meaning, the unit (game units, frames, ⅛-months, money units), and where it comes from
  (upstream file + function, and the `SWOS2.s` line when upstream cites one).
- Values that differ per game style live in the `GameStyle` impls (§5.3). Values that
  differ per world live in world packages (§5.2). Neither is a `const` in engine code.
- Data tables (random table, sine table…) are documented `const`/`static` arrays.
- Checked during code review (no automated lint): a numeric literal other than `0`/`1`
  outside a documented `const`/`static` item, or a `const` without a doc comment, is a
  review finding.

```rust
/// X of the left touchline, in game units (≈ original pixels).
/// Upstream: docs/GAMEPLAY.md §1.1; ball-out test in update_players.c.
pub const LEFT_TOUCHLINE_X: GameUnits = GameUnits(81);
```

### 5.5 Idiomatic port, bit-exact behaviour

- C ints that encode states become `#[repr(u8)]` enums with `TryFrom<u8>`. Bit sets
  become `bitflags`. Packed nibbles become separate typed fields.
- No global state, no `static mut`, no `thread_local` in `core`. Every function gets its
  state explicitly. The RNG is part of the state.
- Integer widths mirror the C. Values wrap only where the original wraps, and only through
  explicit `wrapping_*` calls or a `Word` newtype. Any other overflow is a bug: debug
  builds panic on it, and the oracle catches the difference.
- 68k semantics (DIVS truncation, ASR, sign extension, SWAP…) live in `openswos_math`,
  with tests.
- RNG call order is behaviour, including the crowd, advert and animation consumers (§3).
- **Quirks are preserved.** Original bugs stay, commented `Faithful to the original:` with
  a reference. If a quirk depends on the world, it becomes a named behaviour component or
  rule (§5.2), never a world check.

### 5.6 Lossless resources

Every reader/writer pair round-trips **byte-identically** on every original file, and on
every upstream JSON file. In practice:

- Undefined or unknown fields are kept raw (`field_1`, `some_flag`). C-shaped JSON fields
  (`name_table_offset`, `team_list_offset`…) are kept as they are, and `core::world`
  resolves them into typed references when it validates the world.
- Names keep their exact bytes. A name that fills its field has no NUL, and trailing
  spaces are data.
- A value outside the engine's range is kept with a typed accessor. Example: skill nibbles
  0..=15, which the engine masks to 0..=7 — `RawSkill(u8)` with
  `fn effective(self) -> Skill`.

---

## 6. Assets and test-data policy (D2, D8)

- **Runtime data, committed (`data/`).** The files the game loads, kept apart from the
  test fixtures:
  - upstream's GPL data: World 1996-97's competitions (`data/worlds/1996-97/`), the
    built-in tactics and the career params, which upstream loads for every world;
  - our own: World 1996-97's `rules/` (M4 #4–#5).

  Never original game data. `data/README.md` records where every file comes from and its
  license. Tests read these files where they are, never a copy in `assets/`.
- **Test fixtures, committed (`assets/<FORMAT>/`).** Only the tests read them. Each fixture
  is a file plus a JSON sidecar (`name.ext` + `name.json`, the expected decoded model, as
  in infinitier):
  - a **small, curated set of original files** (e.g. two or three PC `DATA/TEAM.nnn`, one `.TMD`,
    `POOLPLYR.DAT`, one sprite bank, one pitch, a few samples), kept small;
  - synthetic fixtures for edge cases.

  `assets/README.md` records where every file comes from and its license.
- **Downloaded on demand, cached in `target/cache/` (D8):**
  - `OpenSWOS-World-2627.zip` (verified by fingerprint);
  - the swos-port `.RGD.gz` corpus (pinned at swos-port commit `4da94f5`, verified by
    the committed checksums, `test_utils::cache::RGD_CORPUS`).

  `test_utils::cache` downloads a missing item on first use (a Rust HTTP client, no shell
  tools). `just fetch_test_data` pre-fetches everything. If the network is off and the
  cache is empty, the dependent tests are skipped, and the run says which ones and why.
  `cargo clean` empties the cache.
- **Your own game copy, outside the repository.** Its folder is passed as a command-line
  argument or an environment variable (`OPENSWOS_GAME_DIR` for the GOG install,
  `OPENSWOS_ASSETS_DIR` for an OpenSWOS Setup output folder). It is needed for World
  1996-97's rosters and for `just test_real_assets`. Anything derived from it goes to
  `target/cache/`. Nothing local lives in the repository tree outside `target/`.
- **Ground truth for a format** is (a) a byte-identical round-trip of every original and
  upstream file, and (b) where it exists, a field-by-field match against upstream's own
  export (e.g. the `teams/*.json` that OpenSWOS Setup writes from the same copy, compared
  with our decoding of the matching `TEAM.nnn`).

---

## 7. Testing strategy

### 7.1 Unit tests, for every function

Every function has tests for its normal cases and its edge cases. `just coverage`
(`cargo llvm-cov`) tracks line coverage of `core`/`resources`. `cargo mutants` on `core`
is optional (`just mutants`).

### 7.2 Resources

- Decode each fixture and compare it with its JSON sidecar.
- Re-encode each fixture and compare the bytes.
- proptest: `model → bytes → model` is the identity, and `import(arbitrary bytes)` never
  panics (it returns `Err`).
- `just test_real_assets`: every original file decodes and re-encodes byte-identically.
- Every file of the downloaded World 2026-27 package round-trips byte-identically, and its
  fingerprint recomputes.

### 7.3 Differential proptest against upstream (the oracle — the only `unsafe`)

`src/oracle` compiles **unchanged upstream C** from the pinned checkout (including the
rules library submodule). The oracle contains no handwritten C.

- `build.rs`:
  - finds the checkout (`OPENSWOS_UPSTREAM_DIR`, default `target/cache/upstream/OpenSWOS`, created by
    `just upstream_fetch`);
  - compiles the `.c` units listed in `oracle/units.toml` with `cc`, as upstream's SDL build
    does (its Makefile's flags and defaults, `-include include/swos_platform.h` for
    `SWOS_SDL_BUILD`, LP64), plus upstream's data files, which the linker pulls in as the
    units need them;
  - generates FFI types with `bindgen` (every function `C-unwind`). bindgen needs libclang
    on dev machines and CI;
  - binds the headers listed in `units.toml` without compiling them: their `static inline`
    functions are reached through the wrappers bindgen generates, compiled with the units;
  - declares every global a unit lists (by name) as the global it is: bindgen would make a
    global the unit defines with an evaluable value a Rust `const`. Enumerators and macros
    are listed apart, by pattern;
  - without the checkout, builds the crate unavailable (`cfg(oracle_unavailable)`): its API
    and tests are compiled out and a test reports the skip (a failure in strict mode), so
    the main CI job builds on every OS without C.
- Every extern a compiled unit references but we do not compile is defined in Rust
  (`#[unsafe(no_mangle)]` statics/functions in `oracle/src/ffi/stubs.rs`). `unsafe` stays inside
  the crate's FFI module, and the crate exposes only safe functions.
- C globals are process-wide. Each oracle test takes a global `Mutex` guard, which resets
  the touched globals.
- Upstream's halt (`PanicHaltLoop`) is a Rust panic unwinding through the C (compiled with
  `-fexceptions`); the safe wrapper returns its reason code. A wrapper never makes a call
  upstream would make undefined: it returns an error instead.
- Each ported function gets `src/oracle/tests/<unit>.rs` (the oracle dev-depends on the
  port: the unavailable cfg is visible to the oracle's own targets only):
  1. generate the state and arguments;
  2. write them into the C globals;
  3. call the C function and read back the resulting state;
  4. run the Rust function, given the **world package** whose upstream path it mirrors
     (1996-97 for the original code, 2026-27 for the rules library / `world_*.c`) and the
     **game style** set on both sides;
  5. convert the C snapshot into Rust types with `TryFrom`;
  6. `assert_eq!`.
- `just test_oracle` runs the default 256 cases per property;
  `just test_oracle_long` runs `PROPTEST_CASES=20000`.
- **Level 2 (match milestone):** a whole-match differential. It feeds random input
  sequences to both engines and compares the SIM state every frame, for both game styles.
  It may need SDL2 headers; that gets decided when we reach it.

### 7.4 Golden traces and known answers

- **Match:** an `rgd` resource crate, plus a replay test that drives the Rust engine
  (`Dos70Hz`) with the recorded inputs and the pinned random value, as upstream's
  `replay_test.c` does. The target is upstream's level: 100% at scalars/teams/info.
- **Career, end to end (D4):** our autopilot plays each golden career for 5 seasons and
  writes OpenSWOS-format JSON saves. Their SHA-1s must equal `tests/career_golden.txt` for
  the World 1996-97 careers and the World 2026-27 careers. This is the strongest proof of
  D6/D7: one engine, two packages, both bit-exact.
- **Competitions:** upstream's known-answer cases (`tests/league_playoffs/*_cases.h`) and
  swos-competition-engine's contract tests and rule tests, turned into Rust test tables
  with their provenance noted.
- **Generic invariants (proptest over generated valid world packages).** Examples: every
  pair in a round robin meets `legs` times; pyramid moves keep division sizes; every
  knockout has exactly one winner; package → load → write is the identity; save → load →
  save is the identity.

---

## 8. Tracking upstream

**`upstream.toml`** is the single source of truth. It is written by the tool, one line per
file:

```toml
[upstream]
repository = "https://github.com/benbaker76/OpenSWOS"
pinned_commit = "ec2ed2be5c26e510f7c7b8d7af2710e5b909ca8b"   # 2026-11-10
tracked = ["src/**", "include/**", "bin/data_json/**", "tests/**",
           "third_party/swos-competition-engine/rules/**", "third_party/swos-competition-engine/tests/**"]
checkout = ["src", "include", "bin/data_json", "tests", "docs", "third_party/swos-competition-engine"]
# M4 adds: world_2627 = { url = "https://worldofswos.com/static/openswos/OpenSWOS-World-2627.zip", fingerprint = "efe1464b6d1d62b8" }

[[submodule]]
path = "third_party/swos-competition-engine"
commit = "87b65278a15007223ec4f387f1027eb4d5cd7625"

[files]
"src/netplay.c" = { area = "out_of_scope", status = "not_ported", notes = "netplay/online (out of scope)" }
"src/random.c" = { area = "core", status = "done", migrated_at = "ec2ed2be…", rust = ["src/rng/src/rng.rs"] }
```

- `area`: `untriaged | core | resource | world_data | test_data | view | out_of_scope`.
- `status`: `todo | partial | done | not_ported`.
- Optional fields: `migrated_at`, `rust` (our files holding the port), `functions_done` (for
  big files that are `partial`), `notes`.

Every tracked upstream file has an entry (807 at the baseline), so nothing is dropped
silently. `world_data` marks files whose content goes into world packages (via
`world_extract`) rather than into code (`competition_data.c`, `world_data.c`,
`career_params.c`, `bin/data_json/competitions/*`…). `test_data` marks upstream test data
the port reuses (golden careers, known-answer cases). Files under a tracked submodule are
migrated against that submodule's commits.

**`src/tools/upstream`** (Rust; it uses the `git` CLI):

| Command | What it does |
|---|---|
| `fetch` | Clone (blobless) or update the checkout in `target/cache/upstream/OpenSWOS`, sparse-checked-out to `checkout`, at the pinned commit, with the ledger's submodules. |
| `sync` | Add the tracked upstream files the ledger lacks as `untriaged`. Drop entries of removed files that hold no ported work, and keep (and report) the others. |
| `mark <globs>… [--area] [--status] [--migrated-at] [--rust]… [--function]… [--note]` | Update the matching entries. A pattern matching nothing is an error. `partial`/`done` without `--migrated-at` record the pin. |
| `status` | Counts per area × status, migrated files upstream changed since their `migrated_at`, and "fully migrated at X" once every in-scope file is `done` at the pin or `not_ported`. |
| `check` | CI gate. The ledger must cover exactly the tracked upstream files. No entry may be untriaged. Out-of-scope entries must be `not_ported`. `partial`/`done` entries need `migrated_at` (an ancestor of the pin) and existing `rust` paths. Submodule commits must match upstream's. |
| `diff <path>` (later) | The upstream diff for a file since its `migrated_at`. |
| `bump --to <sha>` (later) | Move the pin and list every in-scope file that changed. Also update the expected World 2026-27 fingerprint. |

**Integration loop:**

1. `upstream_bump` → `upstream_status`.
2. Port the changed files. Run `world_extract` again if a `world_data` file changed (this
   regenerates `data/worlds/1996-97/` and the cached 2026-27 completion).
3. The oracle tests and golden careers pass against the new pin.
4. `mark` the files.

When C behaviour changes, the oracle tests fail, so integration also gets checked by tests.

---

## 9. Milestones

Each milestone ends with `just check_code` and `just test` passing in CI. Units inside a
milestone are done **one at a time**, each finished (§10) before the next starts.

### M0 — Bootstrap ✅ (2026-11-12)

Done. 807 upstream files classified: 192 core, 19 resource, 44 world_data, 14 test_data,
207 view, 331 out of scope. Also delivered: the ledger tool's
`sync`/`mark`. Note: `src/resources/*` joins the workspace members with its first crate (cargo refuses a glob that matches nothing).

- Cargo workspace, `justfile` + `just/*`, workspace lints, `.gitignore` (nothing beyond
  `target/`: every temporary or local file lives there).
- GitHub Actions: fmt, clippy `-D warnings`, test. A separate oracle job installs libclang
  and fetches upstream. CI caches `target/cache/` between runs.
- `CLAUDE.md` with these conventions.
- Skeletons of `common`, `core`, `test_utils` (incl. the download cache), and
  `assets/README.md`.
- `upstream.toml` with every upstream file classified, and the `upstream` tool with
  `fetch`/`status`/`check`.
- `license = "GPL-3.0-or-later"` in the workspace metadata.

### M1 — Resource foundation ✅ (2026-11-12)

Done.

- `common`:
  - `ByteReader`/`ByteWriter`: bounds-checked BE/LE reads and writes. A failed read moves
    nothing.
  - `Importer`/`Exporter`.
  - `FixedName<N>`: lossless. It serialises as text when canonical, as raw bytes otherwise.
  - `TeamId` (`TeamFileNumber` + `TeamOrdinal`) and `Position`.

  Country and nationality newtypes come with the team crate that first needs them.
- `test_utils`:
  - `fixtures`: listing, JSON sidecars named `<fixture>.json`, `OPENSWOS_BLESS=1` to
    (re)write them, byte-for-byte `assert_round_trip`.
  - `data`: `OPENSWOS_GAME_DIR` / `OPENSWOS_ASSETS_DIR`, case-insensitive game files.
  - `cache`: downloads into `target/cache/downloads/`, SHA-256-verified, atomic, capped at
    64 MiB. A checksum mismatch fails the test; no network means a skip.
  - `skip` + `require!`: skips report why; `OPENSWOS_STRICT_TEST_DATA=1` turns them into
    failures.
- `just test_real_assets <game_dir>` passes your game folder to the tests.

### M2 — Core-data resources, in this order

All binary formats are the PC (DOS) game's (D9).

| # | Crate | File(s) | Notes |
|---|---|---|---|
| 1 ✅ | `team` | PC `DATA/TEAM.nnn`, `DATA/TEAM.CUS` | BE count + 684-byte `TeamFileEx` (76-byte header + 16 × 38-byte player). Not RNC-packed. Every team database of the PC game round-trips (79 files, 1,794 teams); `DATA/TEAM.248` is a stray file in another layout, rejected. |
| 2 ✅ | `team_json` | `teams/<COUNTRY>.json` | Upstream's team JSON (uuids, checksums): the rosters of world packages. Two dialects, both round-tripped byte for byte: Setup's (signed numbers, no trailing newline) and World 2026-27's (unsigned, trailing newline). Ids are data (World 2026-27's come from the Mod's club ids, not derivable); derived 96/97 ids and checksums match upstream's documented values. All 101 World 2026-27 rosters (2,385 teams) and every PC team file convert both ways. |
| 3 ✅ | `tmd` | PC `DATA/EUROCUP/EUROCWC/UEFACUP.TMD` | Bare `TeamFileEx[]` (16/32/32). All three round-trip; every pool team is the team its id names in `TEAM.nnn`. |
| 4 ✅ | `poolplyr` | PC `DATA/POOLPLYR.DAT` | 900 58-byte `PoolPlayer` records. Round-trips; every record is a copy of a squad player: the player at its ordinal in the team its id names. |
| 5 ✅ | (`team`) | `CUSTOMS.EDT`, `SAFE.EDT` | No crate of its own: the layout of `TEAM.nnn` (BE count + 684-byte `TeamFileEx[]`, 48 teams), read by the `team` crate. Both PC files round-trip. Faithful to the original: they store the global team numbers little-endian, which the game recomputes on load. |
| 6 ✅ | `tac` | `.TAC` | The 370-byte `TeamTactics` record: name, 10 outfield players × 35 ball areas (7 × 5) of packed x/y cells, pair ids, the ball-out-of-play slot. Bytes after the record (which the game ignores) are kept: SWOS 2020's CPU tactics end with six zeros. Fixtures from SWOS 2020 (the PC game ships no `.TAC`). OpenSWOS's JSON `.TAC` (`write_tactics_buf`) is read as the same record: #13c. |
| 7 ✅ | `competitions_json` | `competitions/*.json` | Upstream's ten files (leagues, cups, tournaments, playoffs, qualifying, league blocks, career, country competitions, countries, seed buckets) as one model picked by `type`, field for field. All ten round-trip byte for byte (committed, GPL), and World 2026-27's `leagues.json` (download cache). Bytes kept as written; packed bytes split as upstream splits them; competition types, continents and list markers are enums; fields whose meaning the engine gives stay numbers (M5). The JSON helpers moved to `openswos_common::json`. |
| 8 | `world_rules` | `rules/*.json` | Our new files (§5.2). Schemas are defined as M4/M5/M7 find what upstream hardcodes; the crate grows with them. |
| 9 ✅ | `world_package` | `WORLD.ini`, `mod.json`, the layout | Opens a package from a folder or a zip (paths kept inside, 64 MiB cap), checks its fingerprint (upstream's algorithm: World 2026-27's files give `efe1464b6d1d62b8`, the pin), writes one into an empty folder stating the fingerprint of what it writes. `data/` (Amiga, D9) is fingerprinted but not read; other files are listed as ignored. `rules/` joins with #8. |
| 10 ✅ | `tactics_json`, `career_params_json`, `national_teams_json` | upstream `bin/data_json/…` | Inputs to `world_extract`. **Tactics:** the 12 presets and their Amiga/DOS releases as `TeamTactics` (the `tac` crate's record); all 20 upstream files round-trip byte for byte (committed, GPL), and SWOS 2020's CPU 5-3-2 is the built-in one. **Career params:** 53 knobs typed by their C width, notes and order kept; upstream's hand-written file round-trips byte for byte, and the knobs are exactly the keys its loader reads. **National teams:** the 6-byte records named field by field, the manager's nationalities, the job-offer pick, with the loader's checks; OpenSWOS ships none, so the original's compiled tables (read from upstream's source) are the real-data test. `Nationality` moved to `openswos_common`. |
| 11 ✅ | `career_save`, `competition_save` | `.CAR`/`.SEA`, `.PRE`/`.DIY` (JSON) + DOS `.DIY` import | Byte-identical writer (D4), including the raw competition blobs in the x86_64 layout upstream's golden record uses. Both worlds. |
| 11a ✅ | `save_json` | the saves' JSON dialect | The base of #11: what upstream's save writer (mjson) writes. Strings are bytes (only `\b \f \n \r \t \\ \"` escaped; other bytes as they are, so a save need not be UTF-8), numbers are integers, one top-level member a line, values compact, a career's first four members `"key": value`. A save is its members in order: every save of every kind round-trips byte for byte. |
| 11b ✅ | `career_save` | `.CAR`: the save, its teams and players | Every top-level member a field, in the writer's order (`SaveJson_WriteCareer`), the conditional ones options, the layout the writer's (inline members included). Typed: the header, season informations, the teams (career, current, selected, parked, transfer market) as the team crate's `Team`, the reserves and market players as its `Player`. The rest kept as written. A real World 2026-27 career round-trips byte for byte. |
| 11c ✅ | `career_save` | `.CAR`: the offers | The transfer offers and the two job-offer queues, typed in their C widths. |
| 11d ✅ | `career_save` | `.CAR`: the world sections | The team and state pools, the world's cups (national, league, Serie D, the other confederations'), its league contests (play-offs, stage titles, season winners, stage tables, promedio), the UEFA coefficients and qualifying rounds: field by field in their C widths, flattened records as fixed-size records. Only the pools and two cups are in the real save; the rest is checked against the writer's code. |
| 11e ✅ | `career_save` | `.CAR`: copies, caches, overrides | The league tables by team identity (the primary's and each slot's, a world league's split), the competition copies' dynamic bytes and team lists, the key cache's strides and scores. |
| 11f ✅ | `career_save` | `.CAR`: the competition slots | The five slots and the tournaments' stages: a header and, by the slot's type, a league's progress, a cup's bracket and scores, or a tournament's stage slots; a slot must write back exactly as read. The real save has league and cup slots; the tournament path is checked against the writer's code. |
| 11g ✅ | `career_save` | `.CAR`: `career_scalars` | Upstream's scalar table, 249 fields in its order and types (generated from `s_career_scalars`); the five competition slots' raw state kept as the x86_64-layout bytes it is written as; the user's tactics as `TeamTactics`. Both keys of the transfer list (raw bytes before 2026-11-05, words since) read. The career save is now typed throughout but the mods' sections. |
| 11h ✅ | `competition_save` | `.SEA`/`.PRE`/`.DIY` (JSON) | The competition saves typed in the writer's order (`write_competition_buf`): the 35 scalars tagged for them, the primary slot, its overrides, a tournament's stage, the teams; the sections are `career_save`'s. Guarded against upstream's table; no real save available yet. |
| 11i ✅ | `competition_save` | DOS `.DIY` | The DOS game's DIY competition file (PC, SWOS 2020) read and written byte for byte: the header typed, the type-dependent body and the tail as bytes, the user's tactics and the teams as the `tac` and `team` crates' records. SWOS 2020's two cups round-trip (committed). Turning one into our JSON `.DIY` goes through the competition's state (M5/M7), as upstream's does. |
| 12 ✅ | `rgd` | swos-port `.RGD(.gz)` | Versions 1.0, 1.1, 1.3–1.5 (1.2's frames are unknown, upstream's too). The header typed (teams and tactics as the `team` and `tac` crates' records), frames streamed as their bytes with their scalars typed, every other field read by name through upstream's schema (`rgd_schema.py`: type, offset, level, comparison). The 15-recording corpus is downloaded (pinned commit, checksums): headers round-trip, every frame holds upstream's decode invariants, six end as upstream's table says. |
| 13 ✅ | `hil`, `srp`, `rpl` | highlights, OpenSWOS replay v2, DOS replay | `.HIL` binary + JSON. `.SRP` is chunked (`META`/`FRMS`/`INPT`/`HASH`…). In steps: |
| 13a ✅ | `hil` | DOS `.HIL` + `.RPL` | The DOS game's replays (PC, SWOS 2020), one format: the header (the teams as typed `TeamGame` records), scenes of frames (camera, score, animation state, sprites), the slots' leftover kept. `.RPL` lives here (the same format, one scene). SWOS 2020's 40 highlights and its 2.35 MB replay round-trip; a one-goal highlight and a 100-frame cut of the replay are committed. OpenSWOS's big-endian `.HIL` is the Amiga's layout (D9: not read). |
| 13b ✅ | `srp` | OpenSWOS `.SRP` | Replay format v2: the container (header, directory, CRC; a deflated chunk keeps its bytes while unchanged, since miniz's output is not reproducible) and typed views of the match, frames, sounds, seek table, chapters and players; the netplay chunks kept as their content. A real recording (14,915 frames) round-trips and every view encodes back to its chunk. |
| 13c ✅ | `hil`, `tac` | JSON `.HIL`, JSON `.TAC` | OpenSWOS's JSON highlights (`SaveJson_WriteHighlights`), typed in the writer's order: the header's values, the two teams as `TeamGame` records (the `.SRP`'s; kit words unsigned, players' bytes signed, names up to their NUL), a scene a goal (frames as the writer decodes the slot: camera, score, animation state, sprites; the made-up frame for leading sprites; where the end word leaves off and the slot's bytes after it). No real file exists yet: texts printed as upstream's writer prints them round-trip byte for byte, and `just test_saves` round-trips every `.HIL` of a saves folder, JSON or DOS. With them, OpenSWOS's JSON tactics (`write_tactics_buf`), which #11 left out: read as the `tac` crate's `TeamTactics`; the SWOS 2020 fixtures printed as the writer prints them read back as their records. |
| 14 ✅ | `settings_ini` | OpenSWOS's `SWOS.ini` | The settings file (`swos_settings.c`), in the INI dialect of inih (`ini.c`, BSD-3-Clause: rules ported from it keep its notice). Typed where the game reads it: `[options]` (game length, pitch type, auto replays, all teams equal, auto-save highlights), `[gameplay]` (the style: `GameStyle`, D1), `[tweaks]` (rules, D6), `[general]` (the world, the language), `[replays]`; the view's sections (`[video]`, `[audio]`, `[input]`…) typed or kept as read. Upstream's writer is non-destructive (it changes only the values that changed and keeps comments, blank lines, unknown keys and their order), so a file round-trips byte for byte. Tested on synthetic files and, like the saves, on yours: `SWOS.ini` sits beside the saves folder (`OPENSWOS_SAVES_DIR/..`), never committed. In steps: |
| 14a ✅ | `settings_ini` | the file | The file kept whole (its bytes: the game's reader and its writer read lines by different rules). What the game reads: inih's rules ported (comments, inline comments after whitespace, `=`/`:`, continuation lines, the byte order mark, its 199-byte reads and 49-byte names, errors read past). How it saves (`Settings_Save`), from a table of settings (section, key, comment, value, kept by the player or not): the writer's own line rules (256-byte reads, 400 lines, `=` only), a file left alone when nothing differs, changed values rewritten after their `key=`, second occurrences dropped, missing keys at the end of their section's first block, missing sections after a comment; the fresh file (`settings_write_full`). Your `SWOS.ini` round-trips, reads without an error and saves unchanged with its own values. |
| 14b ✅ | `settings_ini` | the settings | The game's sections typed (`[gameplay]`, `[replays]`, `[testing]`, `[general]`, `[mods]`, `[tweaks]`, `[options]`), their defaults `main.c`'s: the handler's reading (case-insensitive but the tweaks' keys, values the game ignores left in the file, texts cut to its buffers), upstream's table for them (`settings_build`'s 36 rows in order, comments verbatim, guarded against the checkout) and `Settings_Save`'s baseline. The view's, netplay's and the arcade's sections stay in the file as read. Upstream's self-tests pass; your `SWOS.ini` (an older build's: 22 of the keys missing) saves as upstream would, every line kept and the missing keys added. |

### M4 — Oracle, foundations, world packages v0

| # | Unit | Notes |
|---|---|---|
| 1 ✅ | `oracle` | The crate (build.rs: `units.toml`, cc with upstream's SDL flags, bindgen; Rust stubs; the global lock; halts unwinding back through the C), proven on `src/random.c`: its two generators, the save stack and its halt, with upstream's data files linked as needed. Unavailable without the checkout, so CI's main job builds on every OS; a CI job of its own runs `just test_oracle`. |
| 2 | `rng`, `math`, `game_style` | The foundations, with their oracle proptests. In steps: |
| 2a ✅ | `rng` | The crate `openswos_rng` (random.c, and `Randomize2` from game_utils.c): both generators, the second one's save stack (the save that fills it halts the original: an error), a recording's fixed value. Checked against the C by random sequences of draws, reseeds, saves and restores. |
| 2b ✅ | `math` | The crate `openswos_math` (math68k.c/h): `CalculateDeltaXAndY`, its movement scale a `DeltaScale` parameter (41/64 DOS, done with shifts; none Amiga) instead of two globals; the 68k shifts. Checked against the C on every angle in every quadrant, on any two points at any speed and scale, and on upstream's known answers. The oracle reaches a header's `static inline` functions through bindgen's wrappers. |
| 2c ✅ | `game_style` | The crate `openswos_game_style` (dos_gameplay.c): the trait, its inventory taken from `ApplyGameStyle` (the frame rate, the `DeltaScale`, the releases whose code runs, the goalkeepers' constants, the ball's physics, the clock and stoppage timers), the five styles upstream plays and the registry (by the settings' id and by upstream's number). Checked against the C for every style, with and without the stronger keepers tweak. |
| 3 ✅ | `core::world` | The typed, validated world built from a package: its identity, countries (by number and `CN_*` name, the list in its order) and competitions with the tables that tie them (qualifying groups, league blocks, career index, competition table slots, UEFA Cup berths), each record as read, its references typed handles. Checked whole, every problem reported naming its file and field: upstream's ten files at their paths, one definition per id, every reference of the right kind, every country the world's, each record of a type its file holds. `core::data::GameData`: the world, its rosters (by identity and id, the canonical check of src/team_db.c), the pool and the built-in tactics, checked together (every country's roster, no identity or id twice, every seeded team and pool player's team in the rosters). Upstream's 96/97 competition files are a valid world; with the PC game's teams, pool and upstream's tactics, valid game data. |
| 4 | inventory | The 76 upstream world branch points and the 96/97 hardcoded facts found in the competition/career code, each mapped to a package field (`rules/…`) or a behaviour component. |
| 5 | `world_extract` v0 | `data/worlds/1996-97/` (upstream competition JSON + the extracted tables); the assembled `target/cache/worlds/1996-97/`; the fetched, verified and completed `target/cache/worlds/2026-27/`; the re-extraction diff tests. |

### M5 — Competitions (generic engine)

- Leagues/divisions, cups, tournaments, groups, play-offs/outs, fixtures, tie-breakers,
  deciders, draws, pyramid moves, outcomes, access lists and coefficients, all driven by
  the loaded world.
- `sim/` (`match_sim.c`): results of matches nobody plays, so competitions run end to end
  before the match engine exists.
- World 1996-97's authored `rules/` for competitions.
- Then the behaviour components World 2026-27 needs (splits, play-offs/play-outs,
  promedio, parallel groups, league phase, continental cups, state championships), from
  the rules library and `world_*.c`.
- Exit:
  - oracle proptests per unit against the 96/97 code and the rules library;
  - upstream known answers and swc contract tests pass;
  - generic invariants pass over generated packages.

### M6 — Match engine

- First, define `MatchState` from the sim_state.c SIM registry and agree on the `step` API.
  Match rules (substitutes, extra time, penalties…) come from the competition, i.e. the
  world.
- Then port in dependency order:
  1. game state machine & timers
  2. ball physics
  3. player movement & animation tables
  4. control/passing/shooting
  5. AI
  6. goalkeeper
  7. tackles/fouls/cards/injuries/referee
  8. restarts (throw-in, corner, free kick, penalty, shoot-out)
  9. bench/substitutions/tactics changes
  10. camera & on-screen flags
  11. crowd/advert/animation RNG consumers
  12. statistics
  13. highlights ring
- Exit: per-unit oracle proptests for both game styles, the Level-2 whole-match
  differential, and the RGD corpus replayed at upstream's level.

### M7 — Career

- New career/setup, calendar & season loop, season end (driven by the package's outcomes,
  season-end steps and access lists), finances, transfer market & pool, job offers,
  board/chairman, records & trophies, national-team manager. Every parameter comes from
  the package.
- World 1996-97's authored `rules/` for the career.
- The autopilot is ported as a test driver.
- Logic is pulled out of the mixed menu files into `CareerCommand`s.
- Exit: per-unit oracle proptests, plus golden careers for both worlds matching
  upstream's save hashes.

### M8 — Integration

- Played matches feed competitions and the career. Full save/load through the M2 crates.
- A headless Rust CLI (`src/tools/cli`) to run a match, a season or a career on any world
  package, for debugging and reports.

Order rationale: competitions (mostly pure functions) mature the oracle tooling and the
world model before the stateful match engine. The career needs both. Swapping M5 and M6
also works.

### M3 — View resources (readers/writers only)

Done last (decided 2026-11-13): the game logic and its headless integration first (M4–M8),
the view's files when the view comes. It keeps its name, which notes elsewhere use.

| Crate | File(s) |
|---|---|
| `dos_image_256` | `*.256` (title, stadium, fill, loader screens + palette) |
| `dos_sprite_bank` | sprite `.DAT` banks (`CHARSET`, `SPRITE`, `SCORE`, `TEAM1-3`, `BENCH`, `GOAL1`) |
| `pitch_blk`, `pitch_dat` | `PITCHn.BLK` tiles / `PITCHn.DAT` map (SWOS 2020's pitches use the same formats) |
| `dos_sample` | `sound_dos/**.raw` |
| `xmi`, `adlib_bank` | `.xmi` music, `.ad` instrument bank |
| `menus_json`, `translations_json` | upstream UI JSON |

---

## 10. Definition of done, per unit (function or upstream file)

1. Entry in `upstream.toml` set to `todo`. Read the C, the matching upstream docs and the
   SWOS-RE notes.
2. Every hardcoded rule found is mapped to a package field or behaviour component, and
   added to World 1996-97 (and to the 2026-27 completion if upstream bakes it there). Every
   style difference is mapped to a `GameStyle` method.
3. Rust types designed: enums, newtypes, documented constants (§5.4).
4. Idiomatic port with no globals and no world checks. Quirks preserved and commented.
5. Unit tests for every function (normal cases and edge cases). Rustdoc on public items.
6. Oracle binding + differential proptest (core). Fixtures + round-trip + local-original
   tests (resources).
7. `upstream.toml` updated: `status`, `migrated_at`, `rust` paths.
8. `just check_code` and `just test` pass.

---

## 11. Risks

| Risk | Mitigation |
|---|---|
| **One engine reproducing two upstream paths exactly** (the original's season end vs the Mod's modules; different RNGs and orderings) | Behaviour components chosen by data (§5.2). The oracle per function and the golden careers for both worlds show quickly where the model is missing something. A gap is closed by extending the package, never by a world check. |
| The original's ordering/quirks are hard to express as data | Model the ordering explicitly (the season-end step sequence is package data). Quirks become named components. |
| Upstream's World 2026-27 rules exist only as baked C tables | `world_extract` reads them from the compiled upstream (via the oracle) and writes them into the package structure. A diff test runs at every pin. |
| The download changes or disappears (`worldofswos.com`, swos-port) | Fingerprint/checksum checks with clear errors. The pin records the expected fingerprint. CI keeps `target/cache/` between runs. |
| World 2026-27 data is not redistributable | Downloaded to `target/` only, never committed. Derived files stay in `target/` too. |
| Byte-identical saves need upstream's raw competition blobs | They're confined to the `career_save` crate (a format adapter). The golden careers verify them. |
| Upstream velocity (~20 commits/day) | Pin + per-file ledger + `bump`/`status` + re-extraction diff. Integrate in batches. |
| Hidden coupling through globals | `docs/GAMEPLAY.md` §1.3a lifecycles, the sim_state registry, and oracle comparison of the whole touched state. |
| Idiomatic types drifting from C semantics | Width-mirroring types, explicit wrapping, cast lints, differential proptest. |
| Logic tangled with menus in the career files | Pull it out into commands. Mark such files `partial` until the view port finishes them. |
| Oracle build friction (libclang, SDL for Level 2) | Start with self-contained units. Run the oracle as a separate CI job. |

---

## 12. Glossary

- **World package**: a folder with OpenSWOS's world structure (`WORLD.ini`, `mod.json`,
  `teams/`, `data/`, `competitions/`) plus our `rules/`. It holds a whole football world;
  the engine plays whichever package it is given.
- **World 1996-97**: the package that reproduces the original game, a mod with no
  hardcoded logic behind it. **World 2026-27**: the SWOS 96/97 Mod's world, downloaded.
- **Completion**: turning upstream's thin World 2026-27 package into a self-contained one
  (96/97 base merged, baked tables extracted).
- **Behaviour component**: one implementation of one rule behaviour (a tie-breaker, a
  fixture generator, a draw method…), selected by name from package data.
- **Game style**: an implementation of `GameStyle` (Amiga 50 Hz, DOS 70 Hz…), selected by
  settings.
- **Oracle**: the test-only crate running upstream's C for differential tests.

---

## 13. Open points

None blocking. The `rules/` schemas are designed as the inventory (M4) and the
competition/career ports (M5, M7) find each hardcoded rule.

- Which team file a competition plays with (`comp.team_number`: a country's, `0xFF` for a
  `.TMD` pool, 72 for the custom teams) is not checked by `core::world` yet: the pools and
  the custom teams are not in packages. The inventory (M4 #4) and `world_extract` (M4 #5)
  give them files.
