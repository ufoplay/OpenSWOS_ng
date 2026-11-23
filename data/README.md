# Runtime data

The files the game loads, committed. Files only the tests read are in `assets/` (see
`assets/README.md`).

```
data/
├── worlds/
│   └── 1996-97/            World 1996-97's package, without rosters (docs/PLAN.md §5.2)
│       ├── competitions/   its competitions, countries and tables
│       └── rules/          its rules (docs/WORLD_INVENTORY.md §4): the career params,
│                           the calendar, the countries' continents, the team numbering,
│                           the season end's countries, the fixture behaviours
└── tactics/                the built-in tactics, every release
```

`data/worlds/1996-97/` is written by `just world_extract_9697_data` (`world_extract
extract-9697`) from the pinned upstream checkout: never edit it by hand. A test checks it
against a fresh extraction, file for file.

The tactics serve every world, as upstream loads them (from its `data_json/` folder, not
from a world package). The career params are a world's own (docs/WORLD_INVENTORY.md §3.3):
World 1996-97 carries upstream's file, and the completed worlds carry the same file, as
upstream plays them with it.

## What may be committed

- Only files the game loads, under a license that lets us ship them: upstream OpenSWOS's
  GPL data, and our own (e.g. World 1996-97's `rules/`, M4 #4–#5).
- Never original game data. World 1996-97's rosters come from your own copy of the game:
  `world_extract` assembles the full package in `target/cache/worlds/1996-97/` (M4 #5).
- Never the downloaded worlds' data (World 2026-27, Europe 1984-85): it is downloaded into
  `target/cache/`.

Tests read these files where they are (`openswos_test_utils::data_path` and its helpers),
never a copy in `assets/`.

## Provenance

Every committed file is listed here: where it comes from and under which license.

| File | Source | License / notes |
|---|---|---|
| `worlds/1996-97/competitions/*.json` (10 files) | Upstream OpenSWOS `bin/data_json/competitions/` at commit `df643c28354d454cfe2788613b38b7b142282d1a` (`just upstream_fetch`; the same since the baseline `ec2ed2be`), unchanged | GPL-3.0-or-later (upstream's data) |
| `tactics/*.json` (20 files) | Upstream OpenSWOS `bin/data_json/tactics/` at the same commit, unchanged: the 12 built-in tactics, and the Amiga original's and DOS release's 4-4-2, 4-3-3, 3-4-3 and Sweep | GPL-3.0-or-later (upstream's data) |
| `worlds/1996-97/rules/career_params.json` | Upstream OpenSWOS `bin/data_json/career_params.json` at the same commit, unchanged | GPL-3.0-or-later (upstream's data) |
| `worlds/1996-97/rules/country_details.json`, `team_files.json`, `season.json` | Written by `world_extract extract-9697` from upstream's tables at the same commit, as compiled (read through the oracle): `_countriesTable` (src/strings_data.c), `_teamsCountryNumbers` (src/swos_amiga_data_imports.c), `_careerCompetitionIdRemapTable` (src/competition_data.c) | GPL-3.0-or-later (derived from upstream's source) |
| `worlds/1996-97/rules/strings.json`, `competition_classes.json` | Written by `world_extract extract-9697` from upstream's tables at the same commit, as compiled (read through the oracle): `_stringSymbols` (src/string_symbols.c, over src/strings_data.c's strings), `_aDiyLeague`, `_aDiyCup`, `_aDiyTournament`, `_aCup_0`, `_aLeague_1` (src/swos_amiga_strings.c), `_divisionStringTable`, `_cupRoundStringTable`, `_nationalTeamCompetitionsAndQualifications1`, `_europeanCupsandNationalTeamCompetitionsAndQualifications` (src/swos_amiga_ptr_imports.c), `_careerContests` (src/competition_data.c); the Copa by `continental_record`'s id (diy_competition.c, cited in `src/tools/world_extract/src/texts.rs`) | GPL-3.0-or-later (derived from upstream's source) |
| `worlds/1996-97/rules/draws.json` | Written by `world_extract extract-9697`: the competition ids and first-stage places upstream's own draws keep in code (src/diy_competition.c `buildCompSpecificTeamList`, `buildCompSpecificTeamShuffle`), cited in `src/tools/world_extract/src/draws.rs`, each the package's tournaments of the ids | GPL-3.0-or-later (derived from upstream's source) |
| `worlds/1996-97/rules/calendar.json` | Written by `world_extract extract-9697`: a fact upstream keeps in code (src/world.c `kFirstYear`), cited in `src/tools/world_extract/src/extract.rs` | GPL-3.0-or-later (derived from upstream's source) |
| `worlds/1996-97/rules/fixtures.json` | Written by `world_extract extract-9697` from upstream's tables at the same commit, as compiled (read through the oracle): the round robins of 2 to 24 clubs (src/fixture_data.c through `_fixtureSchedulePtrTable`, `FixtureRounds_Real`), the European Championships' (`_euroChampFixtureData`, src/swos_data_bulk_imports.c); odd-sized tournament groups refused (diy_competition.c, cited in `src/tools/world_extract/src/extract.rs`) | GPL-3.0-or-later (derived from upstream's source) |
