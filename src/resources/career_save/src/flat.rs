//! Records of `N` numbers written as one flat array, as the world sections' writer does
//! (`wp_emit_u16` and friends): `[a0,b0,a1,b1,…]`. A length that is not a whole number of
//! records does not read.
//!
//! Use with `#[serde(with = "crate::flat")]` on a `Vec<[T; N]>`.

use serde::de::Error as _;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Writes the records as one flat array.
///
/// # Errors
///
/// As the serializer's.
#[allow(clippy::ptr_arg)] // serde's `with` passes the field
pub fn serialize<S: Serializer, T: Serialize, const N: usize>(
    rows: &Vec<[T; N]>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(rows.len() * N))?;
    for value in rows.iter().flatten() {
        seq.serialize_element(value)?;
    }
    seq.end()
}

/// Reads a flat array as records of `N` numbers.
///
/// # Errors
///
/// When the numbers do not read, or are not a whole number of records.
pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de> + Clone, const N: usize>(
    deserializer: D,
) -> Result<Vec<[T; N]>, D::Error> {
    let flat = Vec::<T>::deserialize(deserializer)?;
    let (rows, rest) = flat.as_chunks::<N>();
    if rest.is_empty() {
        Ok(rows.to_vec())
    } else {
        Err(D::Error::custom(format!(
            "{} numbers: not a whole number of {N}-number records",
            flat.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Rows(#[serde(with = "super")] Vec<[u16; 3]>);

    #[test]
    fn records_are_written_flat() {
        let rows = Rows(vec![[1, 2, 3], [4, 5, 6]]);
        assert_eq!(
            serde_json::to_string(&rows).ok(),
            Some("[1,2,3,4,5,6]".to_owned())
        );
        assert_eq!(
            serde_json::from_str::<Rows>("[1,2,3,4,5,6]").ok(),
            Some(rows)
        );
        assert_eq!(
            serde_json::from_str::<Rows>("[]").ok(),
            Some(Rows(Vec::new()))
        );
        assert!(serde_json::from_str::<Rows>("[1,2,3,4]").is_err());
    }
}
