//! The tweaks the player switches on (`[tweaks]`; upstream include/tweaks.h, src/tweaks.c):
//! small changes to the original's rules, off line only. What each one does is the rules'
//! (D6); the settings hold the player's switches.

use bitflags::bitflags;

bitflags! {
    /// The tweaks switched on (upstream `TWEAK_*`, their bits as upstream numbers them).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Tweaks: u32 {
        /// The computer's players are booked and sent off too.
        const CPU_CARDS = 1 << 0;
        /// A team-mate leaves your player on the ball alone.
        const NO_TEAMMATE_TACKLES = 1 << 1;
        /// The slide is made by the player you control.
        const TACKLE_CONTROLLED_PLAYER = 1 << 2;
        /// One borderline (sideways) tackle in 16 is a foul.
        const MORE_FOULS = 1 << 3;
        /// The computer replaces its injured players.
        const AI_INJURY_SUBS = 1 << 4;
        /// SWOS 2020's: the Amiga's keepers reach a step further.
        const STRONGER_GOALKEEPER_AI = 1 << 5;
        /// SWOS 2020's: buying is not held to the season's quota.
        const UNLIMITED_TRANSFERS = 1 << 6;
        /// A contested ball rates each side by the man at it.
        const CONTEST_OWN_RATINGS = 1 << 7;
        /// SWOS 2020's career fixes 1-3: a league fitted to its teams.
        const LEAGUE_CRASH_FIXES = 1 << 8;
        /// SWOS 2020's career fixes 4-7 in 96/97's leagues.
        const LEAGUE_RULES_2020 = 1 << 9;
        /// Every league gives 3 points for a win.
        const THREE_POINTS = 1 << 10;
        /// Every league and cup: 5 substitutes from a bench of 5.
        const FIVE_SUBSTITUTES = 1 << 11;
    }
}

impl Tweaks {
    /// The tweaks a new `SWOS.ini` starts with (upstream `TWEAKS_DEFAULT_ON`): the contest
    /// fix and SWOS 2020's league rules. Every tweak is built (upstream `PROJECT_TWEAKS`).
    pub const DEFAULT_ON: Self = Self::CONTEST_OWN_RATINGS
        .union(Self::LEAGUE_CRASH_FIXES)
        .union(Self::LEAGUE_RULES_2020)
        .union(Self::THREE_POINTS)
        .union(Self::FIVE_SUBSTITUTES);
}

impl Default for Tweaks {
    fn default() -> Self {
        Self::DEFAULT_ON
    }
}
