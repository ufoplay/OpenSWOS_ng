
# Runs all the tests
[group('test')]
test test_name="":
  cargo test --workspace {{test_name}}
