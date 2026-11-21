# Runtime data

The files the game loads, committed. Files only the tests read are in `assets/` (see
`assets/README.md`).

```
data/
├── worlds/
│   └── 1996-97/            World 1996-97's package, without rosters (docs/PLAN.md §5.2)
│       └── competitions/   its competitions, countries and tables
├── tactics/                the built-in tactics, every release
└── career_params.json      the career's params
```

The tactics and the career params serve every world, as upstream loads them (from its
`data_json/` folder, not from a world package).

## What may be committed

- Only files the game loads, under a license that lets us ship them: upstream OpenSWOS's
  GPL data, and our own (e.g. World 1996-97's `rules/`, M4 #4–#5).
- Never original game data. World 1996-97's rosters come from your own copy of the game:
  `world_extract` assembles the full package in `target/cache/worlds/1996-97/` (M4 #5).
- Never World 2026-27's data: it is downloaded into `target/cache/`.

Tests read these files where they are (`openswos_test_utils::data_path` and its helpers),
never a copy in `assets/`.

## Provenance

Every committed file is listed here: where it comes from and under which license.

| File | Source | License / notes |
|---|---|---|
| `worlds/1996-97/competitions/*.json` (10 files) | Upstream OpenSWOS `bin/data_json/competitions/` at commit `ec2ed2be5c26e510f7c7b8d7af2710e5b909ca8b` (`just upstream_fetch`), unchanged | GPL-3.0-or-later (upstream's data) |
| `tactics/*.json` (20 files) | Upstream OpenSWOS `bin/data_json/tactics/` at the same commit, unchanged: the 12 built-in tactics, and the Amiga original's and DOS release's 4-4-2, 4-3-3, 3-4-3 and Sweep | GPL-3.0-or-later (upstream's data) |
| `career_params.json` | Upstream OpenSWOS `bin/data_json/career_params.json` at the same commit, unchanged | GPL-3.0-or-later (upstream's data) |
