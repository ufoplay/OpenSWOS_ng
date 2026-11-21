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
| `country_details.json` | `countries[]`: each country's `number` and `continent` (code: 0 Europe, 1 North America, 2 South America, 3 Asia, 4 Oceania, 5 Africa; upstream `_countriesTable`) | not a world |
| `team_files.json` | `bases[256]`: each team file's first global team number (upstream `_teamsCountryNumbers`); `second_files[]`: `{country, file}` for a country over two team files (upstream `World_SecondFile`); `squad_numbers`: whether players keep their own shirt numbers (`World_SquadNumbers`) | not a world |
| `season.json` | `end.countries[]`: the season end's countries, in its order (upstream `_careerCompetitionIdRemapTable`, `World_SeasonCountries`); `end.own_country_first`: a club coach's own country first (career_season.c) | not a world |
| `fixtures.json` | `odd_round_robin`: `original_pair_tables` or `rounds_with_rest` (an odd-sized league's fixtures); `odd_tournament_groups`: whether a tournament's groups may be odd-sized | not a world |

## Tests

Unit tests: each file reads and writes back byte for byte; an unknown key, a missing one, a
value out of its type, an unknown code or behaviour does not read; the files read into their
places and are named when broken. World 1996-97's committed files are checked by
`openswos_world_extract` (against a fresh extraction) and `openswos_core` (a valid world).
