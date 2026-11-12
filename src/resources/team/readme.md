# openswos_team_resource

Reader and writer for the PC SWOS 96/97 team database files: `DATA/TEAM.nnn` (one per
country, `nnn` being its number) and `DATA/TEAM.CUS` (the custom teams).

## Format

A big-endian `u16` team count, then that many 684-byte team records, and nothing after
them. The PC game's files are not compressed. A team record is a 76-byte header followed
by 16 player records of 38 bytes. `team.rs` and `player.rs` document every field's offset.

Names are fixed-size fields that keep every byte (`FixedName`), so every file round-trips
byte for byte.

Some fields are written by the game while it runs: own goals, form, cards, goal counters,
flags. They are 0 in every file of the PC game, and in use in career saves, which store
the same records. They are grouped as `TeamRuntime` and `PlayerRuntime`. Their meaning
belongs to the game logic.

What numbers such as nationalities, kit colours or tactics *mean* is world or view data,
so they are plain newtypes here. Only values the format fixes are enums: position, face,
shirt type and the player flags.

## Sources

Upstream OpenSWOS: `include/game_types.h` (`TeamFile`, `PlayerFile`, the `PF_*` and
`PLAYER_FORM_*` defines), `include/team_colors.h` (shirt types), `include/mod_api.h`
(faces), `docs/DATA_JSON.md` (layouts and byte-faithfulness notes).

## Tests

- Unit tests and proptests: every field, and any valid record round-trips.
- `assets/TEAM/`: original files of the PC game. `TEAM.057` and `TEAM.085` are checked
  against their JSON sidecars, and `TEAM.008` with targeted assertions. All of them must
  round-trip.
- `just test_real_assets <game_dir>`: every `DATA/TEAM.*` team database of your copy
  round-trips. `DATA/TEAM.248`, which is not a team database, is rejected.
