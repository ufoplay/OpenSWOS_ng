# openswos_rng

The game's random numbers, ported from upstream's `src/random.c` (and `Randomize2`, from
`src/game_utils.c`). They are bit-exact: every draw is the original's, in the same order.

- **Two generators** (`Generator`), each a counter walking a 256-byte table (`RANDOM_TABLE`),
  each byte XORed with a key that moves on to the table's next byte every time the counter
  wraps. `Rng::rand` draws from the first one (upstream `Rand`, mostly the match),
  `Rng::rand2` and `Rng::random_word` from the second one (`Rand2`, `GetRandomWord`:
  mostly the career, the competitions and simulated matches).
- **Reseeding.** `Rng::randomize2` seeds the second generator from a value XORed with the
  session's random seed (`Randomize2`). The seed itself is career data, so the caller
  passes it.
- **The save stack.** The second generator's state can be pushed and popped
  (`SaveRand2State`, `RestoreRand2State`), up to 5 saves. The save that fills the stack
  halts the original game: it returns `SaveStackFull`. Restoring with nothing saved
  returns `NothingSaved`, where upstream would read before its buffer.
- **A fixed value** (`Rng::fixed`, upstream `_replayFixedRandom`) pins both generators, as
  the swos-port recordings were made.

No globals: `Rng` is part of the game's state, and every function that draws a number
takes it. The oracle checks every function against upstream's C
(`src/oracle/tests/rng.rs`).
