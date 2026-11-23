# openswos_world_rules_resource

Reader and writer for a world package's `rules/` files: what upstream OpenSWOS hardcodes or
bakes into C, made data (docs/PLAN.md §5.2, D6). The files are this port's own. The full set
is listed in docs/WORLD_INVENTORY.md §4; each joins this crate with the port that reads it.

`openswos_world_package_resource` reads them into `Rules`, and `openswos_core::World` checks
them. World 1996-97's are committed in `data/worlds/1996-97/rules/`, written by
`world_extract extract-9697` from upstream's compiled tables (docs/PLAN.md M4 #5c).

## Layout

Every file is laid out as upstream lays out its JSON (Python's `json.dump(indent=2)`): two
spaces, characters beyond ASCII escaped, a trailing newline. Codes upstream keeps as bytes are
written as numbers. A behaviour component (docs/WORLD_INVENTORY.md §5) is written by name. A
file reads with its keys in any order and its text laid out in any way. It writes back in this
one layout, so a file this crate wrote writes back byte for byte. A key a file does not have is
an error.

## The files

| file | holds | absent |
|---|---|---|
| `calendar.json` | `first_year`: the year the first season starts (upstream src/world.c `kFirstYear`) | not a world |
| `country_details.json` | `countries[]`: each country's `number`, `continent` (code: 0 Europe, 1 North America, 2 South America, 3 Asia, 4 Oceania, 5 Africa) and `adjective` (a club competition's name starts with it; upstream `_countriesTable`) | not a world |
| `team_files.json` | `bases[256]`: each team file's first global team number (upstream `_teamsCountryNumbers`); `second_files[]`: `{country, file}` for a country over two team files (upstream `World_SecondFile`); `squad_numbers`: whether players keep their own shirt numbers (`World_SquadNumbers`) | not a world |
| `season.json` | `end.countries[]`: the season end's countries, in its order (upstream `_careerCompetitionIdRemapTable`, `World_SeasonCountries`); `end.own_country_first`: a club coach's own country first (career_season.c) | not a world |
| `strings.json` | `symbols`: each compiled string's text by its symbol's name (upstream `_stringSymbols`); a name of the world is its symbol's text, or itself (`dj_intern_names`); `league_reset_name`, `cup_reset_name`, `tournament_reset_name`: the names a league's, a cup's and a tournament's resets write first (`_aDiyLeague`, `_aDiyCup`, `_aDiyTournament`); `division_names[]`: each division's name for a league whose record names none (`_divisionStringTable`); `cup_name`, `tournament_name`: a cup's and a tournament's when its record names none (`_aCup_0`, `_aLeague_1`); `cup_round_names[]`: a cup's rounds' names, by index (`_cupRoundStringTable`) | not a world |
| `competition_classes.json` | `national_teams[]`: the competitions whose names take no country (`_nationalTeamCompetitionsAndQualifications1`); `continental[]`: the contests of the continental suspensions' track (`continental_record`); each a competition's id or the name of the copy a career plays it in (`career.json`) | not a world |
| `draws.json` | `own_draws[]`: `{competitions, first_stage}`, the tournaments whose stages upstream draws by its own tables, picked by competition id (diy_competition.c `buildCompSpecificTeamList`, `buildCompSpecificTeamShuffle`), and the places their first stage keeps in order (the later stages' tables join with M5 #7) | not a world |
| `fixtures.json` | `odd_tournament_groups`: whether a tournament's groups may be odd-sized; `schedules[]`: each size's round robin, `{teams, rounds, pairs}` (one cycle's `[home, away]` pairs in playing order; `rounds`: every club at most once a match day, which a career league needs: upstream `FixtureRounds_Real`); `competition_schedules[]`: `{competitions, schedule}`, competitions that play their own (upstream `getCurrentFixtureTeamPtrs`: the European Championships') | not a world |

## Tests

Unit tests: each file reads and writes back byte for byte; an unknown key, a missing one, a
value out of its type, an unknown code or behaviour does not read; the files read into their
places and are named when broken. World 1996-97's committed files are checked by
`openswos_world_extract` (against a fresh extraction) and `openswos_core` (a valid world).
