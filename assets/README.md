# Test assets

Fixtures for the tests: one folder per file format (`assets/TEAM/`, `assets/TMD/`, ...).
Each fixture is the file itself plus a JSON sidecar with the expected decoded model
(`name.ext` + `name.json`). See `docs/PLAN.md` §6.

Only the tests read these files. The files the game loads (upstream's GPL data: World
1996-97's competitions, the built-in tactics, the career params) are in `data/` (see
`data/README.md`), and the tests read them there.

## What may be committed

- A **small, curated set of original game files**, only where a test needs real data.
  Keep it small.
- Synthetic fixtures for edge cases.
- The expected outputs of the tests (sidecars, files written from a fixture).

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
| `TAC/01_442DOS.TAC` | SWOS 2020 (community release of the PC game), `swos/tactics/dos/01_442DOS.TAC`: a DOS 4-4-2, 370 bytes | Game data © its rights holders; a small test excerpt (D2) |
| `TAC/06_433.TAC` | SWOS 2020, `swos/tactics/common/06_433.TAC`: a 4-3-3 whose name keeps bytes after its end | Game data © its rights holders; a small test excerpt (D2) |
| `TAC/04_5-3-2.tac` | SWOS 2020, `tactics/cpu-tactics/tactics/04_5-3-2.tac`: a CPU 5-3-2, 376 bytes (six trailing zeros) | Game data © its rights holders; a small test excerpt (D2) |
| `DIY/2005CLFINAL.DIY` | SWOS 2020, `swos/2005CLFINAL.DIY`: a DOS DIY cup, the 2005 Champions League final (2 teams), 6559 bytes | Game data © its rights holders; a small test excerpt (D2) |
| `DIY/EC1964.diy` | SWOS 2020, `swos/EC1964.diy`: a DOS DIY cup, UEFA Euro 1964 (4 national teams), 7927 bytes | Game data © its rights holders; a small test excerpt (D2) |
| `HIL/26-1111-1654-1-0.HIL` | SWOS 2020, `swos/highlights/26-1111-1654-1-0.HIL`: a DOS highlights file of one goal (Spain v Germany, World Cup), 64626 bytes | Game data © its rights holders; a small test excerpt (D2) |
| `HIL/LATEST-100.RPL` | Cut from SWOS 2020's `swos/replays/LATEST.RPL` (Lazio v Advanced, team training, 2.35 MB): its header unchanged, the first 100 frames of its scene, then the end word; 15630 bytes | Game data © its rights holders; a small test excerpt (D2) |
| `TAC/01_442DOS.TAC.json` | Written by `openswos_tac_resource`'s tests (`OPENSWOS_BLESS=1`), reviewed | The expected decoded model |
