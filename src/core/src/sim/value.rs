//! A player's match value and a team's strength (docs/PLAN.md M5 #8a; upstream src/match_sim.c
//! `UpdatePlayerPositionAndReturnRole`, `GetPlayerPrice`, `ComputeOldPlayerSkillForViewResult`;
//! src/diy_competition.c `CountNearnessFactor`; src/career_status.c `IsPlayerInjured`). An
//! outfield player's value is his price for his place in the team's tactics: his skills, each
//! weighed by every ball area's ponders, by how much the area counts for his position and how
//! near it is to where the tactics put him, then made a price step. Upstream also copies each
//! value into the edit-tactics screen's `_playerPositionsArray` and the strength into
//! `_teamSkillCurrent` (the view's).

use openswos_tac_resource::{BALL_AREA_COLUMNS, PlayerSpots, TeamTactics};
use thiserror::Error;

use super::tables::{
    AREA_DISTANCES, AREA_PONDERS, AREAS, POSITION_WEIGHTS, PRICE_RISES, SKILL_PONDERS, SKILLS,
    SPOT_AREAS,
};
use super::team::MatchTeam;
use crate::competition::running::TeamBytes;

/// The controls of a team the computer plays (`TEAM_COMPUTER`).
const COMPUTER: u8 = 1;

/// The goalkeeper's position.
const GOALKEEPER: u8 = 0;

/// How far a player's position is shifted in `positionAndFace` (`PF_POSITION_SHIFT`).
const POSITION_SHIFT: u8 = 5;

/// A player on loan (`PF_LOAN`).
const ON_LOAN: u8 = 0x02;

/// The goalkeeper's place in the line-up.
const KEEPER_PLACE: usize = 0;

/// The first substitute's place in the line-up (the eleven before it the team).
const FIRST_SUBSTITUTE: usize = 11;

/// A player's cards (`cardsInjuries` bits 1-0).
const CARD_BITS: u8 = 0x03;

/// A player's injuries (bits 4-2).
const INJURY_BITS: u8 = 0x1C;

/// The injury that does not keep a player out.
const MINOR_INJURY: u8 = 0x04;

/// A player's status bucket (bits 7-5).
const STATUS_BUCKET: u8 = 0xE0;

/// The status bucket that does not keep a player out.
const MINOR_BUCKET: u8 = 0x20;

/// The mode in which a player's cards keep him out, not his injuries.
const CARDS_MODE: i16 = 1;

/// The bits of each skill nibble a value counts (3 of 4).
const SKILL_NIBBLES: u32 = 0x0777_7777;

/// What each skill nibble is raised by: 1 to 8.
const SKILL_ONE: u32 = 0x0111_1111;

/// A nibble.
const NIBBLE: u32 = 0xF;

/// A nibble's bits.
const NIBBLE_BITS: u32 = 4;

/// The first ball area where the teammates the tactics put in a player's area share his value
/// (the second band); [`SHARED_END`] the first past them.
const SHARED_FIRST: usize = 5;

/// The first ball area past those where teammates share a value (the seventh band's).
const SHARED_END: usize = 30;

/// The ball area of the shared bands where teammates do not share.
const NOT_SHARED: usize = 7;

/// How far an area's weighed value is shifted down for its distance.
const DISTANCE_SHIFT: u32 = 3;

/// What the summed values are divided by into a price step.
const PRICE_DIVISOR: u32 = 350;

/// The steps the price step is lowered by.
const PRICE_OFFSET: u16 = 8;

/// The highest price step.
const MAX_PRICE: u16 = 49;

/// The distance from which an area counts as near the player's area, for his nearness.
const NEAR_PLAYER: u8 = 8;

/// The distance from which a spot counts as near the ball's area.
const NEAR_BALL: u8 = 4;

/// What each substitute weighs in the team's strength against an outfield player's 1 (a
/// goalkeeper on the bench 1).
const SUBSTITUTE_WEIGHT: u16 = 4;

/// How far a substitute's value is shifted up for his weight.
const SUBSTITUTE_SHIFT: u16 = 2;

/// What the team's summed values are divided by.
const STRENGTH_DIVISOR: u16 = 15;

/// What makes a player unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusRules {
    /// Upstream's mode (`_inEditTactics`; a simulated result sets it to the competition's
    /// suspensions' track, `substitutesCount`): 1, a player's cards keep him out; 0 or above,
    /// his injuries; below 0, both.
    pub mode: i16,
    /// A training game (`_trainingGame`): only the status bucket counts.
    pub training: bool,
    /// The testers' rule (`TestRules_NoInjuriesCards`, SWOS.ini's `[testing]
    /// no_injuries_cards`): nobody is unavailable.
    pub no_injuries_cards: bool,
}

/// A player's match value and his position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatchValue {
    /// His position (0 the goalkeeper to 7 the striker).
    pub role: u8,
    /// His value (a signed byte's, as a word).
    pub value: u16,
}

/// Why a value, or a simulated result, cannot be reckoned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SimError {
    /// A line-up place past the team's line-up (upstream reads past it).
    #[error("line-up place {0} is past the team's")]
    LineUpPastOrder(usize),
    /// A line-up place naming no record of the team (upstream reads past it).
    #[error("player {0} is past the team's records")]
    PlayerPastTeam(i8),
    /// A record past the team's.
    #[error("record {0} is past the team's")]
    RecordPastTeam(usize),
    /// A ball area past the pitch's 35.
    #[error("area {0} is past the pitch's")]
    AreaPastPitch(usize),
    /// A price whose rise is past the table's (a price past the steps: upstream reads past
    /// it).
    #[error("a value {0} steps above the price")]
    RisePastTable(u8),
    /// Tactics past the session's table (upstream reads past it).
    #[error("tactics {0} are past the table")]
    TacticsPastTable(i8),
    /// A place the tactics put no outfield player in (upstream reads past them).
    #[error("place {0} has no outfield player in the tactics")]
    PlayerPastTactics(usize),
    /// A spot past the pitch grid's rows (upstream reads past its map).
    #[error("spot {0:#04x} is past the pitch")]
    SpotPastPitch(u8),
    /// No substitute counted where the team has some (upstream divides by 0).
    #[error("{0} substitutes counted to no weight")]
    NoWeight(u16),
    /// A league's place or team past the competition's image (upstream reads past it).
    #[error("index {0} is past the competition's")]
    IndexPastCompetition(usize),
    /// A team not in its league (upstream halts, 0x6006).
    #[error("team {0:?} is not in its league (upstream halts, 0x6006)")]
    NotInCompetition(TeamBytes),
    /// A team that is not loaded.
    #[error("team {0:?} is not loaded")]
    UnknownTeam(TeamBytes),
    /// A record the stand-ins' table does not hold (upstream reads past it).
    #[error("record {0} is past the stand-ins' table")]
    StandInPastTable(i8),
    /// Substitutes past the stand-ins' tables (upstream reads past them).
    #[error("{0} substitutes are past the stand-ins' tables")]
    SubstitutesPastTable(u16),
    /// A player's goals past the team's record (upstream writes past it).
    #[error("goal column {0} is past the team's record")]
    ColumnPastTeam(u16),
    /// A side whose identity names no squad (upstream reads past its loaded teams).
    #[error("side {0} names no squad")]
    NoSquad(i32),
    /// Both sides the same squad (upstream plays one record as both).
    #[error("both sides are squad {0}")]
    SameSquad(i32),
    /// Extra time asked of a simulation before any normal time.
    #[error("extra time before a normal time")]
    NoNormalTime,
}

/// A player's position (`positionAndFace` bits 7-5).
fn role(position_and_face: u8) -> u8 {
    position_and_face >> POSITION_SHIFT
}

/// Whether a player of `cards_injuries` is unavailable under `rules` (upstream
/// `IsPlayerInjured`): his injuries (a minor one aside) unless his cards count, his cards
/// where they count, his status bucket (a minor one aside) always; in a training game only his
/// bucket; nobody under the testers' rule.
#[must_use]
pub fn unavailable(cards_injuries: u8, rules: &StatusRules) -> bool {
    if rules.no_injuries_cards {
        return false;
    }
    if !rules.training {
        if rules.mode != CARDS_MODE {
            let injury = cards_injuries & INJURY_BITS;
            if injury != 0 && injury != MINOR_INJURY {
                return true;
            }
        }
        if (rules.mode == CARDS_MODE || rules.mode < 0) && cards_injuries & CARD_BITS != 0 {
            return true;
        }
    }
    let bucket = cards_injuries & STATUS_BUCKET;
    bucket != 0 && bucket != MINOR_BUCKET
}

/// The tactics `team` plays, from the session's `tactics` (upstream `_teamTacticsTable`).
fn team_tactics<'a>(
    team: &MatchTeam,
    tactics: &'a [TeamTactics],
) -> Result<&'a TeamTactics, SimError> {
    let index = team.tactics().cast_signed();
    usize::try_from(index)
        .ok()
        .and_then(|index| tactics.get(index))
        .ok_or(SimError::TacticsPastTable(index))
}

/// The spot `spots` puts a player in when the ball is in area `area`.
fn spot(spots: &PlayerSpots, area: usize) -> u8 {
    u8::from(spots.0[area / BALL_AREA_COLUMNS][area % BALL_AREA_COLUMNS])
}

/// The ball area of spot `spot` (upstream `_quadrantCoordinatesToBallQuadrant`).
fn spot_area(spot: u8) -> Result<usize, SimError> {
    let [row, column] = [spot >> NIBBLE_BITS, spot & 0xF];
    SPOT_AREAS
        .get(usize::from(row))
        .map(|row| usize::from(row[usize::from(column)]))
        .ok_or(SimError::SpotPastPitch(spot))
}

/// A value scaled by how near the ball's area `ball` is to where `spots` puts the player for
/// the areas near area `area` (upstream `CountNearnessFactor`): `price` times the near areas
/// whose spot is near the ball, over the near areas.
///
/// # Errors
///
/// [`SimError`] when an area is past the pitch or a spot past the pitch grid.
pub fn nearness(
    area: usize,
    ball: usize,
    price: u16,
    spots: &PlayerSpots,
) -> Result<u16, SimError> {
    for checked in [area, ball] {
        if checked >= AREAS {
            return Err(SimError::AreaPastPitch(checked));
        }
    }
    let (mut near_ball, mut near_player) = (0u32, 0u32);
    for other in (0..AREAS).rev() {
        if other == area || AREA_DISTANCES[area][other] < NEAR_PLAYER {
            continue;
        }
        near_player += 1;
        if AREA_DISTANCES[ball][spot_area(spot(spots, other))?] >= NEAR_BALL {
            near_ball += 1;
        }
    }
    let scaled = u32::from(price) * near_ball / near_player;
    Ok(low_word(scaled))
}

/// The low word of `value`.
fn low_word(value: u32) -> u16 {
    let [low, high, _, _] = value.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// The price of the player of record `slot` at line-up place `place` (1 to 10) for his place
/// in `team`'s tactics (upstream `GetPlayerPrice`): over the 35 ball areas, his seven skills
/// (each 1 to 8) weighed by the area's ponders, halved, by how much the area counts for his
/// position and how near it is; a team the computer does not play shares an area's value
/// among the teammates the tactics put there and scales it by its nearness. The sum, as a
/// price step, is the player's price, or a step above it.
///
/// # Errors
///
/// [`SimError`] when the record is past the team's, the tactics past the session's table, the
/// place has no outfield player in them, a spot is past the pitch grid, or the price is past
/// the steps.
pub fn player_price(
    team: &MatchTeam,
    slot: usize,
    place: usize,
    tactics: &[TeamTactics],
) -> Result<i16, SimError> {
    if slot >= team.records() {
        return Err(SimError::RecordPastTeam(slot));
    }
    let position = usize::from(role(team.position_and_face(slot)));
    let skills = u32::from_be_bytes(team.skills(slot)) & SKILL_NIBBLES;
    let skills = skills + SKILL_ONE;
    let tactics = team_tactics(team, tactics)?;
    let spots = place
        .checked_sub(1)
        .and_then(|player| tactics.players.get(player))
        .ok_or(SimError::PlayerPastTactics(place))?;
    let human = team.controls() != COMPUTER;
    let mut sum: u16 = 0;
    for area in (0..AREAS).rev() {
        let ball = spot_area(spot(spots, area))?;
        let mut sharing: u16 = 1;
        if human && (SHARED_FIRST..SHARED_END).contains(&area) && area != NOT_SHARED {
            sharing = 0;
            for other in &tactics.players {
                if spot_area(spot(other, area))? == ball {
                    sharing += 1;
                }
            }
        }
        let mut weighed: u16 = 0;
        let (mut nibbles, mut ponders) = (skills, AREA_PONDERS[ball]);
        for _ in 0..SKILLS {
            let ponder = ponders & NIBBLE;
            if ponder != 0 {
                let skill = u16::try_from(nibbles & NIBBLE).expect("a nibble");
                let ponder = usize::try_from(ponder).expect("a nibble");
                weighed += skill * u16::from(SKILL_PONDERS[ponder]);
            }
            nibbles >>= NIBBLE_BITS;
            ponders >>= NIBBLE_BITS;
        }
        weighed >>= 1;
        let weight = if position == usize::from(GOALKEEPER) {
            0
        } else {
            POSITION_WEIGHTS[position - 1][ball]
        };
        let weighted = low_word(u32::from(weighed) * u32::from(weight));
        let mut distant =
            (u32::from(weighted) * u32::from(AREA_DISTANCES[area][ball])) >> DISTANCE_SHIFT;
        if human {
            // Faithful to the original: one teammate less, a word (none, the player alone).
            let others = sharing.wrapping_sub(1);
            if others != 0 {
                distant /= u32::from(others);
            }
        }
        let mut value = low_word(distant);
        if human {
            value = nearness(area, ball, value, spots)?;
        }
        sum = sum.wrapping_add(value);
    }
    // Faithful to the original: the sum sign-extended, then divided as unsigned.
    let step = i32::from(sum.cast_signed()).cast_unsigned() / PRICE_DIVISOR;
    let step = low_word(step).wrapping_sub(PRICE_OFFSET);
    let step = if step.cast_signed() < 0 {
        0
    } else {
        step.min(MAX_PRICE)
    };
    let [step, _] = step.to_le_bytes();
    let price = team.price(slot).cast_unsigned();
    let above = step.wrapping_sub(price);
    if above.cast_signed() < 0 {
        return Ok(i16::from(step));
    }
    let rise = PRICE_RISES
        .get(usize::from(above))
        .ok_or(SimError::RisePastTable(above))?;
    Ok(i16::from(rise.wrapping_add(price)))
}

/// The record line-up place `place` fields, checked within the team's records.
fn place_slot(team: &MatchTeam, place: usize) -> Result<usize, SimError> {
    let slot = team
        .line_up(place)
        .ok_or(SimError::LineUpPastOrder(place))?;
    usize::try_from(slot)
        .ok()
        .filter(|&slot| slot < team.records())
        .ok_or(SimError::PlayerPastTeam(slot))
}

/// The match value of the player at line-up place `place` of `team` (upstream
/// `UpdatePlayerPositionAndReturnRole`), kept in his record: the value kept, while the team's
/// values are; else a goalkeeper's price in goal, a substitute's price, an outfield player's
/// price for his place in the tactics ([`player_price`]), 0 for a player out of his place
/// (an outfield player in goal, a goalkeeper outfield); at least 1; for a team the computer
/// does not play, 1 on loan and 0 unavailable ([`unavailable`]).
///
/// # Errors
///
/// [`SimError`] when the place is past the line-up, names no record of the team, or the price
/// cannot be reckoned.
pub fn match_value(
    team: &mut MatchTeam,
    place: usize,
    tactics: &[TeamTactics],
    rules: &StatusRules,
) -> Result<MatchValue, SimError> {
    let slot = place_slot(team, place)?;
    let position_and_face = team.position_and_face(slot);
    let role = role(position_and_face);
    if team.values_dirty() == 0 {
        let value = i16::from(team.match_value(slot)).cast_unsigned();
        return Ok(MatchValue { role, value });
    }
    let in_goal = place == KEEPER_PLACE;
    let value = if in_goal || place >= FIRST_SUBSTITUTE {
        if in_goal && role != GOALKEEPER {
            None
        } else {
            Some(i16::from(team.price(slot)))
        }
    } else if role == GOALKEEPER {
        None
    } else {
        Some(player_price(team, slot, place, tactics)?)
    };
    let value = match value {
        None => 0,
        Some(value) => {
            let mut value = if value == 0 { 1 } else { value };
            if team.controls() != COMPUTER {
                if position_and_face & ON_LOAN != 0 {
                    value = 1;
                }
                if unavailable(team.cards_injuries(slot), rules) {
                    value = 0;
                }
            }
            value
        }
    };
    let [low, _] = value.to_le_bytes();
    team.set_match_value(slot, low.cast_signed());
    Ok(MatchValue {
        role,
        value: value.cast_unsigned(),
    })
}

/// The strength of `team` for a simulated result (upstream
/// `ComputeOldPlayerSkillForViewResult`), its `substitutes` on the bench counted: the
/// substitutes' values, each weighing 4 (a goalkeeper 1), averaged, rounded; plus the eleven's,
/// over 15; halved when no goalkeeper is in goal; divided by one more for each outfield player
/// of no value. Every value is reckoned again ([`match_value`]), then kept.
///
/// # Errors
///
/// [`SimError`] when a value cannot be reckoned, or the substitutes are counted to no weight
/// (more than a signed word holds: upstream divides by 0).
pub fn strength(
    team: &mut MatchTeam,
    substitutes: u16,
    tactics: &[TeamTactics],
    rules: &StatusRules,
) -> Result<u16, SimError> {
    let mut divisor: u16 = 1;
    let mut sum: u16 = 0;
    if substitutes != 0 {
        let counted = usize::try_from(substitutes.cast_signed()).unwrap_or_default();
        let mut weight: u16 = 0;
        for place in FIRST_SUBSTITUTE..FIRST_SUBSTITUTE + counted {
            let MatchValue { role, value } = match_value(team, place, tactics, rules)?;
            let (scaled, weighs) = if role == GOALKEEPER {
                (value, 1)
            } else {
                (value << SUBSTITUTE_SHIFT, SUBSTITUTE_WEIGHT)
            };
            weight = weight.wrapping_add(weighs);
            sum = sum.wrapping_add(scaled);
        }
        if weight == 0 {
            return Err(SimError::NoWeight(substitutes));
        }
        let rounded = u32::from(weight >> 1) + u32::from(sum);
        sum = low_word(rounded / u32::from(weight));
    }
    for place in 0..FIRST_SUBSTITUTE {
        let MatchValue { role, value } = match_value(team, place, tactics, rules)?;
        if value == 0 && role != GOALKEEPER {
            divisor += 1;
        }
        sum = sum.wrapping_add(value);
    }
    let mut strength = sum / STRENGTH_DIVISOR;
    let keeper = place_slot(team, KEEPER_PLACE)?;
    if role(team.position_and_face(keeper)) != GOALKEEPER {
        strength >>= 1;
    }
    team.set_values_dirty(0);
    Ok(strength / divisor)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rules of a competition whose injuries count.
    const INJURIES: StatusRules = StatusRules {
        mode: 0,
        training: false,
        no_injuries_cards: false,
    };

    #[test]
    fn a_player_is_out_by_his_injuries_his_cards_or_his_bucket_as_the_mode_says() {
        let cards = StatusRules {
            mode: CARDS_MODE,
            ..INJURIES
        };
        let both = StatusRules {
            mode: -1,
            ..INJURIES
        };
        // A serious injury (8), a minor one (4), a card (1), a serious bucket (0x40).
        assert!(unavailable(0x08, &INJURIES));
        assert!(!unavailable(0x04, &INJURIES));
        assert!(!unavailable(0x01, &INJURIES));
        assert!(!unavailable(0x08, &cards));
        assert!(unavailable(0x01, &cards));
        assert!(unavailable(0x01, &both) && unavailable(0x08, &both));
        assert!(unavailable(0x40, &INJURIES) && !unavailable(0x20, &INJURIES));
        let training = StatusRules {
            training: true,
            ..both
        };
        assert!(!unavailable(0x09, &training) && unavailable(0x40, &training));
        let testers = StatusRules {
            no_injuries_cards: true,
            ..both
        };
        assert!(!unavailable(0xFF, &testers));
    }

    #[test]
    fn a_spot_past_the_grid_s_rows_is_refused() {
        assert_eq!(spot_area(0x00), Ok(0));
        assert_eq!(spot_area(0xE5), Ok(usize::from(SPOT_AREAS[14][5])));
        assert_eq!(spot_area(0xF0), Err(SimError::SpotPastPitch(0xF0)));
    }

    #[test]
    fn a_word_s_low_half_is_kept() {
        assert_eq!(low_word(0x0001_2345), 0x2345);
    }
}
