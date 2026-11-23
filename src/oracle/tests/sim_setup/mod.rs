//! What the simulation's checks share (`sim.rs`, `sim_status.rs`): the team records' and the
//! tactics records' layouts, random teams (a selected team's record or the career club's),
//! tactics and status rules, and the core's view of them. Each check uses a part of it.
#![allow(dead_code, reason = "each check uses a part of it")]

use openswos_common::ByteReader;
use openswos_core::competition::running::{IMAGE_LEN, RunningCompetition};
use openswos_core::competition::start::LoadedTeam;
use openswos_core::sim::tables::SPOT_AREAS;
use openswos_core::sim::{
    CAREER_RECORD_LEN, CareerLineUp, GameType, MatchTeam, PLAYER_LEN, RECORD_LEN, StatusRules,
};
use openswos_oracle::cup::SelectedTeam;
use openswos_oracle::draw::Prices;
use openswos_oracle::sim::{CAREER_LINE_UP, Status, TACTICS_LEN, TACTICS_SLOTS};
use openswos_tac_resource::TeamTactics;
use openswos_team_resource::TeamControls;
use proptest::collection::vec;
use proptest::prelude::*;

/// The offset of a team's tactics byte.
pub(crate) const TACTICS: usize = 24;

/// The offset of a team's controls.
pub(crate) const CONTROLS: usize = 4;

/// The offset of a team's values' flag.
pub(crate) const VALUES_DIRTY: usize = 59;

/// The offset of a team's line-up.
pub(crate) const LINE_UP: usize = 60;

/// The places of a selected team's line-up.
pub(crate) const LINE_UP_LEN: usize = 16;

/// The offset of the first player's record.
pub(crate) const PLAYERS: usize = 76;

/// The offset of a player's price in his record.
pub(crate) const PRICE: usize = 32;

/// The offset of a player's skills in his record, and their bytes.
pub(crate) const SKILLS: std::ops::Range<usize> = 28..32;

/// A skill byte of two top skills (each nibble's three bits all set).
pub(crate) const TOP_SKILLS: u8 = 0x77;

/// The offset of a tactics record's ball-out-of-play slot.
pub(crate) const BALL_OUT_OF_PLAY: usize = 369;

/// The offset of a tactics record's spots, and their bytes.
pub(crate) const SPOTS: std::ops::Range<usize> = 9..359;

/// A spot: within the pitch grid mostly (a high nibble below 15), past it now and then.
pub(crate) fn spot() -> impl Strategy<Value = u8> {
    prop_oneof![
        49 => (0..=14u8, 0..=15u8).prop_map(|(row, column)| row << 4 | column),
        1 => any::<u8>(),
    ]
}

/// A tactics record: any name and pairs, spots as [`spot`], a slot out of play.
pub(crate) fn tactics_record() -> impl Strategy<Value = [u8; TACTICS_LEN]> {
    (
        vec(any::<u8>(), TACTICS_LEN),
        vec(spot(), SPOTS.len()),
        0..=17u8,
    )
        .prop_map(|(bytes, spots, out_of_play)| {
            let mut record: [u8; TACTICS_LEN] = bytes.try_into().expect("a record's bytes");
            record[SPOTS].copy_from_slice(&spots);
            record[BALL_OUT_OF_PLAY] = out_of_play;
            record
        })
}

/// The ball's areas.
pub(crate) const AREAS: usize = 35;

/// A spot within area `area` of the pitch grid.
pub(crate) fn spot_in(area: usize) -> u8 {
    let (row, column) = SPOT_AREAS
        .iter()
        .enumerate()
        .find_map(|(row, spots)| {
            spots
                .iter()
                .position(|&spot_area| usize::from(spot_area) == area)
                .map(|column| (row, column))
        })
        .expect("every area has spots");
    u8::try_from(row << 4 | column).expect("a spot")
}

/// A tactics record whose players all follow the ball: each area's spot within it (so the
/// players' values reach the top step).
pub(crate) fn following_record() -> impl Strategy<Value = [u8; TACTICS_LEN]> {
    tactics_record().prop_map(|mut record| {
        for (index, spot) in record[SPOTS].iter_mut().enumerate() {
            *spot = spot_in(index % AREAS);
        }
        record
    })
}

/// A session's tactics: a record a slot, a time in four the players following the ball.
pub(crate) fn tactics() -> impl Strategy<Value = Vec<[u8; TACTICS_LEN]>> {
    vec(
        prop_oneof![3 => tactics_record(), 1 => following_record()],
        TACTICS_SLOTS,
    )
}

/// The core's tactics of `records`.
pub(crate) fn core_tactics(records: &[[u8; TACTICS_LEN]]) -> Vec<TeamTactics> {
    records
        .iter()
        .map(|record| TeamTactics::read(&mut ByteReader::new(record)).expect("a tactics record"))
        .collect()
}

/// Status rules: the modes a competition gives (0, 1) and any, a training game half the time,
/// the testers' rule now and then.
pub(crate) fn status() -> impl Strategy<Value = Status> {
    (
        prop_oneof![Just(0u16), Just(1u16), Just(u16::MAX), any::<u16>()],
        any::<bool>(),
        prop_oneof![9 => Just(false), 1 => Just(true)],
    )
        .prop_map(|(mode, training, testers_rule)| Status {
            mode,
            training,
            testers_rule,
        })
}

/// The core's rules of `status`.
pub(crate) fn core_status(status: Status) -> StatusRules {
    StatusRules {
        mode: status.mode.cast_signed(),
        training: status.training,
        no_injuries_cards: status.testers_rule,
    }
}

/// A team record of `len` bytes: any, its tactics a slot of the table mostly, its controls one
/// of upstream's mostly, its values' flag 0 or 1, its line-up's places records of the
/// `players` it holds mostly, its players' prices steps mostly, a time in four every player
/// of top skills (so the prices reach the top step).
pub(crate) fn record(len: usize, players: u8) -> impl Strategy<Value = Vec<u8>> {
    (
        vec(any::<u8>(), len),
        prop_oneof![9 => 0..=18u8, 1 => any::<u8>()],
        prop_oneof![9 => 0..=3u8, 1 => any::<u8>()],
        0..=1u8,
        vec(prop_oneof![19 => 0..players, 1 => any::<u8>()], LINE_UP_LEN),
        vec(
            prop_oneof![9 => 0..=49u8, 1 => any::<u8>()],
            usize::from(players),
        ),
        prop_oneof![3 => Just(false), 1 => Just(true)],
    )
        .prop_map(
            move |(mut bytes, tactics, controls, dirty, line_up, prices, top)| {
                bytes[TACTICS] = tactics;
                bytes[CONTROLS] = controls;
                bytes[VALUES_DIRTY] = dirty;
                bytes[LINE_UP..LINE_UP + LINE_UP_LEN].copy_from_slice(&line_up);
                for (slot, price) in prices.into_iter().enumerate() {
                    let player = PLAYERS + slot * PLAYER_LEN;
                    bytes[player + PRICE] = price;
                    if top {
                        for skill in SKILLS {
                            bytes[player + skill] = TOP_SKILLS;
                        }
                    }
                }
                bytes
            },
        )
}

/// A team in a match: a selected team's record, or the career club's with its line-up (its
/// places records of its 32 mostly).
pub(crate) fn team() -> impl Strategy<Value = (Vec<u8>, Option<([u8; CAREER_LINE_UP], u16)>)> {
    let career_places = u8::try_from(CAREER_LINE_UP).expect("32");
    prop_oneof![
        3 => record(RECORD_LEN, 16).prop_map(|record| (record, None)),
        1 => (
            record(CAREER_RECORD_LEN, career_places),
            vec(prop_oneof![19 => 0..career_places, 1 => any::<u8>()], CAREER_LINE_UP),
            any::<u16>(),
        )
            .prop_map(|(record, line_up, players)| {
                (record, Some((line_up.try_into().expect("32 places"), players)))
            }),
    ]
}

/// The core's team of `record` and `career`.
pub(crate) fn core_team(record: &[u8], career: Option<([u8; CAREER_LINE_UP], u16)>) -> MatchTeam {
    match career {
        Some((order, players)) => MatchTeam::career(
            record.try_into().expect("the career club's record"),
            CareerLineUp { order, players },
        ),
        None => MatchTeam::selected(record.try_into().expect("a team's record")),
    }
}

/// The selected teams the stubbed loader gives, by ordinal.
pub(crate) const SELECTED_TEAMS: usize = 160;

/// The country of the career club's team numbers in the stubbed loader.
pub(crate) const CAREER_COUNTRY: u8 = 0xFF;

/// The most teams a league of these checks holds.
pub(crate) const LEAGUE_TEAMS: usize = 20;

/// The bytes of a standings row.
pub(crate) const ROW_BYTES: u16 = 18;

/// The offset of a team record's country and ordinal.
pub(crate) const TEAM_ID: std::ops::Range<usize> = 0..2;

/// Upstream's halt for a team not in its league.
pub(crate) const NOT_IN_LEAGUE: u16 = 0x6006;

/// The offset of a player's cards and injuries in his record.
pub(crate) const CARDS_INJURIES: usize = 27;

/// The offset of a player's position in his record.
pub(crate) const POSITION: usize = 26;

/// A position byte's looks and flags (its position, bits 7-5, cleared: a goalkeeper).
pub(crate) const NOT_POSITION: u8 = 0x1F;

/// The eleven's last place, the first a match's buckets look at.
pub(crate) const LAST_OF_ELEVEN: usize = 10;

/// A status that keeps a player out: a serious bucket, a serious injury, a card.
pub(crate) const OUT: [u8; 3] = [0x40, 0x08, 0x01];

/// The minor status bucket.
pub(crate) const MINOR_BUCKET: u8 = 0x20;

/// A squad's player's status when it is not any: fit mostly (so the budget of statuses less
/// the players out is not spent), the minor bucket, or out.
pub(crate) fn squad_status() -> impl Strategy<Value = u8> {
    prop_oneof![
        28 => Just(0u8),
        4 => Just(MINOR_BUCKET),
        3 => proptest::sample::select(&OUT[..]),
    ]
}

/// The values both generators are pinned to now and then: at the odds' edges (the keeper's 2,
/// a fit player's 5, a minor bucket's 64), both sides of a coin.
pub(crate) const PINNED: [u8; 9] = [0, 1, 2, 3, 4, 5, 62, 63, 64];

/// The loader the stub is, over the selected teams' and the career club's prices.
pub(crate) fn loader<'a>(
    teams: &'a [SelectedTeam],
    career_team: &'a SelectedTeam,
) -> impl Fn([u8; 2]) -> Option<LoadedTeam> + 'a {
    move |[country, ordinal]| {
        let team = if country == CAREER_COUNTRY {
            career_team
        } else {
            &teams[usize::from(ordinal) % SELECTED_TEAMS]
        };
        Some(LoadedTeam {
            identity: 0,
            controls: TeamControls::Computer,
            prices: team.prices,
        })
    }
}

/// A selected team: any prices, one of upstream's controls.
pub(crate) fn selected() -> impl Strategy<Value = SelectedTeam> {
    (any::<Prices>(), 0..=3u8).prop_map(|(prices, controls)| SelectedTeam { prices, controls })
}

/// A league under way: any image, a league's type mostly, its first row's games 0 to 20, 1 to
/// 20 teams mostly (selected teams, or the career club), each place its team's row mostly.
pub(crate) fn league() -> impl Strategy<Value = ([u8; IMAGE_LEN], Vec<[u8; 2]>)> {
    (
        vec(any::<u8>(), IMAGE_LEN),
        prop_oneof![9 => Just(0i16), 1 => any::<i16>()],
        0..=20u16,
        prop_oneof![19 => 1..=20i16, 1 => any::<i16>()],
        vec(
            (
                prop_oneof![9 => 0..=3u8, 1 => Just(CAREER_COUNTRY)],
                any::<u8>(),
            ),
            LEAGUE_TEAMS,
        ),
        vec(prop_oneof![19 => 0..20u16, 1 => any::<u16>()], LEAGUE_TEAMS),
    )
        .prop_map(|(bytes, kind, games, teams, ids, rows)| {
            let image: [u8; IMAGE_LEN] = bytes.try_into().expect("the image's bytes");
            let mut competition = RunningCompetition::from_image(&image);
            competition.set_competition_type(kind);
            competition.set_num_teams(teams);
            let ids: Vec<[u8; 2]> = ids
                .into_iter()
                .map(|(country, ordinal)| [country, ordinal])
                .collect();
            for (index, (&id, &row)) in ids.iter().zip(&rows).enumerate() {
                competition.set_league_team(index, id);
                let entry = if row < 20 { row * ROW_BYTES } else { row };
                competition.set_team_entry_offset(index, entry);
            }
            let mut first = competition.row(0);
            first.line.played = games;
            competition.set_row(0, first);
            (*competition.image(), ids)
        })
}

/// A DIY competition's `_gameType`.
pub(crate) const DIY_GAME: u16 = 1;

/// A season's `_gameType`.
pub(crate) const SEASON_GAME: u16 = 3;

/// A career's `_gameType`.
pub(crate) const CAREER_GAME: u16 = 4;

/// A kind of game: a competition's, a season's, a career's.
pub(crate) fn game_type() -> impl Strategy<Value = GameType> {
    prop_oneof![
        Just(GameType::Competition),
        Just(GameType::Season),
        Just(GameType::Career),
    ]
}

/// `game`'s `_gameType` (a competition's a DIY one's).
pub(crate) fn game_type_code(game: GameType) -> u16 {
    match game {
        GameType::Competition => DIY_GAME,
        GameType::Season => SEASON_GAME,
        GameType::Career => CAREER_GAME,
    }
}

/// The kind of game of `_gameType` `code` (1 to 4).
pub(crate) fn game_type_of(code: u16) -> GameType {
    match code {
        SEASON_GAME => GameType::Season,
        CAREER_GAME => GameType::Career,
        _ => GameType::Competition,
    }
}
