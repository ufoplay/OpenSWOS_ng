# openswos_competitions_json_resource

Reader and writer for OpenSWOS's competition JSON, `competitions/*.json`: every league,
cup and tournament of a world, and the tables that tie them together. Upstream's ten files
(`bin/data_json/competitions/`, written by `scripts/export_competitions.py`) and World
2026-27's `leagues.json` round-trip byte for byte.

## The files

One model, `CompetitionsFile`, picked by each file's `type` key:

| file | `type` | records (upstream struct) |
|---|---|---|
| `leagues.json` | `competition_leagues` | `League` (`CompetitionLeague`) and its `Division`s |
| `cups.json` | `competition_cups` | `Cup` (`CompetitionCup`) |
| `tournaments.json` | `competition_tournaments` | `Tournament` (`CompetitionTournament`) and its `Round`s |
| `playoffs.json` | `playoffs` | `Playoff` (`Playoffs`) |
| `qualifying.json` | `qualifying_groups` | `QualifyingGroup` |
| `league_blocks.json` | `league_blocks` | `LeagueBlock` (`Leagues`) |
| `career.json` | `career_index` | `CareerContest` (`careerContests`) |
| `country_competitions.json` | `country_competitions` | `CountryCompetitions` (`_competitionsTable`) |
| `countries.json` | `countries` | `Country` (`_allCountriesList`, then the continents) |
| `seed_buckets.json` | `seed_buckets` | `SeedBucket` (`EuroCupQualSeedBucketBlock`) |

## The model

- **Field for field**, in upstream's key order. Keys some records lack (a league's
  `playoff_links`, World 2026-27's `new`) are kept absent; a key that may be `null` keeps
  `null` apart from absent. Unknown keys do not import, nor does a `count` that disagrees
  with the records.
- **Bytes as written.** Every C byte is a `JsonByte`: the exporter writes each one as the C
  initialiser spells it (`"comp_id": -128` in one file, `162` in another).
- **Packed bytes as upstream splits them.** The substitutes byte is two `Nibble`s
  (`substitutes` and `points_for_win` or `max_subs`); a round's rule byte is four 2-bit
  `RuleBits` (`legs`, `extra_time`, `penalties`, and `low`, written only when set).
- **Codes as enums**, written as their numbers: `CompetitionType` (league, cup,
  tournament: the switch of upstream `comp_name_table`), a new country's `Continent` (the
  first byte of its name in the game's country table, 0 being Europe), the markers between
  a qualifying group's competitions (`Marker`: 252, -2, -1, 0). The record kinds (`"cup"`,
  `"tournament"`) and the file types are fixed tags.
- **Ids**: every object has a `SymbolId`, its C symbol name upstream (`_euroCup`), by which
  the files name each other.
- **Kept as numbers, on purpose**: the fields whose meaning the game logic gives, which
  upstream's names do not always tell (`away_goals_mode` is a cup's play-each-team count and
  a tournament's points for a win), and upstream's layout bytes (`name_table_offset`,
  `team_list_offset`, `reserved1`, `qualifier_link_offset`). Their meaning is the
  competitions engine's (M5); what a world's rules are is `world_rules` (M2 #8).

## The text

Python's `json.dump(indent=2, ensure_ascii=False)`: characters beyond ASCII are written as
they are (`openswos_common::json::to_text` with `NonAscii::Raw`). Upstream's files end
without a newline, World 2026-27's with one; both round-trip.

## Sources

Upstream OpenSWOS: `scripts/export_competitions.py` (the writer), `include/game_types.h`
(the structs), `src/save_format_json.c` (`dj_round_rules`, `dj_subs_pair`),
`src/competition_data.c` (`comp_name_table`), `scripts/world_import.py` (`CONTINENTS`, the
`new` record), `docs/DATA_INTERNAL_JSON.md`.

## Tests

- Unit tests: every record, its optional keys, its codes and their errors.
- `assets/COMPETITIONS_JSON/`: upstream's ten files round-trip byte for byte, are the files
  their names say, define 180 distinct ids, and every competition they name is one of them.
- World 2026-27 (downloaded into `target/cache/`, D8): its `leagues.json` round-trips byte
  for byte (91 leagues; Italy's 14 divisions; 26 new countries and their continents).
