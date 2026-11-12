//! The DOS game's DIY competition file (`.DIY`, the PC game and SWOS 2020): the
//! competition region as it lay in memory, the teams' count, the teams (SWOS2.s
//! `SaveDIYFile`; upstream src/diy_import_dos.c, swos-port swos.asm).
//!
//! | offset | bytes | field |
//! |---|---|---|
//! | 0 | 95 | the competition's header ([`DosCompetitionHeader`]) |
//! | 95 | 2815 | the competition's state after it ([`DosDiyFile::body`]) |
//! | 0xB5E | 6 × 370 | the user's tactics A to F |
//! | 0x140A | 6 × 2 | the match options ([`DosMatchOptions`]) |
//! | 0x1416 | 47 | the rest of the region ([`DosDiyFile::tail`]) |
//! | 5189 | 2 | the teams' count |
//! | 5191 | count × 684 | the teams |
//!
//! Words are little-endian. The layout is the Amiga's with three differences: the header
//! is a byte shorter (its name holds 35 bytes, not 36), a career flag is missing and two
//! option words are extra, and the tail lays the random seeds out differently.
//!
//! - **The body** depends on the competition's type. A league or cup: the rest of its
//!   `DIYCompetitionEx` (to 0x2B3), then its 64 standings rows (to 0x733), each selecting
//!   its team as a byte offset into the teams. A tournament: its warnings word, its team
//!   order (from 0x59, over the header's last bytes), team ids (0x99), group map (0x119),
//!   six stage slots (0x159) and the current stage, itself a `DIYCompetitionEx` (0x41F).
//!   It is kept as bytes: what it holds is the competitions' (M5).
//! - **The teams** are the 684-byte team records as they lay in memory. Faithful to the
//!   original: as in an `.EDT`, their global team numbers are little-endian, which the
//!   team crate reads byte-swapped (the game recomputes them).
//! - **Making a JSON `.DIY`** from one (upstream `--convert-diy`) goes through the
//!   competition's state, whose raw slot the JSON save writes in the x86_64 layout: that is
//!   the competitions' (M5, M7), not this reader's.

use openswos_common::{ByteReader, ByteWriter, FixedName, ReadError};
use openswos_tac_resource::{TACTICS_RECORD_LEN, TacticsError, TeamTactics};
use openswos_team_resource::{RecordError, TEAM_RECORD_LEN, Team};
use thiserror::Error;

use openswos_common::{Exporter, Importer};

/// Bytes of the competition region (upstream `kRegion`: `competitionFileBuffer` up to
/// `numSelectedTeams`).
pub const DOS_REGION_LEN: usize = 5189;

/// Bytes of a DOS competition header (upstream `kHeader`; the Amiga's is 96).
pub const DOS_HEADER_LEN: usize = 0x5F;

/// Bytes of a DOS competition's name (upstream `kNameLen`; the Amiga's is 36).
pub const DOS_NAME_LEN: usize = 35;

/// Where the user's tactics start in the region (upstream `kTactics`).
const TACTICS_OFFSET: usize = 0xB5E;

/// The user's tactics: A to F.
pub const USER_TACTICS: usize = 6;

/// Where the match options start in the region (upstream `kOptions`).
const OPTIONS_OFFSET: usize = 0x140A;

/// The match options' words.
const OPTION_WORDS: usize = 6;

/// Bytes of a word.
const WORD_LEN: usize = 2;

/// The type word of a tournament.
const TOURNAMENT: u16 = 2;

/// Bytes of the body: from the header's end to the tactics.
pub const DOS_BODY_LEN: usize = TACTICS_OFFSET - DOS_HEADER_LEN;

/// Bytes of the tail: from the options' end to the region's.
pub const DOS_TAIL_LEN: usize = DOS_REGION_LEN - OPTIONS_OFFSET - WORD_LEN * OPTION_WORDS;

/// The most teams a DOS `.DIY` holds (upstream `kMaxTeams`).
pub const DOS_MAX_TEAMS: usize = 64;

/// A DOS competition's type (the header's first word).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DosCompetitionType {
    /// A league.
    League,
    /// A cup.
    Cup,
    /// A tournament (groups, then knockouts).
    Tournament,
}

/// A word that is no DOS competition type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is no competition type (0 league, 1 cup, 2 tournament)")]
pub struct InvalidCompetitionType(pub u16);

impl TryFrom<u16> for DosCompetitionType {
    type Error = InvalidCompetitionType;

    fn try_from(word: u16) -> Result<Self, Self::Error> {
        match word {
            0 => Ok(Self::League),
            1 => Ok(Self::Cup),
            TOURNAMENT => Ok(Self::Tournament),
            _ => Err(InvalidCompetitionType(word)),
        }
    }
}

impl From<DosCompetitionType> for u16 {
    fn from(competition_type: DosCompetitionType) -> Self {
        match competition_type {
            DosCompetitionType::League => 0,
            DosCompetitionType::Cup => 1,
            DosCompetitionType::Tournament => TOURNAMENT,
        }
    }
}

/// A DOS competition's header (upstream `DIYCompetition`, 95 bytes in the DOS layout).
/// The fields are the JSON slot header's (`SlotHeader`), at their DOS offsets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DosCompetitionHeader {
    /// The competition's type.
    pub competition_type: DosCompetitionType,
    /// In use (a cup), or over.
    pub is_active: u16,
    /// The competition's name.
    pub name: FixedName<DOS_NAME_LEN>,
    /// The DOS game's pointer to the name (0 in SWOS 2020's files), kept as written.
    pub name_ptr: u32,
    /// The slot's cache key byte.
    pub comp_slot: u8,
    /// The slot's index for cache matching.
    pub slot_id: u8,
    /// The team configuration byte.
    pub field_2e: u8,
    /// The month or phase for scheduling.
    pub view_phase: u8,
    /// The teams' average skill, or a trophy bonus.
    pub avg_skill: u8,
    /// The strength bracket.
    pub str_tier: u8,
    /// The teams: the file's count.
    pub num_teams: i16,
    /// The pitch type or the season.
    pub pitch_type_or_season: i16,
    /// The pitch type.
    pub pitch_type: i16,
    /// The first month.
    pub start_month: i8,
    /// The last month.
    pub end_month: i8,
    /// The result code, or the winner.
    pub result_code: i16,
    /// The substitutes, or the squad view's parameter.
    pub subs_count: i16,
    /// Slot state A.
    pub slot_state_a: i16,
    /// Slot state B.
    pub slot_state_b: i16,
    /// Slot state C.
    pub slot_state_c: i16,
    /// 1 once a match has begun.
    pub match_started: i16,
    /// 1 when the career club takes part.
    pub career_team_involved: i16,
    /// 0 no groups, 1 groups, 2 groups and a final.
    pub has_groups: i16,
    /// The cup menu's state.
    pub menu_state: i16,
    /// The round iterator, or finished.
    pub round_iterator: i16,
    /// The teams in the current stage.
    pub num_teams_in_stage: i16,
    /// The match pairs a round.
    pub match_pairs_per_round: i16,
    /// The matches of the round.
    pub total_matches_in_round: i16,
    /// The round is complete, or the lowest division; a tournament's stage in progress.
    pub round_complete: i16,
    /// The competition is finished, or the highest division's offset.
    pub competition_finished: i16,
    /// The round's index, or the promotions.
    pub current_round_index: i16,
    /// The rounds, or the relegations.
    pub num_rounds: i16,
    /// The sub-round counter.
    pub sub_round_counter: i16,
    /// Each team plays the others this many times.
    pub play_each_team: i16,
}

impl DosCompetitionHeader {
    fn read(reader: &mut ByteReader<'_>) -> Result<Self, DosDiyError> {
        Ok(Self {
            competition_type: DosCompetitionType::try_from(reader.read_u16_le()?)?,
            is_active: reader.read_u16_le()?,
            name: FixedName::from_bytes(reader.read_array()?),
            name_ptr: reader.read_u32_le()?,
            comp_slot: reader.read_u8()?,
            slot_id: reader.read_u8()?,
            field_2e: reader.read_u8()?,
            view_phase: reader.read_u8()?,
            avg_skill: reader.read_u8()?,
            str_tier: reader.read_u8()?,
            num_teams: reader.read_i16_le()?,
            pitch_type_or_season: reader.read_i16_le()?,
            pitch_type: reader.read_i16_le()?,
            start_month: reader.read_i8()?,
            end_month: reader.read_i8()?,
            result_code: reader.read_i16_le()?,
            subs_count: reader.read_i16_le()?,
            slot_state_a: reader.read_i16_le()?,
            slot_state_b: reader.read_i16_le()?,
            slot_state_c: reader.read_i16_le()?,
            match_started: reader.read_i16_le()?,
            career_team_involved: reader.read_i16_le()?,
            has_groups: reader.read_i16_le()?,
            menu_state: reader.read_i16_le()?,
            round_iterator: reader.read_i16_le()?,
            num_teams_in_stage: reader.read_i16_le()?,
            match_pairs_per_round: reader.read_i16_le()?,
            total_matches_in_round: reader.read_i16_le()?,
            round_complete: reader.read_i16_le()?,
            competition_finished: reader.read_i16_le()?,
            current_round_index: reader.read_i16_le()?,
            num_rounds: reader.read_i16_le()?,
            sub_round_counter: reader.read_i16_le()?,
            play_each_team: reader.read_i16_le()?,
        })
    }

    fn write(&self, writer: &mut ByteWriter) {
        let Self {
            competition_type,
            is_active,
            name,
            name_ptr,
            comp_slot,
            slot_id,
            field_2e,
            view_phase,
            avg_skill,
            str_tier,
            num_teams,
            pitch_type_or_season,
            pitch_type,
            start_month,
            end_month,
            result_code,
            subs_count,
            slot_state_a,
            slot_state_b,
            slot_state_c,
            match_started,
            career_team_involved,
            has_groups,
            menu_state,
            round_iterator,
            num_teams_in_stage,
            match_pairs_per_round,
            total_matches_in_round,
            round_complete,
            competition_finished,
            current_round_index,
            num_rounds,
            sub_round_counter,
            play_each_team,
        } = self;
        writer.write_u16_le(u16::from(*competition_type));
        writer.write_u16_le(*is_active);
        writer.write_bytes(name.as_bytes());
        writer.write_u32_le(*name_ptr);
        writer.write_bytes(&[
            *comp_slot,
            *slot_id,
            *field_2e,
            *view_phase,
            *avg_skill,
            *str_tier,
        ]);
        for word in [*num_teams, *pitch_type_or_season, *pitch_type] {
            writer.write_i16_le(word);
        }
        writer.write_i8(*start_month);
        writer.write_i8(*end_month);
        for word in [
            *result_code,
            *subs_count,
            *slot_state_a,
            *slot_state_b,
            *slot_state_c,
            *match_started,
            *career_team_involved,
            *has_groups,
            *menu_state,
            *round_iterator,
            *num_teams_in_stage,
            *match_pairs_per_round,
            *total_matches_in_round,
            *round_complete,
            *competition_finished,
            *current_round_index,
            *num_rounds,
            *sub_round_counter,
            *play_each_team,
        ] {
            writer.write_i16_le(word);
        }
    }
}

/// The match options a DOS `.DIY` carries (upstream `kOptions`: `gameLength` to
/// `pitchType`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DosMatchOptions {
    /// The game's length.
    pub game_length: u16,
    /// Automatic replays.
    pub auto_replays: u16,
    /// The menus' music.
    pub menu_music: u16,
    /// Automatic highlights.
    pub auto_save_highlights: u16,
    /// All the player's teams equal.
    pub all_player_teams_equal: u16,
    /// The pitch (65535 and 65534: the game picks it).
    pub pitch_type: u16,
}

/// A DOS `.DIY` file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DosDiyFile {
    /// The competition's header.
    pub header: DosCompetitionHeader,
    /// The competition's state after the header, by its type (see the module).
    pub body: Vec<u8>,
    /// The user's tactics A to F.
    pub user_tactics: [TeamTactics; USER_TACTICS],
    /// The match options.
    pub options: DosMatchOptions,
    /// The rest of the region: option words and random seeds, kept as written.
    pub tail: Vec<u8>,
    /// The competition's teams.
    pub teams: Vec<Team>,
}

/// Why bytes are not a DOS `.DIY`.
#[derive(Debug, Error)]
pub enum DosDiyError {
    /// The data ended early.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// A JSON save, not the DOS game's.
    #[error("a JSON save: OpenSWOS's .DIY, not the DOS game's")]
    Json,
    /// The competition's type is not one.
    #[error(transparent)]
    CompetitionType(#[from] InvalidCompetitionType),
    /// A user's tactic does not read.
    #[error("user tactics: {0}")]
    Tactics(#[from] TacticsError),
    /// A team does not read.
    #[error("team: {0}")]
    Team(#[from] RecordError),
    /// No teams, or more than the format holds.
    #[error("{0} teams: a DOS .DIY holds 1 to {DOS_MAX_TEAMS}")]
    TeamCount(usize),
    /// The header's teams disagree with the file's.
    #[error("the header says {stated} teams, the file holds {teams}")]
    HeaderTeams {
        /// The header's count.
        stated: i16,
        /// The file's teams.
        teams: usize,
    },
    /// A part is not its size.
    #[error("{part}: {len} bytes, not {expected}")]
    PartLength {
        /// The part.
        part: &'static str,
        /// Its bytes.
        len: usize,
        /// The bytes it has.
        expected: usize,
    },
}

impl DosDiyFile {
    /// Fails when the teams' count is outside the format or disagrees with the header,
    /// or a raw part is not its size.
    fn check(&self) -> Result<(), DosDiyError> {
        let teams = self.teams.len();
        if !(1..=DOS_MAX_TEAMS).contains(&teams) {
            return Err(DosDiyError::TeamCount(teams));
        }
        if usize::try_from(self.header.num_teams).ok() != Some(teams) {
            return Err(DosDiyError::HeaderTeams {
                stated: self.header.num_teams,
                teams,
            });
        }
        for (part, len, expected) in [
            ("body", self.body.len(), DOS_BODY_LEN),
            ("tail", self.tail.len(), DOS_TAIL_LEN),
        ] {
            if len != expected {
                return Err(DosDiyError::PartLength {
                    part,
                    len,
                    expected,
                });
            }
        }
        Ok(())
    }
}

/// The DOS `.DIY` format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DosDiyFormat;

impl Importer for DosDiyFormat {
    type Model = DosDiyFile;
    type Error = DosDiyError;

    fn import(bytes: &[u8]) -> Result<DosDiyFile, DosDiyError> {
        if bytes.first() == Some(&b'{') {
            return Err(DosDiyError::Json);
        }
        let mut reader = ByteReader::new(bytes);
        let header = DosCompetitionHeader::read(&mut reader)?;
        let body = reader.read_bytes(DOS_BODY_LEN)?.to_vec();
        let mut user_tactics = Vec::with_capacity(USER_TACTICS);
        for _ in 0..USER_TACTICS {
            user_tactics.push(TeamTactics::read(&mut reader)?);
        }
        let options = DosMatchOptions {
            game_length: reader.read_u16_le()?,
            auto_replays: reader.read_u16_le()?,
            menu_music: reader.read_u16_le()?,
            auto_save_highlights: reader.read_u16_le()?,
            all_player_teams_equal: reader.read_u16_le()?,
            pitch_type: reader.read_u16_le()?,
        };
        let tail = reader.read_bytes(DOS_TAIL_LEN)?.to_vec();
        let count = usize::from(reader.read_u16_le()?);
        if !(1..=DOS_MAX_TEAMS).contains(&count) {
            return Err(DosDiyError::TeamCount(count));
        }
        let mut teams = Vec::with_capacity(count);
        for _ in 0..count {
            teams.push(Team::read(&mut reader)?);
        }
        reader.finish()?;
        let file = DosDiyFile {
            header,
            body,
            user_tactics: user_tactics.try_into().expect("six tactics were read"),
            options,
            tail,
            teams,
        };
        file.check()?;
        Ok(file)
    }
}

impl Exporter for DosDiyFormat {
    type Model = DosDiyFile;
    type Error = DosDiyError;

    fn export(model: &DosDiyFile) -> Result<Vec<u8>, DosDiyError> {
        model.check()?;
        let DosDiyFile {
            header,
            body,
            user_tactics,
            options,
            tail,
            teams,
        } = model;
        let mut writer =
            ByteWriter::with_capacity(DOS_REGION_LEN + WORD_LEN + teams.len() * TEAM_RECORD_LEN);
        header.write(&mut writer);
        writer.write_bytes(body);
        for tactics in user_tactics {
            tactics.write(&mut writer);
        }
        let DosMatchOptions {
            game_length,
            auto_replays,
            menu_music,
            auto_save_highlights,
            all_player_teams_equal,
            pitch_type,
        } = options;
        for word in [
            game_length,
            auto_replays,
            menu_music,
            auto_save_highlights,
            all_player_teams_equal,
            pitch_type,
        ] {
            writer.write_u16_le(*word);
        }
        writer.write_bytes(tail);
        writer.write_u16_le(u16::try_from(teams.len()).expect("at most 64 teams"));
        for team in teams {
            team.write(&mut writer);
        }
        Ok(writer.into_bytes())
    }
}

// The user's tactics end where the options start.
const _: () = assert!(TACTICS_OFFSET + USER_TACTICS * TACTICS_RECORD_LEN == OPTIONS_OFFSET);

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // fixed offsets into a file the tests build
mod tests {
    use super::*;

    /// Where the header holds its teams' count (upstream `kNumTeamsOffset`).
    const NUM_TEAMS_OFFSET: usize = 0x31;

    /// A cup of `teams` blank teams, all else zero.
    fn file(teams: u16) -> Vec<u8> {
        let mut bytes = vec![0; DOS_REGION_LEN];
        bytes[0] = 1;
        bytes[4..10].copy_from_slice(b"MY CUP");
        bytes[NUM_TEAMS_OFFSET..NUM_TEAMS_OFFSET + WORD_LEN].copy_from_slice(&teams.to_le_bytes());
        bytes.extend_from_slice(&teams.to_le_bytes());
        bytes.extend(std::iter::repeat_n(0, usize::from(teams) * TEAM_RECORD_LEN));
        bytes
    }

    #[test]
    fn a_blank_cup_round_trips() {
        let bytes = file(2);
        let read = DosDiyFormat::import(&bytes).expect("a DOS .DIY");
        assert_eq!(read.header.competition_type, DosCompetitionType::Cup);
        assert_eq!(read.header.name.text(), "MY CUP");
        assert_eq!(read.teams.len(), 2);
        assert_eq!(DosDiyFormat::export(&read).ok(), Some(bytes));
    }

    #[test]
    fn what_is_not_a_dos_diy_does_not_read() {
        assert!(matches!(
            DosDiyFormat::import(b"{\"schema_version\":3}"),
            Err(DosDiyError::Json)
        ));
        let mut wrong_type = file(2);
        wrong_type[0] = 3;
        assert!(matches!(
            DosDiyFormat::import(&wrong_type),
            Err(DosDiyError::CompetitionType(InvalidCompetitionType(3)))
        ));
        assert!(matches!(
            DosDiyFormat::import(&file(0)),
            Err(DosDiyError::TeamCount(0))
        ));
        let mut disagreeing = file(2);
        disagreeing[NUM_TEAMS_OFFSET] = 3;
        assert!(matches!(
            DosDiyFormat::import(&disagreeing),
            Err(DosDiyError::HeaderTeams {
                stated: 3,
                teams: 2
            })
        ));
        let mut long = file(2);
        long.push(0);
        assert!(matches!(
            DosDiyFormat::import(&long),
            Err(DosDiyError::Read(_))
        ));
        let short = &file(2)[..DOS_REGION_LEN];
        assert!(matches!(
            DosDiyFormat::import(short),
            Err(DosDiyError::Read(_))
        ));
    }

    #[test]
    fn writing_checks_the_counts_and_parts() {
        let mut read = DosDiyFormat::import(&file(2)).expect("a DOS .DIY");
        read.teams.pop();
        assert!(matches!(
            DosDiyFormat::export(&read),
            Err(DosDiyError::HeaderTeams {
                stated: 2,
                teams: 1
            })
        ));
        read.header.num_teams = 1;
        read.tail.pop();
        assert!(matches!(
            DosDiyFormat::export(&read),
            Err(DosDiyError::PartLength { part: "tail", .. })
        ));
    }
}
