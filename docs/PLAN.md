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
| D8 | Test data we can't commit (World 2026-27, Europe 1984-85, the RGD recordings) is **downloaded on demand and cached under `target/`** (§6). |
| D9 | **Importers read only the DOS (PC) game's file formats**, plus OpenSWOS's own platform-neutral formats that D4/D7 need (JSON saves, team/competition JSON, world packages, `.SRP`). Amiga formats are not read: RNC-packed files, the `data/` folder of world packages (Amiga `TEAM.nnn`), OCS/AGA graphics, RJP1 music. The installed PC game's files are not RNC-packed (RNC only wraps the floppy release's installer archives), so there is no RNC codec. |
| D10 | **Europe 1984-85 is in scope** (2026-11-21): upstream's third world, the SWOS 96/97 Mod's classic world, is a third downloaded package, fetched, verified and completed as World 2026-27 is, once World 1996-97 and World 2026-27 play (docs/WORLD_INVENTORY.md §3.1). |
| D11 | **World 1996-97 reproduces upstream's 96/97 path**, OpenSWOS's Copa Libertadores included (2026-11-21). The Copa is data (its record and its qualification rule), so a package without it is the pure original (docs/WORLD_INVENTORY.md §3.2). |
| D12 | **The career params are a world's** (2026-11-21): each package carries `rules/career_params.json` in upstream's schema; World 1996-97's is upstream's file, and the completed worlds carry the same (docs/WORLD_INVENTORY.md §3.3). |

---

## 2. Scope

**In scope now**

- Readers and writers for every game file format (`src/resources/*`), including the
  ones only the view will use.
- **World packages**: OpenSWOS's world package structure, extended (in the same
  structure) with every rule upstream still hardcodes or bakes into C. World 1996-97,
  World 2026-27 and then Europe 1984-85 (D10) load through one loader into one engine.
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
| **Upstream has two paths:** 96/97 runs the original season-end code untouched, and World 2026-27 runs the ported Mod modules (`world_*.c` + the rules library, `src/rules/`: `swos-competition-engine` until 2026-11-11) on top of it. Since 2026-11-13 a third world, Europe 1984-85 (the Mod's classic world), runs some of them. At `df643c2` there are 116 `World_Current()` checks in 31 files (46 in 15 at the baseline), plus about 120 accessors whose data is absent in 96/97. | We merge them (D6). One generic engine, and world packages that must each reproduce their upstream path exactly. The upstream branch points are the inventory of what must become data: docs/WORLD_INVENTORY.md (M4 #4). |
| **The World 2026-27 package** (`OpenSWOS-World-2627.zip`, 4.2 MB at the pin; hosted by OpenSWOS at `openswos.net/dlc/` since 2026-11-12, upstream's `PROJECT_WORLD_PACKAGE_URL`; `worldofswos.com` before) holds `WORLD.ini` (tag, name, fingerprint), `mod.json` (the world as a mod, its settings), `teams/` (101 countries' team JSON files, the six continents' national teams, and the world's foreign players' pool `POOLPLYR.DAT`, RNC-packed), `data/` (102 Amiga `TEAM.nnn`), and `competitions/`: `leagues.json` (91 leagues, the same schema as upstream's 96/97 `bin/data_json/competitions/leagues.json`), `national_teams.json` (211 FIFA members), `bteams.json` (the B teams' call-up rules), `stadiums.json`. Its team JSON adds players 17.. (`extra_players`), years of birth and looks; the sealed package's files are written `indent=1` (`scripts/worldgen/seal.py`). | This is the structure (D7). Its JSON schemas are upstream's existing ones or small files of their own, so the resource crates read upstream packages unchanged. |
| **The package is not the whole world.** Upstream applies it *over* 96/97's records, and the rest of World 2026-27 (96 cups, splits, play-off win lines, league levels, parallel groups, cup quotas/orders, the UEFA/continental specs) is baked into `world_data.c` by `world_import.py`. Upstream has no JSON for any of that. | We extend the package, in the same structure, with files for everything baked or hardcoded. Our packages are **self-contained**: no layering over 96/97 (§5.2). |
| `WORLD.ini`'s fingerprint = the first 16 hex digits of SHA-1 over `teams/`, `competitions/`, `data/` (each file's path + NUL + bytes, in path order). The pinned upstream expects `80e20cdd3e92ec9c` (`efe1464b6d1d62b8` at the baseline). | The download cache is validated against the pin's fingerprint (D8). |
| World 2026-27 *data* (the SWOS 96/97 Mod's) is **not MIT** ("ask the maintainer"). The rules library (≈2.2k lines) **is** MIT: upstream's submodule `swos-competition-engine` until 2026-11-11 (commit `a21b56d`), copied unchanged into `src/rules/` since, its licence beside it; its contract tests stayed in the library's own repository (tag v1, `87b6527`). | 2026-27 is never committed: we download and cache it (D8). The rules library is an oracle source; its contract tests are known answers (fetched from its repository at `87b6527`). |
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
│   └── worlds.just         # world_extract_9697, world_extract_9697_data, world_fetch_2627
├── upstream.toml           # pinned upstream commit + per-file migration ledger (§8)
├── data/                   # runtime data: the files the game loads (committed, GPL) — §6
│   ├── worlds/1996-97/     # the World 1996-97 package, without rosters — §5.2
│   │                       #   (its rules/ holds the career params, D12)
│   ├── tactics/            # the built-in tactics (upstream bin/data_json/tactics/)
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
                                  (the files: docs/WORLD_INVENTORY.md §4)
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
  job with different algorithms. Examples: tie-breaker order, draws that use the game's
  `Rand2` versus the Mod's table-seeded LCG. In those
  cases the engine has one implementation **per behaviour**, behind a trait or an enum,
  and the package names the one it uses. Components are named after what they do, never
  after a world. A new behaviour adds a component, never a branch.

**World 1996-97: a mod that reproduces the original.**

- It reproduces upstream's 96/97 path, OpenSWOS's Copa Libertadores included (D11).
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
  every upstream pin and diffs it against its committed SHA-256 digests, which hold none of
  its data (M4 #5e).
- Proof: the Rust engine + this package reproduces upstream's 2627 path (the rules library,
  `world_*.c`, and the 2627 golden careers).

**Europe 1984-85: the same, later (D10).** Its package (`OpenSWOS-World-8485.zip`, upstream's
`PROJECT_WORLD8485_PACKAGE_URL`, DaniSwos's data: never committed) is downloaded, verified and
completed like World 2026-27's, its tables from `world_data_8485.c`, once World 1996-97 and
World 2026-27 play. Proof: its three golden careers.

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
  - upstream's GPL data: World 1996-97's competitions and career params
    (`data/worlds/1996-97/`, D12), and the built-in tactics, which upstream loads for every
    world;
  - our own: the rest of World 1996-97's `rules/` (M4 #4–#5).

  Never original game data. `data/README.md` records where every file comes from and its
  license. Tests read these files where they are, never a copy in `assets/`.
- **Test fixtures, committed (`assets/<FORMAT>/`).** Only the tests read them. Each fixture
  is a file plus a JSON sidecar (`name.ext` + `name.json`, the expected decoded model, as
  in infinitier):
  - a **small, curated set of original files** (e.g. two or three PC `DATA/TEAM.nnn`, one `.TMD`,
    `POOLPLYR.DAT`, one sprite bank, one pitch, a few samples), kept small;
  - synthetic fixtures for edge cases;
  - known answers of data we may not commit: the completed World 2026-27's SHA-256 digests
    (`assets/WORLD_2026_27/completion.json`, M4 #5e), which hold none of its data.

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
  the rules library's contract tests and rule tests (swos-competition-engine at `87b6527`:
  upstream no longer carries them), turned into Rust test tables with their provenance
  noted.
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
pinned_commit = "df643c28354d454cfe2788613b38b7b142282d1a"   # 2026-11-21 (baseline ec2ed2be, 2026-11-10)
tracked = ["src/**", "include/**", "bin/data_json/**", "tests/**", "tools/swosdata/lib/**",
           "scripts/world_import.py", "scripts/assign_team_uuids.py", "scripts/rgd_schema.py"]
checkout = ["src", "include", "bin/data_json", "tests", "docs", "tools/swosdata/lib", "scripts"]
# M4 adds: world_2627 = { url = "https://openswos.net/dlc/OpenSWOS-World-2627.zip", fingerprint = "80e20cdd3e92ec9c" }

# [[submodule]] entries (path, commit) for tracked submodules: none since the rules library
# moved into upstream's src/rules/ (2026-11-11).

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
2. Port the changed files. When a `world_data` file changed, the re-extraction tests (M4
   #5e) name what it changes: after review, run `world_extract` again
   (`just world_extract_9697_data` regenerates `data/worlds/1996-97/`,
   `just world_complete_2627` the cached 2026-27 completion) and bless the completion's
   digests (`OPENSWOS_BLESS=1 just test world_2627_digests`).
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
| 2 ✅ | `team_json` | `teams/<COUNTRY>.json` | Upstream's team JSON (uuids, checksums): the rosters of world packages. Three dialects, all round-tripped byte for byte: Setup's (signed numbers, no trailing newline), upstream's world import (unsigned, trailing newline) and World 2026-27's sealed package (`indent=1`, characters as they are, its national teams' players face first). A world's additions typed: players 17.. (`extra_players`), years of birth, looks (the record's byte 25); a name longer than its field is cut as the game cuts it. Ids are data (World 2026-27's come from the Mod's club ids, not derivable); derived 96/97 ids and checksums match upstream's documented values. All 107 World 2026-27 rosters (2,596 teams, 24,183 players 17..) and every PC team file convert both ways. |
| 3 ✅ | `tmd` | PC `DATA/EUROCUP/EUROCWC/UEFACUP.TMD` | Bare `TeamFileEx[]` (16/32/32). All three round-trip; every pool team is the team its id names in `TEAM.nnn`. |
| 4 ✅ | `poolplyr` | PC `DATA/POOLPLYR.DAT` | 900 58-byte `PoolPlayer` records. Round-trips; every record is a copy of a squad player: the player at its ordinal in the team its id names. |
| 5 ✅ | (`team`) | `CUSTOMS.EDT`, `SAFE.EDT` | No crate of its own: the layout of `TEAM.nnn` (BE count + 684-byte `TeamFileEx[]`, 48 teams), read by the `team` crate. Both PC files round-trip. Faithful to the original: they store the global team numbers little-endian, which the game recomputes on load. |
| 6 ✅ | `tac` | `.TAC` | The 370-byte `TeamTactics` record: name, 10 outfield players × 35 ball areas (7 × 5) of packed x/y cells, pair ids, the ball-out-of-play slot. Bytes after the record (which the game ignores) are kept: SWOS 2020's CPU tactics end with six zeros. Fixtures from SWOS 2020 (the PC game ships no `.TAC`). OpenSWOS's JSON `.TAC` (`write_tactics_buf`) is read as the same record: #13c. |
| 7 ✅ | `competitions_json` | `competitions/*.json` | Upstream's ten files (leagues, cups, tournaments, playoffs, qualifying, league blocks, career, country competitions, countries, seed buckets) as one model picked by `type`, field for field. All ten round-trip byte for byte (committed, GPL), and World 2026-27's `leagues.json` (download cache). Bytes kept as written; packed bytes split as upstream splits them; competition types, continents and list markers are enums; fields whose meaning the engine gives stay numbers (M5). The JSON helpers moved to `openswos_common::json`. |
| 8 | `world_rules` | `rules/*.json` | Our new files (§5.2). The files and what each holds: docs/WORLD_INVENTORY.md §4 (M4 #4); their schemas are fixed as M4 #5, M5 and M7 port what reads them, and the crate grows with them. Started with M4 #5c1: `calendar`, `country_details`, `team_files`, `season`, `fixtures`. |
| 9 ✅ | `world_package` | `WORLD.ini`, `mod.json`, the layout | Opens a package from a folder or a zip (paths kept inside, 80 MiB cap), checks its fingerprint (upstream's algorithm: World 2026-27's files give `80e20cdd3e92ec9c`, the pin), writes one into an empty folder stating the fingerprint of what it writes. `competitions/`' own formats: `national_teams.json` (#10's crate), `bteams.json`, `stadiums.json`; `mod.json`'s settings. `data/` (Amiga, D9) and `teams/POOLPLYR.DAT` (RNC) are fingerprinted but not read; other files are listed as ignored. `rules/` joins with #8. |
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
| 4 ✅ | inventory | docs/WORLD_INVENTORY.md, at upstream `df643c2`: every world branch point of the three worlds (116 `World_Current()` checks in 31 files and about 120 data-gated accessors, about 600 rows in five area files under docs/world_inventory/) and the 96/97 hardcoded facts of the original season end and career setup, each mapped to a package field or a behaviour component. The design: 23 `rules/` files, no new field in upstream's files; 20 behaviour components; the random generator each one draws from; the season end's steps; the limits that change behaviour; the completion's merge. No world check is needed. |
| 5 | `world_extract` v0 | `data/worlds/1996-97/` (upstream competition JSON + the extracted tables); the assembled `target/cache/worlds/1996-97/`; the fetched, verified and completed `target/cache/worlds/2026-27/`; the re-extraction diff tests. In steps: |
| 5a ✅ | World 1996-97 assembled | The tool `src/tools/world_extract` (`just world_extract_9697 <game_dir>`): the committed package with your copy of the PC game's rosters (each country's `DATA/TEAM.nnn` as upstream's Setup makes its team JSON, laid out as World 2026-27's clubs, every team its derived ids and checksum) and its foreign pool (`teams/POOLPLYR.DAT`, unpacked), into `target/cache/worlds/1996-97/`, its `WORLD.ini` stating its fingerprint. Checked before it is written: a valid world with valid game data (`core`). World packages now read the career params (`rules/career_params.json`, D12) and an unpacked pool. The `.TMD` pools need no files: their records are the team files' (but for the global number, recomputed on load, and the runtime team control). |
| 5b ✅ | World 2026-27 fetched | `just world_fetch_2627`: downloaded into the cache (its checksum pinned), its `WORLD.ini` fingerprint its files' and the pin's (`WorldData2627_DataFingerprint` in the checkout's `world_data.c`), else "package and upstream pin out of step". |
| 5c | World 1996-97's `rules/` | The `world_rules` crate (M2 #8) and the files of docs/WORLD_INVENTORY.md §4, each with its schema: the facts read from upstream's compiled 96/97 tables through the oracle, the authored ones citing their functions, the first season's European clubs (team ids, from your copy's `.TMD`). In steps: |
| 5c1 ✅ | The pipeline and the first files | `world_extract extract-9697` (`just world_extract_9697_data`) writes the whole committed `data/worlds/1996-97/` from the pinned checkout: upstream's competition files and career params as they are, and the rules read from upstream's compiled tables through the oracle (`openswos_oracle::tables`: always built, `Unavailable` without the checkout; a data file may bind globals), or stated as constants citing the code. The crate `openswos_world_rules_resource` with `calendar.json`, `country_details.json` (each country's continent: `_countriesTable`), `team_files.json` (the team files' bases: `_teamsCountryNumbers`), `season.json` (the season end's countries: `_careerCompetitionIdRemapTable`) and `fixtures.json` (its schema replaced by M5 #1's round robins). World packages read them; `core::World` requires and checks them (every country's details, 256 bases never going down, a second file on a free number, the season's countries the world's, each once). `tests/committed.rs`: the committed folder is the extraction, byte for byte (#5e for these files). |
| 5c2 | The other files | The rest of §4 (`europe.json` with the first season's European clubs, `continental_cups.json`, `national_cycle.json`, `career.json`, `limits.json`…), each with the M5 or M7 unit that reads it (its schema is fixed by its reader), extracted here and checked by `tests/committed.rs`. |
| 5d | World 2026-27 completed | The overlay merged (docs/WORLD_INVENTORY.md §9) and the baked tables of `world_data.c` read through the oracle into the same files (the five of #5c1 first), into `target/cache/worlds/2026-27/`; its RNC pool decided (§13). In steps: |
| 5d1 ✅ | The merge and the five rules files | `just world_complete_2627`: the oracle runs World 2026-27's baked install (`tables::world_2627`: `world_data.c` compiled, `World_Current` stubbed; the install writes 96/97's tables for good, so once a process, and the oracle serves nothing else afterwards). Over World 1996-97's committed package: the download's identity, mod, 107 rosters (every 96/97 country's), national teams, B teams and stadiums; its leagues replacing 96/97's by id with 96/97's play-off links kept (`install_league`, `dj_inplace_load`), its 26 new leagues with their countries and tables; 96/97's career params (D12); the five rules files (2026; the continents and the bases the install leaves, the national files grown as `fit_world_national_files` grows them; the season's 91 countries; real rounds). A valid world: 97 countries, 92 leagues. |
| 5d2 ✅ | European cups, pool, game data | The completed World 2026-27 has valid game data (107 rosters, 2,596 teams, a pool of 900, the built-in tactics). Its European cups are its three league phases (`LeaguePhase_InstallCups`): the install's records, read by the oracle field for field (`TournamentRecord`) and converted into `tournaments.json`, named by the career index's first three contests and by Europe's group. Its foreign pool is unpacked by upstream's own decoder (`src/rnc.c` through the oracle's `unpack`, its header checked first) and carried unpacked (§13). `core::World::is_played`: the competitions the career, the tables and the groups name, and what their leagues lead to; the game data checks the seeds of those alone, so 96/97's European cups, kept as the templates upstream's world modules copy, are not drawn. `RoundRules` converts from and to its byte. |
| 5d3 ✅ | National and league cups | The world's 101 cups (`WorldData2627_Cups`, laid as `WorldCups_Install` lays them): each record read by the oracle field for field, with its whole list of clubs (the English cup's 64 run past the record's 32). In `cups.json`, 51 replace 96/97's records of their ids and 50 are added, 126 cups in all. In the world's order, each country's table names its cup in its slot (1 the national cup, 2 the league cup). The table grows by one for the next slot; a league cup without a national cup is dropped, as upstream drops it; a cup past upstream's tables (96 countries of 12 entries) is refused. The ids are the records' C symbols, read from the rows of upstream's generated table and checked against the oracle's country and slot. The oracle's units may bind types nothing else reaches (`CompetitionCup`). The cups' draws (`national_cups.json`) come with M5. |
| 5e ✅ | Re-extraction diff tests | Each run of the tests extracts again, at the pin, and diffs (`world_extract::diff`): a failure names each file that differs, with its first line that does, and the command that writes it again. World 1996-97's extraction against the committed `data/worlds/1996-97/` (`tests/committed.rs`). World 2026-27's completion, which may not be committed, against its committed SHA-256 digests (`assets/WORLD_2026_27/completion.json`, `tests/world_2627_digests.rs`), so the check holds on any machine, CI included. When the cached completion is the one the digests know, the failure diffs against it line by line. After review, `OPENSWOS_BLESS=1 just test world_2627_digests` writes the digests and `just world_complete_2627` the cache. |

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
  - upstream known answers and the rules library's contract tests (at `87b6527`) pass;
  - generic invariants pass over generated packages.

The units, in dependency order. Each brings the `rules/` files it reads, for World 1996-97
and the 2026-27 completion (M4 #5c2), and every competition-id check it meets becomes data,
as the European Championships' round robin did (#1).

| # | Unit | Notes |
|---|---|---|
| 1 ✅ | Round robins | `rules/fixtures.json` holds the world's round robins as data: each size's pairs in playing order, whether it plays real rounds (`FixtureRounds_Real`: a career league needs them), and the competitions that play their own. World 1996-97's are the original's tables (`_fixtureSchedulePtrTable`, fixture_data.c) and the European Championships' order (`_euroChampFixtureData`, which diy_competition.c `getCurrentFixtureTeamPtrs` picks by competition id), read through the oracle. World 2026-27's are what its baked install leaves (`FixtureRounds_Install`: its odd sizes 5 to 23 as real rounds), read the same way, so nothing is transcribed and no rounds algorithm is ported. The engine has none of its own: `core::competition::schedule` (`Schedules::of`: a competition's own, else its size's). `core::World` checks them: every pair of its clubs once, real rounds where it says so, a size once; a competition's own of its groups' size, none for a knockout. M4 #5c1's `OddRoundRobin` component is gone: it is data now. |
| 2 ✅ | League tables | `core::competition::table`: a team's `Line` (upstream `DIYTeamStats`, 16-bit counts that wrap), a result recorded into the two lines (`DIY_PlayOrSimulateLeagueFixture`: a win the record's points, a draw 1 each, the goals both ways; the games counted apart with the progress, `DIY_AdvanceLeagueProgressAfterMatch`), and `sort_groups`, upstream's exchange sort of each group (`DIY_SortLeagueTableByStandings`: points, the goal difference in signed 16 bits, goals, the name byte for byte; not stable, kept so). Every world orders an ordinary table so; a league phase's and a split league's own orders come with their data (#9). The oracle now compiles `diy_competition.c`: only what its bound functions reach is linked (cc's function sections, the linker's dead-code stripping), so it binds them one at a time with a few stubs (a world's league phase and split), and an SDL shim (`build.rs`) declares the types upstream's headers name. Checked against `DIY_SortLeagueTableByStandings` by random tables in groups, ties down to the names' bytes, goal differences that wrap. |
| 3 ✅ | Team values and draws | `core::competition::value`: a team's average price over its 16 players (`GetAveragePlayerPrice`) and a competition's strength, the average and its tier of four prices up to 7 (`DIY_ComputeAverageTeamStrengthTier`; none without teams, where upstream divides by zero). `core::competition::draw`: the shuffle every draw makes (`DIY_ShuffleTeams`, `DIY_ShuffleTeamOrderList`, `DIY_ShuffleTeamGridPairs`, `DIY_ShuffleLeagueTeamOrder`: from the last place down, each swapped with `Rand2() × n >> 8`), and the seeded draw (`DIY_SortAndShuffleTeamsByValue`: a bubble sort by value, then each pot shuffled; a group stage's pots one per group place, a knockout's four quarters, none shuffled when a quarter is two teams or fewer). Checked against the C from any state of the second generator: the same orders, the same state after (the oracle's `VerifyTeamLoaded` stub gives the teams the tests load). `CountNearnessFactor` is the match engine's (M6: ball quadrants, player positions) and `GetLowestAndHighestPlayerPrice` the career's (M7: the career club's squad). |
| 4 | The running competition | A competition under way (upstream `DIYCompetitionEx`, in SWOS2.s's layout: upstream docs/DIYCOMPETITIONEX_AUDIT.md) as typed state, every byte of it accounted for (its unions as views over their bytes), so the units after it are checked against the C on the same image, and the saves' raw images can be written back (D4; their mapping is the save unit's, M8). In steps: |
| 4a ✅ | The state and its image | `core::competition::running::RunningCompetition`: upstream's image itself (2,640 bytes, its 64-bit build's), each field a typed accessor at its offset: the header (`DIYCompetition`, its dual-use fields by both names), the match data's league and cup views, the counters, the league's section, the cup's teams, the grid, the shuffle order and its bracket scratch, the standings rows (`StandingsRow`: a team and its `Line`). Held whole because upstream's unions give regions two meanings and its code runs past arrays' ends into the next field (a league of more than 14 clubs writes `leagueTeamIds` into `competitionFormat`): an index past an array reaches the bytes after it, as the C does. The oracle checks the layout: any image, every field read through the C struct is the accessor's (both views of each union). |
| 4b ✅ | A league from its record | `core::competition::league::start_division` (upstream `InitDIYCompetitionFromTableEntry`, the type-0 branch of `InitCompetitionFromRecord`): the round robin's real rounds checked (the 0x1313 halt), the reset (`DIY_InitCompetitionDefaults`: its defaults, the default name, the table cleared, the places in order), the record's and the division's fields (play-off places below 0 halved, as the original), the name (the country's adjective and the division's name, 35 bytes at most, what the default left past its end kept), the derived fields (`DIY_RecomputeCompetitionDerivedFields`: each team's identity in its row, the section's counts, the season's match days and their months, the strength), the places shuffled. `LeagueRecord` is a `leagues.json` league, or a `cups.json` cup played as a league read through the league's layout as upstream reads it (its `teams` the rounds, `round_string_base` the division's clubs). Faithful to upstream's build: the groups flag tests the points byte's sign (it reads a word little-endian). Checked against the C from any image: every byte alike but the C's name pointer, odd divisions refused as upstream halts (oracle stubs: the two record lists, the default division names, the session's pitch, `StringCopy`; the team loader now gives `_selectedTeams`' own records). The texts and classes come in resolved (#4b2). |
| 4b2 ✅ | A league's texts and classes as data | `core::competition::league::WorldDivision`: what `start_division` takes, resolved from a world for a league or a cup played as one, a division, and the record it is played from (`world::Source`: its own, or the career's copy, which upstream's lists name apart). Two new rules files, written by `world_extract` through the oracle: `strings.json` (the texts of the compiled strings the records' names name, from `_stringSymbols`; the reset's name `_aDiyLeague`; the default division names `_divisionStringTable`) and `competition_classes.json` (`national_teams`: `_nationalTeamCompetitionsAndQualifications1`, no country before their names; `continental`: `_europeanCupsandNationalTeamCompetitionsAndQualifications` and the records of the ids `continental_record` names, the Copa 162 in both worlds; a listed copy by `career.json`'s name for it). `country_details.json` gains each country's adjective. `core::World` resolves every record's names at build (a symbol's text, else the name itself, as upstream's `dj_intern_names`; a byte a character), and checks the classes name competitions or career copies. World 2026-27 keeps 96/97's strings and lists (they name 96/97's records, which it keeps); ids 239-243 have no record in either world yet, and the Super Cup's tie (249, while `SuperCup_UefaTie`) joins with World 2026-27's components (#9). Faithful to the original: a record whose name table has no first name takes the default name (96/97's tour is a PREMIER DIVISION). Checked against the C on upstream's compiled records with its own tables: every country table's league and every career cup played as a league, from its record and its copy (130 divisions), byte for byte. |
| 4c ✅ | A cup from its record | `core::competition::cup::start_cup` (upstream `InitDIYCupFromRecord`, the type-1 branch of `InitCompetitionFromRecord`): odd teams refused as upstream halts (0x1313), the reset (`InitDIYCup`: its defaults, the cup's reset name, every round's rules 1, the pairings in order), the record's fields and each of its bracket's rounds' rules (legs, extra time, penalties), the name (as a league's: the adjective unless national, 35 bytes at most), the bracket (`DIY_InitCupBracketState`: each team's identity and the teams no computer controls, `DIY_BuildTeamSelectionPointerCache`; the pairs and rounds of half the teams; the first round's format, a one-leg final's 0; the results cleared; the matches' months, two a round but one a single-leg round settled on penalties, a replay's date after them, `DIY_InitCupBracketProgressionTable`; the strength), the record's rounds after the bracket's, the pairings shuffled unless the record keeps its order (`DIY_ShuffleTeamGridPairs`). `CupRecord` is a `cups.json` knockout or a `playoffs.json` play-off, read through the cup's layout (a play-off's bench where a cup's points are), its round rules as many as upstream's record holds (6, 2). Refused where upstream runs past its memory: fewer than two teams, more than 64, a bracket of more rounds than the record's rules (a play-off past 7 teams). `cup::WorldKnockout` resolves its texts and classes from a world as `WorldDivision` does (`World::name_prefix`, shared); `strings.json` gains the cup's reset name (`_aDiyCup`) and default name (`_aCup_0`), and `reset_name` is `league_reset_name`. What both starts share is `competition::start` (the pitch, a `LoadedTeam`, now with its controls, the name, the months' progression). Faithful to the original: whether a replay's date is kept is the lowest single-leg round's (upstream names it the last round's). Upstream's world hooks after the draw join with their worlds: World 2026-27's national cups (`WorldCups_CupMade`, #9), a classic world's European cups (`WorldEurope_DrawMade`, Europe 1984-85, D10). Checked against the C from any image (a cup's or a play-off's record, any rules, names, controls), and on upstream's compiled records with its own tables: World 1996-97's 51 country cups, its two career knockouts from their records and copies, its 7 play-offs (62), byte for byte. |
| 4d1 ✅ | A tournament's state and its stages | `core::competition::tournament::TournamentState`: upstream's `DIYTournamentState` image (1,120 bytes, its 64-bit build's), each field a typed accessor as #4a's: its header the same `DIYCompetition` accessors (one `header_fields!` for both states), its team ids, group map and team order, its six stage slots (`DIYTournamentSlot`: each stage's teams, groups, teams a group, the teams that go through, a group stage's plays and points, a knockout's legs, extra time and penalties, its matches, its 90 months), an index past an array reaching the bytes after it. `tournament::reset` (`InitDIYTournament`: a DIY tournament's defaults, the reset's name, the World Cup's stages as the default, the teams in order) and `tournament::recalculate` (`DIYTournament_RecalculateMatchesPerTeam`: each stage's matches, a group stage's days of its round robin as many times as it plays, a knockout's two legs or one match settled on penalties; their months from the first to the last, `DIYTournament_BuildMatchesPerTeamProgressionTable`; the strength). Refused, nothing written, where upstream runs past its memory or divides by zero: stages other than 1 to 6, groups of fewer than two teams, no match in all, months past the state, teams other than 1 to those the state holds. Faithful to the original: a stage of no match takes the next match and every match after it, its months running on past its table. A world's league phase and state championships count their own matches (`LeaguePhase_StageDays`, `States_StageDays`): with World 2026-27's components (#9). `strings.json` gains `tournament_reset_name` (`_aDiyTournament`) and `tournament_name` (`_aLeague_1`). What both states share moved: `competition::start::copied` (a name over a buffer). Checked against the C from any image: the layout (every field through the C struct), the reset, and the stages counted from any state of valid stages (both images after alike). |
| 4d2 ✅ | A tournament from its record | `core::competition::tournament::start_tournament` (upstream `InitDIYLeagueFromRecord`, the type-2 branch of `InitCompetitionFromRecord`): the reset, the suspensions' track, the seeds ordering the first stage when the record says so (each seed found among the teams moved to its place), the months, the pitch, the name (`_aLeague_1` the default), the substitutes, the record's stages into the slots (a group stage's points and plays, a byte below 0 a doubled knockout, the teams that go through the next stage's, the sixth's the reserved byte), the slot after the last its teams left, the stages counted (#4d1), then the first stage loaded into the competition under way (`DIY_LoadCurrentCompetitionSlot`): a stage of groups as a league (the league's reset, the stage's fields, the qualifiers a group, `DIY_BuildTeamGridAndIndexMaps`: its order kept, shuffled, or seeded by value into pots and dealt into the groups; each place's index and row, each team's group; the derived fields), a knockout as a cup (the cup's reset, `DIY_ShuffleTeamsAndBuildCompTeamList`: kept, shuffled, or seeded into quarters and each pair's home side drawn; its places, doubled for a doubled bracket; the teams no computer controls; the bracket), then the tournament's settings and the stage's months. `TournamentRecord` is a `tournaments.json` record, its seeds the world's identities. New rules file `draws.json`: the tournaments upstream draws by its own tables, which its code picks by competition id (the World Cup, 24 places kept; the European Championships, 16), written by `world_extract` for both worlds; `core::World` checks each names a tournament once, keeping 1 to 64. `tournament::WorldTournament` resolves the names, adjective, classes, own draw and odd groups. Refused where upstream runs past its memory, divides by zero or reads stale scratch: stages other than 1 to 6, a seeded first stage of more than 32 teams, first stages of fewer than two knockout teams or more than 64, groups past 64 teams (or, seeded, other than the teams), an own draw keeping fewer places than the teams, an odd seeded knockout; odd groups as upstream halts where the world refuses them. A world's league phase and state championships start their stages their own way (#9). Checked against the C from any images and any record (both images byte for byte, the generator after), and on World 1996-97's tournaments with upstream's compiled records (the career's eleven from records and copies, the World Cup and the European Championships bound by name; the Copa, which upstream's SDL build loads from the JSON, from its JSON record): 25. |
| 5 ✅ | Leagues | `core::competition::league`'s play. `fixture` (upstream `getCurrentFixtureTeamPtrs`): the current pair of the round robin the league's groups play (the caller's: a world's `Schedules::of`, so the European Championships' own is data, #1), in the current group, each side the row of its place, the other way round in every other cycle (the cycles counted down in `minSubstitutes`). `advance` (`DIY_AdvanceLeagueProgressAfterMatch`): the sides' games counted, the group, the day, the phase and its month, the cycle. `play_result` (the bookkeeping of `DIY_PlayOrSimulateLeagueFixture` around its match, which comes in as its score: the match begun, the progress, the career club's part, both kept in the tournament's state for a stage (`DIY_SyncProgressToDiyFileBufferCopy`), the match counted, the result in the table (#2's `record`), and at the season's end the qualifiers). `sort_table` (`DIY_SortLeagueTableByStandings` on the competition: #2's sort of each group, the teams' names by their identities). `update_qualifiers` (`DIY_UpdateLeagueQualifiers`: the table sorted, its winner unless a tournament's stage before its last, the promotion play-offs' places after the promoted, then the relegation play-offs' above the relegated). `season_moves` (`DIY_UpdatePromotionRelegationScratchCounts`: a league's promoted and relegated, by identity, which upstream counts in its scratch of the teams' divisions and flags for the career club; the season end applies them, M7). Faithful to the original: a row that plays itself takes both sides' counts. Refused where upstream reads past the competition (places past the order, a place naming no row, an entry no row's, a phase's month past the image) or the round robin (a season over). The career walks a season's fixtures into its calendar (`recordLeagueFixtures`, `diy_ResetCompetitionStatsAndProgress`: its cache) with M7; a world's league split, league phase and state championships pair, count, record and sort their own way (#9); `core::World` checks every division has its round robin once those are data (#9). Checked against the C: any league under way (random groups, places, rows, progress and counts, the European Championships' own round robin among them), a result recorded alike (upstream's whole `DIY_PlayOrSimulateLeagueFixture`, its result cached so no simulation runs, the match engine's and the career's calls stubbed), both images byte for byte; any table's qualifiers and season's moves alike; every division of World 1996-97's country leagues (88) started as upstream starts its compiled records and played through its season alike, match by match, then its moves. |
| 6 ✅ | Cups | `core::competition::cup`'s play. `decide` (upstream `ProcessDIYKnockoutMatchResultAndCareerUpdates`): a tie decided from its match's results as it needs them (`MatchResults`: normal time on the home side's ground or neutral, extra time, penalties, in upstream's order; a simulated match's from the generators, #8, a played one's from the match engine, M6): a first leg by its goals; a return leg by aggregate, then away goals after normal time (the rule's 1), extra time, away goals again (its rule on), penalties; a single leg or a replay by its goals, then extra time and penalties where the round's rules say so (a code of 2 on in a replay only); else a draw to replay; packed as upstream packs it (the goals, the aggregate, the penalties or the first leg's, the flags: `ResultFlags`). `play_cup_match` (the bookkeeping of `DIY_PlayCupMatchAndAdvance` around its match): the tie's places (`cup_fixture`), the result into the bracket, the career club's part, the match counted, the round's end and its next month, the eliminated teams no computer controls counted off (`DIY_RecountMissingTeamsInTeamGrid`), the undecided counted (`DIY_CountUnselectedTeams`), the next round's draw (`DIY_ShuffleTeamOrderList`), the cup's or a stage's end and its winner. `next_round` (`DIY_UpdateCompetitionProgress`): the second legs the other way round, the first legs' goals kept for the aggregate; the drawn ties replayed first, the other way round (`DIY_ReorderTeamsBySelectionFlags`); otherwise the winners into the next round in the draw's order (`DIY_ApplyHomeAwaySwapFlagsToTeamGridPairs`, `DIY_ReorderTeamGridByOrderList`), the round and its months moved on, its pairs halved, its format its legs'. `round_name`, `stage_name` (`GetNextCupRoundStringIndex`, `GetCupRoundStringIndex`): a round's name in `strings.json`'s new `cup_round_names` (upstream `_cupRoundStringTable`, its 23). Faithful to the original: a side wins on its goals' low bytes; the next stage's and the next month's indices are ints, past a word. Refused where upstream runs past its bracket or the competition: a round of no ties (a cup whose rounds are more than its bracket's), a place naming no team, a draw's order lacking a place (one never drawn: a cup whose rounds are fewer), ties past the grid, a month or a round's rules past the image. The career records the bracket into its calendar (`DIY_RecordCupBracketProgress`) with M7; a world's play-offs, national cups, Serie D cup, Super Cups, UEFA qualifying and continental ties decide and arrange their own (#9), a classic world's European cups with Europe 1984-85 (D10). Checked against the C, its results the simulation's stubs' (which record what they are asked, normal time's ground among it): any tie decided alike, the same parts asked; any next round alike; any cup's or play-off's record started and played through to its winner alike, match by match (212 of each run's 256; the rest refused where upstream runs past its bracket); World 1996-97's 62 knockouts (#4c's) played through alike; the round names alike. |
| 7 ✅ | Tournaments | `core::competition::tournament::next_stage` (upstream src/career_teams.c `DIY_AdvanceToNextCompetitionSlot`): nothing once the tournament has ended; after its last stage, ended (`isActive` 2); otherwise the stage's end, then the next stage loaded. A stage of groups' end (`DIY_BuildSeededBracketAndUpdateScores`): its teams ranked into the order the next stage draws from, by the engine's ranking (each group's places a bonus down from its winner's, so the groups' winners come first in the groups' order, then the runners-up; the place only some go through on a smaller one; one table sorted; those of that place a bonus by their group, sorted again) or a world's (`DIY_AdjustWorldCupBracketScores`: the first stage only, its qualifiers group by group, the best four by their group; a later stage's tables kept), the first two swapped before a doubled bracket. A knockout's end (`DIY_CopyTeamGridFromCompToDIY`): its winners first (`DIY_ApplyHomeAwaySwapFlagsToTeamGridPairs`, now shared with the cup's next round), the first two ties swapped when its bracket or the next one is doubled, a world's ranking (upstream ranks the competition's table whatever its stage), the bracket's places the order. The stage loader is now any stage's (`DIY_LoadCurrentCompetitionSlot`, #4d2's generalised, `stage::load`, its errors `LoadError`): a later stage of groups deals the ranked order on a group for each of the groups before (no seeding), a later knockout keeps the order after groups or a seeded stage; a world's own draw from its tables. `draws.json`'s own draws now hold each stage's draw (`keep` the first places; `order`, each place the team of a place before; `best_placed`, the order set by the groups the best placed come from: the World Cup's patterns and map, `_diyGroupPatternLookup`, `_diyLeagueTeamShuffleMap`) and the ranking (`by_place`, the engine's; `by_group`, the World Cup's), a stage past the list drawn as its last: `world_extract` reads upstream's compiled tables through the oracle (`tables::own_draw_tables`; the World Cup's semi-finals' `diyCompSpecificShuffle4`, a table of the code's own, stated), and `core::World` checks them (places within the order's 64, a pattern's groups within a byte, each ordering every best placed). Refused where upstream runs past its memory, divides by zero, reads stale scratch or halts: a stage past the state, a stage of groups (or ranked by group) of no groups, a deal leaving a place, more teams than the order holds, a best placed team's group past upstream's shifts, no pattern for the best placed (0x8034), and #4d2's. The career copies a stage's progress into the state before the advance (`Career_UpdateHybridCompetitionProgressAndMaybeAdvanceSlot`) and records the new stage into its calendar (`DIY_PostLoadCompetitionDispatch`) with M7; a world's league phase, other confederations' main rounds and state championships end their stages their own way (`LeaguePhase_GroupsEnd`, `Conti_GroupsEnd`, `States_GroupsEnd`, `States_KnockoutEnd`, #9). The DIY editor's group layout (`computeGroupLayoutForTeams`, `DIYTournament_RecalculateNumStages`: its menus' answers as a stage is edited) is the editor's, with the view's menus. The oracle compiles src/career_teams.c (its `VerifyTeamLoaded` replaced by the stub: `units.toml`'s `replaced` makes upstream's definitions weak). Checked against the C: random tournaments whose stages mostly follow each other (the own draws' ids among them; at some stage ends the teams through and the next bracket's doubling changed on both sides) played through as the career plays them, every match, every stage's end and every next stage alike (both images byte for byte, the second generators); every tournament of World 1996-97 (25: the World Cup's 52 matches, its best placed and ranking, the European Championships' 31) played through alike. |
| 8 | Simulated results | `sim/` (`match_sim.c`): the result of a match nobody plays, from its teams' players and the generators. Every league, cup and tournament then runs to its end without the match engine. In steps: |
| 8a ✅ | A team in a match | `core::sim::MatchTeam`: upstream's team record as a match reads and writes it (`TeamFileEx`, 684 bytes, its global number in the host's order as upstream loads it; the career club's 1,292, its 16 reserves after its players, its line-up its own, `_currentMatchPlayers`), each field a typed accessor, an index past an array reaching the bytes after it. `match_value` (`UpdatePlayerPositionAndReturnRole`): the value kept while the team's are; else a goalkeeper's price in goal, a substitute's price, an outfield player's price for his place in the tactics (`player_price`, `GetPlayerPrice`: his seven skills, 1 to 8, weighed over the 35 ball areas by each area's ponders, halved, by how much the area counts for his position and how near it is to where the tactics put him; for a team the computer does not play, shared among the teammates there and scaled by `nearness`, `CountNearnessFactor`; then a price step, his price or a step above it), 0 out of his place, at least 1, 1 on loan and 0 unavailable for a team the computer does not play (`unavailable`, `IsPlayerInjured`: by `StatusRules`, the mode upstream checks in, a training game, the testers' rule), kept in his record. `strength` (`ComputeOldPlayerSkillForViewResult`): the substitutes' values weighed and averaged, the eleven's added, over 15, halved with no goalkeeper in goal, divided by one more for each outfield player of no value; the values then kept. The session's tactics are a table the team's tactics byte indexes (`TeamTactics`). The engine's tables (`sim::tables`: the positions' weights, the areas' distances and ponders, the grid's areas, the ponders' weights, the price rises) are Rust constants checked against upstream's compiled ones. Faithful to the original: a goalkeeper's position has no weights (upstream's 0s, as the DOS game's); a lone player's share divides by a word wrapped below 0. Refused where upstream reads past its records, tables or memory or divides by 0: a line-up place past the line-up or naming no record, tactics past the table, a place without an outfield player in them, a spot past the grid's rows, a price whose rise is past the table, substitutes counted to no weight. The oracle compiles src/match_sim.c, src/career_status.c and src/test_rules.c (their simulation's entry points and the career's updates replaced by the stubs, `replaced`). Checked against the C: the tables; the layout (any record, every field through the C struct); from any team (selected or the career club), any tactics and any status rules, a price, a match value and a strength alike, the record after byte for byte; any nearness and availability; England's 93 teams of a real team file (`assets/TEAM/TEAM.008`), as selected teams and as the career club, their values and strengths with upstream's compiled tactics. |
| 8b ✅ | A team's standing, form and availability | `core::sim::price_rank` (`CalcTeamPriceRankPercentOffset`, with the league's paths of `FindTeamInDIY` and `GetMoreExpensiveTeamsCount`, the only ones it reaches): a league's team only, once its first row has played 3 games (a computer club's league) or 10, its strength moved by its rank by price, the teams dearer than it less its place over the teams less one, up to 50% either way, halved; a team not in its league refused as upstream halts (0x6006), a national selector's given upstream's place. `form` (`CalcTeamViewOffset`): a team's form on the `Day` (the season shown, the session's seed, the view's month), each half's drawn through the random table by its number from upstream's table, a computer club's biased by a second pick, another's a point lower, at most 12 either way. `available_players`, `available_keepers` (`GetNumberOfAvailablePlayers`, `GetNumberOfAvailableSubstitutes`): the squad's players and goalkeepers out and available (the career club's whole squad). `status_buckets` (`InitPlayerRemapTableForTeam`): after a simulated match, the eleven's places (the career club's squad when it fields no stand-ins), each a player or, a draw in two, his stand-in, put in a status bucket by upstream's odds (a fit player 5 draws in 256, a minor bucket's 64, the keeper in goal one in 128 and a keeper only with every keeper fit), up to two players (four in a career game) less those already out; a computer club's records keep only the kinds `SimRules::computer` opens (upstream's `cpu_kept`: the original keeps none, the SWOS 96/97 Mod's CPU RED CARDS and CPU INJURIES open them in another world). Whether a match's statuses apply at all (`ShouldApplyRandomPlayerStatusAfterSim`: the career's competitions) is the career's (M7); a friendly club's long bench (`LongBench_FrOrder`, `SquadExtras_BenchRecord`) and a world's league in zones (`LeagueSplit_Zone*`) join with their components (#9). The form's and the buckets' tables join `sim::tables`, checked against upstream's. Checked against the C: any league, team and prices, the rank alike (the halt matched); any record and day, the form; any team, mode and rules, the players and keepers available; any team, stand-ins, game, computer statuses and generator (pinned at the odds' edges now and then, the squads mostly fit), the buckets and the generator after; England's 93 teams of the real team file, their ranks in a league of 20, forms and buckets over 256 seeds. |
| 8c ✅ | The simulated result | `core::sim::simulate` (`CalculateViewResult`): each side's stand-ins (`stand_ins`, `BuildAvailablePlayersRemapAndMask`: a count by the fewest substitutes, candidates by the most, each a fit player, a goalkeeper a draw in 64, standing in for a place his position may take by the formation's defensive order, `defensive_order`, src/menus.c `InitializeTacticsPositions`), its players' status buckets (#8b), cards and injuries (`after_match`, `ApplyRandomPlayerStatusAfterSim`: a red card under the competition's track of cards, else an injury, minor, or none where the career says statuses do not apply, worse for a player injured already; a computer club's records keeping only what its world opens); each side's strength (#8a, its bench the most substitutes) moved by its rank by price (#8b) unless the computer plays both (`ScaleViewIndexByTeamPriceRank`), its form added; the difference, the home side 2 up on its own ground, at most 23, draws the first side's goals from upstream's table and the other's by the odds of that count; each goal a scorer (`AssignSimulatedGoalsToScorers`: an own goal a high draw, else a place by the ball-out-of-play tactics' odds, its player or his stand-in, counted in the result's scorers and the record's goals' column). `extra_time` (`ExtraTimeSecondPhaseScoring`): the second half's forms, the stronger side's goals by their difference, the weaker's by their odds. `penalties` (`PenaltyShootoutScorerPair`). A `Session` carries the tactics, their formations' orders, the rules, the career's answers (statuses apply, a national selector) and the day; `Simulated` the state extra time continues from (upstream's module state). The result's tables join `sim::tables`. Faithful to the original: past four goals' odds the count is 3 again, raised by the still-more odds; extra time's second odds draw from 1; the strength's scaling compares the long with 31, its low word with 0; strengths and forms add as wrapping words. A world's computer clubs fielding available players and its keepers' injuries (`WorldRules_CpuLineupsBegin`, `End`, `GkAfterSim`, the SWOS 96/97 Mod's) join with their components (#9); the career club's form after a match (`Career_ApplyPostMatchPlayerStatModsIfCareerTeam`: the first generator, form and wages only) is the career's (M7). The oracle compiles src/match_sim.c with the three entry points renamed `upstream_*` (`units.toml`'s per-unit `renamed`): the stubs of their names give the competitions' tests their set results, or upstream's on a test's ask (#8d); src/menus.c for the formations. Checked against the C: any tactics' formations; from any league, two sides (selected teams or the career club, mostly clean, now and then noise), setup, tactics, rules, game, computer statuses, career's answers, day and generator (pinned now and then), a match, its extra time and its shoot-out alike (goals, both records after byte for byte, scorers, the generator after), a side not in its league as upstream halts; England's 19 pairs of the real team file over 32 seeds each, with upstream's compiled tactics. |
| 8d ✅ | Competitions simulated to their end | A match's result asked of a source (`core::competition::results`: `MatchResults`; `Sides`, the identities, -1 the career club; `MatchSetup`, `Venue`), the source's refusal the competition's (`PlayError::Match`, `CupPlayError::Match`), each part drawn by the competition's generator: a league's normal time at upstream's point, the match begun and before its progress (`ProcessDIYLeagueMatchResultAndCareerUpdates`), on neutral ground in a league played once each way round, its benches `leagueMinSubstitutes` and `leagueMaxSubstitutes`; a cup's parts as its tie needs them (#6), its benches `minSubstitutes` and `pointsForAWin`; the goals' column the word of `compSlot` and `slotId`. `core::sim::Simulation` is the source of simulated matches (the simulated path of `DIY_PlayOrSimulateLeagueFixture` and `DIY_PlayCupMatchAndAdvance`; `getCachedOrCalcViewResult` and its two siblings with nothing cached, the cache a played match's, M6): the squads kept by identity (`Squads`: the selected teams, the career club); before each match its sides' statuses counted down (`countdown`, src/career_status.c `InitInGameTeams_WithMatchParam`: each status bucket falling by draws, the seventh never; then outside a training game each card a step under the competition's track of cards, else each injury, a minor one never; a computer club's only where its world opens the kind, in a DIY or a preset competition, `WorldRules_CpuCountdown`) and their match flags' bit 0 cleared; the result simulated (#8c) on the competition's month, its statuses in the competition's track; extra time from the last normal time. `SimRules.game` (`GameType`: a competition, a season, a career) replaces the career flag. Refused: a side naming no squad, both sides one squad (upstream plays one record as both), extra time before a normal time. A season's and a career's computer clubs count down each day instead (`WorldRules_CpuDay`, M7). The oracle compiles `InitInGameTeams_WithMatchParam` renamed (its stub hands over on a test's ask), stubs `WorldRules_CpuCountdown` (its calendar's day aside), keeps the squads in upstream's loaded teams (`Oracle::set_simulation`, `squads`) and lets a test assign team numbers to them (`assign_teams`: a tournament's teams share ordinals). Checked against the C: from any two sides, rules, game, computer statuses and generator, a countdown alike (both records, the generator after); every league (88 divisions), cup (62) and tournament (25) of World 1996-97 simulated to its end on both sides, England's real teams cycled as the squads with a career club, the session varied by competition (game, computer statuses, statuses applying, day): after each match the competition, the tournament's state, every squad and the second generator alike, and the parts of a tie asked. |
| 9 | World 2026-27's components | In steps, each with its data from the completion: the rules library (src/rules/, MIT: its notice kept), splits and zones, play-offs and play-outs, parallel groups, the league phase, the national cups' draws (`national_cups.json`), Super Cups, the continental cups, UEFA coefficients and access, state championships, the Serie D cup. The rules library first, a file or two at a time (upstream's copy at the pin, which has moved on from the tag v1 `87b6527`: OpenSWOS's own since 2026-11-11). Its own tests compare it with the Mod's C, which this repository does not fetch, over cases shaped as World 2026-27's leagues (its data, never committed); here the library's C at the pin is the oracle, over random cases, and the real shapes come with the completion's data (#9b on). |
| 9a1 ✅ | The rules library: moves, cups' clubs, parallel groups, the grouped cup | `core::competition::rules_library` (upstream src/rules/, MIT, Copyright Davide Lorigliola and Ben Baker: `LICENSE-MIT` kept beside the port, each file's notice kept; the port GPL like the game). A league's final `Tables` (16 divisions of 32 rows, the image whole as the library's state holds it) and its `Levels`. The season's moves: `final_moves` (`sr_final_moves`: each club by its record's place, the play-offs' `Move`s added, their sign, the divisions the league's), `second_team_moves` (`sr_b_teams`: a second team coupled with its first team goes down, unless it may not leave the lowest level, the best placed relegated of its division staying; one barred from going up, above the league's highest level for them or into its first team's, is held; its promotion passes to the next club that may go up). A cup's clubs from the second season: `cup_entrants` (`sr_cup_entrants`: each `Quota`'s division ordered relegated into it, stayed, promoted, without a place, then by place and slot, the career club first, the byes after round 1's clubs; none, the real list, when a quota cannot be met). Parallel groups: `parallel_groups` (`sr_pg_season`: each move a level's; into a level of groups, the club's first then second choice by its area byte, room made in five passes, else the nearest group with room; the staying clubs counted first). A grouped cup's later seasons: `GroupedCupEnd`, `grouped_cup_rounds` (`sr_cupd_*`: the entry order by last season's end, a preliminary round and a first round paired within the groups, the Mod's generator seeded from the season). Named by what they do: the Italian rules and Serie D's cup are data's (#9b on). Refused (`RulesError`): upstream's own refusals (a league past the groups' limits, a cup's rounds or divisions past its limits) and where the C reads or writes past its memory (tables or moves past the state, counts by place fewer than the divisions, a slot without a place or a team word, a quota's division without a level, a club staying above the chain, a preliminary round of more sides than clubs). The oracle compiles src/swos_rules_post.c, `_cups.c`, `_groups.c` and `_cupd.c` as upstream builds them. Checked against the C: random tables (the counts past the divisions in use read too), play-offs' moves, leagues, second teams (clubs of the tables mostly) and levels, the state's wants after both functions; random cups' quotas, slots, places, levels and career clubs; random chains of divisions and groups with clubs moving, staying and entering, past the limits now and then; random season ends and grouped cups' rounds, every side written and nothing more. |
| 9a2 ✅ | The rules library: the fixed tiers' brackets, the checks | A league's rules as one `League` (its divisions and counts by place, its second teams, its `FixedTiers`, its knockouts' spec). `league_check`, `spec_check` (`sr_league_check`, `sr_spec_check`): at most 16 divisions, the fixed tiers' 2 to 4 groups and upper tier within them, a spec of some bytes walked play-off by play-off (its kind, the promedio's matches and loser's division, at most 64 entrants and 1 to 16 rounds; each entrant's type, division, place and stage table; each round's members, meetings or legs, host, level and pairing, each member an entrant, a placing or a tie's winner so far, at most 64 placings and ties) up to its 0, the first fault a `CheckError` (its code, play-off, byte and field). `fixed_tier_brackets` (`sr_italy`, dplay.c `dp_season`, ported as the Mod wrote it: its generator, `generator::below`, shared with the grouped cup; seeded from the tables, each club's lot drawn first): the upper tier's 3rd up directly when 15 points clear of the 4th (`play_off_held`), else its play-off (5th v 8th and 6th v 7th single, the semi-finals and final two legs, level to the better placed, or extra time and penalties when level on points); its 17th down directly when 5 points behind the 16th (`play_out_held`), else the play-out's loser; the lower tier's groups' play-off (in each group 5th v 10th, 6th v 9th, 7th v 8th, then the winners and the 4th; national rounds seeded by the thirds, the cup winner's place, as the Mod 4.1 keeps it when its club is neither relegated nor in a play-out, and the best group qualifier, then the seconds and the best winner; a Final Four); a tie by the clubs' strengths, (points + 20) squared, the home side's 10% up, 32-bit and wrapping; the career club's real winners kept (`FixedTiersSeason`). Refused: the league's check (upstream's refusal), and a lower tier of four groups reaching its national rounds (`RulesError::GroupsPastBracket`: the C writes past its tables of 5). The oracle compiles src/swos_rules_dplay.c and `_check.c`. Checked against the C: random fixed tiers' leagues and tables (the upper tier and 2 to 4 groups at any divisions, 17 clubs or more mostly, points falling by small steps so the held rules' edges come, the career club's results now and then), every move or the refusal; the held rules over any points; random specs (play-offs of every kind, entrants and rounds near the limits, a byte overwritten or the bytes cut now and then) and leagues, every fault's code, play-off, byte and field. |
| 9a3 ✅ | The rules library: the pack's knockouts | `Knockouts::season` (`sr_knockouts`, gplay.c `gp_season` whole, ported as the Mod wrote it: its generator, seeds, tie model and order): a league's spec's play-offs in order, each its own draws seeded from its entrants. Every kind: promotion (a final group's top `winners`, else the last tie's winner, to the winners' division, its other entrants from there to the losers'; the reserve out of every division), play-out (the last round's losers down), title and continental (a final group's 1st and 2nd or the final's winner and loser, `KnockoutResult`s), promedio (Argentina's: the last of the promedios over three top-flight seasons and the last of the rest down, a level place a neutral play-off; the `Promedio` history the league's `PromedioSeed`s at a new career, shifted each season), stage title (its final's, or the host's when decided mid-season); the next season's supercup and the kinds the season end does not play skipped. Every entrant: by place (second teams that may not go up passed over, but in a play-out, when the season says), a stage table's place (the bank's this season's, else the final table; a 'keep' stage's frozen table), the previous play-off's loser, the stage titles' finalists, a kept stage's title decider's seeds, a title play-off's seeds, a club by team word (`League::team_file_high`; absent, left out of a group). Group rounds (round robins of any meetings, bonus points, a ranking round of the members' own groups' figures) and knockout rounds (single, two legs, best of three; listed, better placed, drawn or neutral hosts; level after the time to the better placed, or extra time and penalties; paired as listed, drawn or best v worst; clubs too far apart on points the better placed through), the drawn pot. The career club's own: the `KnockoutHost` (its ties, a title played as a group contest, stage titles decided mid-season) and its played tie; the season-end preview (no promedio, its own play-offs left, the page appended). The SEASON WINNERS page (`Winners`: the division played in, each division's 1st, each play-off's winner, a title play-off's last three ties). The state the season ends keep (`Knockouts`: results, stage titles, the groups' figures); the season's check (`season_check`, `sr_season_check`). Refused: the checks (upstream's refusal, nothing played), and where the C reads past its memory (a stage title past the three, an odd best v worst pairing, an absent entrant moved, a final group's winners past the 64 placings, second teams' words or counts by place fewer than read). Where a play-off not played reads a round it did not play and the value counts (a group's member, a placing's figures, a row read), the port reads none and says so (`KnockoutsOutcome::read_unplayed`). The oracle compiles src/swos_rules_gplay.c, the host's callbacks answering from tables. Checked against the C, one league's seasons one after another over one state (the stage tables, the promedio history and the page kept between them; every move, result, stage title, groups' figure, bank, history and page): random leagues of a few divisions (specs of every kind, entrant and round; banks, histories, hosts, played ties, previews, second teams); and the library's packs, World 2026-27's 35 leagues' specs and shapes (its `tests/rules/specs` at `87b6527`, downloaded into the cache, never committed), over random tables of their shapes. |
| 9a4 | The rules library: the career club's contests | `sr_club_playoff`, `sr_contest_entrants`, `sr_contest_seed`, `sr_contest_tie`, `sr_group_shape_of` (the season's check, `sr_season_check`, came with #9a3). |
| 9b | The components | Then the components on the completion's data, each split when reached: splits and zones, play-offs and play-outs (`league_playoffs.json`), parallel groups (`league_groups.json`), the league phase, the national cups' draws, Super Cups, the continental cups, UEFA coefficients and access, state championships, the Serie D cup. |

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
- Europe 1984-85 (D10), once World 1996-97 and World 2026-27 play: fetched, verified and
  completed as World 2026-27 is (M4 #5b, #5d), its tables from `world_data_8485.c`; its golden
  careers pass.

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
| The download changes or disappears (`openswos.net`, swos-port) | Fingerprint/checksum checks with clear errors. The pin records the expected fingerprint. CI keeps `target/cache/` between runs. |
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

None blocking. The `rules/` files are listed by the inventory (docs/WORLD_INVENTORY.md §4);
their schemas are fixed as M4 #5, M5 and M7 port what reads them.

- Which team file a competition plays with (`comp.team_number`: a country's, `0xFF` for the
  European pool, 72 for the custom teams) is not checked by `core::world` yet. The pool's
  first season is three lists of team ids: the PC game's `.TMD` records are its team files'
  records but for the global number, which the game recomputes on load
  (`SetTeamGlobalNumbers`), and the runtime team control (checked on the PC game, M4 #5).
  They join `rules/europe.json` (`first_season_clubs`) with its schema (M4 #5c). The custom
  teams are the player's, outside packages.
- **D9 and a world's foreign players' pool** (since upstream `6dce7b7`, 2026-11-21): World
  2026-27's package carries its own `teams/POOLPLYR.DAT`, RNC-packed, and the PC (SDL) game
  reads it (`LoadPoolplyrDat` through `LoadFileEx`). A world career's foreign market needs
  it (M7): either an RNC unpacker joins D9's exceptions, or the completion step (M4 #5)
  turns it into the `poolplyr` crate's unpacked file. Done in M4 #5d2, the second way:
  upstream's own unpacker (`src/rnc.c`; the pool is method 1, which checks its result) runs in
  the oracle, which `world_extract` already links; the completed package carries the pool
  unpacked. D9 stays whole: no importer reads RNC, and the port has no codec of its own.
