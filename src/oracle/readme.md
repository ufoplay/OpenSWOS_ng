# openswos_oracle

Test-only: upstream OpenSWOS's C, compiled unchanged from the pinned checkout, behind a safe
API, so a ported function can be checked against the C it ports (docs/PLAN.md §7.3). The only
crate with `unsafe` (D3), and only in its FFI module. No shipped crate depends on it: the
differential tests live in its own `tests/`, which depend on the port.

## How it builds

`build.rs` reads `units.toml`:

- the flags and defines of upstream's SDL build (its Makefile's `CFLAGS` and defaults:
  `-std=gnu11 -fno-strict-aliasing -fno-builtin -O2`, the data modules, `SWOS_TEST_RULES`,
  `SWOS_WORLDS`, the JSON saves, `-include include/swos_platform.h`), 64-bit as upstream's
  `X86_64=1`, macOS and aarch64 builds, plus `-fexceptions` (below);
- the units the tests call, each with the functions bindgen binds (every one with the
  `C-unwind` ABI), its globals by name and the constants it sees (enumerators, macros) by
  pattern. A global the unit defines with a value bindgen can evaluate would come out a
  Rust `const`: `build.rs` declares it the `static mut` it is, bindgen's type kept, and
  fails on a listed global that is not there;
- the headers whose `static inline` functions the tests call: bound, not compiled. bindgen
  writes a wrapper for each function, compiled with the units;
- upstream's data files (its globals and tables), which the linker pulls in as the units need
  them.

They are compiled with `cc` into a static library. The checkout is
`OPENSWOS_UPSTREAM_DIR`, by default `target/cache/upstream/OpenSWOS` (`just upstream_fetch`);
bindgen needs libclang. Without the checkout the crate builds unavailable
(`cfg(oracle_unavailable)`): its API and tests are compiled out, and `tests/available.rs`
reports a skip (a failure under `OPENSWOS_STRICT_TEST_DATA=1`).

## Externs, globals, halts

- **Stubs.** Every extern a compiled unit references that the oracle does not compile is
  defined in Rust (`ffi::stubs`), with upstream's value: a global (`#[unsafe(no_mangle)]
  static mut`) or a function.
- **One test at a time.** The C globals are the process's: `Oracle::lock()` takes a global
  lock and resets every global the units touch to upstream's value; all calls go through the
  `Oracle` it returns.
- **Halts.** Upstream halts the game in `PanicHaltLoop`. Its stub panics with the reason code;
  the C is compiled with `-fexceptions`, so the panic unwinds through it to the safe wrapper,
  which returns the code as an error.
- **Undefined behaviour stays out.** A safe wrapper never makes a call upstream would make
  undefined (e.g. a save past the save stack's end): it returns an error instead.

## The units

The FFI module has one module a unit (`ffi::rng`, `ffi::math`, `ffi::style`) over bindgen's declarations
(`ffi::bindings::<file name>`, e.g. `random_c`, `math68k_h`).

| unit | API | what |
|---|---|---|
| `src/random.c` | `rng` | `Rand`, `Rand2`, `GetRandomWord`, the second generator's save stack; both generators' state, the random table, a recording's fixed value |
| `src/game_utils.c` | `rng` | `Randomize2`, the session's seed in `_randomSeed` |
| `src/math68k.c` | `math` | `CalculateDeltaXAndY`, the movement scale in its two globals |
| `include/math68k.h` (header) | `math` | `M68kShiftLeft32`, `M68kShiftLeft16` |
| `src/dos_gameplay.c` | `style` | `Gameplay_SetStyle`, `ApplyGameStyle`, and everything they set: the style's flags, frame rate and name, the formations selected (the stub of `TacticsData_SelectRelease` records it), the keepers', ball's and timers' constants, the movement scale; the tweaks `Tweaks_On` reports (stubbed) |

## Tests

`just test_oracle` (256 cases a property, strict) and `just test_oracle_long` (20,000), after
`just upstream_fetch`. Each port's crate is checked against the C it ports:

- `tests/rng.rs`, `openswos_rng`: the same table; random sequences of draws, reseeds, fixed
  values, saves and restores give the same values and states on both sides; the save that
  fills the stack halts both (the halt unwinding back through the C); the lock resets it
  all.
- `tests/math.rs`, `openswos_math`: every angle of the table in every quadrant, at several
  speeds and the styles' scales; any two points at any speed and scale; both shifts.
- `tests/game_style.rs`, `openswos_game_style`: every style, with and without the stronger
  keepers, sets what upstream sets; a number upstream does not know plays the default; the
  lock resets to the Amiga's style.
