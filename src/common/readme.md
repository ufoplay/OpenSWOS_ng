# openswos_common

Types and traits shared by the other OpenSWOS_ng crates:

- `ByteReader` / `ByteWriter`: bounds-checked reading and writing of binary records, in
  both byte orders. The records inherited from the Amiga are big-endian, the PC game's own
  formats little-endian.
- `Importer` / `Exporter`: the traits each file format implements. Exporters are lossless.
- `FixedName<N>`: a fixed-size name field that keeps every byte, so records round-trip
  exactly.
- `TeamId` (`TeamFileNumber` + `TeamOrdinal`), `Position` and `Tactics` (the engine's 18
  tactic slots, named by team records and tactics files).
- `Nibble`: half a byte, as records pack two values into one.
- `byte_enum!`: the conversions between a fieldless enum and its byte code.
- `json`: what upstream's JSON files share. `to_text` lays a value out as Python's
  `json.dump(indent=2)` does, with or without `ensure_ascii`; `JsonByte` / `JsonWord` keep
  a byte or word as written (signed or unsigned); `code` writes an enum as its number;
  `present` tells an absent key from a `null` one.

What numbers such as countries or team files *mean* is world data (docs/PLAN.md §5.2), so
they are plain newtypes here. Only values the file formats fix are enums.

It depends on no other crate of the workspace.
