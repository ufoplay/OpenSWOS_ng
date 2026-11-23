# openswos_oracle

Test-only: upstream OpenSWOS's C, compiled unchanged from the pinned checkout, behind a safe
API, so a ported function can be checked against the C it ports (docs/PLAN.md §7.3). The only
crate with `unsafe` (D3), and only in its FFI module. No shipped crate depends on it: the
differential tests live in its own `tests/`, which depend on the port.

## How it builds

`build.rs` reads `units.toml`:

- the flags and defines of upstream's SDL build (its Makefile's `CFLAGS` and defaults:
  `-std=gnu11 -fno-strict-aliasing -fno-builtin -O2`, the data modules, `SWOS_TEST_RULES`,
  `SWOS_WORLDS`, the JSON saves, `-include include/swos_platform.h`), 64-bit as upstream's
  `X86_64=1`, macOS and aarch64 builds, plus `-fexceptions` (below);
- the units the tests call, each with the functions bindgen binds (every one with the
  `C-unwind` ABI), its globals by name, the constants it sees (enumerators, macros) by
  pattern, and the types its tests read that nothing bound reaches (a record behind a
  `void *`) by name. A global the unit defines with a value bindgen can evaluate would come out a
  Rust `const`: `build.rs` declares it the `static mut` it is, bindgen's type kept, and
  fails on a listed global that is not there;
- the headers whose `static inline` functions the tests call: bound, not compiled. bindgen
  writes a wrapper for each function, compiled with the units;
- upstream's data files (its globals and tables), which the linker pulls in as the units need
  them; the globals a data file lists are bound, for `tables`;
- a unit's functions renamed (`renamed`: src/match_sim.c's `CalculateViewResult`,
  `ExtraTimeSecondPhaseScoring`, `PenaltyShootoutScorerPair`; src/career_status.c's
  `InitInGameTeams_WithMatchParam`): the unit compiled apart with defines that rename them
  `upstream_*` and bound so, so a stub of the name may call upstream's (the competitions'
  tests get their set results and no countdown, or upstream's simulation when a test asks:
  `ffi::stubs::UPSTREAM_SIMULATION`, armed for a call by `ffi::sim_play`);
- the functions the stubs replace (`replaced`: the team loader `VerifyTeamLoaded`, which
  src/career_teams.c defines; the career's updates after a match; the tactics' release): a generated
  header force-included in every unit makes upstream's definitions weak (`#pragma weak`), so
  the stub is what links and upstream's files stay as they are.

They are compiled with `cc` into a static library. The linker keeps only what is reached
from what the tests call (`cc`'s function sections, rustc's dead-code stripping), so a large
unit such as `src/diy_competition.c` is bound a function at a time, and only what those
functions reach must exist (the stubs). Upstream's SDL build has headers include `<SDL.h>`
for the types they name: `build.rs` writes a shim declaring them, so no SDL install is
needed. The checkout is
`OPENSWOS_UPSTREAM_DIR`, by default `target/cache/upstream/OpenSWOS` (`just upstream_fetch`);
bindgen needs libclang. Without the checkout the crate builds unavailable
(`cfg(oracle_unavailable)`): its API and tests are compiled out, and `tests/available.rs`
reports a skip (a failure under `OPENSWOS_STRICT_TEST_DATA=1`).

## Externs, globals, halts

- **Stubs.** Every extern a compiled unit references that the oracle does not compile is
  defined in Rust (`ffi::stubs`), with upstream's value: a global (`#[unsafe(no_mangle)]
  static mut`) or a function.
- **One test at a time.** The C globals are the process's: `Oracle::lock()` takes a global
  lock and resets every global the units touch to upstream's value; all calls go through the
  `Oracle` it returns.
- **Halts.** Upstream halts the game in `PanicHaltLoop`. Its stub panics with the reason code;
  the C is compiled with `-fexceptions`, so the panic unwinds through it to the safe wrapper,
  which returns the code as an error.
- **Undefined behaviour stays out.** A safe wrapper never makes a call upstream would make
  undefined (e.g. a save past the save stack's end): it returns an error instead.

## The units

The FFI module has one module a unit (`ffi::rng`, `ffi::math`, `ffi::style`) over bindgen's declarations
(`ffi::bindings::<file name>`, e.g. `random_c`, `math68k_h`).

| unit | API | what |
|---|---|---|
| `src/random.c` | `rng` | `Rand`, `Rand2`, `GetRandomWord`, the second generator's save stack; both generators' state, the random table, a recording's fixed value |
| `src/game_utils.c` | `rng` | `Randomize2`, the session's seed in `_randomSeed` |
| `src/math68k.c` | `math` | `CalculateDeltaXAndY`, the movement scale in its two globals |
| `include/math68k.h` (header) | `math` | `M68kShiftLeft32`, `M68kShiftLeft16` |
| `src/strings_data.c`, `src/swos_amiga_strings.c`, `src/swos_amiga_data_imports.c`, `src/competition_data.c`, `src/career_comp_copies.c`, `src/fixture_data.c`, `src/swos_data_bulk_imports.c`, `src/swos_amiga_ptr_imports.c`, `src/string_symbols.c` (data) | `tables` | the 96/97 tables `world_extract` writes into World 1996-97's package: each country's entry in `_countriesTable` (continent, name, adjective), `_teamsCountryNumbers`, the season end's countries (`_careerCompetitionIdRemapTable`), the round robins of 2 to 24 clubs (src/fixture_data.c, through `_fixtureSchedulePtrTable`, each with `FixtureRounds_Real`) and the European Championships' (`_euroChampFixtureData`), the compiled strings by name (`_stringSymbols`), the default names (`_aDiyLeague`, `_aDiyCup`, `_aDiyTournament`, `_divisionStringTable`, `_aCup_0`, `_aLeague_1`), the national teams' and continental track's lists (each record by its `compId`, a career copy told apart through `_careerContests`). Always built: without the checkout each is `Unavailable`, so the tool builds everywhere. The only use of the oracle outside its tests (docs/PLAN.md §4) |
| `src/world_data.c` | `tables` | World 2026-27's baked install (`WorldData2627_InstallBaked`, `World_Current` stubbed to answer World 2026-27, `LeaguePhase_InstallCups` recorded) and its accessors: what it leaves in the tables (the round robins included), and its league phases' and its 101 national and league cups' records field by field (`world_2627`). It writes 96/97's tables for good, so it runs once a process and is remembered; afterwards `Oracle::lock` panics: a process that installs a world serves nothing else |
| `src/diy_competition.c` (with `src/selected_teams.c`, `src/career_team_buffers.c` as data) | `table`, `draw`, `running` | the DIY competition engine, a function at a time as M5 ports it: the league table's sort (`DIY_SortLeagueTableByStandings`) over `_diyCompetitionStart`'s rows, the teams named in `_selectedTeams`; the teams' values (`GetAveragePlayerPrice`, `DIY_ComputeAverageTeamStrengthTier`) and the draws (`DIY_ShuffleTeamOrderList`, `DIY_ShuffleTeamGridPairs`, `DIY_ShuffleLeagueTeamOrder`, `DIY_ShuffleTeams`, `DIY_SortAndShuffleTeamsByValue`, a tournament's state built in Rust); a league started (`InitDIYCompetitionFromTableEntry`, from a league's or a type-0 cup's C record built in Rust, with the national teams' and continental track's lists and the default division names set for the call; or from a compiled record, a country table's league or a career contest's template or copy, with upstream's own tables: `start_compiled_division`); a cup started (`InitDIYCupFromRecord`, from a cup's or a play-off's C record built in Rust, with the two lists set for the call; or from a compiled record, a country table's cup, a career knockout's template or copy, or a play-off, bound by name: `start_compiled_cup`); a tournament's state (`_diyFileBufferCopy`) read field by field through bindgen's declaration of `DIYTournamentState`, reset (`InitDIYTournament`) and its stages counted (`DIYTournament_RecalculateMatchesPerTeam`), a tournament started (`InitDIYLeagueFromRecord`, from a tournament's C record built in Rust, with the two lists set and `World_Current` answering the world asked; or from a compiled record, a career contest's template or copy, or the World Cup or the European Championships, bound by name; the Copa Libertadores has no compiled record in upstream's SDL build, which loads it from the JSON as the tests build it) (`tournament`); stubbed: a world's league phase's and state championships' stage starts and knockout draws (none), a cup played (`cup_play`: `ProcessDIYKnockoutMatchResultAndCareerUpdates`, `DIY_PlayCupMatchAndAdvance`, `DIY_UpdateCompetitionProgress`, `GetNextCupRoundStringIndex`, `GetCupRoundStringIndex`; its results the simulation's stubs', which record what they are asked; a world's tie and next-round hooks none), a league played (`play`: `DIY_GetCurrentFixtureTeamPtrs`, `DIY_AdvanceLeagueProgressAfterMatch`, `DIY_PlayOrSimulateLeagueFixture` simulated with its result given cached (`_cachedViewResultTeam1Goals`), `DIY_UpdateLeagueQualifiers`, `DIY_UpdatePromotionRelegationScratchCounts` with `_diyLeagueScratchBuffer`; the match engine's and the career's calls stubbed, the simulation's never reached, a world's split, league phase and state championships none), a world's league phase's and state championships' own stage matches (none), a world's league phase and split, its national cups' and classic European cups' draws (none), the team loader (`VerifyTeamLoaded`: `_selectedTeams`' record of the number's ordinal, the career club's for country 0xFF, so upstream derives the identities and the controls the tests give), the session's pitch and view month, `StringCopy`; a league's and a cup's matches simulated by upstream (`sim_play`: no result given, the squads written once into `_selectedTeams` and `_careerTeam` and kept by the matches, `Oracle::set_simulation`, `squads`; the loader's team numbers assigned to the selected teams by a test, `assign_teams`, as a tournament's teams share ordinals, cleared by the lock); `_diyCompetitionStart` read field by field through bindgen's declaration of `DIYCompetitionEx` (`running`) |
| `src/career_teams.c` | `stage` | the career's side of a tournament: `DIY_AdvanceToNextCompetitionSlot` (a stage's end, through diy_competition.c's `DIY_BuildSeededBracketAndUpdateScores`, `DIY_AdjustWorldCupBracketScores`, `DIY_CopyTeamGridFromCompToDIY`, then `DIY_LoadCurrentCompetitionSlot`; the own draws' tables and the draws' scratch `_diyTeamShuffleScratch` bound), no career game (`_gameType` 0: `DIY_PostLoadCompetitionDispatch` linked, not run); stubbed: a world's stage-end hooks (`LeaguePhase_GroupsEnd`, `Conti_GroupsEnd`, `States_GroupsEnd`, `States_KnockoutEnd`: none) and the career walk's (`LeaguePhase_Showing`, `LeagueSplit_Walking`); its `VerifyTeamLoaded` replaced by the stub (`replaced`) |
| `src/match_sim.c` (with `src/tactics_data.c`, `src/simulation_data.c`, `src/player_data.c`, `src/career_unavailable_players.c`, `src/career_team_scalars.c` as data), `src/menus.c` | `sim`, `sim_result` | the simulation of a match nobody plays, a function at a time as M5 #8 ports it: a player's price for his place (`GetPlayerPrice`, with diy_competition.c's `CountNearnessFactor`), his match value (`UpdatePlayerPositionAndReturnRole`), the team's strength (`ComputeOldPlayerSkillForViewResult`), over a selected team's record in `_selectedTeams[0]` or the career club's in `_careerTeam` with its line-up (`_currentMatchPlayers`, `_careerNumPlayers`), the session's tactics table pointed at the test's records for each call (`_teamTacticsTable`, restored after); the engine's price tables read for `the_engine_s_tables_are_upstream_s`; a team's rank in its league (`CalcTeamPriceRankPercentOffset`, the competition in `_diyCompetitionStart`, the teams' prices the stubbed loader's), its form (`CalcTeamViewOffset`, the day's globals set), its players' status buckets (`InitPlayerRemapTableForTeam`, the stand-ins' table the test's or none, `_gameType` set); a simulated match, its extra time and its shoot-out (`CalculateViewResult`, `ExtraTimeSecondPhaseScoring`, `PenaltyShootoutScorerPair`, compiled renamed `upstream_*` and called in turn over the oracle's own sides' records, the formations' orders by src/menus.c's `InitializeTacticsPositions`, `sim_result`); stubbed: a world's computer clubs' line-ups and keeper injuries (`WorldRules_CpuLineupsBegin`, `WorldRules_CpuLineupsEnd`, `WorldRules_GkAfterSim`: none), the career's answer whether statuses apply (`ShouldApplyRandomPlayerStatusAfterSim`: the test's) and the career club's form after a match (none); stubbed: the user's tactics records (`_teamTacticsUSERA` to `F`) |
| `src/career_status.c`, `src/test_rules.c` | `sim`, `sim_result` | whether a player is available (`IsPlayerInjured`), the squad's available players and goalkeepers (`GetNumberOfAvailablePlayers`, `GetNumberOfAvailableSubstitutes`), under the testers' rule (`TestRules_Force`) and a training game's flag (`_trainingGame`); the countdown of a simulated match's teams' statuses (`InitInGameTeams_WithMatchParam`, compiled renamed `upstream_*`, over the oracle's own sides' records, the competition's track in `_diyCompetitionStart`, `_gameType` set); the career's updates after a match replaced by the stubs; stubbed: whether a computer club's statuses count down before a match (`WorldRules_CpuCountdown`: the kind kept, in a DIY or a preset competition), the replay rig's queries (`ReplayTest_Armed`, `ReplayTest_Running`, `Rollback_Enabled`: none), a world's computer clubs' kinds of statuses (`WorldRules_CpuOpen`, `WorldRules_CpuAny`: as the test sets them), a friendly club's long bench (`LongBench_FrOrder`, `SquadExtras_BenchRecord`: none), a world's league in zones (`LeagueSplit_ZoneFind`, `LeagueSplit_ZoneOfTeam`, `LeagueSplit_ZoneScale`: none), the career's globals (`_currentSeasonShowing`, `_newRandomSeedCopy`, `_isNationalSelector`, `_primaryCompetitionBuf`) and the menus' current team (`_numPlayers`, `_playerPositions`) |
| `src/fixture_rounds.c` | `tables` | a world's odd round robins as real rounds (`FixtureRounds_Install`, which World 2026-27's install calls; `FixtureRounds_Real`, each round robin's `rounds`): it writes upstream's `_fixtureSchedulePtrTable` (src/swos_amiga_ptr_imports.c), which the readers point at fixture_data.c's tables first |
| `src/swos_rules_post.c`, `src/swos_rules_cups.c`, `src/swos_rules_groups.c`, `src/swos_rules_cupd.c`, `src/swos_rules_dplay.c`, `src/swos_rules_check.c`, `src/swos_rules_gplay.c` (`src/rules/`, MIT: `src/rules/LICENSE-MIT`) | `rules_library`, `knockouts` | the rules library as upstream builds it, a file at a time as M5 #9a ports it: the season's moves (`sr_final_moves`, `sr_b_teams`) over a library state the oracle owns (zeroed, its tables, moves and wants written), a cup's clubs (`sr_cup_entrants`), parallel groups (`sr_pg_season`), the grouped cup's season end (`sr_cupd_end_reset`, `sr_cupd_keep_place`, `sr_cupd_note_arrival`) and later rounds (`sr_cupd_make`, its buffer filled first so a side it did not write shows); the fixed tiers' brackets (`sr_italy`, a season of the career club's results, `sr_out_held`, `sr_playoff_held`) and the checks (`sr_league_check`, `sr_spec_check`, the error's field read as a string); the knockouts (`sr_knockouts`) over seasons one after another on one state, the stage bank, promedio history and SEASON WINNERS page kept between them, the host's callbacks answering from the season's tables (`knockouts`); each unit's own copy of the library's types built from one setup (`league!`, `state!`, `tables!`) |
| `src/rnc.c` | `unpack` | upstream's RNC decoders (`RncDecompressV1`, `RncDecompressV2`), for World 2026-27's packed pool: the header is checked in Rust (signature, method, sizes against the file), the stream is trusted (the checksum-pinned download's) |
| `src/dos_gameplay.c` | `style` | `Gameplay_SetStyle`, `ApplyGameStyle`, and everything they set: the style's flags, frame rate and name, the formations selected (the stub of `TacticsData_SelectRelease` records it), the keepers', ball's and timers' constants, the movement scale; the tweaks `Tweaks_On` reports (stubbed) |

## Tests

`just test_oracle` (256 cases a property, strict) and `just test_oracle_long` (20,000), after
`just upstream_fetch`. Each port's crate is checked against the C it ports:

- `tests/rng.rs`, `openswos_rng`: the same table; random sequences of draws, reseeds, fixed
  values, saves and restores give the same values and states on both sides; the save that
  fills the stack halts both (the halt unwinding back through the C); the lock resets it
  all.
- `tests/table.rs`, `openswos_core::competition::table`: random tables in up to 4 groups,
  small counts and two-letter names (ties down to the names' bytes, prefixes), counts across
  the word (goal differences that wrap), ordered as `DIY_SortLeagueTableByStandings` orders
  them.
- `tests/draw.rs`, `openswos_core::competition::{draw, value}`: any team's average price,
  any competition's strength; from any state of the second generator, every shuffle (an
  order past 256 places, the grid's pairs, a league's places, a tournament's teams) and the
  seeded draw (random prices, ties included, in groups or quarters) give the same order and
  leave the same state.
- `tests/running.rs`, `openswos_core::competition::running`: any image, every field the
  port reads (both views of each union, every array's elements, the standings rows) is the
  one the C struct reads: the port's offsets are the compiler's.
- `tests/league.rs`, `openswos_core::competition::league`: from any image, any division of a
  league's record or a type-0 cup's (random fields and names, national or not, continental
  or not, any pitch, any teams' prices, any state of the second generator), started alike
  byte for byte (the C's name pointer aside), and an odd division refused as upstream
  halts; every division of World 1996-97's leagues upstream compiles (each country table's,
  each career cup played as a league from its record and from its copy), resolved from the
  committed package (`WorldDivision`: the names, the adjective, the classes), started as
  upstream starts its compiled record with its own tables.
- `tests/tournament.rs`, `openswos_core::competition::tournament`: any image, every field
  the port reads is the one the C struct reads (the header checked as `tests/running.rs`
  checks it, `tests/header/mod.rs`); from any state the reset writes the same bytes; from any
  state of valid stages (random counts, groups, plays, legs and penalties, any months, teams
  and prices), the stages are counted alike, both images after compared; from any images,
  any tournament's record (random ids, the own draws' as often as the rest, the seeds' order
  and the seeded draw each on half the time, groups or a knockout first, seeds among the
  state's teams), started alike, both images byte for byte (the name pointers aside), odd
  groups refused as upstream halts, cases the port refuses (where upstream runs past its
  memory) not run; every tournament of World 1996-97 (the career's eleven from their
  records and copies, the World Cup, the European Championships, the Copa from the JSON:
  25), resolved from the committed package (`WorldTournament`), the state's teams its seeds
  backwards, started alike.
- `tests/sim.rs`, `openswos_core::sim` (M5 #8a): the engine's tables are upstream's compiled
  ones; any team record, every field the port reads is the one the C struct reads; from any
  team (a selected team's record or the career club's and its line-up), any tactics (spots
  past the grid now and then, a time in four the players following the ball) and any status
  rules, a player's price for his place, his match value and the team's strength alike, the
  record after byte for byte; any nearness and any availability alike; England's 93 teams
  (`assets/TEAM/TEAM.008`), as selected teams and as the career club, their values and
  strengths alike with upstream's compiled tactics.
- `tests/sim_status.rs`, `openswos_core::sim`'s standing, form and statuses (M5 #8b): any
  league (its teams and places mostly the league's), team and prices, the rank alike, a team
  not in its league refused as upstream halts (0x6006); any record and day, the form; any
  team, mode and rules, its available players and goalkeepers; any team, stand-ins' table or
  none, game, computer statuses and second generator (pinned at the odds' edges now and then,
  the squads mostly fit, a keeper last of the eleven now and then), the status buckets and the
  generator after; England's 93 teams too. What it shares with `tests/sim.rs` is
  `tests/sim_setup/mod.rs`.
- `tests/sim_result.rs`, `openswos_core::sim`'s simulated result (M5 #8c): any tactics'
  formations' defensive orders (`InitializeTacticsPositions`); from any league, two sides
  (selected teams or the career club, mostly clean, now and then any), any setup, tactics,
  rules, game, computer statuses, career's answers, day and second generator (pinned now and
  then), a match (`upstream_CalculateViewResult`), its extra time and its shoot-out alike, the
  records after byte for byte, the scorers and the generator; a side not in its league as
  upstream halts (0x6006); England's 19 pairs of teams over 32 seeds each. The countdown
  before a simulated match (M5 #8d, `upstream_InitInGameTeams_WithMatchParam`): from any two
  sides (either the career club), track, training game, game, computer statuses and second
  generator (pinned now and then), both records and the generator after alike, cases the
  port refuses (a line-up place naming no record) not run.
- The competitions simulated to their end (M5 #8d): every division of World 1996-97's
  country leagues (88, `tests/play.rs`), every knockout (62, `tests/cup.rs`) and every
  tournament (25, `tests/stages.rs`) started alike, then played through by
  `openswos_core::sim::Simulation` on one side and upstream's simulation on the other
  (`Oracle::set_simulation`, no result given): England's real teams cycled as the squads,
  numbered by a tournament's own teams (`assign_teams`), a career club among a league's and a
  cup's teams, the session varied by competition (the kind of game, the computer clubs'
  statuses, whether statuses apply, the day); after each match the competition, the
  tournament's state, every squad and the second generator alike, and the parts of a tie
  asked. What they share is `tests/simulated/mod.rs` (and the given results' source of the
  competitions' checks, `tests/given/mod.rs`).
- `tests/stages.rs`, `openswos_core::competition::tournament::next_stage`: tournaments played
  through as the career plays them, every match (the league's and the cup's play, their
  results given), every stage's end and every next stage (`DIY_AdvanceToNextCompetitionSlot`,
  `Oracle::next_stage`, the draws' scratch filled first so an order read from it shows)
  alike, both images byte for byte (the name pointers aside) and the second generators;
  random tournaments (`tournament()`: stages that mostly follow each other, noise now and
  then, the own draws' ids among them; at some stage ends the teams through made 1 and the
  next bracket doubled on both sides), steps the port refuses (where upstream runs past its
  memory) not run, the World Cup's missing pattern matched with upstream's halt (0x8034);
  every tournament of World 1996-97 (25) played through to its end. What it shares with
  `tests/tournament.rs` (the records, the teams, World 1996-97's tournaments started on both
  sides) is `tests/tournaments/mod.rs`.
- `tests/play.rs`, `openswos_core::competition::league`'s play: from any league under way
  (random groups, places, rows of selected teams or the career club, progress and counts,
  the European Championships' own round robin among them, any tournament's state, any teams'
  names), a match's result recorded alike, both images byte for byte; any table's qualifiers
  and season's moves alike (the moves counted in the scratch by the teams' global numbers);
  every division of World 1996-97's country leagues (88), started as upstream starts its
  compiled record, played through its season alike, match by match, then its moves.
- `tests/cup.rs`, `openswos_core::competition::cup`: from any image, any cup's or play-off's
  record (random fields, rules and names, national or not, continental or not, any pitch,
  any teams' prices and controls, any state of the second generator), started alike byte
  for byte (the C's name pointer aside), and odd teams refused as upstream halts; every
  knockout of World 1996-97 upstream compiles (each country table's cup, the career's Cup
  Winners' Cup and UEFA Cup from their records and their copies, the play-offs: 62),
  resolved from the committed package (`WorldKnockout`), started as upstream starts its
  compiled record with its own tables.
  A cup played: any tie decided alike (the same parts of its results asked, normal time's
  ground among them; goals and penalties past a byte); any next round alike; any cup's or
  play-off's record started and played through to its winner alike, match by match, then
  each next round (those upstream would play past its bracket refused, and not run); World
  1996-97's 62 knockouts played through alike; the round names alike.
- `tests/math.rs`, `openswos_math`: every angle of the table in every quadrant, at several
  speeds and the styles' scales; any two points at any speed and scale; both shifts.
- `tests/game_style.rs`, `openswos_game_style`: every style, with and without the stronger
  keepers, sets what upstream sets; a number upstream does not know plays the default; the
  lock resets to the Amiga's style.
- `tests/world_2627.rs` (a process of its own), `tables::world_2627`: the install resizes
  (Italy 256, Albania 10), adds (Andorra), renames (Serbia) and switches (squad numbers,
  league phases) as upstream's source states; its round robins (the odd sizes 5 to 23 real
  rounds, 3's the original's; 5's first pair 4 v 1); its league phases' records (the
  Champions League's 36 clubs, LASK first); its 101 national and league cups (the Coppa
  Italia first, 14 league cups, the English cup's 64 clubs past its record); it runs once; the oracle serves nothing else
  afterwards.
- `tests/rules_library.rs`, `openswos_core::competition::rules_library` (M5 #9a1): from random
  tables (the counts past the divisions in use read too), play-offs' moves, leagues, second
  teams (mostly clubs of the tables) and levels, the wants after `sr_final_moves` and after
  `sr_b_teams` alone; random cups' quotas, slots, places, levels and career clubs, the clubs
  `sr_cup_entrants` lists; random chains of single divisions and groups with clubs moving,
  staying and entering, now and then past the limits, the offsets after `sr_pg_season` (or its
  refusal, nothing changed); random season ends and grouped cups, the end and every side
  `sr_cupd_make` writes, and nothing more (M5 #9a1). Random fixed tiers' leagues and tables
  (an upper tier and 2 to 4 groups, points falling by small steps), the moves `sr_italy` makes
  or its refusal; the held rules over any points; random specs (every kind of play-off near the
  limits, a byte overwritten or the bytes cut now and then) and leagues, every fault's code,
  play-off, byte and field (`sr_spec_check`, `sr_league_check`) (M5 #9a2). Inputs the port
  refuses because the C reads or writes past its memory are not run.
- `tests/knockouts.rs`, `knockouts` (M5 #9a3): one league's season ends one after another over
  one state, the port's state after each upstream's (every move, result, stage title, groups'
  figure, the bank, the promedio history, the SEASON WINNERS page): random leagues of a few
  divisions, their specs every play-off kind, entrant and round, random banks, histories,
  hosts, played ties, previews and second teams; and the rules library's packs (World
  2026-27's leagues' specs and shapes, `test_utils::cache::RULES_SPECS`, downloaded; skipped
  without the network) over random tables of their shapes. A season the port refuses because
  the C reads past its memory, or where it reads a round not played, ends the run before it.
  What it shares with `tests/rules_library.rs` (the core's league and tables of the oracle's
  setups) is `tests/rules/mod.rs`.
- `tests/unpack.rs`, `unpack`: World 2026-27's pool unpacks to its 900 records (the download
  cache; skipped without the network); what is no RNC file, or states more than it holds, is
  refused before the C is called.
- `tests/tables.rs`, `tables`: the tables read as upstream's source states them (a country's
  continent, name and adjective; a number without a country; 256 bases never going down; the
  season end's 65 countries; each size's round robin every pair once, real rounds the even
  ones, 4 clubs' as fixture_data.c states; the European Championships' 6 pairs; the
  compiled strings by name; the default names; the lists' records and career copies).
