//! Test helpers.

use std::fmt::Debug;

use openswos_save_json_resource::{Compact, from_save_bytes, to_save_bytes};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// `text` read as a `T`, after checking that it writes back as `text` (compact).
pub(crate) fn round_trip<T: Serialize + DeserializeOwned + PartialEq + Debug>(text: &str) -> T {
    let value: T = from_save_bytes(text.as_bytes()).expect("reads");
    let bytes = to_save_bytes(&value, &Compact).expect("writes");
    assert_eq!(String::from_utf8(bytes).expect("ASCII"), text);
    value
}

/// Whether `text` does not read as a `T`.
pub(crate) fn rejects<T: DeserializeOwned>(text: &str) -> bool {
    from_save_bytes::<T>(text.as_bytes()).is_err()
}
