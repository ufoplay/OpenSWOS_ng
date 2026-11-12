# openswos_common

Types and traits shared by the other OpenSWOS_ng crates:

- `ByteReader` / `ByteWriter`: bounds-checked reading and writing of binary records, in
  both byte orders. The records inherited from the Amiga are big-endian, the PC game's own
  formats little-endian.
- `Importer` / `Exporter`: the traits each file format implements. Exporters are lossless.
- `FixedName<N>`: a fixed-size name field that keeps every byte, so records round-trip
  exactly.
- `TeamId` (`TeamFileNumber` + `TeamOrdinal`) and `Position`.

What numbers such as countries or team files *mean* is world data (docs/PLAN.md §5.2), so
they are plain newtypes here. Only values the file formats fix are enums.

It depends on no other crate of the workspace.
