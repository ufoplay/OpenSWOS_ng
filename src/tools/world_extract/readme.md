# openswos_world_extract

The tool that builds the world packages (docs/PLAN.md §5.2, M4 #5). It writes World 1996-97's
committed package, `data/worlds/1996-97/`, from upstream's checkout. Every other package goes
under `target/cache/worlds/` and is never committed: World 1996-97's assembled package holds
the PC game's rosters, and World 2026-27's data is the SWOS 96/97 Mod's (D8).

## Commands

| `just` recipe | command | what it does |
|---|---|---|
| `just world_extract_9697_data` | `world_extract extract-9697 [--out <dir>]` | World 1996-97's committed package, extracted again from the upstream checkout and its compiled tables, over `data/worlds/1996-97/`, or into a new or empty folder |
| `just world_extract_9697 <game_dir>` | `world_extract assemble-9697 --game <dir> [--out <dir>]` | World 1996-97: the committed package with your copy of the PC game's rosters and pool, into `target/cache/worlds/1996-97/` (replaced), or into a new or empty folder |
| `just world_fetch_2627` | `world_extract fetch-2627` | World 2026-27: its package downloaded into `target/cache/downloads/` and checked against the upstream pin |
| `just world_complete_2627` | `world_extract complete-2627 [--out <dir>]` | World 2026-27 completed: its download over World 1996-97, its rules from its baked install, into `target/cache/worlds/2026-27/` (replaced), or into a new or empty folder |

## World 1996-97's committed package, extracted

`data/worlds/1996-97/` holds no original game data, only what upstream's GPL source states:

- `competitions/`, `rules/career_params.json`: upstream's `bin/data_json/` files, as they are.
- the other `rules/` files (`openswos_world_rules_resource`):
  - read from upstream's 96/97 tables as compiled, through the oracle's `tables`
    (`openswos_oracle`, linked by this tool alone), so no table is transcribed by hand:
    - `country_details.json`: each country's continent and adjective, from `_countriesTable`;
    - `team_files.json`: the team files' bases, from `_teamsCountryNumbers`;
    - `season.json`: the season end's countries, from `_careerCompetitionIdRemapTable`;
    - `fixtures.json`: each size's round robin (2 to 24 clubs, `_fixtureSchedulePtrTable`
      over fixture_data.c, each with `FixtureRounds_Real` as its `rounds`), and the European
      Championships' own (`_euroChampFixtureData`'s first pairs, as many as their groups of
      4 play), the two records named by the competition ids upstream's code checks
      (`src/schedules.rs`);
    - `strings.json`: the texts of the compiled strings (`_stringSymbols`) the records'
      names name, the resets' names (`_aDiyLeague`, `_aDiyCup`, `_aDiyTournament`), the
      default division names (`_divisionStringTable`), cup name (`_aCup_0`), tournament
      name (`_aLeague_1`) and cup round names (`_cupRoundStringTable`), each by its symbol
      (`src/texts.rs`);
    - `competition_classes.json`: `_nationalTeamCompetitionsAndQualifications1` and
      `_europeanCupsandNationalTeamCompetitionsAndQualifications`, each record by its id (by
      competition id) and each career copy by `career.json`'s name for it
      (`_careerContests` pairs them), then the records of the ids `continental_record` names
      (the Copa) (`src/texts.rs`);
    - `draws.json`: the tournaments upstream draws by its own tables, which its code picks by
      competition id, each the world's tournament records of the ids, with each stage's draw
      from upstream's compiled tables (`own_draw_tables`: the European Championships' orders
      of eight and four, the World Cup's best placed patterns and map and its order of
      eight; the counts its code copies and the World Cup's semi-finals' table of the code's
      own stated) and how their groups rank (the World Cup's by group) (`src/draws.rs`). The
      completion reads the tables before World 2026-27's install;
  - the facts upstream keeps in code, not in a table: `src/extract.rs`'s constants, each
    citing its source (the first year, a club coach's own country first, no second team
    files or kept squad numbers, odd-sized tournament groups refused).

The package is checked (a valid world, `openswos_core`) before it is written. Without the
oracle (no checkout, `just upstream_fetch`; bindgen needs libclang) it cannot be extracted: the
tool says so, and builds all the same.

## World 1996-97, assembled

- **`competitions/`, `rules/`**: the committed package, `data/worlds/1996-97/`: upstream's
  ten competition files, its career params (D12) and the extracted rules. It has no
  `WORLD.ini`: the fingerprint covers the rosters, which only the assembled package holds. A
  file there that a package does not hold is refused.
- **`teams/<COUNTRY>.json`**: one roster for each country of `countries.json`, made from the
  game's `DATA/TEAM.nnn` as upstream's Setup makes its team JSON (tools/swosdata/lib/
  swosdata.c `step_teams`, team_json.c `Sd_TeamFileToJson`). Each is laid out as World
  2026-27's package lays out its clubs (two spaces, unsigned numbers, a trailing newline),
  and every team gets its derived ids and checksum (upstream docs/TEAM_IDS.md).
- **`teams/POOLPLYR.DAT`**: the game's foreign players' pool, unpacked, at the path World
  2026-27's package uses.
- **`WORLD.ini`**: tag `9697`, name `World 1996-97`, a comment saying the package holds the
  game's data, and the fingerprint of what is written.

The first season's European clubs are not in the package yet. The PC game's `.TMD` pools are
its team files' records, but for the global number the game recomputes on load and the
runtime team control, so they become three lists of team ids in `rules/europe.json`, with
its schema (M4 #5c).

## World 2026-27, fetched

The download cache pins the zip's checksum. The tool then checks the package's `WORLD.ini`:

- its fingerprint must be its files' (upstream's algorithm, `openswos_world_package_resource`);
- it must be the fingerprint the upstream pin was built with: `WorldData2627_DataFingerprint`
  in the checkout's `src/world_data.c`. Otherwise the error says "package and upstream pin out
  of step".

The checkout is the `upstream` tool's: `OPENSWOS_UPSTREAM_DIR`, else
`target/cache/upstream/OpenSWOS` (`just upstream_fetch`).

## World 2026-27, completed

Upstream lays World 2026-27 over 96/97 at start-up: its baked tables (`world_data.c`), then
its JSON over 96/97's (docs/WORLD_INVENTORY.md §9). The completion does it once, ahead of time,
so the package is self-contained:

- **The oracle runs the baked install** (`openswos_oracle::tables::world_2627`): the countries
  resized to the world's clubs, its new countries' entries, its accessors' answers. The
  install writes 96/97's tables for good, so it runs once a process: the tool's, or a test
  file of its own.
- **From the download, as they are**: `WORLD.ini`'s identity, `mod.json`, the 107 rosters (they
  hold every 96/97 country), the national teams, the B teams, the stadiums.
- **Its foreign pool, unpacked**: the download's `teams/POOLPLYR.DAT` is RNC-packed. Upstream's
  own decoder (`src/rnc.c`, through the oracle's `unpack`, which checks the header first)
  unpacks it before the install, and the package carries it unpacked, so no importer reads RNC
  (D9).
- **96/97's competition files**, with these changes:
  - the world's leagues replace 96/97's by id, keeping 96/97's play-off links (upstream
    `install_league`, `dj_inplace_load`);
  - its 26 new leagues come in, each with its country (`countries.json`: its name, a `CN_*`
    name, its roster) and its table (`country_competitions.json`);
  - 96/97's customs league stays;
  - its European cups are its three league phases (upstream `LeaguePhase_InstallCups`):
    the install's records, converted field for field into `tournaments.json`
    (`_uefaChampionsLeague`, `_uefaEuropaLeague`, `_uefaConferenceLeague`, their ids made of
    their names, their seeds named after the world's clubs), named by the career index's first
    three contests and by Europe's group in place of 96/97's cups. 96/97's records stay:
    upstream's world modules copy them as templates, and the world does not play them
    (`World::is_played`);
  - its 101 national and league cups (upstream `WorldCups_Install`), in its order: each
    cup's record, read by the oracle field for field (its whole list of clubs, up to the
    English cup's 64), replaces 96/97's of its id in `cups.json` (51 do) or joins it (50).
    Its country's table names it in its slot: 1 the national cup, 2 the league cup. The
    table grows by one when the slot is the next, and a league cup whose country has no
    national cup is dropped, as upstream drops it. A cup past upstream's tables (96
    countries of 12 entries) is refused rather than dropped. The ids are spelled only in the
    C: each row of upstream's generated table (`kWorld2627Cups` in `src/world_data.c`)
    names its record (`&kWorld2627_italianCup`: `_italianCup`), and its country and slot are
    checked against the oracle's. The cups' draws (the real bracket, the quotas, the
    European place) join `rules/national_cups.json` with M5.
- **96/97's career params** (D12).
- **The seven rules files**:
  - `calendar.json`: 2026;
  - `country_details.json`: each country's continent and adjective, from the countries'
    table the install leaves;
  - `team_files.json`: the bases the install leaves, each national file then grown to the
    world's national teams as upstream's SDL build grows it (`fit_world_national_files`:
    Europe's 55); the second files and kept squad numbers the accessors give;
  - `season.json`: the season end's 91 countries;
  - `fixtures.json`: the round robins the install leaves (`FixtureRounds_Install`: the odd
    sizes 5 to 23 as real rounds), read through the oracle; the competitions that play their
    own, as World 1996-97's (upstream picks them by id in every world); odd-sized tournament
    groups allowed;
  - `strings.json`, `competition_classes.json`: World 1996-97's (upstream's compiled strings
    and lists, which name 96/97's records and copies: the world keeps them), the world's
    records of `continental_record`'s ids joining the continental class;
  - `draws.json`: the world's tournament records of the ids upstream's own draws name (the
    same as World 1996-97's: the world keeps them).

The result is checked before it is written: a valid world with valid game data
(`openswos_core`: its 107 rosters, its pool of 900, the built-in tactics; every seeded team of
a competition it plays its own). Its fingerprint is its own: it is other data than the
download.

## Checks

Before a package is written it must be a valid world, with valid game data: rosters, pool and
the built-in tactics (`data/tactics/`) together, checked by `openswos_core`. The written
folder is then opened again, and its `WORLD.ini` must state its files' fingerprint.

## Re-extraction (M4 #5e)

The tests extract again at the pin and diff (`diff`), so an upstream bump, or a change to the
tool, that changes a package fails and names each file that differs, with its first line that
does:

- World 1996-97's extraction against the committed `data/worlds/1996-97/`
  (`just world_extract_9697_data` writes it again);
- World 2026-27's completion against its committed SHA-256 digests
  (`assets/WORLD_2026_27/completion.json`): its files may not be committed, their digests
  may, so the check holds on any machine. When the cached completion is the one the digests
  know, the failure diffs against it line by line. After review,
  `OPENSWOS_BLESS=1 just test world_2627_digests` writes the digests again, and
  `just world_complete_2627` the cache.

## Tests

- Unit tests:
  - the game folder's files found in any case;
  - a roster equal byte for byte to `assets/TEAM_JSON/OCEANIA.json` (made from `TEAM.085`);
  - the committed package refusing a stray file;
  - the checks naming what is missing;
  - the pin's fingerprint read from upstream's generated code;
  - the compiled records' bytes, names and seeds as the JSON writes them;
  - the cups' rows read from upstream's generated table; a cup record converted field for
    field; a cup taking its slot, growing its table, replacing its id; a league cup without
    a national cup dropped; rows out of step, an unknown slot, a region's table and a cup
    past upstream's tables refused;
  - a package out of step, or stating another fingerprint, refused;
  - a folder refused or replaced;
  - a file's digest its SHA-256; the digests written and read back; files differing by name
    and by line, digests by name.
- `tests/real_assets.rs` (`just test_real_assets <game_dir>`): World 1996-97 assembled from
  your copy (71 rosters, 1,583 teams, a pool of 900), opened again, every team canonical; and
  assembled twice to the same fingerprint.
- `tests/world_2627.rs`: the downloaded package is the pin's data. Skipped without the
  network or the checkout.
- `tests/world_2627_completed.rs` (a process of its own): World 2026-27 completed is a valid
  world with valid game data, from the download and the oracle (97 countries, 92 leagues, 107
  rosters, a pool of 900; Italy's 256 clubs, Europe's 55 national teams, the season's 91
  countries, the new countries' continents, England's play-off links kept; the league phases
  in the career, 96/97's European cups unplayed; 126 cups, Italy's and Wales' tables with
  their league cups, Andorra's with its cup, the English cup's 64 clubs). Skipped without the
  oracle or the network.
- `tests/committed.rs`: `data/worlds/1996-97/` is the extraction from the pinned checkout,
  file for file, byte for byte (after an upstream bump that changes it,
  `just world_extract_9697_data`). Skipped without the oracle.
- `tests/world_2627_digests.rs` (a process of its own): World 2026-27's completion is the one
  its committed digests know. Skipped without the oracle or the network.
