# Formats the code
[group('code_check')]
fmt args="":
  cargo fmt --all {{args}}


# Runs clippy on the code
[group('code_check')]
clippy args="":
  cargo clippy --workspace --all-features --all-targets {{args}}


# Fails on magic numbers and undocumented constants (docs/PLAN.md §5.4)
[group('code_check')]
lint_magic:
  cargo run --quiet -p openswos_lint_magic -- src


# Runs fmt, clippy and lint_magic. It fails if any of the checks fail
[group('code_check')]
check_code:
    just fmt "--check"
    just clippy "-- -D warnings"
    just lint_magic
