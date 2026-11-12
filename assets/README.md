# Test assets

Fixtures for the tests: one folder per file format (`assets/TEAM/`, `assets/TMD/`, ...).
Each fixture is the file itself plus a JSON sidecar with the expected decoded model
(`name.ext` + `name.json`). See `docs/PLAN.md` §6.

## What may be committed

- A **small, curated set of original game files**, only where a test needs real data.
  Keep it small.
- Synthetic fixtures for edge cases.
- Upstream OpenSWOS's GPL data files (e.g. `bin/data_json/...`).

## What must never be committed

- The full original game data. It stays in your own copy, outside the repository. Its
  folder is passed as a command-line argument or through `OPENSWOS_GAME_DIR` (the PC game)
  / `OPENSWOS_ASSETS_DIR` (an OpenSWOS Setup output folder). Anything derived from it goes
  to `target/cache/`.
- World 2026-27's data and the swos-port `.RGD` recordings. They are downloaded on demand
  into `target/cache/` (their data is not ours to redistribute).

## Provenance

Every committed file is listed here: where it comes from and under which license.

| File | Source | License / notes |
|---|---|---|
| `TEAM/TEAM.008` | PC SWOS 96/97 (GOG), `DATA/TEAM.008`: England, 93 teams | Original game data © its rights holders; a small test excerpt (D2) |
| `TEAM/TEAM.057` | PC SWOS 96/97 (GOG), `DATA/TEAM.057`: South Korea, 1 team | Original game data © its rights holders; a small test excerpt (D2) |
| `TEAM/TEAM.085` | PC SWOS 96/97 (GOG), `DATA/TEAM.085`: Oceania, 4 teams | Original game data © its rights holders; a small test excerpt (D2) |
| `TEAM/TEAM.057.json`, `TEAM/TEAM.085.json` | Written by `openswos_team_resource`'s tests (`OPENSWOS_BLESS=1`), reviewed | The expected decoded models |
| `TMD/EUROCUP.TMD` | PC SWOS 96/97 (GOG), `DATA/EUROCUP.TMD`: the 16 Champions' Cup teams | Original game data © its rights holders; a small test excerpt (D2) |
| `TEAM_JSON/OCEANIA.json` | Written by `openswos_team_json_resource`'s tests (`OPENSWOS_BLESS=1`) from `TEAM/TEAM.085`, as World 1996-97's team JSON (World 2026-27's style: unsigned numbers, trailing newline) | Derived from original game data (D2); the expected team JSON of that file |
| `POOLPLYR/POOLPLYR.DAT` | PC SWOS 96/97 (GOG), `DATA/POOLPLYR.DAT`: the 900-player transfer pool | Original game data © its rights holders; a small test excerpt (D2) |
