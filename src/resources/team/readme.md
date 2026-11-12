# openswos_team_resource

Reader and writer for the PC SWOS 96/97 team database files: `DATA/TEAM.nnn` (one per
country, `nnn` being its number) and `DATA/TEAM.CUS` (the custom teams).

## Format

A big-endian `u16` team count, then that many 684-byte team records, and nothing after
them. The PC game's files are not compressed. A team record is a 76-byte header followed
by 16 player records of 38 bytes. `team.rs` and `player.rs` document every field's offset.

Names are fixed-size fields that keep every byte (`FixedName`), so every file round-trips
byte for byte.

Some fields are written by the game while it runs: team control, own goals, form, cards,
goal counters, flags. They are unset in every file of the PC game, and in use in career
saves, which store the same records. They are grouped as `TeamRuntime` and
`PlayerRuntime`.

**Enums and flags** hold every value whose set the format or the engine fixes: position,
face, shirt type, kit colour (the kit-clash rules tell the colours apart), tactics (the
engine's 18 slots; their formations are data), team control, the player-values state,
form's two statuses (`Form`), and the player and match flags. A code outside its set does
not decode; unnamed match-flag bits and any form level are kept as read.

**Skills** (`skill.rs`) are stored as nibbles, two to a byte. Each one is a `Skill`: a
`level` (bits 0-2, `0..=7`, all the match plays with) and `marked` (bit 3, the marker the
transfer market's skill search reads). The passing byte's unused high nibble is kept as a
raw `Nibble`.

**Plain numbers** stay where the meaning is world data or a count: nationalities, prices
(an index into the price table), divisions (World 2026-27 has up to 14 per country), own
goals, the cards-and-injuries levels, goal counters.

## Sources

Upstream OpenSWOS: `include/game_types.h` (`TeamFile`, `PlayerFile`, the `PF_*`,
`PLAYER_FORM_*`, `TEAM_VALUES_*` and `TACTIC_*` defines), `include/team_colors.h` (shirt
types, `KIT_COLOURS`), `include/play_match_menu.h` and `src/teams.c` (team controls),
`include/mod_api.h` (faces), `docs/CUSTOM_TEAMS.md` (kit colour names),
`src/career_status.c` (the cards-and-injuries bits), `docs/DATA_JSON.md` (layouts and
byte-faithfulness notes).

## Tests

- Unit tests and proptests: every field, and any valid record round-trips.
- `assets/TEAM/`: original files of the PC game. `TEAM.057` and `TEAM.085` are checked
  against their JSON sidecars, and `TEAM.008` with targeted assertions. All of them must
  round-trip.
- `just test_real_assets <game_dir>`: every `DATA/TEAM.*` team database of your copy
  round-trips. `DATA/TEAM.248`, which is not a team database, is rejected.
