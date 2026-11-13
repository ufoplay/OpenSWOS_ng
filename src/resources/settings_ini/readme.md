# openswos_settings_ini_resource

Reader and writer for OpenSWOS's settings file (`SWOS.ini`, beside the saves folder):
what the game reads from it, how it saves it, and the game's settings typed.

## The file

An INI file: `[section]` headers, `key=value` lines, `;` comments. Upstream reads it with
inih and saves it with a writer of its own that changes as little as it can. Each works on
the file's lines by rules of its own, so `IniFile` keeps the file's bytes whole: any bytes
are a settings file, and it writes back as read.

## Reading (`IniFile::read`)

What the game reads: inih's rules (upstream `src/ini.c`, configured by `include/ini.h`),
each value handed on in the file's order (`IniRead`; the game keeps the last valid value of
a key).

- A read takes at most 199 bytes; the rest of a longer line is dropped, an error. A NUL
  ends what a read holds. The first line may start with UTF-8's byte order mark.
- A line is trimmed. Empty, or starting with `;` or `#`: a comment.
- Indented, after a `name=value`: a continuation, another value of that name. Blank lines
  and comments between do not end it; a section does. Faithful to the original: an
  indented `[section]` right after a pair is a continuation too.
- `[name]`: a section (its name at most 49 bytes); without `]`, an error.
- `name=value` or `name:value`: both trimmed, the value up to an inline comment (a `;` after
  whitespace); neither separator: an error.

Errors do not stop the reading (`first_error`: the first one's line).

## Saving (`SettingsTable::save`)

How the game saves (`Settings_Save`): from a table of settings, each a section, a key, a
comment and a value (bytes), and whether the player left it unchanged since the file was
read (`keep`: the file's value stands). The game's table is `settings_build` (below).

The save reads the file by its own rules: a line is what `fgets` reads into 256 bytes (a
longer one is two), at most 400 lines; a section is `[name]` (cut to 31 bytes); a pair is
`key=value` (`=` only, the key cut to 47 bytes, the value trimmed of blanks with any comment
in it). Keys and sections match ignoring ASCII case. It writes only when a value differs, a
key comes twice or a key is missing; then:

- a changed value replaces the old one: what precedes the `=` stays, the blanks after it
  and any inline comment go, the line ends with `\n`;
- a key's second line goes;
- a missing key joins the end of the first block of its section; a missing section joins the
  end of the file, after a comment;
- everything else stays as read. Faithful to the original: lines past the 400th are lost.

Without a file the save writes a fresh one (`SettingsTable::fresh_file`,
`settings_write_full`): a header comment, then each section once, in the order the table
first names it, its intro after `]`, every setting of it with its comment.

## The settings (`Settings`)

The sections the game itself reads, typed; their defaults are the game's (upstream
`src/main.c`, before `Settings_Load`):

| section | settings |
|---|---|
| `[gameplay]` | the style (`amiga`, `dos`, `xbox`, `amiga60`, `moon`), the original's fire-hold bug, the computer's tactics |
| `[replays]` | a dated copy of every match, the viewer's frame box, its starting speed |
| `[testing]` | no cards nor injuries (upstream's default build has it) |
| `[general]` | language, currency, the dismissed reminders, the world's tag, the file lists' order, QUICK ACCESS, the intro and loading options |
| `[mods]` | the mods enabled, the game's own disabled |
| `[tweaks]` | the 12 tweaks (`Tweaks`; all built, five on by default) |
| `[options]` | the OPTIONS screen: game length, pitch type, auto replays, all teams equal, auto-save highlights, menu music; each unset (`None`) when the file lacks it |

`Settings::read` is the game's reading (`handler`): from the defaults, each value of the
file in order, the last one taken counting. Sections and keys match ignoring ASCII case,
but a tweak's key exactly (`Tweaks_Bit`). Booleans take `on`/`true`/`yes`/`1` and
`off`/`false`/`no`/`0` in any case; a value the game does not take is ignored, and stays in
the file. Texts are bytes, cut to the game's buffers (`language` 15 bytes, `currency` and
`world` 7, the mods' lists 255, and a line's 199 before them); `file_sort` keeps its spelling.
Faithful to the original: any word sets `cpu_tactics` (`expert`, else normal) and the
replays' `speed` (`half`, else normal).

`Settings::table` is upstream's table for them (`settings_build`): its rows in its order,
its comments verbatim, its spellings (an unset option writes the game's default: `3`,
`seasonal`, `true`/`false`). `Settings::save` saves as the game does, given what it read from
the file (the baseline): a setting the same there keeps the file's value, even one the game
ignored. `Settings::fresh_file` is the file the game writes when there is none.

The view's sections (`[video]`, `[audio]`, `[input]`, `[graphics]`), netplay's (`[netplay]`,
`[chat]`) and the arcade's (`[arcade]`) are not typed: they stay in the file as read and a
save leaves them alone (the view's are M8's; netplay and the arcade are out of scope). A
fresh file holds the game's sections only.

## Sources

Upstream OpenSWOS: `src/ini.c`, `include/ini.h` (inih, Copyright (C) 2009-2025, Ben Hoyt,
BSD-3-Clause: its rules are ported in `reader.rs`, which keeps its notice),
`src/swos_settings.c` (`Settings_Load`, `handler`, `parse_bool`, `settings_build`,
`section_intro`, `Settings_Save`, `flush_section_missing`, `copy_value`,
`settings_write_full`, the self-tests), `src/main.c` (the defaults), `include/tweaks.h` and
`src/tweaks.c` (the tweaks' keys, bits and defaults), `include/dos_mode.h` (`GAMEPLAY_*`),
`include/game_options.h` (the values the setup writes), `src/swos_io.c` (`swos_ini_path`).

## Tests

- Unit tests: each of inih's rules (comments, continuations, errors, the byte order mark,
  long lines, NULs, the buffers' limits) and each of the save's (a file left alone, a
  changed value, a kept one, a key twice, a missing key or section, a fresh file, the 400
  lines, a long line read as two); any bytes round-trip and read (proptest).
- Settings unit tests: the defaults and the table's order, a fresh file read back, every
  value of every setting written alone, read, saved and read back (upstream's self-tests,
  `include/game_options.h`'s values among them), values the game ignores, case, the last
  value, the buffers, the baseline.
- `tests/upstream.rs`: the table is upstream's: the game's sections' rows of
  `settings_build` in its order, every comment line and section intro verbatim (from the
  checkout, `just upstream_fetch`).
- `tests/real_saves.rs`: your `SWOS.ini`, beside `OPENSWOS_SAVES_DIR` (`just test_saves
  <folder>`), writes back byte for byte and reads without an error; a save of its own
  values leaves it alone, a changed value changes one line, a missing key joins its section;
  read as `Settings` and saved with its baseline, every line of it stays (the keys an older
  build did not write are added), it reads back the same, a changed setting changes one line.
  Never committed.
