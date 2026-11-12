# OpenSWOS_ng

A Rust port of [OpenSWOS](https://github.com/benbaker76/OpenSWOS), the C rebuild of
*Sensible World of Soccer 96/97*.

The first goal is a view-independent game core: the match engine, competitions and career,
plus readers and writers for every game file. The rules are data: the original game is a
world package ("World 1996-97") with the same structure as World 2026-27. Rendering, audio
and input come later.

**Status:** M0, bootstrap. See [docs/PLAN.md](docs/PLAN.md) for the plan and milestones.

## Building

Every step goes through [just](https://github.com/casey/just):

```sh
just build        # release build
just test         # all tests
just check_code   # fmt and clippy
just              # every recipe
```

`upstream.toml` records the upstream OpenSWOS commit the port is migrated against, and the
state of every upstream file. `just upstream_fetch` checks upstream out at that commit, and
`just upstream_status` shows how far the migration is.

## Game data

This repository ships no original game data. Playing needs your own copy of *Sensible World
of Soccer 96/97* (the PC version is on GOG).

## License

GPL-3.0-or-later, like OpenSWOS. *Sensible World of Soccer* itself is © its rights holders
and is not covered.
