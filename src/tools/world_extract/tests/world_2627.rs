//! World 2026-27's package, downloaded (D8), is the data the upstream pin was built with: its
//! `WORLD.ini` states its files' fingerprint, and that fingerprint is the checkout's. Skipped
//! without the network or the upstream checkout (`just upstream_fetch`).

use openswos_test_utils::cache::{WORLD_2627_PACKAGE, fetch};
use openswos_test_utils::require;
use openswos_test_utils::skip::{Skip, report};
use openswos_world_extract::commands::upstream_checkout;
use openswos_world_extract::fetch::{pinned_fingerprint, world_2627};

/// The package's rosters: 101 countries' clubs, the six continents' national teams.
const ROSTERS: usize = 107;

#[test]
fn the_download_is_the_pins_data() {
    let checkout = upstream_checkout();
    let Ok(pin) = pinned_fingerprint(&checkout) else {
        report(&Skip::new(format!(
            "{} holds no upstream checkout: just upstream_fetch",
            checkout.display()
        )));
        return;
    };
    let zip = require!(fetch(&WORLD_2627_PACKAGE));
    let fetched = world_2627(&checkout).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(fetched.zip, zip);
    assert_eq!(fetched.opened.fingerprint.stated, pin);
    assert_eq!(fetched.opened.package.identity.name, "World 2026-27");
    assert_eq!(fetched.opened.package.teams.len(), ROSTERS);
}
