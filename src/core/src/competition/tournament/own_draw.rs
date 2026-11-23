//! A world's own draw of a tournament (docs/PLAN.md M5 #4d2, #7; `rules/draws.json`): each
//! stage's order from its own table, not the engine's draw, and how its stages of groups rank
//! the teams they send on (upstream diy_competition.c `buildCompSpecificTeamList`,
//! `buildCompSpecificTeamShuffle`, picked by competition id; `DIY_AdjustWorldCupBracketScores`).
//! Built and checked from the world's data ([`crate::world`]).

use super::TournamentState;
use super::stage::LoadError;

/// The teams a tournament's order holds (`teamIndexOrder[64]`): a draw reads no place past
/// them.
pub(crate) const ORDER_PLACES: u8 = 64;

/// The groups a pattern's bits hold (upstream compares a byte).
pub(crate) const PATTERN_GROUPS: u8 = 8;

/// The highest group upstream shifts a bit by (`1 << group`, an `int`: a group of 31 or
/// more, or below 0, is past its shifts).
const MAX_SHIFT: i8 = 30;

/// How a stage of groups ranks the teams it sends on: the next stage's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ranking {
    /// The engine's: every stage of groups by place, then each place's teams by their record;
    /// a place only some go through on ranks them by their record, then by their group
    /// (upstream `DIY_BuildSeededBracketAndUpdateScores`).
    ByPlace,
    /// The first stage only: its qualifiers group by group, then the best of the place only
    /// some go through on, by their record, the four ranked by their group; a later stage's
    /// tables kept as they are (upstream `DIY_AdjustWorldCupBracketScores`).
    ByGroup,
}

/// A stage's own draw: the order its teams take, from the order the stage before left.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StageDraw {
    /// The first places kept in their order.
    Keep(u8),
    /// Each place the team of that place before.
    Order(Vec<u8>),
    /// The order set by the groups the best placed come from.
    BestPlaced(BestPlaced),
}

/// A stage drawn by where the best placed teams of the stage of groups before come from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BestPlaced {
    /// The first place of the best placed.
    pub(crate) first: u8,
    /// The patterns, the first that matches taken: the bits of the groups the best placed
    /// come from, and their order then.
    pub(crate) patterns: Vec<(u8, Vec<u8>)>,
    /// Each place of the stage: the team of a place before, or the next best placed.
    pub(crate) places: Vec<Option<u8>>,
}

/// A tournament's own draw: its ranking and each stage's draw (a stage past the list drawn as
/// its last).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnDraw {
    /// How its stages of groups rank.
    pub(crate) ranking: Ranking,
    /// Each stage's draw, from the first: one at least.
    pub(crate) stages: Vec<StageDraw>,
}

impl OwnDraw {
    /// How its stages of groups rank.
    #[must_use]
    pub fn ranking(&self) -> Ranking {
        self.ranking
    }

    /// Stage `stage`'s draw: the list's last for a stage past it.
    #[must_use]
    pub fn stage(&self, stage: usize) -> &StageDraw {
        let last = self.stages.len() - 1;
        &self.stages[stage.min(last)]
    }
}

impl StageDraw {
    /// The order the draw gives from `state`'s: as many places as the draw writes (upstream's
    /// scratch holds no others).
    ///
    /// # Errors
    ///
    /// [`LoadError`] when a best placed team's group is past upstream's shifts, or no pattern
    /// matches the best placed's groups (upstream halts, 0x8034).
    pub(crate) fn drawn(&self, state: &TournamentState) -> Result<Vec<u8>, LoadError> {
        let place = |place: u8| state.team_index(usize::from(place));
        match self {
            Self::Keep(places) => Ok((0..*places).map(place).collect()),
            Self::Order(order) => Ok(order.iter().map(|&at| place(at)).collect()),
            Self::BestPlaced(best) => best.drawn(state),
        }
    }
}

impl BestPlaced {
    /// The best placed this draw orders.
    fn count(&self) -> usize {
        self.places.iter().filter(|place| place.is_none()).count()
    }

    /// The order from `state`'s: the groups of the best placed (each team's group, by its
    /// index) matched against the patterns, then each place's team.
    fn drawn(&self, state: &TournamentState) -> Result<Vec<u8>, LoadError> {
        let mut bits: u16 = 0;
        for rank in 0..self.count() {
            let index = state.team_index(usize::from(self.first) + rank);
            let group = state.group(usize::from(index)).cast_signed();
            if !(0..=MAX_SHIFT).contains(&group) {
                return Err(LoadError::BestPlacedGroup(group.cast_unsigned()));
            }
            // Faithful to the original: an `int`'s bit kept in a word, compared as a byte.
            bits |= u16::try_from(1u32 << group & u32::from(u16::MAX)).expect("a word");
        }
        let [groups, _] = bits.to_le_bytes();
        let (_, order) = self
            .patterns
            .iter()
            .find(|(pattern, _)| *pattern == groups)
            .ok_or(LoadError::NoBestPlacedPattern(groups))?;
        let mut next = order.iter();
        Ok(self
            .places
            .iter()
            .map(|place| {
                let at = place.unwrap_or_else(|| {
                    self.first + next.next().expect("a pattern orders every best placed")
                });
                state.team_index(usize::from(at))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state whose order is the reverse of the indices, team `index`'s group `index / 4`.
    fn state() -> TournamentState {
        let mut state = TournamentState::default();
        for place in 0..usize::from(ORDER_PLACES) {
            let index = u8::try_from(usize::from(ORDER_PLACES) - 1 - place).expect("64");
            state.set_team_index(place, index);
            state.set_group(usize::from(index), index / 4);
        }
        state
    }

    #[test]
    fn a_kept_or_ordered_stage_takes_the_places_before() {
        let state = state();
        assert_eq!(StageDraw::Keep(3).drawn(&state), Ok(vec![63, 62, 61]));
        assert_eq!(StageDraw::Order(vec![2, 0, 1]).drawn(&state), Ok(vec![61, 63, 62]));
    }

    #[test]
    fn the_best_placed_s_groups_pick_their_order() {
        let mut state = state();
        // Places 2 and 3: teams 61 (group 15) and 60 (group 15), then made groups 1 and 3.
        state.set_group(61, 1);
        state.set_group(60, 3);
        let best = BestPlaced {
            first: 2,
            patterns: vec![(0b0011, vec![0, 1]), (0b1010, vec![1, 0])],
            places: vec![Some(0), None, Some(1), None],
        };
        assert_eq!(best.drawn(&state), Ok(vec![63, 60, 62, 61]));
        state.set_group(60, 2);
        assert_eq!(best.drawn(&state), Err(LoadError::NoBestPlacedPattern(0b0110)));
        // A group past a byte's bits sets none the patterns compare.
        state.set_group(60, 9);
        assert_eq!(best.drawn(&state), Err(LoadError::NoBestPlacedPattern(0b0010)));
        state.set_group(60, 31);
        assert_eq!(best.drawn(&state), Err(LoadError::BestPlacedGroup(31)));
        state.set_group(60, 0x80);
        assert_eq!(best.drawn(&state), Err(LoadError::BestPlacedGroup(0x80)));
    }

    #[test]
    fn a_stage_past_the_list_is_drawn_as_its_last() {
        let own = OwnDraw {
            ranking: Ranking::ByPlace,
            stages: vec![StageDraw::Keep(4), StageDraw::Keep(2)],
        };
        assert_eq!(own.stage(0), &StageDraw::Keep(4));
        assert_eq!(own.stage(1), &StageDraw::Keep(2));
        assert_eq!(own.stage(5), &StageDraw::Keep(2));
        assert_eq!(own.ranking(), Ranking::ByPlace);
    }
}
