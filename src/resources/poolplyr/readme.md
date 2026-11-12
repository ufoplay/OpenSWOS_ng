# openswos_poolplyr_resource

Reader and writer for the PC SWOS 96/97 transfer pool, `DATA/POOLPLYR.DAT`: 900 players
with the clubs they play for. Each record is a copy of a squad player of the team files
(checked against the whole PC game).

## Format

Bare 58-byte records, with no count and nothing else. Each record holds:

| offset | bytes | field |
|---|---|---|
| 0 | 2 | the club's team id: file number, ordinal |
| 2 | 1 | the player's index in the club's squad |
| 3 | 17 | the club's name |
| 20 | 38 | the player record (`openswos_team_resource::Player`) |

A length that is not a whole number of records is an error. Every file round-trips byte for
byte.

## Sources

Upstream OpenSWOS: `docs/DATA_JSON.md` (§1, §2), `include/game_types.h` (`PoolPlayer`).

## Tests

- Unit tests and proptests: any whole records round-trip, and any bytes never panic.
- `assets/POOLPLYR/POOLPLYR.DAT`, the PC game's file: it round-trips and decodes as
  expected.
- `just test_real_assets <game_dir>`: the file round-trips, and every record is a copy of
  a squad player of `DATA/TEAM.nnn`: the player at its ordinal in the team its id names.
