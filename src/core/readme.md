# openswos_core

The game logic of OpenSWOS_ng: the match engine, competitions and career. It is a
deterministic state machine driven by commands and player inputs. It returns events and
never calls a view.

- It has no rendering, audio, input or file-system access.
- It contains no world-specific branches: every rule comes from the loaded world package
  (World 1996-97, World 2026-27, ...).
- It uses no floating point and no hash-ordered collections: results must be bit-exact
  with the original game and identical on every platform.

See `docs/PLAN.md` §4 and §5.

## The world (`world`)

`World::from_package` turns a world package (`openswos_world_package_resource`) into a
typed, valid world: its identity, its countries, and its competitions with the tables that
tie them together. The rules upstream hardcodes join it as the inventory (M4 #4) and the
competition and career ports (M5, M7) give them files.

| part | from | what it is |
|---|---|---|
| `Countries` | `countries.json` | each country by number (its team file's) and `CN_*` name, with its roster's file; the list in its order |
| `League`, `Cup`, `Tournament`, `Playoff` | `leagues.json`, `cups.json`, `tournaments.json`, `playoffs.json` | the record as read, its links resolved (a league's play-off links, a draw's seeded teams as `TeamId`s) |
| `QualifyingGroup` | `qualifying.json` | its competitions and markers, its countries up to the end byte |
| `LeagueBlock` | `league_blocks.json` | its leagues, play-offs and next block |
| `CareerContest` | `career.json` | the career index's competitions, and their copies' names |
| `CountryTable` | `country_competitions.json` | a slot of upstream's `_competitionsTable`: a country's own leagues and cups, or a region's qualifying group |
| `SeedBucket` | `seed_buckets.json` | the UEFA Cup berths, by country |

References become typed handles (`LeagueRef`, `CupRef`, `CompetitionRef`, `GroupRef`,
`BlockRef`…) into the world that made them; `World::competition` finds a competition by its
id. The records' numbers are kept as read: what they mean is the competition engine's (M5).

**What is checked.** The package is checked whole, and every problem is reported
(`Invalid`), each naming its file and field:

```text
competitions/leagues.json: leagues[1] (_englishLeague).playoff_links[1]: no play-off or league block is called _nowhere
competitions/cups.json: competitions[1] (_cupWinnersCup).team_seed_list[0].country: CN_MARS is not a country of the world
```

- upstream's ten competition files, each at its path with its `type`; no other file in
  `competitions/`. Packages are self-contained (docs/PLAN.md §5.2): World 2026-27's
  download holds only `leagues.json`, and is a world once completed (M4 #5);
- one definition per id, across the six files that define them (upstream's registry, which
  must close: a dangling reference is an error);
- every reference names a record of the kind its place calls for: a league's links a
  play-off or a block, a country table's leagues a league and its cups a cup, a block's
  play-offs a play-off…;
- every country named (seeds by `CN_*` name or number, qualifying groups, berths, a World
  2026-27 league's new country, a country's own table) is one of the world's; a region's
  table slot need not be;
- each record of a type its file holds: `cups.json` holds leagues and cups (its
  qualifying groups, friendlies and tours play as leagues), the play-offs are cups;
- a country listed again must be listed the same, and a table slot listed once.
  Faithful to the original: upstream's list starts with BRAZIL, which it lists again in
  its place.

Not checked yet: which team file a competition plays with (`team_number`: the `.TMD` pools
and the custom teams are not in packages yet).

## The game data (`data`)

`GameData::new(world, teams, pool, tactics)`: the world with its rosters (the package's
`teams/*.json`), the transfer pool and the built-in tactics.

- `Rosters`: every team typed, with the ids its JSON states, found by identity
  (`TeamId`: upstream `TeamDb_Team`) or by id (`TeamDb_ByUuid`). A team whose stated checksum
  is not its players' is not canonical (upstream src/team_db.c): it plays, but is not
  offered as what its id stands for.
- `PresetTactics`: the twelve presets as OpenSWOS plays them (each required), and the
  other releases given.

Checked: every team converts; no identity and no id (team or player) is held twice; every
country's roster is in the package and holds only the country's teams (rosters no country
names, such as World 2026-27's pools and guests, are allowed); every team a draw starts
from and every pool player's team is in the rosters, and the player in its squad; every
preset has its shipped tactics.

## Sources

Upstream OpenSWOS: `src/save_format_json.c` (`DataJson_InstallCompetitionGraph`, the
registry, `dj_selftest_graph`), `src/team_db.c` (the team database, canonical teams),
`docs/DATA_INTERNAL_JSON.md`, `docs/TEAM_IDS.md`.

## Tests

- Unit tests: a small world (`src/fixture.rs`) builds, and each check reports what is
  wrong where, all at once; its game data, with teams, pool and tactics from `assets/`.
- `tests/world_1996_97.rs`: upstream's competition files (`assets/COMPETITIONS_JSON/`) are
  a valid world: 66 leagues, 76 cups, 14 tournaments, 7 play-offs, 76 table slots, 71
  countries (72 listed), every reference resolved.
- `tests/real_assets.rs` (`just test_real_assets <game_dir>`): World 1996-97's game data,
  its rosters made from the PC game's `DATA/TEAM.nnn` as `world_extract` will make them,
  with the game's pool and upstream's tactics, is valid: every seeded team and every pool
  player's team is in the rosters, every team canonical.
