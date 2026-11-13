# openswos_math

The game's integer math, ported from upstream's `src/math68k.c` and `include/math68k.h`.
Every result is bit-exact.

- **Directions and steps.** `calculate_delta` (upstream `CalculateDeltaXAndY`) gives the
  direction from one point to another in 256 steps (0 up, 64 right, 128 down, 192 left) and
  a step towards it at a speed, in 16.16 fixed point. It is how every sprite moves. It
  halves both differences until they are below 32, reads the angle from a 32 × 32 table,
  and the step from a sine table cut down to -128..=127.
- **The game style's scale.** The steps are scaled by a `DeltaScale`, so a game running at
  more frames a second does not play faster: 41/64 for the DOS game at 70 Hz
  (`DeltaScale::DOS`, done with arithmetic shifts exactly as upstream), none for the Amiga
  at 50 Hz (`DeltaScale::UNSCALED`). The game style supplies it (D1); upstream reads it
  from two globals set by `ApplyGameStyle`.
- **68000 arithmetic.** `shift_left_32` and `shift_left_16` (`M68kShiftLeft32`/`16`)
  shift a signed value's bit pattern, as the 68000's `lsl` does.

Where upstream's C is undefined, the port gives a defined answer and documents it: a
difference of -32768, a shift past the width. A `DeltaScale` that could overflow cannot be
built. The oracle checks every function against upstream's C (`src/oracle/tests/math.rs`).

The 16.16 fixed-point type comes with its first user, the match engine's sprites.
