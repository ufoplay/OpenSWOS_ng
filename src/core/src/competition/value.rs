//! Teams' values (docs/PLAN.md M5 #3): a team's average price and a competition's strength,
//! from the players' price bytes (upstream diy_competition.c `GetAveragePlayerPrice`,
//! `DIY_ComputeAverageTeamStrengthTier`). The draws seed by them; the strength is the
//! competition's, for the matches nobody plays (#8).

/// The players a team's value counts: its first sixteen, the team file's squad (upstream
/// `TeamFileEx.players[16]`).
pub const VALUED_PLAYERS: usize = 16;

/// The highest strength tier (upstream `DIY_ComputeAverageTeamStrengthTier` clamps to it).
pub const MAX_TIER: u8 = 7;

/// How far the average price is shifted to give the tier: four prices a tier.
const TIER_SHIFT: u16 = 2;

/// A team's valued players' prices, in the team file's order.
pub type Prices = [u8; VALUED_PLAYERS];

/// The average of a team's valued players' prices, rounded down (upstream
/// `GetAveragePlayerPrice`).
#[must_use]
pub fn average_price(prices: &Prices) -> u16 {
    let total: u16 = prices.iter().map(|&price| u16::from(price)).sum();
    total / u16::try_from(VALUED_PLAYERS).expect("sixteen players")
}

/// A competition's strength: the average price of its teams' valued players, and its tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Strength {
    /// The average price, rounded down.
    pub average: u16,
    /// The average over four, at most [`MAX_TIER`].
    pub tier: u8,
}

/// The strength of a competition of `teams` (upstream `DIY_ComputeAverageTeamStrengthTier`),
/// none without teams (upstream would divide by zero). Faithful to the original: the players
/// are counted in a 16-bit word.
#[must_use]
pub fn strength(teams: &[Prices]) -> Option<Strength> {
    let players = u16::try_from(teams.len())
        .ok()?
        .wrapping_mul(u16::try_from(VALUED_PLAYERS).expect("sixteen players"));
    if players == 0 {
        return None;
    }
    let total: u32 = teams.iter().flatten().map(|&price| u32::from(price)).sum();
    // Faithful to the original: the quotient is kept as a word, its low one.
    let [_, _, high, low] = (total / u32::from(players)).to_be_bytes();
    let average = u16::from_be_bytes([high, low]);
    let tier = u8::try_from((average >> TIER_SHIFT).min(u16::from(MAX_TIER))).expect("at most 7");
    Some(Strength { average, tier })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team(prices: [u8; 4]) -> Prices {
        let mut all = [0; VALUED_PLAYERS];
        all[..4].copy_from_slice(&prices);
        all
    }

    #[test]
    fn a_team_s_value_is_its_squad_s_average_price() {
        assert_eq!(average_price(&[10; VALUED_PLAYERS]), 10);
        assert_eq!(
            average_price(&team([16, 15, 0, 0])),
            1,
            "31 / 16, rounded down"
        );
        assert_eq!(average_price(&[u8::MAX; VALUED_PLAYERS]), 255);
    }

    #[test]
    fn a_competition_s_strength_is_its_teams_average_and_tier() {
        let strength = strength(&[[12; VALUED_PLAYERS], [20; VALUED_PLAYERS]]).expect("teams");
        assert_eq!(
            strength,
            Strength {
                average: 16,
                tier: 4
            }
        );
        let strong = super::strength(&[[60; VALUED_PLAYERS]]).expect("a team");
        assert_eq!(
            (strong.average, strong.tier),
            (60, MAX_TIER),
            "the tier tops out"
        );
        assert_eq!(super::strength(&[]), None);
    }
}
