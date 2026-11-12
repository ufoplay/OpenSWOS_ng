# OpenSWOS_ng

A Rust port of [OpenSWOS](https://github.com/benbaker76/OpenSWOS) (C). The plan, its
decisions (D1–D8) and the milestones are in [docs/PLAN.md](docs/PLAN.md). Read it before
starting a task.

## Commands (everything goes through `just`)

- `just check_code`: fmt check, clippy `-D warnings`, `lint_magic`. Must pass.
- `just test`: all tests.
- `just upstream_fetch`: the upstream checkout at the pinned commit, in `.upstream/OpenSWOS`.
- `just upstream_status` / `just upstream_check` / `just upstream_sync`.
- `just upstream_mark <globs...> [--area A] [--status S] [--rust PATH]... [--function F]... [--note TEXT]`.

## Rules

- Tools are written in Rust only (`src/tools/`).
- **No `unsafe`**, except in the test-only `src/oracle` crate (FFI to upstream C).
- **No magic numbers.** Every literal other than `0`/`1` is a `const` with a doc comment
  giving its meaning, unit and upstream provenance. `just lint_magic` enforces it.
- **Idiomatic port.** Enums with `TryFrom<u8>` instead of ints, exhaustive matches (no
  `_` arm on enums), newtypes. In `core`: no globals, no floats, no `HashMap`/`HashSet`.
- **Bit-exact with upstream.** Integer widths mirror the C and wrap only where the C wraps.
  RNG call order is preserved. Quirks are kept and commented `Faithful to the original:`.
- **Rules are data (D6).** `core` holds no world ids and no `if original` branches. A rule
  the original hardcodes becomes World 1996-97 package data, or a behaviour component that
  the data selects.
- Game-style differences (Amiga 50 Hz / DOS 70 Hz) go through the `GameStyle` trait (D1).
- Resources round-trip byte-identically.
- Every function has tests. Core functions also get a differential proptest against
  upstream C (the oracle).
- Port one unit at a time; the definition of done is in docs/PLAN.md §10. After porting an
  upstream file, update the ledger:
  `just upstream_mark <file> --status done --rust <our files>`.

## Data

- Commit no original game data beyond the small curated test set in `assets/`, and list
  every file there in `assets/README.md` with its provenance.
- Never commit World 2026-27 data or the RGD recordings. They are downloaded into
  `target/cache/`.
- Upstream is GPL-3.0-or-later, and so is this repo. Code ported from MIT sources
  (swos-competition-engine, the SWOS 96/97 Mod) keeps its notice.
