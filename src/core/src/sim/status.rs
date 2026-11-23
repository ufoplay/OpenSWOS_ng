//! A team's players' availability, and the status buckets a simulated match gives them
//! (docs/PLAN.md M5 #8b; upstream src/career_status.c `GetNumberOfAvailablePlayers`,
//! `GetNumberOfAvailableSubstitutes`; src/match_sim.c `InitPlayerRemapTableForTeam`). A player
//! in a status bucket above the minor one is out; after a simulated match, a few players at
//! most are put in one, by the odds of upstream's table. A computer club's records keep only
//! the kinds a world opens (upstream `cpu_kept`, src/world_rules.c: the original keeps none,
//! the SWOS 96/97 Mod's rules open them in another world). A friendly club's long bench
//! (`LongBench_FrOrder`, `SquadExtras_BenchRecord`) joins with its component (M5 #9).

use openswos_rng::Rng;

use super::tables::STATUS_BUCKET_ODDS;
use super::team::{CAREER_LINE_UP, MatchTeam, PLAYERS};
use super::value::{SimError, StatusRules, unavailable};

/// The controls of a team the computer plays (`TEAM_COMPUTER`).
pub(super) const COMPUTER: u8 = 1;

/// The goalkeeper's position.
const GOALKEEPER: u8 = 0;

/// How far a player's position is shifted in `positionAndFace`.
const POSITION_SHIFT: u8 = 5;

/// The mode in which both a player's cards and his injuries keep him out.
const BOTH: i16 = -1;

/// The players a career game's simulated match may put in a status bucket.
const CAREER_BUDGET: i16 = 4;

/// The players another game's may.
const BUDGET: i16 = 2;

/// The last of a selected team's line-up places a match's buckets reach (its eleven).
const LAST_OF_ELEVEN: usize = 10;

/// The draw under which the goalkeeper in goal is looked at.
const KEEPER_ODDS: u16 = 2;

/// The draw under which an outfield player is looked at for his cards and injuries.
const PLAYER_ODDS: u16 = 16;

/// The bits of a draw a card or an injury is picked by (of 32).
const STATUS_DRAW: u16 = 0x1F;

/// The first pick of a red card (or of a worse injury): the 29 below it an injury, or none.
const RED_CARD_DRAW: u16 = 0x1D;

/// A red card's cards bits.
pub(super) const RED_CARD: u8 = 2;

/// A player's cards (`cardsInjuries` bits 1-0).
pub(super) const CARD_BITS: u8 = 0x03;

/// A player's injuries (bits 4-2).
pub(super) const INJURY_BITS: u8 = 0x1C;

/// A minor injury, which keeps no player out.
const MINOR_INJURY: u8 = 0x04;

/// An injury that keeps a player out (a player injured already, or the statuses not applied).
pub(super) const INJURED: u8 = 0x08;

/// A fit player's worse injury.
pub(super) const MODERATE_INJURY: u8 = 0x0C;

/// An injured player's worse injury.
pub(super) const SERIOUS_INJURY: u8 = 0x10;

/// The draw under which a player of no bucket is put in one.
const NEW_BUCKET_ODDS: u16 = 5;

/// The draw under which a player of the minor bucket is put in another.
const MINOR_BUCKET_ODDS: u16 = 0x40;

/// The bits of a draw a new bucket is picked by (out of 64).
const BUCKET_DRAW: u16 = 0x3F;

/// The minor status bucket, the first.
pub(super) const MINOR_BUCKET: u8 = 0x20;

/// A player's status bucket (`cardsInjuries` bits 7-5).
pub(super) const STATUS_BUCKET: u8 = 0xE0;

/// A player's cards and injuries (bits 4-0).
pub(super) const CARDS_AND_INJURIES: u8 = 0x1F;

/// The kinds of statuses a computer club's records keep (upstream `WorldRules_CpuOpen`: the
/// SWOS 96/97 Mod's CPU RED CARDS and CPU INJURIES).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComputerStatuses {
    /// Its cards and injuries (`cardsInjuries` bits 4-0).
    pub cards: bool,
    /// Its status buckets (bits 7-5).
    pub injuries: bool,
}

/// The kind of game a match is played in (upstream `_gameType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameType {
    /// A DIY or a preset competition (1, 2): a computer club's statuses count down before each
    /// of its simulated matches, as a world opens them.
    Competition,
    /// A season (3): a computer club's count down each day instead (the career's calendar,
    /// M7).
    Season,
    /// A career (4): likewise, and a simulated match may put four players in a status bucket,
    /// not two.
    Career,
}

impl GameType {
    /// The players a simulated match may put in a status bucket, those already out counted.
    fn statuses_budget(self) -> i16 {
        match self {
            Self::Career => CAREER_BUDGET,
            Self::Competition | Self::Season => BUDGET,
        }
    }
}

/// What a simulated match's statuses depend on beyond the teams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SimRules {
    /// What makes a player unavailable (its mode the competition's suspensions' track).
    pub status: StatusRules,
    /// The kind of game.
    pub game: GameType,
    /// The kinds of statuses a computer club's records keep.
    pub computer: ComputerStatuses,
}

/// How many players are out and how many available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Availability {
    /// The players out.
    pub out: u16,
    /// The players available (a word: the places counted less those out).
    pub available: u16,
}

/// A player's position (`positionAndFace` bits 7-5).
fn role(position_and_face: u8) -> u8 {
    position_and_face >> POSITION_SHIFT
}

/// The record line-up place `place` fields, checked within the team's records.
pub(super) fn place_slot(team: &MatchTeam, place: usize) -> Result<usize, SimError> {
    let slot = team
        .line_up(place)
        .ok_or(SimError::LineUpPastOrder(place))?;
    usize::try_from(slot)
        .ok()
        .filter(|&slot| slot < team.records())
        .ok_or(SimError::PlayerPastTeam(slot))
}

/// The line-up places a team's counts walk: the career club's players, a selected team's 16
/// (upstream's `totalCount`), as a signed word walks them.
pub(super) fn squad(team: &MatchTeam) -> u16 {
    match team.career_line_up() {
        Some(line_up) => line_up.players,
        None => u16::try_from(PLAYERS).expect("16"),
    }
}

/// A count as upstream walks it: a word that, as a signed one, below 1 walks none.
pub(super) fn walked(count: u16) -> usize {
    usize::try_from(count.cast_signed()).unwrap_or_default()
}

/// `team`'s players out and available under `rules` (upstream `GetNumberOfAvailablePlayers`,
/// its mode `rules.mode`): of its squad's places, or of the first `places` when not 0.
///
/// # Errors
///
/// [`SimError`] when a place is past the line-up or names no record of the team.
pub fn available_players(
    team: &MatchTeam,
    places: u16,
    rules: &StatusRules,
) -> Result<Availability, SimError> {
    let counted = if places == 0 { squad(team) } else { places };
    let mut out: u16 = 0;
    for place in (0..walked(counted)).rev() {
        let slot = place_slot(team, place)?;
        if unavailable(team.cards_injuries(slot), rules) {
            out += 1;
        }
    }
    Ok(Availability {
        out,
        available: counted.wrapping_sub(out),
    })
}

/// `team`'s goalkeepers out and available under `rules` (upstream
/// `GetNumberOfAvailableSubstitutes`), of its squad's places.
///
/// # Errors
///
/// [`SimError`] when a place is past the line-up or names no record of the team.
pub fn available_keepers(team: &MatchTeam, rules: &StatusRules) -> Result<Availability, SimError> {
    let (mut out, mut available): (u16, u16) = (0, 0);
    for place in (0..walked(squad(team))).rev() {
        let slot = place_slot(team, place)?;
        if role(team.position_and_face(slot)) != GOALKEEPER {
            continue;
        }
        if unavailable(team.cards_injuries(slot), rules) {
            out += 1;
        } else {
            available += 1;
        }
    }
    Ok(Availability { out, available })
}

/// The status buckets a simulated match gives `team`'s players (upstream
/// `InitPlayerRemapTableForTeam`), drawn by `rng`: the eleven's places (the career club's
/// whole squad when it fields no stand-ins), last first, each a player or, a draw in two, the
/// stand-in `stand_ins` gives his record; the goalkeeper in goal a draw in 128, a keeper only
/// with every keeper fit; a player of no bucket 5 draws in 256, of the minor one 64, put in a
/// bucket by upstream's odds; up to two players (four in a career game) less those already
/// out. A computer club's records keep only the kinds `rules` opens, none a draw.
///
/// # Errors
///
/// [`SimError`] when a place is past the line-up, names no record of the team or of the
/// stand-ins.
pub fn status_buckets(
    team: &mut MatchTeam,
    stand_ins: Option<&[u8; CAREER_LINE_UP]>,
    rules: &SimRules,
    rng: &mut Rng,
) -> Result<(), SimError> {
    computer_kept(team, rules, |team| {
        bucket_draws(team, stand_ins, rules, rng)
    })
}

/// `draws` on `team`, a computer club's records keeping only the kinds `rules` opens, none a
/// draw (upstream `cpu_kept`): the others' bits put back after.
fn computer_kept(
    team: &mut MatchTeam,
    rules: &SimRules,
    draws: impl FnOnce(&mut MatchTeam) -> Result<(), SimError>,
) -> Result<(), SimError> {
    let mut kept: u8 = 0;
    if team.controls() == COMPUTER {
        if !rules.computer.cards && !rules.computer.injuries {
            return Ok(());
        }
        if !rules.computer.cards {
            kept |= CARDS_AND_INJURIES;
        }
        if !rules.computer.injuries {
            kept |= STATUS_BUCKET;
        }
    }
    let was: Vec<u8> = (0..PLAYERS).map(|slot| team.cards_injuries(slot)).collect();
    let drawn = draws(team);
    if kept != 0 {
        for (slot, &before) in was.iter().enumerate() {
            let after = team.cards_injuries(slot);
            team.set_cards_injuries(slot, (after & !kept) | (before & kept));
        }
    }
    drawn
}

/// The stand-in `stand_ins` gives record `slot`, as a signed byte (below 0, none).
fn stand_in(stand_ins: &[u8; CAREER_LINE_UP], slot: i8) -> Result<i8, SimError> {
    usize::try_from(slot)
        .ok()
        .and_then(|index| stand_ins.get(index))
        .map(|&stand_in| stand_in.cast_signed())
        .ok_or(SimError::StandInPastTable(slot))
}

/// A record of the team's, checked.
fn record(team: &MatchTeam, slot: i8) -> Result<usize, SimError> {
    usize::try_from(slot)
        .ok()
        .filter(|&slot| slot < team.records())
        .ok_or(SimError::PlayerPastTeam(slot))
}

/// The cards and injuries a simulated match gives `team`'s players (upstream
/// `ApplyRandomPlayerStatusAfterSim`), drawn by `rng`: none in a training game; the eleven's
/// places, last first, each a player or, a draw in two, his stand-in; the goalkeeper in goal a
/// draw in 128, the others 16 in 256, a keeper only with every keeper fit; then, under the
/// competition's track of cards (`rules.status.mode` not 0), a red card a draw in ten; else an
/// injury, by a draw of 32 (a fit player's a minor one, or none at all where `statuses_apply`
/// says not; a serious one for a player injured already, or 3 draws in 32), up to two players
/// (four in a career game) less those already out. A computer club's records keep only the
/// kinds `rules` opens.
///
/// # Errors
///
/// [`SimError`] when a place is past the line-up, names no record of the team or of the
/// stand-ins.
pub fn after_match(
    team: &mut MatchTeam,
    stand_ins: &[u8; CAREER_LINE_UP],
    rules: &SimRules,
    statuses_apply: bool,
    rng: &mut Rng,
) -> Result<(), SimError> {
    if rules.status.training {
        return Ok(());
    }
    computer_kept(team, rules, |team| {
        card_draws(team, stand_ins, rules, statuses_apply, rng)
    })
}

/// The cards' and injuries' draws of [`after_match`], for any team.
fn card_draws(
    team: &mut MatchTeam,
    stand_ins: &[u8; CAREER_LINE_UP],
    rules: &SimRules,
    statuses_apply: bool,
    rng: &mut Rng,
) -> Result<(), SimError> {
    if rules.status.no_injuries_cards {
        return Ok(());
    }
    let both = StatusRules {
        mode: BOTH,
        ..rules.status
    };
    let out = available_players(team, 0, &both)?.out;
    let mut budget = rules.game.statuses_budget();
    budget = budget.wrapping_sub(out.cast_signed());
    if budget == 0 {
        return Ok(());
    }
    let cards_track = rules.status.mode != 0;
    for place in (0..=LAST_OF_ELEVEN).rev() {
        let mut slot = team
            .line_up(place)
            .ok_or(SimError::LineUpPastOrder(place))?;
        let draw = rng.rand2();
        let standing_in = stand_in(stand_ins, slot)?;
        if standing_in >= 0 && draw & 1 == 0 {
            slot = standing_in;
        }
        let slot = record(team, slot)?;
        let status = team.cards_injuries(slot);
        let (mut injury, mut cards) = (status & INJURY_BITS, status & CARD_BITS);
        let odds = if place == 0 { KEEPER_ODDS } else { PLAYER_ODDS };
        if draw >= odds {
            continue;
        }
        if role(team.position_and_face(slot)) == GOALKEEPER
            && available_keepers(team, &both)?.out != 0
        {
            continue;
        }
        let pick = rng.rand2() & STATUS_DRAW;
        if pick < RED_CARD_DRAW {
            if cards_track {
                continue;
            }
            if injury == 0 {
                if !statuses_apply {
                    budget = budget.wrapping_sub(1);
                    if budget == 0 {
                        return Ok(());
                    }
                    continue;
                }
                injury = MINOR_INJURY;
            } else {
                injury = INJURED;
            }
        } else if cards_track {
            cards = RED_CARD;
        } else {
            let worse = if pick == RED_CARD_DRAW && injury == 0 {
                MODERATE_INJURY
            } else {
                SERIOUS_INJURY
            };
            injury = if statuses_apply { worse } else { INJURED };
        }
        let status = (status & !INJURY_BITS) | (injury & INJURY_BITS);
        team.set_cards_injuries(slot, (status & !CARD_BITS) | (cards & CARD_BITS));
        budget = budget.wrapping_sub(1);
        if budget == 0 {
            return Ok(());
        }
    }
    Ok(())
}

/// The buckets' draws of [`status_buckets`], for any team.
fn bucket_draws(
    team: &mut MatchTeam,
    stand_ins: Option<&[u8; CAREER_LINE_UP]>,
    rules: &SimRules,
    rng: &mut Rng,
) -> Result<(), SimError> {
    if rules.status.no_injuries_cards {
        return Ok(());
    }
    let both = StatusRules {
        mode: BOTH,
        ..rules.status
    };
    let out = available_players(team, 0, &both)?.out;
    let mut budget = rules.game.statuses_budget();
    budget = budget.wrapping_sub(out.cast_signed());
    if budget <= 0 {
        return Ok(());
    }
    let last = match team.career_line_up() {
        Some(line_up) if stand_ins.is_none() => {
            usize::try_from(line_up.players.wrapping_sub(1).cast_signed()).ok()
        }
        Some(_) | None => Some(LAST_OF_ELEVEN),
    };
    let Some(last) = last else {
        return Ok(());
    };
    for place in (0..=last).rev() {
        let mut slot = team
            .line_up(place)
            .ok_or(SimError::LineUpPastOrder(place))?;
        let draw = rng.rand2();
        if let Some(stand_ins) = stand_ins {
            let stand_in = usize::try_from(slot)
                .ok()
                .and_then(|index| stand_ins.get(index))
                .ok_or(SimError::StandInPastTable(slot))?
                .cast_signed();
            if stand_in >= 0 && draw & 1 == 0 {
                slot = stand_in;
            }
        }
        let slot = usize::try_from(slot)
            .ok()
            .filter(|&slot| slot < team.records())
            .ok_or(SimError::PlayerPastTeam(slot))?;
        if place == 0 && draw >= KEEPER_ODDS {
            continue;
        }
        if role(team.position_and_face(slot)) == GOALKEEPER
            && available_keepers(team, &both)?.out != 0
        {
            continue;
        }
        let status = team.cards_injuries(slot);
        let odds = match status & STATUS_BUCKET {
            0 => NEW_BUCKET_ODDS,
            MINOR_BUCKET => MINOR_BUCKET_ODDS,
            _ => continue,
        };
        if draw >= odds {
            continue;
        }
        let mut pick = rng.rand2() & BUCKET_DRAW;
        let mut bucket = u16::from(MINOR_BUCKET);
        for &odds in &STATUS_BUCKET_ODDS {
            let odds = u16::from(odds);
            if pick < odds {
                break;
            }
            pick -= odds;
            bucket += u16::from(MINOR_BUCKET);
        }
        let [bucket, _] = bucket.to_le_bytes();
        team.set_cards_injuries(
            slot,
            (status & CARDS_AND_INJURIES) | (bucket & STATUS_BUCKET),
        );
        budget -= 1;
        if budget == 0 {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::team::{CareerLineUp, RECORD_LEN};

    /// The rules of a competition whose injuries count, nothing kept from a computer club.
    const RULES: SimRules = SimRules {
        status: StatusRules {
            mode: 0,
            training: false,
            no_injuries_cards: false,
        },
        game: GameType::Competition,
        computer: ComputerStatuses {
            cards: false,
            injuries: false,
        },
    };

    /// A selected team fielding its records in order, of controls `controls`; records 0 and 11
    /// goalkeepers, the others strikers.
    fn team(controls: u8) -> MatchTeam {
        let mut team = MatchTeam::selected(&[0; RECORD_LEN]);
        team.set_controls(controls);
        for slot in 0..PLAYERS {
            team.set_player_number(slot, u8::try_from(slot).expect("16"));
            let position = if slot == 0 || slot == 11 { 0 } else { 7 << 5 };
            team.set_position_and_face(slot, position);
        }
        team
    }

    #[test]
    fn the_squad_s_players_and_keepers_are_counted() {
        let mut team = team(0);
        team.set_cards_injuries(3, 0x08);
        team.set_cards_injuries(11, 0x40);
        let rules = RULES.status;
        assert_eq!(
            available_players(&team, 0, &rules),
            Ok(Availability {
                out: 2,
                available: 14
            })
        );
        assert_eq!(
            available_players(&team, 4, &rules),
            Ok(Availability {
                out: 1,
                available: 3
            })
        );
        assert_eq!(
            available_keepers(&team, &rules),
            Ok(Availability {
                out: 1,
                available: 1
            })
        );
    }

    #[test]
    fn the_career_club_counts_its_own_squad() {
        let mut order = [0; CAREER_LINE_UP];
        order[0] = 31;
        let team = MatchTeam::career(
            &[0; crate::sim::team::CAREER_RECORD_LEN],
            CareerLineUp { order, players: 1 },
        );
        assert_eq!(
            available_players(&team, 0, &RULES.status),
            Ok(Availability {
                out: 0,
                available: 1
            })
        );
    }

    #[test]
    fn a_draw_under_the_odds_puts_a_player_in_a_bucket() {
        // Every draw 0: place 10 first, into the first bucket; then place 9; two at most.
        let mut rng = Rng::default();
        rng.fixed = Some(0);
        let mut team = team(0);
        status_buckets(&mut team, None, &RULES, &mut rng).expect("drawn");
        let buckets: Vec<u8> = (0..PLAYERS).map(|slot| team.cards_injuries(slot)).collect();
        let mut expected = vec![0; PLAYERS];
        expected[9] = MINOR_BUCKET;
        expected[10] = MINOR_BUCKET;
        assert_eq!(buckets, expected);
    }

    #[test]
    fn a_computer_club_keeps_only_the_kinds_its_world_opens() {
        let mut rng = Rng::default();
        rng.fixed = Some(0);
        let mut team = team(COMPUTER);
        status_buckets(&mut team, None, &RULES, &mut rng).expect("none drawn");
        assert!((0..PLAYERS).all(|slot| team.cards_injuries(slot) == 0));
        let cards_only = SimRules {
            computer: ComputerStatuses {
                cards: true,
                injuries: false,
            },
            ..RULES
        };
        status_buckets(&mut team, None, &cards_only, &mut rng).expect("drawn, then put back");
        assert!((0..PLAYERS).all(|slot| team.cards_injuries(slot) == 0));
    }
}
