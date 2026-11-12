# openswos_core

The game logic of OpenSWOS_ng: the match engine, competitions and career. It is a
deterministic state machine driven by commands and player inputs. It returns events and
never calls a view.

- It has no rendering, audio, input or file-system access.
- It contains no world-specific branches: every rule comes from the loaded world package
  (World 1996-97, World 2026-27, ...).
- It uses no floating point and no hash-ordered collections: results must be bit-exact
  with the original game and identical on every platform.

See `docs/PLAN.md` §4 and §5.
