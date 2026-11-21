# The world packages (docs/PLAN.md §5.2): built under target/cache/worlds/, never committed.

WORLD_TOOL := "cargo run --quiet -p openswos_world_extract --"


# Assembles World 1996-97 from your copy of the PC game into target/cache/worlds/1996-97, e.g.: just world_extract_9697 ~/Games/SWOS
[group('worlds')]
world_extract_9697 game_dir="/home/ufo/Temp/Games/OpenSWOS/DataSource/SensibleWorldOfSoccer9697":
  {{WORLD_TOOL}} assemble-9697 --game "{{game_dir}}"


# Extracts World 1996-97's committed package (data/worlds/1996-97) again from the upstream checkout and its compiled tables (just upstream_fetch first; needs libclang)
[group('worlds')]
world_extract_9697_data:
  {{WORLD_TOOL}} extract-9697


# Downloads World 2026-27's package into target/cache/downloads and checks it against the upstream pin (just upstream_fetch first)
[group('worlds')]
world_fetch_2627:
  {{WORLD_TOOL}} fetch-2627


# Completes World 2026-27 into target/cache/worlds/2026-27: its download over World 1996-97, its rules from its baked install (just upstream_fetch first; needs libclang)
[group('worlds')]
world_complete_2627:
  {{WORLD_TOOL}} complete-2627
