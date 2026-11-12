//! The knobs the game reads from the file, by the type of their field in upstream's
//! `CareerParams` (include/career_params.h), in that struct's order. Every key here is
//! one `DataJson_LoadCareerParams` reads (src/save_format_json.c).
//!
//! What a value means is the career's (M7): the file holds numbers, kept as written. The
//! loader's clamps (`num_seasons` to 1..=35, the capacities, the modes) are the game's,
//! not the file's. In brackets, each knob's value in the original game: upstream's
//! defaults (src/career_params.c), which its file states.

/// Declares a kind of knob: an enum of its knobs with their keys.
macro_rules! knobs {
    (
        $(#[$meta:meta])*
        $name:ident {
            $($(#[$doc:meta])* $variant:ident = $key:literal,)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($(#[$doc])* $variant,)+
        }

        impl $name {
            /// Every knob of this kind, in upstream's struct order.
            pub const ALL: &[Self] = &[$(Self::$variant),+];

            /// Its key in the file.
            #[must_use]
            pub fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)+
                }
            }

            /// The knob of this kind named `key`, if any.
            #[must_use]
            pub fn from_key(key: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|knob| knob.key() == key)
            }
        }
    };
}

knobs! {
    /// A knob held in an unsigned word (`uint16_t`).
    UnsignedKnob {
        /// The starting balance: the team value is multiplied by this (3).
        StartBalanceValueMult = "start_balance_value_mult",
        /// The starting balance: then by this percentage at least (80)…
        StartBalancePctMin = "start_balance_pct_min",
        /// …plus a random part of this range (40: 80..=119 %).
        StartBalancePctRange = "start_balance_pct_range",
        /// The starting balance: then by this (1000).
        StartBalanceScale = "start_balance_scale",
        /// Gate receipts: the teams' values are divided by this (4000).
        GateValueDivisor = "gate_value_divisor",
        /// Gate receipts of a league match: the home team's factor (10000).
        GateLeagueMultHome = "gate_league_mult_home",
        /// Gate receipts of a league match: the away team's factor (2000).
        GateLeagueMultAway = "gate_league_mult_away",
        /// Gate receipts of a cup match: both teams' factor (6000).
        GateCupMult = "gate_cup_mult",
        /// A trialist's wage, in % of a full one (33).
        WageTrialistPct = "wage_trialist_pct",
        /// A reserve's wage, in % of a full one (66).
        WageReservePct = "wage_reserve_pct",
        /// The overall wage multiplier (96).
        WageScale = "wage_scale",
        /// The board: a league position in this percentile or better improves the verdict
        /// by a step (25).
        BoardPercentileLow = "board_percentile_low",
        /// The board: one in this percentile or worse makes it a step worse (75).
        BoardPercentileHigh = "board_percentile_high",
        /// The board: a top-two finish in the secondary cup improves the verdict by this
        /// (2).
        BoardSecondaryTop2Bonus = "board_secondary_top2_bonus",
        /// The board: in the tertiary cup (2).
        BoardTertiaryTop2Bonus = "board_tertiary_top2_bonus",
        /// The board: in a European cup (3).
        BoardEuroTop2Bonus = "board_euro_top2_bonus",
        /// A player's weekly wage, in % of the original's (100).
        WagePct = "wage_pct",
        /// The wages the team value is made from, in % of the weekly wages (100).
        TeamValueWagePct = "team_value_wage_pct",
        /// The weekly running costs: 0 the original's, 1 halved then progressive brackets
        /// (0).
        RunningCostMode = "running_cost_mode",
        /// The weekly running costs, in % (100).
        RunningCostPct = "running_cost_pct",
        /// The career club's league home gate is multiplied by this (1).
        LeagueGateMult = "league_gate_mult",
        /// The sponsor: 0 the original's random one, 1 every season by
        /// `sponsorship_weeks` (0).
        SponsorshipMode = "sponsorship_mode",
        /// Whether the sponsorship counts in the season's profit or loss (0).
        ProfitLossSponsorship = "profit_loss_sponsorship",
        /// The season's transfer quota: 0 the original's formula, 1 `transfer_quota` (0).
        TransferQuotaMode = "transfer_quota_mode",
        /// The fixed transfer quota of mode 1 (256).
        TransferQuota = "transfer_quota",
        /// Foreign players added to the market are multiplied by this (1).
        ForeignRefillMult = "foreign_refill_mult",
        /// The chance in 256, a week, of a bid for a player you listed (11).
        ListedBidChance = "listed_bid_chance",
        /// The chance in 256 that a bid comes from abroad (8).
        ForeignBidChance = "foreign_bid_chance",
        /// Buyers at home: 0 the original's, 1 also a club with a player worth half the
        /// listed one's price (0).
        DomesticBuyerMode = "domestic_buyer_mode",
        /// Whether a new job offer may be withdrawn (1).
        JobOfferExpiry = "job_offer_expiry",
        /// Routine job offers start from this week of the season (0).
        JobOfferFirstWeek = "job_offer_first_week",
        /// Job offers come in waves through the season (0).
        JobOfferWaves = "job_offer_waves",
        /// Whether job offers stop once 7 seasons are complete (1).
        JobOfferStopSeason7 = "job_offer_stop_season7",
        /// Whether a club offer comes only in the random phase's week (1).
        JobOfferPhaseGate = "job_offer_phase_gate",
        /// Whether your own national team is tried first (1).
        JobOfferOwnNational = "job_offer_own_national",
        /// The chance in 256 that an attempt goes on to look for a club (42).
        JobOfferPass = "job_offer_pass",
        /// Clubs up to this % of your reputation make offers (100)…
        JobOfferRepHighPct = "job_offer_rep_high_pct",
        /// …and from this % (75).
        JobOfferRepLowPct = "job_offer_rep_low_pct",
        /// The chance in 256 that a national selector gets club offers too (0).
        JobOfferSelectorClubs = "job_offer_selector_clubs",
        /// The chance in 256 that a club manager is looked at for national jobs (0).
        JobOfferNationalChance = "job_offer_national_chance",
        /// The chance in 256 that a club offer is looked for abroad first (0).
        JobOfferAbroadChance = "job_offer_abroad_chance",
        /// The seasons before the manager retires (20).
        NumSeasons = "num_seasons",
        /// The foreign market's player pool (432).
        ForeignPoolCapacity = "foreign_pool_capacity",
        /// The foreign players listed in the market at once (15).
        ForeignListingCapacity = "foreign_listing_capacity",
        /// Your players on the transfer list (5).
        TransferListCapacity = "transfer_list_capacity",
        /// The job offers queued (5).
        JobOffersCapacity = "job_offers_capacity",
        /// Trialists: 0 counts the available players by the caller's flag, 1 counts the
        /// injured and suspended out in every match but training (0).
        TrialistCountMode = "trialist_count_mode",
    }
}

knobs! {
    /// A knob held in a signed word (`int16_t`): the board's verdict thresholds, on the
    /// ratio `(rank - league position) × 100 / teams`.
    SignedKnob {
        /// Below this, the verdict is at least "ok" (30).
        BoardRatioOk = "board_ratio_ok",
        /// Below this, at least "good" (10).
        BoardRatioGood = "board_ratio_good",
        /// At or below this, at least "poor" (-10).
        BoardRatioPoor = "board_ratio_poor",
        /// At or below this, "terrible" (-30).
        BoardRatioTerrible = "board_ratio_terrible",
    }
}

knobs! {
    /// A knob held per division (`uint16_t[4]`, the top division first), as an array.
    PerDivisionKnob {
        /// The season's sponsorship in sponsorship mode 1: the weekly wages times this
        /// (0, 0, 0, 0).
        SponsorshipWeeks = "sponsorship_weeks",
        /// The season's league money: the weekly wages times this (0, 0, 0, 0).
        LeagueDistributionWeeks = "league_distribution_weeks",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The keys `DataJson_LoadCareerParams` reads.
    const LOADER_KEYS: usize = 53;

    #[test]
    fn every_key_is_one_knobs_and_only_once() {
        let keys: Vec<&str> = UnsignedKnob::ALL
            .iter()
            .map(|knob| knob.key())
            .chain(SignedKnob::ALL.iter().map(|knob| knob.key()))
            .chain(PerDivisionKnob::ALL.iter().map(|knob| knob.key()))
            .collect();
        assert_eq!(keys.len(), LOADER_KEYS);
        assert_eq!(keys.iter().collect::<BTreeSet<_>>().len(), LOADER_KEYS);
        assert!(keys.iter().all(|key| !key.starts_with('_')));
    }

    #[test]
    fn keys_name_their_knobs() {
        for knob in UnsignedKnob::ALL {
            assert_eq!(UnsignedKnob::from_key(knob.key()), Some(*knob));
        }
        assert_eq!(
            SignedKnob::from_key("board_ratio_poor"),
            Some(SignedKnob::BoardRatioPoor)
        );
        assert_eq!(UnsignedKnob::from_key("board_ratio_poor"), None);
        assert_eq!(
            PerDivisionKnob::from_key("sponsorship_weeks"),
            Some(PerDivisionKnob::SponsorshipWeeks)
        );
        assert_eq!(UnsignedKnob::from_key("num_season"), None);
    }
}
