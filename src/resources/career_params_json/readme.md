# openswos_career_params_json_resource

Reader and writer for OpenSWOS's career-mode balance knobs, `career_params.json`
(upstream `bin/data_json/career_params.json`, or a mod's). It is an input of
`world_extract` (docs/PLAN.md §5.2): the career's economy is World 1996-97 data.

## Format

One JSON object. The game reads the keys it knows over its defaults
(`DataJson_LoadCareerParams`); every other key is a note.

- **Knobs.** The 53 keys the loader reads, typed by their field in upstream's
  `CareerParams`: 47 unsigned words (`UnsignedKnob`), the board's four signed thresholds
  (`SignedKnob`), and two arrays held per division (`PerDivisionKnob`). Values are kept as
  written. The loader's clamps (`num_seasons` to 1..=35, the list capacities, the modes)
  are the game's, applied when the career loads them (M7), not the file's.
- **Per-division arrays.** The game reads the first four values, the top division first;
  a division without one keeps its value. Arrays of any length are kept.
- **Notes.** Keys starting with `_` (`"_finances_wages": "…"`): text for whoever edits
  the file, which the game ignores. They are kept, in place.
- **What does not read.** A key that is neither (most likely a typo), a key given twice,
  a value outside its field (`-1` for an unsigned word, `20.5`), a note that is not a
  string.

The members are kept in the order written (`CareerParamsDocument::members`), with
getters by knob.

## Layout

Upstream's file is written by hand: two-space indent, one member a line, arrays on one
line, a blank line before every note but the first member, characters beyond ASCII as
they are, a newline at the end (kept as read). Every file is written that way, so
upstream's round-trips byte for byte.

## Sources

Upstream OpenSWOS: `include/career_params.h` (`CareerParams`), `src/career_params.c` (the
defaults), `src/save_format_json.c` (`DataJson_LoadCareerParams`),
`docs/DATA_PIPELINE.md` (what each knob does).

## Tests

- Unit tests: layout, order, errors, per-division arrays, and a property test (any knobs
  and notes, in any order).
- `tests/fixtures.rs`: upstream's file (`assets/CAREER_PARAMS_JSON/`, GPL) round-trips byte
  for byte, sets each knob once, and states the original's values. With upstream's
  checkout in the cache, the knobs are exactly the keys its loader reads, the signed ones
  included.
