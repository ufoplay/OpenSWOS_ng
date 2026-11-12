# openswos_tmd_resource

Reader and writer for the PC SWOS 96/97 European-cup team pools: `DATA/EUROCUP.TMD`
(16 teams), `DATA/EUROCWC.TMD` and `DATA/UEFACUP.TMD` (32 teams each).

## Format

Bare 684-byte team records (`openswos_team_resource::Team`), with no count and nothing
else: the file's length gives the number of teams. A length that is not a whole number of
records is an error. Every file round-trips byte for byte.

## Sources

Upstream OpenSWOS: `docs/DATA_JSON.md` (§1, §2), `include/game_types.h` (`TeamFileEx`).

## Tests

- Unit tests and a proptest (any bytes: no panic).
- `assets/TMD/EUROCUP.TMD`, the PC game's file: it round-trips, and its 16 teams are
  checked by name and id.
- `just test_real_assets <game_dir>`: every `DATA/*.TMD` round-trips, and every pool team
  is the team its id names in `DATA/TEAM.nnn`.
