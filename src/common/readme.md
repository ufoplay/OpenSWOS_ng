# openswos_common

Types and traits shared by the other OpenSWOS_ng crates: the byte reader/writer for the
game's big- and little-endian formats, fixed-size name fields, shared identifiers
(`TeamId`, countries, positions, kit colours), the `Importer`/`Exporter` traits and the
base error types.

It depends on no other crate of the workspace. See `docs/PLAN.md` §4.
