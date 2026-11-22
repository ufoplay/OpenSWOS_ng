//! A league played (upstream src/diy_competition.c): its next fixture
//! (`DIY_GetCurrentFixtureTeamPtrs`), its progress after a match
//! (`DIY_AdvanceLeagueProgressAfterMatch`), a match's result recorded
//! (`DIY_PlayOrSimulateLeagueFixture`, its result given cached, so no simulation runs), its
//! qualifiers (`DIY_UpdateLeagueQualifiers`), its season's moves
//! (`DIY_UpdatePromotionRelegationScratchCounts`), each from an image of the competition.

use crate::Oracle;
use crate::ffi;
use crate::running::IMAGE_LEN;
use crate::tournament::STATE_LEN;

/// The teams a league's functions read: each selected team's name (`teamName`) and global team
/// number (`globalTeamNumber`), by its identity, and the career club's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Teams {
    /// The selected teams' names and numbers, by identity.
    pub selected: Vec<(Vec<u8>, u16)>,
    /// The career club's.
    pub career: (Vec<u8>, u16),
}

/// What `DIY_PlayOrSimulateLeagueFixture` reads: the competition, the tournament's state, the
/// teams, the match's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSetup {
    /// The competition under way before.
    pub competition: Box<[u8; IMAGE_LEN]>,
    /// The tournament's state before.
    pub state: Box<[u8; STATE_LEN]>,
    /// The teams.
    pub teams: Teams,
    /// The home side's goals and the away side's (below 0x8000: the cache holds a signed word).
    pub goals: (u16, u16),
}

/// The season's moves as upstream counts them: each team's change in the scratch of the
/// teams' divisions (by the global numbers [`Teams`] gives), and the career club's flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovesCounted {
    /// Each selected team's change, by identity.
    pub selected: Vec<i8>,
    /// The career club's change.
    pub career: i8,
    /// `_playPromotedAnimation`.
    pub promoted: bool,
    /// `_playRelegatedAnimation`.
    pub relegated: bool,
}

impl Oracle {
    /// `DIY_GetCurrentFixtureTeamPtrs` of `image`: each side's standings row, home first (its
    /// record's offset in the image, 18 bytes a row).
    ///
    /// The league's round robin is upstream's compiled one of its groups' size (the European
    /// Championships' by their id): the image must state a size of 2 to 24 and a match within
    /// it.
    #[must_use]
    pub fn fixture(&mut self, image: &[u8; IMAGE_LEN]) -> (usize, usize) {
        ffi::play::fixture(image)
    }

    /// `DIY_AdvanceLeagueProgressAfterMatch` of `image` after the match of rows `home` and
    /// `away`: the image after.
    #[must_use]
    pub fn advance(
        &mut self,
        image: &[u8; IMAGE_LEN],
        home: usize,
        away: usize,
    ) -> Box<[u8; IMAGE_LEN]> {
        ffi::play::advance(image, home, away)
    }

    /// `DIY_PlayOrSimulateLeagueFixture` of `setup`, its match simulated to the result given:
    /// the competition and the tournament's state after.
    #[must_use]
    pub fn play_result(
        &mut self,
        setup: &ResultSetup,
    ) -> (Box<[u8; IMAGE_LEN]>, Box<[u8; STATE_LEN]>) {
        ffi::play::play_result(setup)
    }

    /// `DIY_UpdateLeagueQualifiers` of `image`, the tournament's state `state`, the teams
    /// `teams`: the image after.
    #[must_use]
    pub fn update_qualifiers(
        &mut self,
        image: &[u8; IMAGE_LEN],
        state: &[u8; STATE_LEN],
        teams: &Teams,
    ) -> Box<[u8; IMAGE_LEN]> {
        ffi::play::update_qualifiers(image, state, teams)
    }

    /// `DIY_UpdatePromotionRelegationScratchCounts` of `image`, the teams `teams` (each its
    /// own global number): what it counts.
    #[must_use]
    pub fn season_moves(&mut self, image: &[u8; IMAGE_LEN], teams: &Teams) -> MovesCounted {
        ffi::play::season_moves(image, teams)
    }
}
