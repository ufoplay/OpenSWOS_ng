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
| `League`, `Cup`, `Tournament`, `Playoff` | `leagues.json`, `cups.json`, `tournaments.json`, `playoffs.json` | the record as read, its links resolved (a league's play-off links, a draw's seeded teams as `TeamId`s), its names resolved to their bytes (`rules/strings.json`) |
| `QualifyingGroup` | `qualifying.json` | its competitions and markers, its countries up to the end byte |
| `LeagueBlock` | `league_blocks.json` | its leagues, play-offs and next block |
| `CareerContest` | `career.json` | the career index's competitions, and their copies' names |
| `CountryTable` | `country_competitions.json` | a slot of upstream's `_competitionsTable`: a country's own leagues and cups, or a region's qualifying group |
| `SeedBucket` | `seed_buckets.json` | the UEFA Cup berths, by country |
| `first_year`, `continent`, `adjective` | `rules/calendar.json`, `rules/country_details.json` | the year the first season starts; each country's continent and adjective |
| `TeamNumbering` | `rules/team_files.json` | each team file's first global team number (a club's global number: its file's base plus its place), the second team files, whether squad numbers are kept |
| `SeasonEnd` | `rules/season.json` | the season end's countries, in its order; a club coach's own country first |
| `Texts` | `rules/strings.json` | the texts the records' names name (a name is its symbol's text, or itself, a byte a character), the names a league's, a cup's and a tournament's resets write first, the default division names and the cup's and tournament's names |
| `is_national_teams`, `is_continental`, `name_prefix` | `rules/competition_classes.json` | the national teams' competitions and the continental suspensions' contests, each played from its record or from the career's copy (`Source`); the adjective a competition's name takes (its record's country's, none for a national teams' one) |
| `own_draw` | `rules/draws.json` | the tournaments whose stages are drawn by their own tables (upstream picks the World Cup and the European Championships by id): each stage's draw (`OwnDraw`: places kept, an order, or the order the best placed's groups set) and how its stages of groups rank |
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
- the rules files `calendar.json`, `country_details.json`, `team_files.json`, `season.json`,
  `fixtures.json`, `strings.json`, `competition_classes.json` and `draws.json`, each required (no world's
  behaviour is a default): every country with one entry of details and no other number;
  every name, string and adjective a byte a character (Latin-1, no NUL); each class's id a
  competition's or a career copy's; each own draw's ids tournaments, each once, a stage at
  least, every place within the order's 64, a pattern's groups within a byte, each ordering
  every best placed; 256 bases never going down; a second file a country's, numbered as
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
  continental track) come in resolved (`DivisionStart`); `WorldDivision::resolve` resolves
  them from a world, for a league or a cup played as one, a division, and the record it is
  played from (`Source`). A league is played: `fixture` (the next pair, from the round robin
  its groups play), `advance` (the progress after a match), `play_result` (a match's result
  asked of a `MatchResults` source at upstream's point and recorded: the progress, the
  table, and at the season's end the qualifiers),
  `sort_table`, `update_qualifiers` (the winner and the play-offs' qualifiers),
  `season_moves` (the promoted and relegated, by identity).
- `cup`: a cup started. `start_cup` makes a `CupRecord` (a `cups.json` knockout, or a
  `playoffs.json` play-off, read through the cup's layout as upstream reads it) the
  competition under way, as upstream's `InitDIYCupFromRecord` does: the reset
  (`InitDIYCup`), the record's fields and its bracket's rounds' rules, the name, the bracket
  (`DIY_InitCupBracketState`: each team's identity and the teams the computer does not
  control, the pairs and rounds, the first round's format, the matches' months with a
  replay's date, the strength), the record's rounds, the pairings shuffled unless the record
  keeps its order. `WorldKnockout::resolve` resolves its texts and classes from a world
  (`CupStart`). A cup is played: `decide` (a tie decided from its match's results, each
  part asked of a `MatchResults` source as it is needed), `play_cup_match` (a match's result
  kept in the bracket,
  the round's and the cup's end, the next round's draw), `next_round` (the second legs, the
  replays, or the winners into the next round), `round_name` and `stage_name` (a round's name
  among the world's `cup_round_name`s).
- `tournament`: a tournament's state and its stages. `TournamentState` is upstream's
  `DIYTournamentState` image (1,120 bytes), each field a typed accessor as `running`'s (the
  same header accessors; the team ids, group map and team order; the six stage slots by
  stage: `slot_num_teams(stage)`, `slot_month(stage, index)`…). `reset` gives it a DIY
  tournament's defaults (`InitDIYTournament`); `recalculate` counts each stage's matches,
  their months and the teams' strength (`DIYTournament_RecalculateMatchesPerTeam`).
  `start_tournament` starts a `TournamentRecord` (a `tournaments.json` record, its seeds the
  world's identities) as upstream's `InitDIYLeagueFromRecord` does: the reset, the seeds
  ordering the first stage, the stages into the slots, the stages counted, then the first
  stage loaded into the competition under way (`DIY_LoadCurrentCompetitionSlot`): a stage of
  groups as a league (its draw: kept, shuffled, or seeded into pots and dealt into the
  groups; each team's group), a knockout as a cup (kept, shuffled, or seeded into quarters
  and each pair's home side drawn; its bracket). `WorldTournament::resolve` resolves its
  texts, classes, own draw and odd groups from a world (`TournamentStart`). `next_stage`
  moves it on after a stage as upstream's `DIY_AdvanceToNextCompetitionSlot` does: a stage
  of groups' teams ranked (`Ranking`: the engine's by place, or a world's by group) or a
  knockout's winners first, into the order the next stage draws from, then that stage
  loaded (any stage: a later stage of groups deals the ranked order on, a later knockout
  keeps it; a world's `OwnDraw` from its tables), or the tournament ended after its last
  (`Advance`).
- `results`: a match's result as a competition asks for it (docs/PLAN.md M5 #8d): a
  `MatchResults` source (normal time, extra time, penalties, each drawn by the competition's
  generator: the simulation's, `sim::Simulation`, or the match engine's, M6), the match's
  `Sides` (the identities) and its `MatchSetup` (the `Venue`, the benches, the goals'
  column).
- `rules_library`: the rules library (docs/PLAN.md M5 #9a; upstream src/rules/, MIT,
  Copyright Davide Lorigliola and Ben Baker: its licence kept beside the port in
  `rules_library/LICENSE-MIT`). A league's final `Tables` and `Levels`; the season's moves
  (`final_moves` by place and the play-offs' `Move`s, `second_team_moves` for the second
  teams); a cup's clubs from the second season by its `Quota`s (`cup_entrants`); parallel
  groups at the season end, by the clubs' areas (`parallel_groups`); a grouped cup's later
  first two rounds (`GroupedCupEnd`, `grouped_cup_rounds`); a league's rules (`League`) and
  their checks (`league_check`, `spec_check`: the first fault a `CheckError`); the fixed tiers'
  brackets (`fixed_tier_brackets`: an upper tier's play-off and play-out, a lower tier of
  groups' play-off, `play_off_held`, `play_out_held`); the pack's knockouts at the season end
  (`Knockouts::season`: every play-off kind, entrant and round, the moves and results, the
  stage titles, the promedio, the SEASON WINNERS page; what the season keeps, `StageBank`,
  `Promedio`, `Winners`, checked by `season_check`; the career club's own through a
  `KnockoutHost`). Refusals are `RulesError`s.
- `start`: what both starts share: the session's `Pitch`, a `LoadedTeam` as upstream's
  loader finds it (its identity, who controls it, its prices), the name written into its
  buffer, the months a competition's matches are played in.

## Simulated results (`sim`)

Results of matches nobody plays (upstream src/match_sim.c), in steps (docs/PLAN.md M5 #8):

- `MatchTeam`: a team in a match, upstream's record as the simulation reads and writes it
  (`TeamFileEx`: a selected team's 684 bytes, the career club's 1,292 with its reserves and
  its own line-up, `CareerLineUp`), each field a typed accessor; an index past an array
  reaches the bytes after it, as the C does. `MatchTeam::from_team` takes a team file's
  record, its global number in the host's order as upstream loads it.
- `match_value` (`UpdatePlayerPositionAndReturnRole`): a player's match value, kept in his
  record: the value kept while the team's are, a goalkeeper's or a substitute's price, an
  outfield player's price for his place in the tactics (`player_price`, `GetPlayerPrice`: his
  skills weighed over the 35 ball areas by their ponders, how much each counts for his
  position and how near it is, shared among teammates and scaled by `nearness`,
  `CountNearnessFactor`, for a team the computer does not play), 0 out of his place, 1 on
  loan, 0 unavailable (`unavailable`, `IsPlayerInjured`, by `StatusRules`: the mode, a
  training game, the testers' rule). `strength` (`ComputeOldPlayerSkillForViewResult`): the
  team's strength for a simulated result, its substitutes counted. The session's tactics come
  in as a table (`TeamTactics`, by the team's tactics byte). Refused (`SimError`) where
  upstream reads past its records, tables or memory or divides by zero.
- `price_rank` (`CalcTeamPriceRankPercentOffset`): how much a league's team's rank by price
  moves its strength (the league's paths of `FindTeamInDIY`, `GetMoreExpensiveTeamsCount`);
  `form` (`CalcTeamViewOffset`): a team's form on a `Day`, each half's.
- `available_players`, `available_keepers` (`GetNumberOfAvailablePlayers`,
  `GetNumberOfAvailableSubstitutes`): a squad's players and goalkeepers out and available;
  `status_buckets` (`InitPlayerRemapTableForTeam`): the status buckets a simulated match gives
  its players, drawn by the second generator, by `SimRules` (the status rules, the kind of
  game, `GameType`, the kinds a computer club's records keep: `ComputerStatuses`).
- `simulate` (`CalculateViewResult`): a simulated match between two `MatchTeam`s in a
  competition, by a `MatchSetup` (venue, substitutes, goals' column) and a `Session` (the
  tactics and their formations' orders, `formation_orders`; the rules; the career's answers;
  the day): each side's `stand_ins`, status buckets and cards and injuries (`after_match`),
  strengths, ranks and forms, then the goals and their scorers; `extra_time`
  (`ExtraTimeSecondPhaseScoring`) continues from its `Simulated` state; `penalties`
  (`PenaltyShootoutScorerPair`).
- `countdown` (`InitInGameTeams_WithMatchParam`): before a simulated match, its sides'
  status buckets fall by draws, then their cards or injuries a step (a computer club's only
  where its world opens the kind, in a DIY or a preset competition).
- `Simulation`: a competition's matches simulated, a `MatchResults` source over the `Squads`
  kept by identity (the selected teams, the career club): before each match the countdown
  and the match flags cleared, then the result on the competition's month, its statuses in
  the competition's track; extra time from the last normal time.
- `tables`: the engine's tables a value is reckoned by and a form, a status and a result are
  drawn by (upstream's, the same in every world).
