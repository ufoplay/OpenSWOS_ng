//! What the save's player and team objects need from the records: names in their slots,
//! bit fields in their bytes.

use openswos_save_json_resource::SaveString;
use openswos_team_resource::RecordError;
use thiserror::Error;

/// Why a save's player or team object is not a record.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecordTextError {
    /// A name longer than its slot.
    #[error("{field}: {len} bytes, its slot holds {max}")]
    NameTooLong {
        /// The field.
        field: &'static str,
        /// The name's bytes.
        len: usize,
        /// The slot's bytes.
        max: usize,
    },
    /// A name holding a NUL, which the writer never writes (it ends a name).
    #[error("{field}: a NUL inside the name")]
    NameHasNul {
        /// The field.
        field: &'static str,
    },
    /// A number wider than its bit field.
    #[error("{field}: {value} does not fit its bits (at most {max})")]
    FieldTooWide {
        /// The field.
        field: &'static str,
        /// The number.
        value: u8,
        /// The field's largest number.
        max: u8,
    },
    /// A team code that is not one (the team record's checks).
    #[error(transparent)]
    Team(#[from] RecordError),
}

/// The name `text` in a slot of `N` bytes, zero-padded.
///
/// # Errors
///
/// When it is longer than the slot or holds a NUL.
pub fn name_slot<const N: usize>(
    field: &'static str,
    text: &SaveString,
) -> Result<[u8; N], RecordTextError> {
    let bytes = text.as_bytes();
    if bytes.contains(&0) {
        return Err(RecordTextError::NameHasNul { field });
    }
    let mut slot = [0; N];
    slot.get_mut(..bytes.len())
        .ok_or(RecordTextError::NameTooLong {
            field,
            len: bytes.len(),
            max: N,
        })?
        .copy_from_slice(bytes);
    Ok(slot)
}

/// The name in `slot`, as the writer prints it: up to its first NUL. Faithful to the
/// original: the bytes after it are not written (`jb_key_str` zeroes them).
#[must_use]
pub fn slot_name(slot: &[u8]) -> SaveString {
    let end = slot
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(slot.len());
    SaveString(slot.get(..end).unwrap_or_default().to_vec())
}

/// The number in the bits of `byte` that `mask` selects.
#[must_use]
pub fn bits(byte: u8, mask: u8) -> u8 {
    (byte & mask) >> mask.trailing_zeros()
}

/// `fields` (number, mask, name) packed into a byte.
///
/// # Errors
///
/// When a number does not fit its bits.
pub fn pack(fields: &[(u8, u8, &'static str)]) -> Result<u8, RecordTextError> {
    fields.iter().try_fold(0, |byte, &(value, mask, field)| {
        let max = mask >> mask.trailing_zeros();
        if value > max {
            Err(RecordTextError::FieldTooWide { field, value, max })
        } else {
            Ok(byte | (value << mask.trailing_zeros()))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A three-bit field at the top of a byte (upstream `PF_POSITION_MASK`).
    const TOP: u8 = 0xE0;

    /// A two-bit field below it (upstream `PF_FACE_MASK`).
    const MIDDLE: u8 = 0x18;

    #[test]
    fn names_fill_their_slots() {
        assert_eq!(
            name_slot::<4>("name", &SaveString::from("AB")),
            Ok(*b"AB\0\0")
        );
        assert_eq!(name_slot::<2>("name", &SaveString::from("AB")), Ok(*b"AB"));
        assert_eq!(
            name_slot::<2>("name", &SaveString::from("ABC")),
            Err(RecordTextError::NameTooLong {
                field: "name",
                len: 3,
                max: 2
            })
        );
        assert_eq!(
            name_slot::<4>("name", &SaveString(b"A\0B".to_vec())),
            Err(RecordTextError::NameHasNul { field: "name" })
        );
    }

    #[test]
    fn a_name_ends_at_its_first_nul() {
        assert_eq!(slot_name(b"AB\0C"), SaveString::from("AB"));
        assert_eq!(slot_name(b"ABCD"), SaveString::from("ABCD"));
    }

    #[test]
    fn bit_fields_pack_and_unpack() {
        let byte = pack(&[(5, TOP, "top"), (2, MIDDLE, "middle")]).expect("fits");
        assert_eq!(byte, 0b1011_0000);
        assert_eq!(bits(byte, TOP), 5);
        assert_eq!(bits(byte, MIDDLE), 2);
        assert_eq!(
            pack(&[(4, MIDDLE, "middle")]),
            Err(RecordTextError::FieldTooWide {
                field: "middle",
                value: 4,
                max: 3
            })
        );
    }
}
