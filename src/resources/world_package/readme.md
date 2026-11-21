# openswos_world_package_resource

Reader and writer for OpenSWOS world packages: a world's data, the same layout for every
world (docs/PLAN.md §5.2, D7). It opens a package from a folder or from a zip (upstream's
`OpenSWOS-World-2627.zip`, its files in one top folder), checks its fingerprint, and writes
one into a folder.

## The package

| path | read as |
|---|---|
| `WORLD.ini` | `WorldIni`: tag, name, fingerprint (comment lines and other entries kept) |
| `mod.json` | `ModManifest`, if there (older packages lack it), with the world's `settings` if any |
| `teams/*.json` | `openswos_team_json_resource` (the clubs, and World 2026-27's national teams by continent) |
| `competitions/national_teams.json` | `openswos_national_teams_json_resource`: the world's own national teams, if any |
| `competitions/bteams.json` | `BTeams` (`bteams.rs`): the B teams and their countries' call-up rules (upstream src/bteam.c) |
| `competitions/stadiums.json` | `Stadiums` (`stadiums.rs`): the clubs' grounds and the finals' venues (upstream src/world_stadium.c) |
| `competitions/*.json` (the others) | `openswos_competitions_json_resource` |
| `data/` | not read (the Amiga's team files, D9), but in the fingerprint |
| `teams/POOLPLYR.DAT` | not read: the world's foreign players' pool, RNC-packed (D9), but in the fingerprint |
| anything else | not read; listed in `OpenedPackage::ignored` |

Each file writes back byte for byte: its layout is kept as read (`JsonFile`: Python's
`json.dump`, its indentation, its characters beyond ASCII, its trailing newline). Values are
kept as written; what the game makes of them (a missing call-up rule's default, the lists'
limits, a stadium name too long) is given by accessors.

`rules/` will join the list with `world_rules` (M2 #8).

## The fingerprint

Upstream scripts/world_import.py `world_fingerprint`: the SHA-1 of every file directly in
`teams/`, `competitions/` and `data/`, folder by folder and by name, each as its path
(`teams/ALBANIA.json`), a NUL and its bytes; its first 16 hex digits. `WORLD.ini` states
it, and upstream's build is pinned to one (`World_DataFingerprint`).

- **Opening** computes it over the files as they are and returns both
  (`FingerprintCheck`). A mismatch is not an error: the game itself only warns.
- **Writing** states the fingerprint of what is written. `data/` is not written, so a
  package written back from upstream's has a fingerprint of its own: it is other data.

## Safety

The paths of a zip stay inside the package (no `..`, absolute paths or backslashes), a
package holds at most 80 MiB (`MAX_PACKAGE_BYTES`, over twice World 2026-27's), and a package
is only written into a folder that does not exist or is empty.

## Sources

Upstream OpenSWOS: `scripts/world_import.py` (`world_fingerprint`, the `WORLD.ini`
writer, `stadiums`, `bteams`, `write_pool`), `scripts/make_world_package.sh` (the zip and its
`mod.json`), `scripts/worldgen/nations.py` (World 2026-27's national teams), `src/main.c`
(how the game reads `WORLD.ini`), `src/world_data.c` (`World_DataFingerprint`), `src/bteam.c`
(`load`), `src/world_stadium.c` (`WorldStadium_Name`), `src/transfer_market.c`
(`LoadPoolplyrDat`: a world's pool), `docs/MODS.md`.

## Tests

- Unit tests: the fingerprint against upstream's algorithm (known answers computed with
  it), `WORLD.ini`, `mod.json` (with settings), `bteams.json` and `stadiums.json` as upstream
  writes them, the game's defaults and limits, unsafe zip paths, folders, a small package
  opened, written and reopened.
- World 2026-27 (downloaded into `target/cache/`, D8): the package opens (107 rosters,
  `leagues.json`, its 211 national teams, 20 B teams, its stadiums, `mod.json`'s ten
  settings; `data/` and the pool ignored), its files give the fingerprint its `WORLD.ini`
  states and upstream's build is pinned to, and it writes back every file it reads byte for
  byte.
