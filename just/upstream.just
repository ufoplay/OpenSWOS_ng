# The migration ledger tool (docs/PLAN.md §8). The checkout lives in .upstream/OpenSWOS
# (override: OPENSWOS_UPSTREAM_DIR).

UPSTREAM_TOOL := "cargo run --quiet -p openswos_upstream --"


# Clones or updates the upstream OpenSWOS checkout at the commit pinned in upstream.toml
[group('upstream')]
upstream_fetch:
  {{UPSTREAM_TOOL}} fetch


# Adds new upstream files to upstream.toml as untriaged; drops removed ones holding no ported work
[group('upstream')]
upstream_sync:
  {{UPSTREAM_TOOL}} sync


# Updates ledger entries, e.g.: just upstream_mark 'src/netplay*.c' --area out_of_scope --status not_ported
[group('upstream')]
[positional-arguments]
upstream_mark +args:
  {{UPSTREAM_TOOL}} mark "$@"


# Shows how far the migration is against the pinned commit
[group('upstream')]
upstream_status:
  {{UPSTREAM_TOOL}} status


# Fails unless upstream.toml covers exactly the tracked upstream files, consistently
[group('upstream')]
upstream_check:
  {{UPSTREAM_TOOL}} check
