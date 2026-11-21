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
typed, valid world: its identity, its countries, its competitions with the tables that tie
them together, and the rules it reads so far. The rules upstream hardcodes join it as their
files come (docs/WORLD_INVENTORY.md §4; M4 #5c, M5, M7).

| part | from | what it is |
|---|---|---|
| `Countries` | `countries.json` | each country by number (its team file's) and `CN_*` name, with its roster's file; the list in its order |
| `League`, `Cup`, `Tournament`, `Playoff` | `leagues.json`, `cups.json`, `tournaments.json`, `playoffs.json` | the record as read, its links resolved (a league's play-off links, a draw's seeded teams as `TeamId`s) |
| `QualifyingGroup` | `qualifying.json` | its competitions and markers, its countries up to the end byte |
| `LeagueBlock` | `league_blocks.json` | its leagues, play-offs and next block |
| `CareerContest` | `career.json` | the career index's competitions, and their copies' names |
| `CountryTable` | `country_competitions.json` | a slot of upstream's `_competitionsTable`: a country's own leagues and cups, or a region's qualifying group |
| `SeedBucket` | `seed_buckets.json` | the UEFA Cup berths, by country |
| `first_year`, `continent` | `rules/calendar.json`, `rules/country_details.json` | the year the first season starts; each country's continent |
| `TeamNumbering` | `rules/team_files.json` | each team file's first global team number (a club's global number: its file's base plus its place), the second team files, whether squad numbers are kept |
| `SeasonEnd` | `rules/season.json` | the season end's countries, in its order; a club coach's own country first |
| `Schedules` | `rules/fixtures.json` | the round robins (`competition::schedule`): each size's pairs in playing order and whether it plays real rounds, and the competitions that play their own (`Schedules::of`: a competition's own, else its size's); whether tournament groups may be odd-sized |

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
  its place;
- the rules files `calendar.json`, `country_details.json`, `team_files.json`, `season.json`
  and `fixtures.json`, each required (no world's behaviour is a default): every country with one entry of details
  and no other number; 256 bases never going down; a second file a country's, numbered as
  no country is; the season end's countries the world's, each once; each round robin every
  pair of its two or more clubs once, real rounds where it says so, a size once; a
  competition's own round robin of the size its record states (a league's divisions, a
  tournament's groups; a cup played as a league draws its groups, a knockout plays none),
  each competition once.

Not checked yet: whether every division has the round robin it plays (M5 #5: World
2026-27's leagues in zones play their own pairs). Which team file a competition plays with (`team_number`): the European
pool's first season joins `rules/europe.json` (M4 #5c), and the custom teams are the
player's.

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
names, such as World 2026-27's pools and guests, are allowed); every team a draw of a
competition the world plays starts from and every pool player's team is in the rosters, and
the player in its squad; every preset has its shipped tactics.

`World::is_played`: a competition the career index, a country table or a qualifying group
names, or that a league it plays leads to (a play-off, a block's leagues and play-offs). A
record nothing names is never drawn: World 2026-27 keeps 96/97's European cups, which its
league phases replace, because upstream's world modules copy them as templates.

## Sources

Upstream OpenSWOS: `src/save_format_json.c` (`DataJson_InstallCompetitionGraph`, the
registry, `dj_selftest_graph`), `src/team_db.c` (the team database, canonical teams),
`docs/DATA_INTERNAL_JSON.md`, `docs/TEAM_IDS.md`.

## Tests

- Unit tests: a small world (`src/fixture.rs`) builds, and each check reports what is
  wrong where, all at once; its game data, with teams and pool from `assets/` and the
  game's tactics (`data/tactics/`).
- `tests/world_1996_97.rs`: World 1996-97's committed package (`data/worlds/1996-97/`,
  upstream's competition files) is a valid world: 66 leagues, 76 cups, 14 tournaments, 7
  play-offs, 76 table slots, 71 countries (72 listed), every reference resolved.
- `tests/real_assets.rs` (`just test_real_assets <game_dir>`): World 1996-97's game data,
  its rosters made from the PC game's `DATA/TEAM.nnn` as `world_extract` makes them,
  with the game's pool and the built-in tactics, is valid: every seeded team and every pool
  player's team is in the rosters, every team canonical.

## The competition engine (`competition`)

The generic engine of docs/PLAN.md M5, unit by unit. Today:

- `schedule`: the round robins a world states. `Schedule` is one cycle's pairs (`Pair`: home
  and away, by place) in playing order, whether it plays real rounds, and how many pairs a
  match day counts; `Schedules::of(competition, teams)` gives a competition's own, else its
  size's (upstream diy_competition.c `getCurrentFixtureTeamPtrs`: the European Championships'
  by id, else `_fixtureSchedulePtrTable[teams - 2]`). The engine has no table of its own.
- `table`: a league table. `Line` is a team's (upstream `DIYTeamStats`: 16-bit counts that
  wrap, the goal difference in a signed word); `record` writes a result into the two lines
  (a win the competition's points, a draw a point each, the goals both ways; the games are
  counted with the progress, `Line::count_game`); `sort_groups` orders each group as
  upstream's `DIY_SortLeagueTableByStandings` does: points, goal difference, goals, the
  name byte for byte, by its exchange sort (not stable, faithfully).
- `value`: a team's average price (its 16 players' price bytes, `average_price`) and a
  competition's `Strength` (the average over all its teams' players, and its tier: four
  prices a tier, at most 7).
- `draw`: `shuffle`, the one every draw of upstream's makes (from the last place down, each
  swapped with the place `Rand2() × n >> 8`), `shuffled_order`, and `seed_by_value`, the
  seeded draw: ordered by value (a bubble sort, highest first), then each pot shuffled
  (`Pots::Groups`: a pot per group place; `Pots::Quarters`: a knockout's four, left
  unshuffled when a quarter is two teams or fewer). Every draw comes from the second
  generator (`openswos_rng`), in upstream's order.
- `running`: a competition under way (upstream `DIYCompetitionEx`). `RunningCompetition` is
  upstream's image itself (2,640 bytes, its 64-bit build's layout), each field a typed
  accessor at its offset (`num_teams`, `team_entry_offset(place)`, `cup_round_legs(round)`,
  `row(index)`…). Upstream's unions are two sets of accessors over the same bytes (the match
  data's league and cup views; the shuffle order and a cup bracket's scratch), and an index
  past an array's end reaches the bytes after it, as upstream's code does (a league's
  fifteenth team id is `competitionFormat`). So a competition is checked against the C on
  the same image, and a save's image is written back as read.
- `league`: a league started. `start_division` makes a division of a `LeagueRecord` (a
  `leagues.json` league, or a `cups.json` cup played as a league, read through the league's
  layout as upstream reads it) the competition under way, as upstream's
  `InitDIYCompetitionFromTableEntry` does: the reset, the record's fields, the name, the
  derived counts and months, each team's identity and the strength, the places shuffled. Its
  texts and classes (the division's name, the country's adjective, the default name, the
  continental track) come in resolved (`DivisionStart`).
