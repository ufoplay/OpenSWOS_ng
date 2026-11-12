
# Cleans the build artifacts (and the test-data cache in target/cache)
[group('build')]
clean:
  cargo clean


# Builds the workspace in release mode
[group('build')]
build:
  cargo build --workspace --release
