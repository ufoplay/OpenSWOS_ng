//! The countdown of a match's teams' statuses before it is simulated (docs/PLAN.md M5 #8d;
//! upstream src/career_status.c `InitInGameTeams_WithMatchParam`,
//! `maybeDecayPlayerStatusBucketForTeam`, `maybeDecayPlayerStatusBucket`,
//! `cyclePlayerStatusFlagsForTeam`). Each player's status bucket may fall, by draws: the minor
//! one, a draw in two, to none; the sixth, a draw in eight, to one of the first five; the
//! seventh never; the others a step, the minor one reached then, a draw in two, none. Then,
//! outside a training game, under the competition's track of cards each player's card falls
//! (a second yellow to a yellow, the others to none), else his injury heals a step (a serious
//! one to a moderate one, to one that keeps him out, to none; a minor one never). A computer
//! club's statuses count down only where a world opens their kind, in a DIY or a preset
//! competition (upstream `WorldRules_CpuCountdown`; in a season and a career they count down
//! each day, M7).

use openswos_rng::Rng;

use super::status::{
    CARD_BITS, CARDS_AND_INJURIES, COMPUTER, GameType, INJURED, INJURY_BITS, MINOR_BUCKET,
    MODERATE_INJURY, RED_CARD, SERIOUS_INJURY, STATUS_BUCKET, SimRules, place_slot, squad, walked,
};
use super::team::{MatchTeam, PLAYERS};
use super::value::SimError;

/// The sixth status bucket, which a player leaves only by a draw in eight.
const SIXTH_BUCKET: u8 = 0xC0;

/// The seventh status bucket, which a player never leaves (all of `STATUS_BUCKET`'s bits).
const SEVENTH_BUCKET: u8 = STATUS_BUCKET;

/// The bits of a draw that, all clear, take a player out of the sixth bucket (one in eight).
const SIXTH_BUCKET_ODDS: u16 = 0x07;

/// The buckets a player out of the sixth falls to: the first five.
const LOWER_BUCKETS: u16 = 5;

/// How far a bucket's number is shifted in `cardsInjuries`.
const BUCKET_SHIFT: u8 = 5;

/// A yellow card's cards bits, which a red card falls to.
const YELLOW_CARD: u8 = 1;

/// Whether `team`'s statuses of a kind count down before a simulated match: always, but a
/// computer club's only where `open` (the world opens the kind) in a DIY or a preset
/// competition (upstream `WorldRules_CpuCountdown`).
fn counts_down(team: &MatchTeam, open: bool, game: GameType) -> bool {
    team.controls() != COMPUTER || (open && game == GameType::Competition)
}

/// A player's `status` with its bucket fallen (upstream `maybeDecayPlayerStatusBucket`), drawn
/// by `rng`: none for a player in no bucket.
fn decayed(status: u8, rng: &mut Rng) -> u8 {
    let bucket = status & STATUS_BUCKET;
    let fallen = match bucket {
        0 | SEVENTH_BUCKET => return status,
        MINOR_BUCKET => {
            if rng.rand2() & 1 != 0 {
                return status;
            }
            0
        }
        SIXTH_BUCKET => {
            if rng.rand2() & SIXTH_BUCKET_ODDS != 0 {
                return status;
            }
            let level = (rng.rand2() * LOWER_BUCKETS) >> u8::BITS;
            let [low, _] = ((level << BUCKET_SHIFT) + u16::from(MINOR_BUCKET)).to_le_bytes();
            low
        }
        _ => {
            let lower = bucket - MINOR_BUCKET;
            if lower == MINOR_BUCKET && rng.rand2() & 1 == 0 {
                0
            } else {
                lower
            }
        }
    };
    (status & CARDS_AND_INJURIES) | fallen
}

/// `team`'s status buckets fallen (upstream `maybeDecayPlayerStatusBucketForTeam`), drawn by
/// `rng`: a selected team's records in order, the career club's line-up's places, last first.
fn decay_buckets(team: &mut MatchTeam, rules: &SimRules, rng: &mut Rng) -> Result<(), SimError> {
    if !counts_down(team, rules.computer.injuries, rules.game) {
        return Ok(());
    }
    let slots: Vec<usize> = match team.career_line_up() {
        Some(line_up) => (0..walked(line_up.players))
            .rev()
            .map(|place| place_slot(team, place))
            .collect::<Result<_, _>>()?,
        None => (0..PLAYERS).collect(),
    };
    for slot in slots {
        let status = team.cards_injuries(slot);
        team.set_cards_injuries(slot, decayed(status, rng));
    }
    Ok(())
}

/// A player's `status` with its card fallen (`cards`, the competition tracks them) or its
/// injury healed a step.
fn cycled(status: u8, cards: bool) -> u8 {
    if cards {
        let card = match status & CARD_BITS {
            RED_CARD => YELLOW_CARD,
            _ => 0,
        };
        return (status & !CARD_BITS) | card;
    }
    let healed = match status & INJURY_BITS {
        SERIOUS_INJURY => MODERATE_INJURY,
        MODERATE_INJURY => INJURED,
        INJURED => 0,
        _ => return status,
    };
    (status & !INJURY_BITS) | healed
}

/// `team`'s cards or injuries counted down (upstream `cyclePlayerStatusFlagsForTeam`): none in
/// a training game; its line-up's places, last first.
fn cycle_statuses(team: &mut MatchTeam, rules: &SimRules) -> Result<(), SimError> {
    if rules.status.training || !counts_down(team, rules.computer.cards, rules.game) {
        return Ok(());
    }
    let cards = rules.status.mode != 0;
    for place in (0..walked(squad(team))).rev() {
        let slot = place_slot(team, place)?;
        let status = team.cards_injuries(slot);
        team.set_cards_injuries(slot, cycled(status, cards));
    }
    Ok(())
}

/// `home`'s and `away`'s statuses counted down before their simulated match (upstream
/// `InitInGameTeams_WithMatchParam`), drawn by `rng`: each side's status buckets fallen, then
/// each side's cards (`rules.status.mode`, the competition's track, not 0) or injuries.
///
/// # Errors
///
/// [`SimError`] when a line-up place is past the line-up or names no record of the team.
pub fn countdown(
    [home, away]: [&mut MatchTeam; 2],
    rules: &SimRules,
    rng: &mut Rng,
) -> Result<(), SimError> {
    decay_buckets(home, rules, rng)?;
    decay_buckets(away, rules, rng)?;
    cycle_statuses(home, rules)?;
    cycle_statuses(away, rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::status::ComputerStatuses;
    use crate::sim::team::{CAREER_LINE_UP, CAREER_RECORD_LEN, CareerLineUp, RECORD_LEN};
    use crate::sim::value::StatusRules;

    /// A competition's rules, its injuries counted down, a computer club's statuses not.
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

    /// A selected team of controls `controls`, its line-up its records in reverse.
    fn team(controls: u8) -> MatchTeam {
        let mut team = MatchTeam::selected(&[0; RECORD_LEN]);
        team.set_controls(controls);
        for place in 0..PLAYERS {
            team.set_player_number(place, u8::try_from(PLAYERS - 1 - place).expect("16"));
        }
        team
    }

    /// A generator that draws `value` each time.
    fn fixed(value: u8) -> Rng {
        let mut rng = Rng::default();
        rng.fixed = Some(value);
        rng
    }

    #[test]
    fn a_bucket_falls_by_its_draws() {
        // The minor bucket: an odd draw keeps it, an even one empties it.
        assert_eq!(decayed(0x20 | 0x03, &mut fixed(1)), 0x23);
        assert_eq!(decayed(0x20 | 0x03, &mut fixed(0)), 0x03);
        // The sixth: a draw of 0 in eight, then the level by the next (255 * 5 >> 8 = 4).
        assert_eq!(decayed(0xC0, &mut fixed(1)), 0xC0);
        assert_eq!(decayed(0xC0, &mut fixed(0)), 0x20);
        assert_eq!(decayed(0xC0 | 0x1F, &mut fixed(8)), 0x20 | 0x1F);
        // The seventh never falls; none draws nothing.
        let mut rng = Rng::default();
        let before = rng.clone();
        assert_eq!(decayed(0xE0, &mut rng), 0xE0);
        assert_eq!(decayed(0x1F, &mut rng), 0x1F);
        assert_eq!(rng, before);
        // The others a step, the minor one reached then a draw in two.
        assert_eq!(decayed(0xA0, &mut fixed(0)), 0x80);
        assert_eq!(decayed(0x40, &mut fixed(1)), 0x20);
        assert_eq!(decayed(0x40, &mut fixed(0)), 0x00);
    }

    #[test]
    fn the_sixth_bucket_falls_to_the_level_of_its_second_draw() {
        // 248: a draw of 0 in eight, then 248 * 5 >> 8 = 4, the fifth bucket.
        assert_eq!(decayed(0xC0 | 0x01, &mut fixed(248)), 0xA0 | 0x01);
        // 0x30: 0x30 * 5 >> 8 = 0, the minor bucket.
        assert_eq!(decayed(0xC0, &mut fixed(0x30)), 0x20);
    }

    #[test]
    fn a_card_or_an_injury_counts_down() {
        assert_eq!(cycled(0x02, true), 0x01);
        assert_eq!(cycled(0x01 | 0xE0, true), 0xE0);
        assert_eq!(cycled(0x03, true), 0x00);
        assert_eq!(cycled(0x10 | 0x02, false), 0x0C | 0x02);
        assert_eq!(cycled(0x0C, false), 0x08);
        assert_eq!(cycled(0x08 | 0x20, false), 0x20);
        // A minor injury and the others are left.
        for injury in [0x04, 0x14, 0x18, 0x1C] {
            assert_eq!(cycled(injury, false), injury);
        }
    }

    #[test]
    fn both_sides_count_down_their_buckets_first() {
        let (mut home, mut away) = (team(0), team(2));
        home.set_cards_injuries(0, 0x40 | 0x10);
        away.set_cards_injuries(15, 0x60 | 0x0C);
        countdown([&mut home, &mut away], &RULES, &mut fixed(1)).expect("counted down");
        assert_eq!(home.cards_injuries(0), 0x20 | 0x0C);
        assert_eq!(away.cards_injuries(15), 0x40 | 0x08);
        let cards = SimRules {
            status: StatusRules {
                mode: 1,
                ..RULES.status
            },
            ..RULES
        };
        home.set_cards_injuries(1, 0x02);
        countdown([&mut home, &mut away], &cards, &mut fixed(1)).expect("counted down");
        assert_eq!(home.cards_injuries(1), 0x01);
        assert_eq!(home.cards_injuries(0), 0x20 | 0x0C);
    }

    #[test]
    fn a_training_game_counts_down_only_the_buckets() {
        let (mut home, mut away) = (team(0), team(0));
        home.set_cards_injuries(0, 0x40 | 0x10);
        let training = SimRules {
            status: StatusRules {
                training: true,
                ..RULES.status
            },
            ..RULES
        };
        countdown([&mut home, &mut away], &training, &mut fixed(1)).expect("counted down");
        assert_eq!(home.cards_injuries(0), 0x20 | 0x10);
    }

    #[test]
    fn a_computer_club_counts_down_only_what_a_competition_s_world_opens() {
        let status = 0x40 | 0x10 | 0x02;
        let counted = |computer: ComputerStatuses, game: GameType| {
            let (mut home, mut away) = (team(COMPUTER), team(0));
            home.set_cards_injuries(0, status);
            let rules = SimRules {
                computer,
                game,
                ..RULES
            };
            countdown([&mut home, &mut away], &rules, &mut fixed(1)).expect("counted down");
            home.cards_injuries(0)
        };
        let none = ComputerStatuses {
            cards: false,
            injuries: false,
        };
        let both = ComputerStatuses {
            cards: true,
            injuries: true,
        };
        assert_eq!(counted(none, GameType::Competition), status);
        assert_eq!(counted(both, GameType::Competition), 0x20 | 0x0C | 0x02);
        assert_eq!(counted(both, GameType::Career), status);
        assert_eq!(counted(both, GameType::Season), status);
        let injuries = ComputerStatuses {
            cards: false,
            injuries: true,
        };
        assert_eq!(counted(injuries, GameType::Competition), 0x20 | 0x10 | 0x02);
    }

    #[test]
    fn the_career_club_counts_down_its_line_up_s_places() {
        let mut order = [0; CAREER_LINE_UP];
        order[0] = 20;
        order[1] = 20;
        let mut career =
            MatchTeam::career(&[0; CAREER_RECORD_LEN], CareerLineUp { order, players: 2 });
        career.set_cards_injuries(20, 0x60 | 0x10);
        let mut away = team(0);
        countdown([&mut career, &mut away], &RULES, &mut fixed(1)).expect("counted down");
        // Its place twice in the line-up: twice a step.
        assert_eq!(career.cards_injuries(20), 0x20 | 0x08);
        // A place naming no record: refused.
        let mut order = [0; CAREER_LINE_UP];
        order[0] = 0x80;
        let mut career =
            MatchTeam::career(&[0; CAREER_RECORD_LEN], CareerLineUp { order, players: 1 });
        assert_eq!(
            countdown([&mut career, &mut away], &RULES, &mut fixed(1)),
            Err(SimError::PlayerPastTeam(-128))
        );
    }
}
