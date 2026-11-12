# openswos_world_package_resource

Reader and writer for OpenSWOS world packages: a world's data, the same layout for every
world (docs/PLAN.md §5.2, D7). It opens a package from a folder or from a zip (upstream's
`OpenSWOS-World-2627.zip`, its files in one top folder), checks its fingerprint, and writes
one into a folder.

## The package

| path | read as |
|---|---|
| `WORLD.ini` | `WorldIni`: tag, name, fingerprint (comment lines and other entries kept) |
| `mod.json` | `ModManifest`, if there (older packages, the pinned World 2026-27's among them, lack it) |
| `teams/*.json` | `openswos_team_json_resource` |
| `competitions/*.json` | `openswos_competitions_json_resource` |
| `data/` | not read (the Amiga's team files, D9), but in the fingerprint |
| anything else | not read; listed in `OpenedPackage::ignored` |

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
package holds at most 64 MiB (`MAX_PACKAGE_BYTES`, twice World 2026-27's), and a package
is only written into a folder that does not exist or is empty.

## Sources

Upstream OpenSWOS: `scripts/world_import.py` (`world_fingerprint`, the `WORLD.ini`
writer), `scripts/make_world_package.sh` (the zip and its `mod.json`), `src/main.c` (how
the game reads `WORLD.ini`), `src/world_data.c` (`World_DataFingerprint`), `docs/MODS.md`.

## Tests

- Unit tests: the fingerprint against upstream's algorithm (known answers computed with
  it), `WORLD.ini` and `mod.json` as upstream writes them, unsafe zip paths, folders, a
  small package opened, written and reopened.
- World 2026-27 (downloaded into `target/cache/`, D8): the package opens (101 rosters,
  `leagues.json`, `data/` ignored), its files give the fingerprint its `WORLD.ini` states
  and upstream's build is pinned to, and it writes back every roster and competition file
  byte for byte.
