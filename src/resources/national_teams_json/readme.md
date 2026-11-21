# openswos_national_teams_json_resource

Reader and writer for OpenSWOS's national teams JSON, `national_teams.json`: the national
teams' records and the career manager's nationalities, which a mod may bring in place of
the original's (upstream `docs/DATA_PIPELINE.md`). OpenSWOS ships none: the original's
tables are compiled in. World packages carry them (docs/PLAN.md §5.2); `world_extract`
writes World 1996-97's from upstream's tables.

## Format

```json
{ "records": [ { "country": 80, "ordinal": 0, "nationality": 0, "team_file": 0, "skill": 19 }, … ],
  "nationalities": [ 8, 31, … ],
  "job_offer_pick": 41 }
```

| key | meaning |
|---|---|
| `about` | optional, first: a comment (World 2026-27's says where its records come from), which the game does not read |
| `records` | the national teams (`NationalTeamRecord`), 1..=256, in order: a nationality's national team is the first record with it |
| `nationalities` | optional: the nationalities the manager may take, at most 127, in the menu's order. The game adds OTHER (255) last, so 255 is not listed. Absent or empty, the game keeps the original's. |
| `job_offer_pick` | optional: how many of the first records a random national job offer is drawn from, 1..=256 |

A record is the game's 6-byte `NationalTeamRecord`, named field by field
(`from_bytes` / `to_bytes`):

| offset | key | field |
|---|---|---|
| 0 | `country` | the national team's team file (80..=85 in the original: one per continent) |
| 1 | `ordinal` | its place in that file |
| 2 | `nationality` | its nationality |
| 3 | `team_file` | the team file of the nation's clubs; 255 when the game has none (`club_file: None`): the selector's ADD PLAYER then has no HOME |
| 4 | `skill` | its skill, big-endian in the table: national job offers compare the manager's reputation with it |

What upstream's loader (`DataJson_LoadNationalTeams`) refuses, whole, does not read here
either: no records, more than the maxima, a record without a field or with one out of
range, a listed nationality outside 0..=254. A `job_offer_pick` outside 1..=256, which
the loader ignores, does not read either. Other keys do not read (the loader ignores them;
no file known has any).

## Layout

OpenSWOS itself has no writer for this file. It is written as Python's `json.dump` writes:
keys in the order above, the optional ones only when present. The indentation is kept as
read (`indent=2` as upstream's other data JSON, `DATA_JSON_INDENT`; `indent=1` in World
2026-27's, `scripts/worldgen/nations.py`), and so is whether the text ends with a newline.
World 2026-27's package carries its file in `competitions/` (the game reads a world's there
first: `DataJson_LoadNationalTeams`).

## Sources

Upstream OpenSWOS: `docs/DATA_PIPELINE.md` (the format), `src/save_format_json.c`
(`DataJson_LoadNationalTeams`), `include/national_teams.h` (the maxima, no team file),
`include/game_types.h` (`NationalTeamRecord`), `src/teams.c` (how records are looked up),
`src/swos_data_bulk_imports.c` (the original's tables).

## Tests

- Unit tests: the record's bytes, upstream's example, the layout, what the loader
  refuses, and property tests (any record's bytes, any file).
- `tests/original.rs`: the original's tables, read from upstream's source in the cache
  (`just upstream_fetch`), make a file that round-trips with every record's 6 bytes; the
  63 records name 63 nationalities, and every nationality a manager may take has its
  national team.
