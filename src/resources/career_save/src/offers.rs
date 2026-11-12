//! The offers of a career: other clubs' offers for the manager's players, and the job
//! offers (upstream `TransferOfferEntry`, `JobOffer`, include/game_types.h;
//! `s_transfer_offer_fields`, `s_job_offer_fields`, src/save_format_json.c).
//!
//! What the numbers mean is the career's (M7); they are kept as written, in their fields'
//! C widths.

use serde::{Deserialize, Serialize};

/// Bytes of a job offer's team-name field.
pub const JOB_OFFER_TEAM_NAME_LEN: usize = 18;

/// A club's offer for the manager's players (5 in a save, the unused ones zero).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferOffer {
    /// The offering team (upstream `teamOffering`, compared with team numbers).
    pub team_offering: u16,
    /// The first player wanted: an ordinal in the manager's squad.
    pub primary_player: i8,
    /// The second player wanted; negative for a one-player offer.
    pub second_player: i8,
    /// The player offered in exchange; negative for none.
    pub swap_player: i8,
    /// A second player offered; negative for none.
    pub extra_player: i8,
    /// The amount offered.
    pub offer_amount: u16,
    /// The first improved amount; -1 and -2 mark where the haggling stands.
    pub negotiation_step1: i16,
    /// The second improved amount; -1 marks where the haggling stands.
    pub negotiation_step2: i16,
    /// Turns left before the offer expires.
    pub lifespan: u8,
    /// 1 once the manager has seen it.
    pub seen_flag: u8,
}

/// A job offer to the manager (5 in a save at least, the original's queue; more when the
/// queue is bigger, `job_offers_capacity`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobOffer {
    /// The offering team's number.
    pub team_no: u16,
    /// The offering team's name field, as raw bytes: `0xFF` marks it unset, so it is not a
    /// NUL-ended name.
    pub team_name: [u8; JOB_OFFER_TEAM_NAME_LEN],
    /// The wage offered.
    pub amount_offered: u32,
    /// The record's last word (upstream `reserved`: never read), kept as written.
    pub reserved: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use openswos_save_json_resource::{Compact, from_save_bytes, to_save_bytes};

    const TRANSFER: &str = r#"{"team_offering":5314,"primary_player":14,"second_player":-1,"swap_player":-1,"extra_player":-1,"offer_amount":120,"negotiation_step1":-2,"negotiation_step2":-1,"lifespan":3,"seen_flag":1}"#;

    const JOB: &str = r#"{"team_no":575,"team_name":[76,39,65,81,85,73,76,65,0,0,0,0,0,0,0,0,0,255],"amount_offered":70000,"reserved":0}"#;

    #[test]
    fn a_transfer_offer_round_trips() {
        let offer: TransferOffer = from_save_bytes(TRANSFER.as_bytes()).expect("an offer");
        assert_eq!(offer.team_offering, 5314);
        assert_eq!(offer.second_player, -1);
        assert_eq!(offer.negotiation_step1, -2);
        let bytes = to_save_bytes(&offer, &Compact).expect("writes");
        assert_eq!(bytes, TRANSFER.as_bytes().to_vec());
    }

    #[test]
    fn a_job_offer_keeps_its_name_bytes() {
        let offer: JobOffer = from_save_bytes(JOB.as_bytes()).expect("an offer");
        assert_eq!(offer.team_name.get(..8), Some(&b"L'AQUILA"[..]));
        assert_eq!(offer.team_name.last(), Some(&0xFF));
        assert_eq!(offer.amount_offered, 70_000);
        let bytes = to_save_bytes(&offer, &Compact).expect("writes");
        assert_eq!(bytes, JOB.as_bytes().to_vec());
    }

    #[test]
    fn fields_keep_their_widths() {
        for (text, from, to) in [
            (TRANSFER, "\"primary_player\":14", "\"primary_player\":200"),
            (TRANSFER, "\"offer_amount\":120", "\"offer_amount\":-1"),
            (
                TRANSFER,
                "\"negotiation_step1\":-2",
                "\"negotiation_step1\":40000",
            ),
            (JOB, "\"amount_offered\":70000", "\"amount_offered\":-1"),
            (JOB, ",255]", ",256]"),
            (JOB, ",0,255]", ",255]"),
        ] {
            let changed = text.replace(from, to);
            assert_ne!(changed, text);
            let read = if text == TRANSFER {
                from_save_bytes::<TransferOffer>(changed.as_bytes()).is_err()
            } else {
                from_save_bytes::<JobOffer>(changed.as_bytes()).is_err()
            };
            assert!(read, "{to}");
        }
    }
}
