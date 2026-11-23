//! What a season end's knockouts read beside the tables and keep for the next season (upstream
//! src/rules/swosrules.h `sr_stage_bank`, `sr_promedio`, `sr_promedio_seed`, `sr_winners`,
//! `sr_result`, `sr_host`, `sr_season`; src/rules/sr_check.c `sr_season_check`): the league's
//! stage tables, the promedio's history, the SEASON WINNERS page, what the career club played
//! itself.
//!
//! Copyright (c) 2026 Davide Lorigliola, Ben Baker. MIT licence (`LICENSE-MIT` beside this
//! module).

use super::check::{CheckCode, CheckError};
use super::table::{CLUBS, DIVISIONS, Levels, Row};

/// The stage tables a bank keeps (`SR_STAGE_TABLES`).
pub const STAGE_TABLES: usize = 4;

/// The clubs a promedio's history holds (`SR_PROMEDIO_CLUBS`).
pub const PROMEDIO_CLUBS: usize = 48;

/// The entries the SEASON WINNERS page holds (`SR_WINNERS_MAX`).
pub const WINNERS: usize = 40;

/// A league's stage tables (`sr_stage_bank`, gplay.c's `GP_SS`), kept by the caller through the
/// season and saved with it: table `k` a fixture stage's (written this season when `fresh`'s bit
/// `k` is set), and a 'keep' stage's frozen table.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StageBank {
    /// The caller's: whether it is in use.
    pub used: u8,
    /// The caller's: its key (the country).
    pub key: u8,
    /// Bit `k`: table `k` is this season's.
    pub fresh: u8,
    /// Each table's clubs.
    pub counts: [u8; STAGE_TABLES],
    /// Each table's rows.
    pub tables: [[Row; CLUBS]; STAGE_TABLES],
    /// The kept table's division.
    pub kept_division: u8,
    /// The kept table's clubs (0: none).
    pub kept_count: u8,
    /// The kept table's rows.
    pub kept: [Row; CLUBS],
}

impl Default for StageBank {
    fn default() -> Self {
        Self {
            used: 0,
            key: 0,
            fresh: 0,
            counts: [0; STAGE_TABLES],
            tables: [[Row::default(); CLUBS]; STAGE_TABLES],
            kept_division: 0,
            kept_count: 0,
            kept: [Row::default(); CLUBS],
        }
    }
}

/// A club's promedio history (`sr_promedio_club`): its points and matches of the two seasons
/// before, the older first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PromedioClub {
    /// Its global number.
    pub glob: u16,
    /// Its points, two seasons ago and last season.
    pub points: [u16; 2],
    /// Its matches, likewise.
    pub matches: [u8; 2],
}

/// The promedio's history (`sr_promedio`, gplay.c's `PR_STATE`): each top-flight club's
/// points and matches of the two seasons before; the caller's, saved.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Promedio {
    /// Whether it has been seeded.
    pub init: bool,
    /// Its clubs.
    pub count: u8,
    /// Each club's history.
    pub clubs: [PromedioClub; PROMEDIO_CLUBS],
}

impl Default for Promedio {
    fn default() -> Self {
        Self {
            init: false,
            count: 0,
            clubs: [PromedioClub::default(); PROMEDIO_CLUBS],
        }
    }
}

/// A club's promedio at a new career (`sr_promedio_seed`, gplay.c's `GP_PROM_TAB`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PromedioSeed {
    /// Its global number.
    pub glob: u16,
    /// Its points two seasons ago.
    pub points_two: u16,
    /// Its points last season.
    pub points_one: u16,
    /// Its matches two seasons ago.
    pub matches_two: u8,
    /// Its matches last season.
    pub matches_one: u8,
}

/// An entry of the SEASON WINNERS page (`sr_winners_entry`): its type (`M` the division the
/// career club played in, `L` a division's 1st, `K` a play-off's winner, 0x80 | play-off a
/// title play-off's last ties' winners), its index, the team word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WinnersEntry {
    /// Its type.
    pub kind: u8,
    /// Its index.
    pub index: u8,
    /// The team word.
    pub team: u16,
}

/// The SEASON WINNERS page's last season (`sr_winners`, gplay.c's `WN_ST`), written for the
/// career club's league.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Winners {
    /// The season.
    pub season: u8,
    /// Its entries.
    pub count: u8,
    /// Written this season.
    pub fresh: u8,
    /// The entries.
    pub entries: [WinnersEntry; WINNERS],
}

impl Default for Winners {
    fn default() -> Self {
        Self {
            season: 0,
            count: 0,
            fresh: 0,
            entries: [WinnersEntry::default(); WINNERS],
        }
    }
}

/// What a result the knockouts decide is (`SR_KIND_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultKind {
    /// A title's champion and runner-up.
    Title,
    /// A continental play-off's winner and loser.
    Continental,
    /// A stage title's champion and runner-up.
    Stage,
}

/// A result's kind codes (`SR_KIND_TITLE`, `SR_KIND_CONTINENTAL`, `SR_KIND_STAGE`).
const TITLE: u8 = 5;
/// See [`TITLE`].
const CONTINENTAL: u8 = 6;
/// See [`TITLE`].
const STAGE: u8 = 8;

impl TryFrom<u8> for ResultKind {
    type Error = u8;

    fn try_from(code: u8) -> Result<Self, u8> {
        match code {
            TITLE => Ok(Self::Title),
            CONTINENTAL => Ok(Self::Continental),
            STAGE => Ok(Self::Stage),
            other => Err(other),
        }
    }
}

impl From<ResultKind> for u8 {
    fn from(kind: ResultKind) -> Self {
        match kind {
            ResultKind::Title => TITLE,
            ResultKind::Continental => CONTINENTAL,
            ResultKind::Stage => STAGE,
        }
    }
}

/// A result the knockouts decide beside the moves (`sr_result`): its kind (none: not decided),
/// its division, its two clubs' team words and global numbers (65535 none).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnockoutResult {
    /// Its kind.
    pub kind: Option<ResultKind>,
    /// The division.
    pub division: u8,
    /// The champion's (winner's) team word, then the runner-up's (loser's).
    pub team: [u16; 2],
    /// Their global numbers.
    pub glob: [u16; 2],
}

/// No club of a result: a team word or a global number.
pub const NO_CLUB: u16 = 0xFFFF;

impl KnockoutResult {
    /// A result not decided.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            kind: None,
            division: 0,
            team: [NO_CLUB; 2],
            glob: [NO_CLUB; 2],
        }
    }
}

impl Default for KnockoutResult {
    /// Zeroed, as the library's state begins.
    fn default() -> Self {
        Self {
            kind: None,
            division: 0,
            team: [0; 2],
            glob: [0; 2],
        }
    }
}

/// What the game around the knockouts knows the career club played itself (`sr_host`): each
/// method answers none when it played no such thing.
pub trait KnockoutHost {
    /// Play-off `play_off`'s tie of `a` v `b` (team words), played: its winner.
    fn played_tie(&mut self, play_off: i32, a: u16, b: u16) -> Option<u16>;

    /// Title play-off `play_off`, played as a group contest: its champion and runner-up.
    fn played_title(&mut self, play_off: i32) -> Option<(u16, u16)>;

    /// The career club's division's stage title `stage` (0 to 2), decided mid-season: its
    /// champion and runner-up's team words and global numbers.
    fn stage_title(&mut self, stage: i32) -> Option<([u16; 2], [u16; 2])>;
}

/// A tie the career club played (`sr_season`'s `tie_a`, `tie_b`, `tie_w`: gplay.c's `pl_w`): its
/// clubs' team words and its winner's (0: none played).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PlayedTie {
    /// One club.
    pub a: u16,
    /// The other.
    pub b: u16,
    /// The winner, 0 none.
    pub winner: u16,
}

/// A season end's knockouts' season (`sr_season`): the career club, its played tie, the league's
/// stage tables, its promedio history, the SEASON WINNERS page, the host, the season-end
/// preview, the second teams in the knockouts.
pub struct KnockoutSeason<'a> {
    /// The career club's team word (`mgr`), 0 none.
    pub manager: u16,
    /// A group tie it played.
    pub played: PlayedTie,
    /// The league's stage tables, none for none.
    pub bank: Option<&'a mut StageBank>,
    /// The promedio's history, none for none.
    pub promedio: Option<&'a mut Promedio>,
    /// The SEASON WINNERS page, none for not kept.
    pub winners: Option<&'a mut Winners>,
    /// The game around, none for none.
    pub host: Option<&'a mut dyn KnockoutHost>,
    /// The season-end preview (gplay.c `GP_PRE`): this season's titles decided at the career
    /// club's league end, no promedio, the stage tables kept, the SEASON WINNERS page appended.
    pub preview: bool,
    /// The career club's own play-offs in the preview, each its index plus 1 (0 none).
    pub own_play_offs: [u8; 2],
    /// Second teams passed over as entrants by place when they may not go up (`b_in_ko`).
    pub second_teams_in_knockouts: bool,
    /// The league's first global number (a second team's is it plus its ordinal).
    pub base: u16,
    /// The league's levels, none a level per division.
    pub levels: Option<&'a Levels>,
}

impl std::fmt::Debug for KnockoutSeason<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnockoutSeason")
            .field("manager", &self.manager)
            .field("played", &self.played)
            .field("bank", &self.bank)
            .field("promedio", &self.promedio)
            .field("winners", &self.winners)
            .field("host", &self.host.is_some())
            .field("preview", &self.preview)
            .field("own_play_offs", &self.own_play_offs)
            .field("second_teams_in_knockouts", &self.second_teams_in_knockouts)
            .field("base", &self.base)
            .field("levels", &self.levels)
            .finish()
    }
}

/// A fault at element `offset`, outside a play-off.
fn fault(code: CheckCode, offset: usize, field: &'static str) -> CheckError {
    CheckError {
        code,
        play_off: None,
        offset: u32::try_from(offset).expect("a few"),
        field,
    }
}

/// A season's saved state checked (upstream `sr_season_check`): the stage tables of at most 32
/// clubs, the kept one's division within the 16; the promedio's at most 48 clubs; the SEASON
/// WINNERS page's at most 40 entries; at most 16 levels.
///
/// # Errors
///
/// The first fault, a [`CheckError`].
pub fn season_check(season: &KnockoutSeason<'_>) -> Result<(), CheckError> {
    if let Some(bank) = season.bank.as_deref() {
        if let Some(table) = bank
            .counts
            .iter()
            .position(|&count| usize::from(count) > CLUBS)
        {
            return Err(fault(
                CheckCode::Count,
                table,
                "stage table's clubs (at most 32)",
            ));
        }
        if usize::from(bank.kept_count) > CLUBS {
            return Err(fault(
                CheckCode::Count,
                0,
                "kept table's clubs (at most 32)",
            ));
        }
        if bank.kept_count != 0 && usize::from(bank.kept_division) >= DIVISIONS {
            return Err(fault(CheckCode::Index, 0, "kept table's division"));
        }
    }
    if season
        .promedio
        .as_deref()
        .is_some_and(|promedio| usize::from(promedio.count) > PROMEDIO_CLUBS)
    {
        return Err(fault(CheckCode::Count, 0, "promedio's clubs (at most 48)"));
    }
    if season
        .winners
        .as_deref()
        .is_some_and(|winners| usize::from(winners.count) > WINNERS)
    {
        return Err(fault(
            CheckCode::Count,
            0,
            "SEASON WINNERS entries (at most 40)",
        ));
    }
    if season
        .levels
        .is_some_and(|levels| levels.first.len() > DIVISIONS)
    {
        return Err(fault(CheckCode::Count, 0, "levels"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A season of nothing but `bank`.
    fn season(bank: Option<&mut StageBank>) -> KnockoutSeason<'_> {
        KnockoutSeason {
            manager: 0,
            played: PlayedTie::default(),
            bank,
            promedio: None,
            winners: None,
            host: None,
            preview: false,
            own_play_offs: [0; 2],
            second_teams_in_knockouts: false,
            base: 0,
            levels: None,
        }
    }

    #[test]
    fn a_damaged_state_is_refused() {
        assert_eq!(season_check(&season(None)), Ok(()));
        let mut bank = StageBank::default();
        bank.counts[2] = 33;
        assert_eq!(
            season_check(&season(Some(&mut bank))).map_err(|error| (error.code, error.offset)),
            Err((CheckCode::Count, 2))
        );
        let mut bank = StageBank {
            kept_count: 1,
            kept_division: 16,
            ..StageBank::default()
        };
        assert_eq!(
            season_check(&season(Some(&mut bank))).map_err(|error| error.code),
            Err(CheckCode::Index)
        );
        let mut winners = Winners {
            count: 41,
            ..Winners::default()
        };
        let mut many = season(None);
        many.winners = Some(&mut winners);
        assert_eq!(
            season_check(&many).map_err(|error| error.code),
            Err(CheckCode::Count)
        );
        assert_eq!(KnockoutResult::none().team, [NO_CLUB; 2]);
        assert!(format!("{:?}", season(None)).contains("host: false"));
    }

    #[test]
    fn a_result_s_kind_round_trips_its_code() {
        for kind in [
            ResultKind::Title,
            ResultKind::Continental,
            ResultKind::Stage,
        ] {
            assert_eq!(ResultKind::try_from(u8::from(kind)), Ok(kind));
        }
        assert_eq!(u8::from(ResultKind::Stage), STAGE);
        assert_eq!(ResultKind::try_from(PROMEDIO_KIND), Err(PROMEDIO_KIND));
    }

    /// The promedio's play-off kind: no result's.
    const PROMEDIO_KIND: u8 = 7;
}
