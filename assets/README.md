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

- The full original game data. It lives in your own copy: `assets/local/` (gitignored),
  or the folders named by `OPENSWOS_GAME_DIR` (the PC game) / `OPENSWOS_ASSETS_DIR` (an
  OpenSWOS Setup output folder).
- World 2026-27's data and the swos-port `.RGD` recordings. They are downloaded on demand
  into `target/cache/` (their data is not ours to redistribute).

## Provenance

Every committed file is listed here: where it comes from and under which license.

| File | Source | License / notes |
|---|---|---|
| | | |
