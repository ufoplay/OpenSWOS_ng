# openswos_game_style

How a match plays (D1), ported from upstream's `src/dos_gameplay.c`: the constants and code
paths that differ between the game's two releases and SWOS 2020's styles.

| style | id | number | frames a second | what |
|---|---|---|---|---|
| `Amiga50Hz` | `amiga` | 0 | 50 | the Amiga's, the definitive release and the default |
| `Dos70Hz` | `dos` | 1 | 70 | the PC release's: its physics, timers, goalkeepers and formations, pre-scaled to play like the Amiga's, and its own code where the releases differ |
| `Xbox60Hz` | `xbox` | 2 | 60 | SWOS 2020's style 4: the Amiga's game at 60 Hz with the DOS goalkeepers |
| `Amiga60Hz` | `amiga60` | 3 | 60 | SWOS 2020's style 3: the Amiga's game at 60 Hz |
| `Moon50Hz` | `moon` | 4 | 50 | SWOS 2020's style 2: the Amiga's with a moon's gravity |

The `GameStyle` trait is object-safe: the match holds a `&'static dyn GameStyle`, found by the
id the settings store (`by_id`, the `[gameplay] style` spellings) or by upstream's number,
which recordings carry (`by_code`). A new style is a new implementation and one registry line.

What a style gives (upstream's `ApplyGameStyle` writes it into globals at kick-off):

- **the frame rate** and **the movement scale** (`openswos_math::DeltaScale`: 41/64 for the
  DOS game, 50/60 at 60 Hz, none at 50 Hz);
- **the releases whose code runs**: where the releases' code differs (`release`, upstream
  `_gDosGameplay`), the goalkeepers' code (`goalkeepers`, `_gDosKeepers`) and the formations
  (`formations`, `TacticsData_SelectRelease`), each a `Release` the match engine matches on;
- **the goalkeepers' constants** (`Goalkeeping`): save distance, catch height, reach table,
  the loose ball's range. `dive_deltas` applies the stronger keepers tweak (`KeeperReach`),
  which reaches a step further with the Amiga's goalkeepers only;
- **the ball's physics** (`BallPhysics`): friction on the ground, in the air and per pitch
  type, gravity;
- **the match clock and the stoppage timers** (`Timers`), in frames.

Each style's values are documented constants with their provenance; styles that share values
build on each other's (MOON is the Amiga's but for two values). The match engine adds the
methods its units need as they are ported (docs/PLAN.md §10). The oracle checks every
style, with and without the stronger keepers, against upstream's `ApplyGameStyle`
(`src/oracle/tests/game_style.rs`).
