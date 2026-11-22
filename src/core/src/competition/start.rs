//! What a league's and a cup's starts share (docs/PLAN.md M5 #4b, #4c): the session's pitch,
//! a team as upstream's loader finds it, the resets' common defaults, the name written into
//! its buffer, and the months a competition's matches are played in (upstream
//! diy_competition.c `setPitchTypeFields`, `copyCompetitionName`, `StringCopy`,
//! `DIY_RecomputeCompetitionDerivedProgressionTable`, `DIY_InitCupBracketProgressionTable`).

use openswos_team_resource::TeamControls;

use super::running::{NAME_LEN, RunningCompetition};
use super::value::Prices;

/// How far a substitutes byte's high nibble is shifted.
pub(super) const NIBBLE_SHIFT: u8 = 4;

/// A substitutes byte's low nibble.
pub(super) const NIBBLE_MASK: u8 = 0x0F;

/// The longest name the name buffer takes, its end aside (`copyCompetitionName`).
const NAME_ROOM: usize = NAME_LEN - 1;

/// The bytes of upstream's name scratch (`copyCompetitionName`'s `tmp[96]`): the adjective,
/// the space and the name, its end aside, fit in them.
const NAME_SCRATCH: usize = 96;

/// The months a progression wraps by when it ends before it starts (upstream
/// `DIY_RecomputeCompetitionDerivedProgressionTable`, `DIY_InitCupBracketProgressionTable`:
/// twelve months of eight steps).
const MONTHS_WRAP: u8 = 0x60;

/// The defaults a league's reset and a cup's share (upstream `DIY_InitCompetitionDefaults`,
/// `InitDIYCup`).
pub(super) mod defaults {
    /// No result yet (`resultCode`).
    pub const RESULT_CODE: i16 = -1;
    /// Slot state A (a menu ordinal).
    pub const SLOT_STATE_A: i16 = 3;
    /// Slot state B.
    pub const SLOT_STATE_B: i16 = 2;
    /// Slot state C (a menu ordinal).
    pub const SLOT_STATE_C: i16 = 8;
    /// No competition id.
    pub const COMP_ID: u8 = 0xFF;
    /// The first month: the season's start, in eighths of a month.
    pub const START_MONTH: i8 = 0x38;
    /// The last month.
    pub const END_MONTH: i8 = 0x20;
    /// No pitch type: the season's.
    pub const PITCH_TYPE: i16 = -1;
}

/// The pitch the session plays on (upstream `_pitchType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pitch {
    /// The season's (`_pitchType` -2).
    Seasonal,
    /// This type, all season.
    Type(i16),
}

/// A team of the competition, as upstream finds it (`VerifyTeamLoaded`): its identity among
/// the session's teams, who controls it and its valued players' prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadedTeam {
    /// -1 the career club, otherwise its index among the selected teams.
    pub identity: i32,
    /// Who controls it (`teamControls`): a cup counts the teams the computer does not.
    pub controls: TeamControls,
    /// Its valued players' prices.
    pub prices: Prices,
}

/// `text` (up to its first 0) and its end into the name buffer, as upstream's `StringCopy`
/// does: the bytes after its end are left.
pub(super) fn copy_string(competition: &mut RunningCompetition, text: &[u8]) {
    competition.set_name(copied(competition.name(), text));
}

/// `name` with `text` (up to its first 0) and its end copied over its start, as upstream's
/// `StringCopy` copies: the bytes after its end are left.
pub(super) fn copied(mut name: [u8; NAME_LEN], text: &[u8]) -> [u8; NAME_LEN] {
    let text = until_nul(text);
    let len = text.len().min(NAME_ROOM);
    name[..len].copy_from_slice(&text[..len]);
    name[len] = 0;
    name
}

/// The competition's name into its buffer (upstream `copyCompetitionName`): the adjective and
/// a space, then the name, at most 35 bytes and their end; the bytes after its end are left.
pub(super) fn write_name(competition: &mut RunningCompetition, prefix: Option<&[u8]>, name: &[u8]) {
    competition.set_name(named(competition.name(), prefix, name));
}

/// `buffer` with the competition's name written over its start (upstream
/// `copyCompetitionName`): the adjective and a space, then the name, at most 35 bytes and
/// their end; the bytes after its end are left.
pub(super) fn named(buffer: [u8; NAME_LEN], prefix: Option<&[u8]>, name: &[u8]) -> [u8; NAME_LEN] {
    let mut text: Vec<u8> = prefix
        .map(|adjective| {
            let mut prefix = until_nul(adjective).to_vec();
            prefix.push(b' ');
            prefix
        })
        .unwrap_or_default();
    let room = (NAME_SCRATCH - 1).saturating_sub(text.len());
    text.extend(until_nul(name).iter().take(room));
    copied(buffer, &text)
}

/// The pitch fields (upstream `setPitchTypeFields`): the season's, or one type all season.
pub(super) fn set_pitch(competition: &mut RunningCompetition, pitch: Pitch) {
    match pitch {
        Pitch::Seasonal => competition.set_pitch_type_or_season(1),
        Pitch::Type(pitch_type) => {
            competition.set_pitch_type_or_season(0);
            competition.set_pitch_type(pitch_type);
        }
    }
}

/// `bytes` up to their first 0.
fn until_nul(bytes: &[u8]) -> &[u8] {
    bytes.split(|&byte| byte == 0).next().unwrap_or_default()
}

/// The low word of `value`, as upstream keeps a quotient.
pub(super) fn low_word(value: u32) -> u16 {
    let [low, high, _, _] = value.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// The months of `count` matches, from `start` to `end` in even steps (upstream
/// `DIY_RecomputeCompetitionDerivedProgressionTable`, `DIY_InitCupBracketProgressionTable`):
/// one match is played at the start; an end before the start wraps by a year. None for no
/// match (upstream's loop runs on past its table, which its callers never let it do).
pub(super) fn months(start: u8, end: u8, count: u16) -> Vec<u8> {
    match count {
        0 => Vec::new(),
        1 => vec![start],
        _ => {
            let end = if end < start {
                end.wrapping_add(MONTHS_WRAP)
            } else {
                end
            };
            let range = u32::from(end.wrapping_sub(start));
            let last = count - 1;
            (0..=last)
                .map(|step| {
                    let offset = low_word(u32::from(step) * range / u32::from(last));
                    offset.to_le_bytes()[0].wrapping_add(start)
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_months_run_from_the_start_to_the_end() {
        assert_eq!(months(0x40, 0x48, 0), Vec::<u8>::new());
        assert_eq!(months(0x40, 0x48, 1), [0x40]);
        assert_eq!(months(0x40, 0x48, 3), [0x40, 0x44, 0x48]);
        // August to February: the end wraps by a year (0x10 + 0x60 = 0x70), 56 steps in
        // thirds, each rounded down.
        assert_eq!(months(0x38, 0x10, 4), [0x38, 0x4A, 0x5D, 0x70]);
    }

    #[test]
    fn a_month_past_a_byte_wraps() {
        // 0x20 + 0x60 = 0x80, 0x90 past 0xF0 as a byte: upstream's sums are bytes.
        assert_eq!(months(0xF0, 0x20, 2), [0xF0, 0x80]);
        assert_eq!(months(0xF0, 0xFF, 2), [0xF0, 0xFF]);
    }

    #[test]
    fn the_name_is_the_adjective_a_space_and_the_name() {
        let mut competition = RunningCompetition::default();
        write_name(&mut competition, Some(b"ENGLISH\0X"), b"CUP\0TAIL");
        assert_eq!(&competition.name()[..12], b"ENGLISH CUP\0");
        copy_string(&mut competition, b"FA");
        assert_eq!(
            &competition.name()[..12],
            b"FA\0LISH CUP\0",
            "its tail left"
        );
    }

    #[test]
    fn a_pitch_is_the_seasons_or_one_type() {
        let mut competition = RunningCompetition::default();
        competition.set_pitch_type(-1);
        set_pitch(&mut competition, Pitch::Seasonal);
        assert_eq!(
            (competition.pitch_type_or_season(), competition.pitch_type()),
            (1, -1)
        );
        set_pitch(&mut competition, Pitch::Type(4));
        assert_eq!(
            (competition.pitch_type_or_season(), competition.pitch_type()),
            (0, 4)
        );
    }
}
