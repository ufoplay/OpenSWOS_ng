//! Without upstream's checkout the oracle is not built, and its tests are not either: this one
//! says so, a skip, or a failure with `OPENSWOS_STRICT_TEST_DATA=1` (`just test_oracle`).

#[cfg(oracle_unavailable)]
#[test]
fn the_oracle_is_built() {
    openswos_test_utils::skip::report(&openswos_test_utils::skip::Skip::new(
        "the oracle is not built: no upstream checkout (just upstream_fetch)",
    ));
}
